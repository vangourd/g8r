use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::utils::{Instruction, InstructionContext};

/// A unique identifier for a stack execution context
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExecutionUnitId(pub Uuid);

impl ExecutionUnitId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
    
    pub fn from_string(s: &str) -> Result<Self> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl std::fmt::Display for ExecutionUnitId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Variable storage for a stack execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub key: String,
    pub value: JsonValue,
    pub source: VariableSource,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub metadata: Option<HashMap<String, JsonValue>>,
}

/// Source of a variable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VariableSource {
    /// Output from a duty execution
    DutyOutput { duty_name: String },
    /// Static constant defined in config
    StaticConst,
    /// Environment variable
    Environment,
    /// Secret from external source
    Secret { backend: String },
    /// Resolved from remote execution unit
    Remote { unit_id: ExecutionUnitId },
}

/// Local key-value store for stack execution context
#[derive(Debug)]
pub struct LocalKVStore {
    variables: Arc<RwLock<HashMap<String, Variable>>>,
    constants: HashMap<String, JsonValue>,
}

impl LocalKVStore {
    pub fn new() -> Self {
        Self {
            variables: Arc::new(RwLock::new(HashMap::new())),
            constants: HashMap::new(),
        }
    }

    pub fn with_constants(constants: HashMap<String, JsonValue>) -> Self {
        Self {
            variables: Arc::new(RwLock::new(HashMap::new())),
            constants,
        }
    }

    /// Store a variable from duty output
    pub async fn store_duty_output(
        &self,
        duty_name: &str,
        key: &str,
        value: JsonValue,
    ) -> Result<()> {
        let variable = Variable {
            key: key.to_string(),
            value,
            source: VariableSource::DutyOutput {
                duty_name: duty_name.to_string(),
            },
            created_at: chrono::Utc::now(),
            metadata: None,
        };

        self.variables.write().await.insert(key.to_string(), variable);
        Ok(())
    }

    /// Store a remote variable resolved from another execution unit
    pub async fn store_remote_variable(
        &self,
        key: &str,
        value: JsonValue,
        unit_id: ExecutionUnitId,
    ) -> Result<()> {
        let variable = Variable {
            key: key.to_string(),
            value,
            source: VariableSource::Remote { unit_id },
            created_at: chrono::Utc::now(),
            metadata: None,
        };

        self.variables.write().await.insert(key.to_string(), variable);
        Ok(())
    }

    /// Get a variable by key (checks variables first, then constants)
    pub async fn get(&self, key: &str) -> Option<JsonValue> {
        // Check dynamic variables first
        if let Some(var) = self.variables.read().await.get(key) {
            return Some(var.value.clone());
        }

        // Check static constants
        self.constants.get(key).cloned()
    }

    /// List all available variables and constants
    pub async fn list_all(&self) -> HashMap<String, JsonValue> {
        let mut result = self.constants.clone();
        
        for (key, var) in self.variables.read().await.iter() {
            result.insert(key.clone(), var.value.clone());
        }
        
        result
    }

    /// Get variable with metadata
    pub async fn get_variable(&self, key: &str) -> Option<Variable> {
        self.variables.read().await.get(key).cloned()
    }
}

/// Stack execution context with local KV store and distributed capabilities
#[derive(Debug)]
pub struct StackExecutionContext {
    pub unit_id: ExecutionUnitId,
    pub stack_name: String,
    pub kv_store: LocalKVStore,
    pub instruction_context: InstructionContext,
    pub distributed_client: Option<Arc<dyn DistributedKVClient>>,
}

impl StackExecutionContext {
    pub fn new(stack_name: String) -> Self {
        Self {
            unit_id: ExecutionUnitId::new(),
            stack_name,
            kv_store: LocalKVStore::new(),
            instruction_context: InstructionContext::new(),
            distributed_client: None,
        }
    }

