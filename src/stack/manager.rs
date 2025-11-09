use anyhow::{Context, Result};
use log::{error, info};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{info_span, instrument, Instrument};

use crate::controller::Controller;
use crate::db::{Stack, StateManager};
use super::git::{GitSource, GitSourceConfig};
use super::source::StackSource;

type StackId = i32;
type TaskHandle = JoinHandle<()>;

pub struct StackManager {
    state: StateManager,
    controller: Arc<Controller>,
    tasks: Arc<RwLock<HashMap<StackId, TaskHandle>>>,
}

impl StackManager {
    pub fn new(state: StateManager, controller: Arc<Controller>) -> Self {
        Self {
            state,
            controller,
            tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn start(&self) -> Result<()> {
        info!("Starting Stack Manager");
        
        let stacks = self.state.list_stacks().await
            .context("Failed to load stacks from database")?;
        
        info!("Found {} stacks to manage", stacks.len());
        
        for stack in stacks {
            if let Some(interval) = stack.reconcile_interval {
                if interval > 0 {
                    self.spawn_reconciliation_task(stack).await?;
                }
            }
        }
        
        Ok(())
    }
    
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping Stack Manager");
        let mut tasks = self.tasks.write().await;
        
        for (stack_id, handle) in tasks.drain() {
            info!("Stopping reconciliation task for stack {}", stack_id);
            handle.abort();
        }
        
        Ok(())
    }
    
    pub async fn register_stack(&self, stack: Stack) -> Result<()> {
        if let Some(interval) = stack.reconcile_interval {
            if interval > 0 {
                self.spawn_reconciliation_task(stack).await?;
            }
        }
        Ok(())
    }
    
    pub async fn unregister_stack(&self, stack_id: i32) -> Result<()> {
        let mut tasks = self.tasks.write().await;
        
        if let Some(handle) = tasks.remove(&stack_id) {
            info!("Stopping reconciliation task for stack {}", stack_id);
            handle.abort();
        }
        
        Ok(())
    }
    
    #[instrument(
        skip(self, stack), 
        fields(
            stack.name = %stack.name, 
            stack.id = ?stack.id,
            stack.source_type = %stack.source_type,
            reconcile_interval_sec = ?stack.reconcile_interval
        )
    )]
    async fn spawn_reconciliation_task(&self, stack: Stack) -> Result<()> {
        let stack_id = stack.id.context("Stack missing ID")?;
        let interval = Duration::from_secs(stack.reconcile_interval.unwrap_or(60) as u64);
        
        info!(
            "Spawning reconciliation task for stack '{}' with interval {:?}",
            stack.name, interval
        );
        
        let state = self.state.clone();
        let controller = self.controller.clone();
        let stack_clone = stack.clone();
        
        let stack_name = stack_clone.name.clone();
        let stack_source_type = stack_clone.source_type.clone();
        let handle = tokio::spawn(async move {
            Self::reconciliation_loop(state, controller, stack_clone, interval)
                .instrument(info_span!(
                    "stack_reconciliation", 
                    stack.name = %stack_name,
                    stack.source_type = %stack_source_type,
                    reconcile.cycles = 0_u64,
                ))
                .await
        });
        
        self.tasks.write().await.insert(stack_id, handle);
        
        Ok(())
    }
    
    #[instrument(
        skip(state, controller, stack), 
        fields(
            stack.name = %stack.name,
            stack.source_type = %stack.source_type
        )
    )]
    async fn reconciliation_loop(
        state: StateManager,
        controller: Arc<Controller>,
        stack: Stack,
        interval: Duration,
    ) {
        info!("Reconciliation loop started");
        let span = tracing::Span::current();
        let mut cycle_count = 0_u64;
        
        let source = match Self::create_source(&stack) {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to create source for stack '{}': {}", stack.name, e);
                if let Err(e) = state.update_stack_status(&stack.name, "error").await {
                    error!("Failed to update stack status: {}", e);
                }
                return;
            }
        };
        
        if let Err(e) = source.init().await {
            error!("Failed to initialize source: {}", e);
            if let Err(e) = state.update_stack_status(&stack.name, "error").await {
                error!("Failed to update stack status: {}", e);
            }
            return;
        }
        
        loop {
            cycle_count += 1;
            span.record("reconcile.cycles", cycle_count);
            
            if let Err(e) = Self::reconcile_once(&state, &controller, &stack, &source).await {
                error!("Reconciliation failed: {}", e);
                if let Err(e) = state.update_stack_status(&stack.name, "error").await {
                    error!("Failed to update stack status: {}", e);
                }
            }
            
            tokio::time::sleep(interval).await;
        }
    }
    
    #[instrument(
        skip(state, controller, source), 
        fields(
            stack.name = %stack.name,
            stack.source_type = %stack.source_type,
            stack.id = ?stack.id,
            reconcile.result = tracing::field::Empty,
            reconcile.version = tracing::field::Empty,
            reconcile.duration_ms = tracing::field::Empty,
            reconcile.has_updates = tracing::field::Empty,
        )
    )]
    async fn reconcile_once(
        state: &StateManager,
        controller: &Arc<Controller>,
        stack: &Stack,
        source: &Box<dyn StackSource>,
    ) -> Result<()> {
        let start = Instant::now();
        let span = tracing::Span::current();
        
        info!("Checking for updates");
        
        if let Err(e) = source.fetch().await {
            error!("Failed to fetch from source: {}", e);
            span.record("reconcile.result", "fetch_failed");
            span.record("reconcile.duration_ms", start.elapsed().as_millis() as i64);
            return Err(e);
        }
        
        let current_version = source.get_version().await
            .context("Failed to get version")?;
        
        let last_version = stack.last_sync_version.as_deref().unwrap_or("");
        
        if current_version == last_version {
            info!("No updates detected (version: {})", current_version);
            span.record("reconcile.result", "no_updates");
            span.record("reconcile.version", current_version.as_str());
            span.record("reconcile.has_updates", false);
            span.record("reconcile.duration_ms", start.elapsed().as_millis() as i64);
            return Ok(());
        }
        
        info!(
            "Update detected: {} -> {}",
            last_version.chars().take(8).collect::<String>(),
            current_version.chars().take(8).collect::<String>()
        );
        span.record("reconcile.has_updates", true);
        
        state.update_stack_status(&stack.name, "syncing").await
            .context("Failed to update stack status to syncing")?;
        
        let config_path = source.get_config_path().await
            .context("Failed to get config path")?;
        
        let config_path_str = config_path.to_str()
            .context("Config path is not valid UTF-8")?;
        
        info!("Reconciling from config: {}", config_path_str);
        
        match controller.reconcile_from_nickel_with_variables(config_path_str, &stack.name).await {
            Ok(_) => {
                state.update_stack_sync(&stack.name, &current_version, "synced").await
                    .context("Failed to update stack sync status")?;
                
                info!("Reconciliation complete, updated to version {}", 
                      current_version.chars().take(8).collect::<String>());
                
                span.record("reconcile.result", "success");
                span.record("reconcile.version", current_version.as_str());
                span.record("reconcile.duration_ms", start.elapsed().as_millis() as i64);
                
                Ok(())
            },
            Err(e) => {
                error!("Reconciliation failed: {}", e);
                span.record("reconcile.result", "reconcile_failed");
                span.record("reconcile.duration_ms", start.elapsed().as_millis() as i64);
                Err(e).context("Failed to reconcile from Nickel config")
            }
        }
    }
    
    fn create_source(stack: &Stack) -> Result<Box<dyn StackSource>> {
        match stack.source_type.as_str() {
            "git" => {
                let config: GitSourceConfig = serde_json::from_value(stack.source_config.clone())
                    .context("Failed to parse git source config")?;
                
                let source = GitSource::new(config, stack.config_path.clone());
                
                Ok(Box::new(source))
            }
            _ => Err(anyhow::anyhow!(
                "Unsupported source type: {}",
                stack.source_type
            )),
        }
    }
    
    
    #[instrument(
        skip(self), 
        fields(
            stack.name = %stack_name,
            sync.trigger = "manual"
        )
    )]
    pub async fn sync_stack(&self, stack_name: &str) -> Result<()> {
        info!("Manual sync requested for stack '{}'", stack_name);
        
        let stack = self.state.get_stack_by_name(stack_name).await
            .context("Failed to load stack")?;
        
        let source = Self::create_source(&stack)?;
        source.init().await
            .context("Failed to initialize source for manual sync")?;
        
        Self::reconcile_once(&self.state, &self.controller, &stack, &source).await
            .context("Manual sync failed")?;
        
        Ok(())
    }

    #[instrument(
        skip(self), 
        fields(
            stack.name = %stack_name,
            destroy.trigger = "manual"
        )
    )]
    pub async fn destroy_stack(&self, stack_name: &str) -> Result<()> {
        info!("Manual destroy requested for stack '{}'", stack_name);
        
        let stack = self.state.get_stack_by_name(stack_name).await
            .context("Failed to load stack")?;
        
        let source = Self::create_source(&stack)?;
        source.init().await
            .context("Failed to initialize source for manual destroy")?;
        
        let config_path = source.get_config_path().await?;
        
        info!("Destroying stack from config: {}", config_path.display());
        self.controller.destroy_from_nickel(&config_path.to_string_lossy()).await
            .context("Destroy failed")?;
        
        info!("Stack '{}' destroyed successfully", stack_name);
        Ok(())
    }
}
