use anyhow::{Context, Result, anyhow};
use log::info;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::instrument;

use crate::db::StateManager;
use crate::modules::AutomationModule;
use crate::nickel::NickelEvaluator;
use crate::utils::{DependencyGraph, Duty, Roster, RosterSelector};

#[derive(Clone)]
pub struct Controller {
    state: StateManager,
    modules: Arc<HashMap<String, Arc<dyn AutomationModule>>>,
}

impl Controller {
    pub fn new(state: StateManager) -> Self {
        Self {
            state,
            modules: Arc::new(HashMap::new()),
        }
    }

    pub fn register_module(&mut self, module: Arc<dyn AutomationModule>) {
        let name = module.name().to_string();
        Arc::get_mut(&mut self.modules)
            .expect("Cannot register module after Controller has been cloned")
            .insert(name, module);
    }

    #[instrument(skip(self, duty), fields(duty_name = %duty.name))]
    pub async fn match_roster(&self, duty: &Duty) -> Result<Roster> {
        let selector: RosterSelector = serde_json::from_value(duty.roster_selector.clone())?;
        
        let mut query = vec![];
        
        if let Some(ref traits) = selector.traits {
            let rosters = self.state.find_rosters_by_traits(
                &traits.iter().map(|s| s.as_str()).collect::<Vec<_>>()
            ).await?;
            query = rosters;
        } else {
            query = self.state.list_rosters().await?;
        }
        
        if let Some(ref roster_type) = selector.roster_type {
            query.retain(|r| &r.roster_type == roster_type);
        }
        
        query.into_iter()
            .next()
            .ok_or_else(|| anyhow!("No matching roster found for duty '{}'", duty.name))
    }

    fn select_module(&self, duty: &Duty) -> Result<Arc<dyn AutomationModule>> {
        for module in self.modules.values() {
            if module.supported_duty_types().contains(&duty.duty_type.as_str()) {
                return Ok(module.clone());
            }
        }
        
        Err(anyhow!(
            "No module found supporting duty type '{}'",
            duty.duty_type
        ))
    }