    pub fn with_constants(
        stack_name: String,
        constants: HashMap<String, JsonValue>,
    ) -> Self {
        Self {
            unit_id: ExecutionUnitId::new(),
            stack_name,
            kv_store: LocalKVStore::with_constants(constants),
            instruction_context: InstructionContext::new(),
            distributed_client: None,
        }
    }

    pub fn with_distributed_client(
        mut self,
        client: Arc<dyn DistributedKVClient>,
    ) -> Self {
        self.distributed_client = Some(client);
        self
    }

    /// Resolve an instruction token to its actual value
    pub async fn resolve_instruction(&self, instruction: &Instruction) -> Result<JsonValue> {
        match instruction.instruction_type.as_str() {
            "g8r_output" => {
                if instruction.args.len() != 2 {
                    return Err(anyhow::anyhow!(
                        "g8r_output requires 2 arguments: duty_name and output_key"
                    ));
                }
                
                let duty_name = &instruction.args[0];
                let output_key = &instruction.args[1];
                let full_key = format!("{}.{}", duty_name, output_key);
                
                // Try local store first
                if let Some(value) = self.kv_store.get(&full_key).await {
                    return Ok(value);
                }
                
                // Try distributed lookup if available
                if let Some(client) = &self.distributed_client {
                    if let Some(value) = client.get_variable(&full_key).await? {
                        // Cache locally for future access
                        self.kv_store.store_remote_variable(
                            &full_key,
                            value.clone(),
                            ExecutionUnitId::new(), // TODO: Get actual unit_id from response
                        ).await?;
                        return Ok(value);
                    }
                }
                
                Err(anyhow::anyhow!(
                    "Variable not found: {}.{}", duty_name, output_key
                ))
            }
            
            "g8r_env" => {
                if instruction.args.len() != 1 {
                    return Err(anyhow::anyhow!(
                        "g8r_env requires 1 argument: env_var_name"
                    ));
                }
                
                let env_var = &instruction.args[0];
                std::env::var(env_var)
                    .map(JsonValue::String)
                    .with_context(|| format!("Environment variable '{}' not found", env_var))
            }
            
            "g8r_secret" => {
                // TODO: Implement secret resolution
                Err(anyhow::anyhow!("g8r_secret not yet implemented"))
            }
            
            "g8r_const" => {
                if instruction.args.len() != 1 {
                    return Err(anyhow::anyhow!(
                        "g8r_const requires 1 argument: const_name"
                    ));
                }
                
                let const_name = &instruction.args[0];
                self.kv_store.get(const_name).await
                    .ok_or_else(|| anyhow::anyhow!("Constant '{}' not found", const_name))
            }
            
            _ => Err(anyhow::anyhow!(
                "Unknown instruction type: {}", instruction.instruction_type
            )),
        }
    }

    /// Resolve all instructions in the context
    pub async fn resolve_all_instructions(&self) -> Result<HashMap<String, JsonValue>> {
        let mut resolved = HashMap::new();
        
        for instruction in &self.instruction_context.instructions {
            let value = self.resolve_instruction(instruction).await?;
            resolved.insert(instruction.token.clone(), value);
        }
        
        Ok(resolved)
    }

    /// Store duty output and make it available for cross-references
    pub async fn store_duty_output(
        &self,
        duty_name: &str,
        outputs: &HashMap<String, JsonValue>,
    ) -> Result<()> {
        for (key, value) in outputs {
            let full_key = format!("{}.{}", duty_name, key);
            self.kv_store.store_duty_output(duty_name, &full_key, value.clone()).await?;
        }
        Ok(())
    }

