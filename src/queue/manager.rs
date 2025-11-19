use anyhow::{Context, Result};
use log::{error, info};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{info_span, instrument, Instrument};

use crate::controller::Controller;
use crate::db::{models::Queue, StateManager};
use super::mqtt::{MqttSource, MqttSourceConfig};
use super::source::QueueSource;

type QueueId = i32;
type TaskHandle = JoinHandle<()>;

pub struct QueueManager {
    state: StateManager,
    controller: Arc<Controller>,
    tasks: Arc<RwLock<HashMap<QueueId, TaskHandle>>>,
}

impl QueueManager {
    pub fn new(state: StateManager, controller: Arc<Controller>) -> Self {
        Self {
            state,
            controller,
            tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    #[instrument(skip(self))]
    pub async fn start(&self) -> Result<()> {
        info!("Starting Queue Manager");
        Ok(())
    }
    
    #[instrument(skip(self))]
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping Queue Manager");
        let mut tasks = self.tasks.write().await;
        
        for (queue_id, handle) in tasks.drain() {
            info!("Stopping consumer task for queue {}", queue_id);
            handle.abort();
        }
        
        Ok(())
    }
    
    #[instrument(skip(self, queue))]
    pub async fn register_queue(&self, queue: Queue) -> Result<()> {
        info!("Registering queue '{}'", queue.name);
        Ok(())
    }
    
    #[instrument(skip(self))]
    pub async fn unregister_queue(&self, queue_id: i32) -> Result<()> {
        info!("Unregistering queue {}", queue_id);
        let mut tasks = self.tasks.write().await;
        
        if let Some(handle) = tasks.remove(&queue_id) {
            info!("Stopping consumer task for queue {}", queue_id);
            handle.abort();
        }
        
        Ok(())
    }
    
    #[instrument(skip(self))]
    pub async fn pause_queue(&self, queue_name: &str) -> Result<()> {
        info!("Pausing queue '{}'", queue_name);
        Ok(())
    }
    
    #[instrument(skip(self))]
    pub async fn resume_queue(&self, queue_name: &str) -> Result<()> {
        info!("Resuming queue '{}'", queue_name);
        Ok(())
    }
    
    fn create_source(queue: &Queue) -> Result<Box<dyn QueueSource>> {
        match queue.queue_type.as_str() {
            "mqtt" => {
                let config: MqttSourceConfig = serde_json::from_value(queue.queue_config.clone())
                    .context("Failed to parse MQTT source config")?;
                
                let source = MqttSource::new(config);
                Ok(Box::new(source))
            }
            _ => Err(anyhow::anyhow!(
                "Unsupported queue type: {}",
                queue.queue_type
            )),
        }
    }
}