    #[instrument(skip(self))]
    pub async fn reconcile_duty(&self, duty_name: &str) -> Result<()> {
        info!("Reconciling duty: {}", duty_name);
        
        let duty = self.state.get_duty_by_name(duty_name).await?;
        
        let module = self.select_module(&duty)?;
        info!("Selected module: {}", module.name());
        
        let roster = self.match_roster(&duty).await?;
        info!("Matched roster: {}", roster.name);
        
        module.validate(&roster, &duty).await?;
        
        let required_traits = module.required_roster_traits();
        for trait_name in required_traits {
            if !roster.has_trait(trait_name) {
                return Err(anyhow!(
                    "Roster '{}' missing required trait '{}' for module '{}'",
                    roster.name,
                    trait_name,
                    module.name()
                ));
            }
        }
        
        info!("Applying duty '{}'", duty.name);
        let result_json = module.apply(&roster, &duty).await?;
        
        self.state.update_duty_status(&duty.name, result_json).await?;
        self.state.record_duty_execution(&duty.name, "completed").await?;
        
        info!("Reconciliation complete for duty: {}", duty_name);
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn destroy_duty(&self, duty_name: &str) -> Result<()> {
        info!("Destroying duty: {}", duty_name);
        
        let duty = self.state.get_duty_by_name(duty_name).await?;
        let module = self.select_module(&duty)?;
        let roster = self.match_roster(&duty).await?;
        
        module.destroy(&roster, &duty).await?;
        
        self.state.delete_duty(duty_name).await?;
        
        info!("Duty '{}' destroyed", duty_name);
        Ok(())
    }

    #[instrument(skip(self, config_path))]
    pub async fn reconcile_from_nickel(&self, config_path: &str) -> Result<()> {
        info!("Loading initial configuration from: {}", config_path);
        
        let evaluator = NickelEvaluator::new(config_path);
        
        let rosters = evaluator.load_rosters()?;
        info!("Found {} rosters in config", rosters.len());
        
        for roster in rosters {
            info!("Creating/updating roster: {}", roster.name);
            self.state.create_roster(roster).await
                .with_context(|| format!("Failed to create/update roster"))?;
        }
        
        let initial_duties = evaluator.load_duties()?;
        
        for duty in &initial_duties {
            info!("Persisting duty '{}' to database", duty.name);
            self.state.upsert_duty(duty.clone()).await
                .with_context(|| format!("Failed to persist duty '{}' to database", duty.name))?;
        }
        
        let persisted_duties = self.state.list_duties().await
            .context("Failed to reload duties from database")?;
        
        info!("Building dependency graph for {} duties", persisted_duties.len());
        let graph = DependencyGraph::new(persisted_duties);
        let execution_plan = graph.topological_sort()?;
        
        info!("Execution plan: {} batches", execution_plan.len());
        
        let mut runtime_context: HashMap<String, JsonValue> = HashMap::new();
        let mut duties_outputs: HashMap<String, JsonValue> = HashMap::new();
        
        for (batch_idx, batch_names) in execution_plan.iter().enumerate() {
            info!("Executing batch {}/{} with {} duties", 
                  batch_idx + 1, execution_plan.len(), batch_names.len());
            
            runtime_context.insert("duties".to_string(), serde_json::json!(duties_outputs));
            
            let current_duties = if batch_idx == 0 {
                evaluator.load_duties()?
            } else {
                info!("Re-evaluating Nickel config with runtime context");
                evaluator.load_duties_with_runtime_context(&runtime_context)?
            };
            
            for duty_name in batch_names {
                let duty = current_duties.iter()
                    .find(|d| &d.name == duty_name)
                    .ok_or_else(|| anyhow!("Duty '{}' not found after re-evaluation", duty_name))?;
                
                info!("Reconciling duty '{}' in batch {}", duty.name, batch_idx + 1);
                
                let module = self.select_module(duty)?;
                let roster = self.match_roster(duty).await?;
                
                module.validate(&roster, duty).await?;
                
                for trait_name in module.required_roster_traits() {
                    if !roster.has_trait(trait_name) {
                        return Err(anyhow!(
                            "Roster '{}' missing required trait '{}' for module '{}'",
                            roster.name,
                            trait_name,
                            module.name()
                        ));
                    }
                }
                
                let result_json = module.apply(&roster, duty).await?;
                
                self.state.update_duty_status(&duty.name, result_json.clone()).await?;
                self.state.record_duty_execution(&duty.name, "completed").await?;
                
                if let Some(outputs) = result_json.get("outputs") {
                    duties_outputs.insert(duty.name.clone(), serde_json::json!({
                        "outputs": outputs.clone()
                    }));
                    info!("Stored outputs for duty '{}': {:?}", duty.name, outputs);
                }
            }
            
            info!("Batch {}/{} complete. {} duties executed successfully", 
                  batch_idx + 1, execution_plan.len(), batch_names.len());
        }
        
        info!("All {} batches completed successfully", execution_plan.len());
        Ok(())
    }
    
    #[instrument(skip(self, duties))]
    pub async fn reconcile_duties_dag(&self, duties: Vec<Duty>) -> Result<()> {
        info!("Building dependency graph for {} duties", duties.len());
        
        let graph = DependencyGraph::new(duties);
        let execution_plan = graph.get_execution_plan()?;
        
        info!("Execution plan: {} batches", execution_plan.len());
        
        let mut runtime_outputs: HashMap<String, JsonValue> = HashMap::new();
        
        for (batch_idx, batch) in execution_plan.iter().enumerate() {
            info!("Executing batch {}/{} with {} duties", 
                  batch_idx + 1, execution_plan.len(), batch.len());
            
            let mut batch_results = Vec::new();
            
            for duty in batch {
                info!("Reconciling duty '{}' in batch {}", duty.name, batch_idx + 1);
                
                let module = self.select_module(duty)?;
                let roster = self.match_roster(duty).await?;
                
                module.validate(&roster, duty).await?;
                
                for trait_name in module.required_roster_traits() {
                    if !roster.has_trait(trait_name) {
                        return Err(anyhow!(
                            "Roster '{}' missing required trait '{}' for module '{}'",
                            roster.name,
                            trait_name,
                            module.name()
                        ));
                    }
                }
                
                let result_json = module.apply(&roster, duty).await?;
                
                self.state.update_duty_status(&duty.name, result_json.clone()).await?;
                self.state.record_duty_execution(&duty.name, "completed").await?;
                
                if let Some(outputs) = result_json.get("outputs") {
                    runtime_outputs.insert(duty.name.clone(), outputs.clone());
                    info!("Stored outputs for duty '{}'", duty.name);
                }
                
                batch_results.push((duty.name.clone(), result_json));
            }
            
            info!("Batch {}/{} complete. {} duties executed successfully", 
                  batch_idx + 1, execution_plan.len(), batch_results.len());
        }
        
        info!("All {} batches completed successfully", execution_plan.len());
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn destroy_from_nickel(&self, config_path: &str) -> Result<()> {
        info!("Loading configuration from: {}", config_path);
        
        let evaluator = NickelEvaluator::new(config_path);
        let duties = evaluator.load_duties()?;
        
        info!("Found {} duties to destroy", duties.len());
        
        self.destroy_duties_dag(duties).await
    }

    #[instrument(skip(self, duties))]
    pub async fn destroy_duties_dag(&self, duties: Vec<Duty>) -> Result<()> {
        info!("Building dependency graph for {} duties to destroy", duties.len());
        
        let graph = DependencyGraph::new(duties);
        let mut execution_plan = graph.get_execution_plan()?;
        
        execution_plan.reverse();
        
        info!("Destroy plan: {} batches (reversed)", execution_plan.len());
        
        for (batch_idx, batch) in execution_plan.iter().enumerate() {
            info!("Destroying batch {}/{} with {} duties", 
                  batch_idx + 1, execution_plan.len(), batch.len());
            
            for duty in batch {
                info!("Destroying duty '{}'", duty.name);
                
                let module = self.select_module(duty)?;
                let roster = self.match_roster(duty).await?;
                
                match module.destroy(&roster, duty).await {
                    Ok(_) => {
                        info!("Successfully destroyed duty '{}'", duty.name);
                    },
                    Err(e) => {
                        log::error!("Failed to destroy duty '{}': {}", duty.name, e);
                        return Err(e).context(format!("Failed to destroy duty '{}'", duty.name));
                    }
                }
            }
            
            info!("Batch {}/{} destroyed successfully", 
                  batch_idx + 1, execution_plan.len());
        }
        
        info!("All {} batches destroyed successfully", execution_plan.len());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::aws::AwsStaticSiteModule;
    use serde_json::json;
    
    async fn setup_test_state() -> StateManager {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://g8r:g8r_dev_password@localhost:5432/g8r_state".to_string());
        
        let state = StateManager::new(&database_url).await.unwrap();
        
        sqlx::query("TRUNCATE rosters, duties, duty_executions CASCADE")
            .execute(state.pool())
            .await
            .unwrap();
        
        state
    }
    
    #[tokio::test]
    async fn test_controller_creation() {
        let state = setup_test_state().await;
        let controller = Controller::new(state);
        assert_eq!(controller.modules.len(), 0);
    }
    
    #[tokio::test]
    async fn test_register_module() {
        let state = setup_test_state().await;
        let mut controller = Controller::new(state);
        
        let module = Arc::new(AwsStaticSiteModule::new());
        controller.register_module(module);
        
        assert_eq!(controller.modules.len(), 1);
        assert!(controller.modules.contains_key("aws-static-site"));
    }
    
    #[tokio::test]
    async fn test_match_roster_by_traits() {
        let state = setup_test_state().await;
        
        let roster = Roster {
            id: None,
            name: "test-aws".to_string(),
            roster_type: "aws-account".to_string(),
            traits: vec!["cloud-provider".to_string(), "aws".to_string()],
            connection: json!({"region": "us-east-1"}),
            auth: json!({"type": "iam-user"}),
            metadata: None,
            created_at: None,
            updated_at: None,
        };
        state.create_roster(roster).await.unwrap();
        
        let duty = Duty {
            id: None,
            name: "test-duty".to_string(),
            duty_type: "StaticSite".to_string(),
            backend: "aws".to_string(),
            roster_selector: json!({"traits": ["cloud-provider", "aws"]}),
            spec: json!({"site": {"domain": "test.com"}}),
            status: None,
            metadata: None,
            created_at: None,
            updated_at: None,
        };
        
        let controller = Controller::new(state);
        let matched = controller.match_roster(&duty).await.unwrap();
        assert_eq!(matched.name, "test-aws");
    }
    
    #[tokio::test]
    async fn test_match_roster_no_match() {
        let state = setup_test_state().await;
        
        let duty = Duty {
            id: None,
            name: "test-duty".to_string(),
            duty_type: "StaticSite".to_string(),
            backend: "aws".to_string(),
            roster_selector: json!({"traits": ["nonexistent"]}),
            spec: json!({"site": {"domain": "test.com"}}),
            status: None,
            metadata: None,
            created_at: None,
            updated_at: None,
        };
        
        let controller = Controller::new(state);
        let result = controller.match_roster(&duty).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_select_module() {
        let state = setup_test_state().await;
        let mut controller = Controller::new(state);
        
        let module = Arc::new(AwsStaticSiteModule::new());
        controller.register_module(module);
        
        let duty = Duty {
            id: None,
            name: "test-duty".to_string(),
            duty_type: "StaticSite".to_string(),
            backend: "aws".to_string(),
            roster_selector: json!({}),
            spec: json!({"site": {"domain": "test.com"}}),
            status: None,
            metadata: None,
            created_at: None,
            updated_at: None,
        };
        
        let selected = controller.select_module(&duty).unwrap();
        assert_eq!(selected.name(), "aws-static-site");
    }
    
    #[tokio::test]
    async fn test_select_module_not_found() {
        let state = setup_test_state().await;
        let controller = Controller::new(state);
        
        let duty = Duty {
            id: None,
            name: "test-duty".to_string(),
            duty_type: "UnsupportedType".to_string(),
            backend: "aws".to_string(),
            roster_selector: json!({}),
            spec: json!({}),
            status: None,
            metadata: None,
            created_at: None,
            updated_at: None,
        };
        
        let result = controller.select_module(&duty);
        assert!(result.is_err());
    }
}