    /// Get runtime context for Nickel evaluation
    pub async fn get_runtime_context(&self) -> HashMap<String, JsonValue> {
        let mut context = HashMap::new();
        
        // Add all variables
        let all_vars = self.kv_store.list_all().await;
        
        // Organize by duty outputs for backward compatibility
        let mut duties = HashMap::new();
        for (key, value) in all_vars {
            if let Some((duty_name, output_key)) = key.split_once('.') {
                let duty_outputs = duties.entry(duty_name.to_string())
                    .or_insert_with(|| serde_json::json!({"outputs": {}}));
                
                if let Some(outputs) = duty_outputs.get_mut("outputs").and_then(|v| v.as_object_mut()) {
                    outputs.insert(output_key.to_string(), value);
                }
            }
        }
        
        context.insert("duties".to_string(), JsonValue::Object(duties.into_iter().collect()));
        context.insert("unit_id".to_string(), JsonValue::String(self.unit_id.to_string()));
        context.insert("stack_name".to_string(), JsonValue::String(self.stack_name.clone()));
        
        context
    }
}

/// Trait for distributed KV client implementations
#[async_trait::async_trait]
pub trait DistributedKVClient: Send + Sync {
    /// Get a variable from a remote execution unit
    async fn get_variable(&self, key: &str) -> Result<Option<JsonValue>>;
    
    /// Set a variable that can be accessed by other execution units
    async fn set_variable(&self, key: &str, value: JsonValue) -> Result<()>;
    
    /// List all variables accessible from remote units
    async fn list_variables(&self) -> Result<HashMap<String, JsonValue>>;
    
    /// Query variables by pattern (glob-style)
    async fn query_variables(&self, pattern: &str) -> Result<HashMap<String, JsonValue>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_local_kv_store() {
        let store = LocalKVStore::new();
        
        // Store duty output
        store.store_duty_output("bucket", "bucket.arn", json!("arn:aws:s3:::my-bucket")).await.unwrap();
        
        // Retrieve value
        let value = store.get("bucket.arn").await.unwrap();
        assert_eq!(value, json!("arn:aws:s3:::my-bucket"));
    }

    #[tokio::test]
    async fn test_execution_context_with_constants() {
        let mut constants = HashMap::new();
        constants.insert("app_name".to_string(), json!("my-app"));
        constants.insert("environment".to_string(), json!("production"));
        
        let context = StackExecutionContext::with_constants("test-stack".to_string(), constants);
        
        // Check constants are accessible
        assert_eq!(context.kv_store.get("app_name").await.unwrap(), json!("my-app"));
        assert_eq!(context.kv_store.get("environment").await.unwrap(), json!("production"));
    }

    #[tokio::test]
    async fn test_duty_output_storage() {
        let context = StackExecutionContext::new("test-stack".to_string());
        
        let mut outputs = HashMap::new();
        outputs.insert("arn".to_string(), json!("arn:aws:s3:::my-bucket"));
        outputs.insert("website_endpoint".to_string(), json!("my-bucket.s3-website.amazonaws.com"));
        
        context.store_duty_output("bucket", &outputs).await.unwrap();
        
        // Check values are stored with full keys
        assert_eq!(
            context.kv_store.get("bucket.arn").await.unwrap(),
            json!("arn:aws:s3:::my-bucket")
        );
        assert_eq!(
            context.kv_store.get("bucket.website_endpoint").await.unwrap(),
            json!("my-bucket.s3-website.amazonaws.com")
        );
    }

    #[tokio::test]
    async fn test_runtime_context_generation() {
        let context = StackExecutionContext::new("test-stack".to_string());
        
        // Store some duty outputs
        let mut outputs = HashMap::new();
        outputs.insert("arn".to_string(), json!("arn:aws:s3:::my-bucket"));
        context.store_duty_output("bucket", &outputs).await.unwrap();
        
        let runtime_context = context.get_runtime_context().await;
        
        // Check structure matches expected format
        assert!(runtime_context.contains_key("duties"));
        assert!(runtime_context.contains_key("unit_id"));
        assert!(runtime_context.contains_key("stack_name"));
        
        let duties = runtime_context["duties"].as_object().unwrap();
        assert!(duties.contains_key("bucket"));
        
        let bucket_outputs = duties["bucket"]["outputs"].as_object().unwrap();
        assert_eq!(bucket_outputs["arn"], json!("arn:aws:s3:::my-bucket"));
    }
}