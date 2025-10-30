use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

pub mod stack_context;
pub mod global_store;

pub use stack_context::StackContext;
pub use global_store::GlobalKvStore;

/// Variable types supported by the KV system
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VariableType {
    /// Constants locked at runtime, sourced from config only
    Const,
    /// Variables read/write within stack context
    Var,
    /// Variables accessible across stacks via REST API
    Global,
}

/// Variable metadata and value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub key: String,
    pub value: JsonValue,
    pub var_type: VariableType,
    pub description: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Variable {
    pub fn new_const(key: String, value: JsonValue, description: Option<String>) -> Self {
        let now = chrono::Utc::now();
        Self {
            key,
            value,
            var_type: VariableType::Const,
            description,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn new_var(key: String, value: JsonValue, description: Option<String>) -> Self {
        let now = chrono::Utc::now();
        Self {
            key,
            value,
            var_type: VariableType::Var,
            description,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn new_global(key: String, value: JsonValue, description: Option<String>) -> Self {
        let now = chrono::Utc::now();
        Self {
            key,
            value,
            var_type: VariableType::Global,
            description,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn is_writable(&self) -> bool {
        match self.var_type {
            VariableType::Const => false,
            VariableType::Var | VariableType::Global => true,
        }
    }
}

/// KV store trait for different storage backends
#[async_trait::async_trait]
pub trait KvStore: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Variable>>;
    async fn set(&self, variable: Variable) -> Result<()>;
    async fn delete(&self, key: &str) -> Result<bool>;
    async fn list_keys(&self) -> Result<Vec<String>>;
    async fn list_variables(&self) -> Result<Vec<Variable>>;
    
    /// Check if a key exists
    async fn exists(&self, key: &str) -> Result<bool> {
        Ok(self.get(key).await?.is_some())
    }
    
    /// Set a value only if it doesn't exist
    async fn set_if_not_exists(&self, variable: Variable) -> Result<bool> {
        if self.exists(&variable.key).await? {
            Ok(false)
        } else {
            self.set(variable).await?;
            Ok(true)
        }
    }
}

/// Context for variable resolution within a stack execution
#[derive(Debug, Clone)]
pub struct VariableContext {
    pub stack_context: StackContext,
    pub global_store: GlobalKvStore,
    pub constants: HashMap<String, JsonValue>,
}

impl VariableContext {
    pub fn new(stack_name: String, global_store: GlobalKvStore) -> Self {
        Self {
            stack_context: StackContext::new(stack_name),
            global_store,
            constants: HashMap::new(),
        }
    }

    /// Add constants from configuration (locked at runtime)
    pub fn add_constants(&mut self, constants: HashMap<String, JsonValue>) {
        self.constants.extend(constants);
    }

    /// Resolve a variable by checking in order: constants, stack context, global store
    pub async fn resolve(&self, key: &str) -> Result<Option<JsonValue>> {
        // Check constants first
        if let Some(value) = self.constants.get(key) {
            return Ok(Some(value.clone()));
        }

        // Check stack context
        if let Some(var) = self.stack_context.get(key).await? {
            return Ok(Some(var.value));
        }

        // Check global store
        if let Some(var) = self.global_store.get(key).await? {
            return Ok(Some(var.value));
        }

        Ok(None)
    }

    /// Set a variable in the appropriate store based on type
    pub async fn set(&self, key: &str, value: JsonValue, var_type: VariableType) -> Result<()> {
        let variable = match var_type {
            VariableType::Const => {
                return Err(anyhow::anyhow!("Cannot set constant '{}' at runtime", key));
            }
            VariableType::Var => Variable::new_var(key.to_string(), value, None),
            VariableType::Global => Variable::new_global(key.to_string(), value, None),
        };

        match var_type {
            VariableType::Var => self.stack_context.set(variable).await,
            VariableType::Global => self.global_store.set(variable).await,
            VariableType::Const => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable_creation() {
        let value = serde_json::json!({"test": "value"});
        
        let const_var = Variable::new_const("test_const".to_string(), value.clone(), None);
        assert_eq!(const_var.var_type, VariableType::Const);
        assert!(!const_var.is_writable());

        let var_var = Variable::new_var("test_var".to_string(), value.clone(), None);
        assert_eq!(var_var.var_type, VariableType::Var);
        assert!(var_var.is_writable());

        let global_var = Variable::new_global("test_global".to_string(), value, None);
        assert_eq!(global_var.var_type, VariableType::Global);
        assert!(global_var.is_writable());
    }

    #[tokio::test]
    async fn test_variable_context() {
        let global_store = GlobalKvStore::new_in_memory();
        let mut context = VariableContext::new("test-stack".to_string(), global_store);
        
        // Add a constant
        let mut constants = HashMap::new();
        constants.insert("config_value".to_string(), serde_json::json!("from_config"));
        context.add_constants(constants);

        // Test constant resolution
        let resolved = context.resolve("config_value").await.unwrap();
        assert_eq!(resolved, Some(serde_json::json!("from_config")));

        // Test setting stack variable
        context.set("stack_var", serde_json::json!("stack_value"), VariableType::Var).await.unwrap();
        let resolved = context.resolve("stack_var").await.unwrap();
        assert_eq!(resolved, Some(serde_json::json!("stack_value")));

        // Test setting global variable
        context.set("global_var", serde_json::json!("global_value"), VariableType::Global).await.unwrap();
        let resolved = context.resolve("global_var").await.unwrap();
        assert_eq!(resolved, Some(serde_json::json!("global_value")));

        // Test that constants cannot be set at runtime
        let result = context.set("new_const", serde_json::json!("value"), VariableType::Const).await;
        assert!(result.is_err());
    }
}