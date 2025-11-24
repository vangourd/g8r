use anyhow::{Result, anyhow};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use super::{KvStore, Variable, VariableType};

/// Global KV store for variables accessible across stacks via REST API
/// This can operate in two modes:
/// 1. In-memory mode for testing and development
/// 2. Database-backed mode for production (requires database schema)
#[derive(Debug, Clone)]
pub struct GlobalKvStore {
    storage: GlobalKvStorage,
}

#[derive(Debug, Clone)]
enum GlobalKvStorage {
    InMemory(Arc<RwLock<HashMap<String, Variable>>>),
    Database {
        state_manager: crate::db::StateManager,
    },
}

impl GlobalKvStore {
    /// Create a new in-memory global KV store (for testing/development)
    pub fn new_in_memory() -> Self {
        Self {
            storage: GlobalKvStorage::InMemory(Arc::new(RwLock::new(HashMap::new()))),
        }
    }

    /// Create a new database-backed global KV store
    pub fn new_with_database(state_manager: crate::db::StateManager) -> Self {
        Self {
            storage: GlobalKvStorage::Database { state_manager },
        }
    }

    /// Get all variables as a JSON object for context injection
    pub async fn to_json_context(&self) -> Result<JsonValue> {
        let variables = self.list_variables().await?;
        let mut context = serde_json::Map::new();
        
        for variable in variables {
            context.insert(variable.key, variable.value);
        }
        
        Ok(JsonValue::Object(context))
    }

    /// Set a variable from JSON with automatic type detection
    pub async fn set_json(&self, key: &str, value: JsonValue, description: Option<String>) -> Result<()> {
        let variable = Variable::new_global(key.to_string(), value, description);
        self.set(variable).await
    }

    /// Bulk set variables from a JSON object
    pub async fn set_bulk(&self, variables: &JsonValue, key_prefix: Option<&str>) -> Result<()> {
        match variables {
            JsonValue::Object(obj) => {
                for (key, value) in obj {
                    let full_key = if let Some(prefix) = key_prefix {
                        format!("{}.{}", prefix, key)
                    } else {
                        key.clone()
                    };
                    
                    self.set_json(&full_key, value.clone(), None).await?;
                }
            }
            _ => {
                if let Some(prefix) = key_prefix {
                    self.set_json(prefix, variables.clone(), None).await?;
                } else {
                    return Err(anyhow!("Cannot set non-object value without key"));
                }
            }
        }
        Ok(())
    }

    /// Clear all variables (for testing)
    pub async fn clear(&self) -> Result<()> {
        match &self.storage {
            GlobalKvStorage::InMemory(vars) => {
                let mut vars = vars.write()
                    .map_err(|_| anyhow!("Failed to acquire write lock on global variables"))?;
                vars.clear();
                Ok(())
            }
            GlobalKvStorage::Database { .. } => {
                // TODO: Implement database clear when schema is ready
                Err(anyhow!("Database clear not yet implemented"))
            }
        }
    }

    /// Get variables by key prefix
    pub async fn get_by_prefix(&self, prefix: &str) -> Result<Vec<Variable>> {
        let all_vars = self.list_variables().await?;
        Ok(all_vars.into_iter()
            .filter(|var| var.key.starts_with(prefix))
            .collect())
    }
}

#[async_trait::async_trait]
impl KvStore for GlobalKvStore {
    async fn get(&self, key: &str) -> Result<Option<Variable>> {
        match &self.storage {
            GlobalKvStorage::InMemory(vars) => {
                let vars = vars.read()
                    .map_err(|_| anyhow!("Failed to acquire read lock on global variables"))?;
                Ok(vars.get(key).cloned())
            }
            GlobalKvStorage::Database { state_manager } => {
                // TODO: Implement database get when schema is ready
                // For now, use a placeholder that will be replaced once we have the schema
                self.get_from_database(state_manager, key).await
            }
        }
    }

    async fn set(&self, variable: Variable) -> Result<()> {
        // Only allow Global and Const types in global store
        match variable.var_type {
            VariableType::Var => {
                return Err(anyhow!(
                    "Cannot store stack variable '{}' in global store", 
                    variable.key
                ));
            }
            VariableType::Global | VariableType::Const => {
                // Allow global variables and constants
            }
        }

        match &self.storage {
            GlobalKvStorage::InMemory(vars) => {
                let mut vars = vars.write()
                    .map_err(|_| anyhow!("Failed to acquire write lock on global variables"))?;
                vars.insert(variable.key.clone(), variable);
                Ok(())
            }
            GlobalKvStorage::Database { state_manager } => {
                // TODO: Implement database set when schema is ready
                self.set_to_database(state_manager, &variable).await
            }
        }
    }

    async fn delete(&self, key: &str) -> Result<bool> {
        match &self.storage {
            GlobalKvStorage::InMemory(vars) => {
                let mut vars = vars.write()
                    .map_err(|_| anyhow!("Failed to acquire write lock on global variables"))?;
                Ok(vars.remove(key).is_some())
            }
            GlobalKvStorage::Database { state_manager } => {
                // TODO: Implement database delete when schema is ready
                self.delete_from_database(state_manager, key).await
            }
        }
    }

    async fn list_keys(&self) -> Result<Vec<String>> {
        match &self.storage {
            GlobalKvStorage::InMemory(vars) => {
                let vars = vars.read()
                    .map_err(|_| anyhow!("Failed to acquire read lock on global variables"))?;
                Ok(vars.keys().cloned().collect())
            }
            GlobalKvStorage::Database { state_manager } => {
                // TODO: Implement database list_keys when schema is ready
                self.list_keys_from_database(state_manager).await
            }
        }
    }

    async fn list_variables(&self) -> Result<Vec<Variable>> {
        match &self.storage {
            GlobalKvStorage::InMemory(vars) => {
                let vars = vars.read()
                    .map_err(|_| anyhow!("Failed to acquire read lock on global variables"))?;
                Ok(vars.values().cloned().collect())
            }
            GlobalKvStorage::Database { state_manager } => {
                // TODO: Implement database list_variables when schema is ready
                self.list_variables_from_database(state_manager).await
            }
        }
    }
}

impl GlobalKvStore {
    // Placeholder methods for database operations
    // These will be implemented once the database schema is created
    
    async fn get_from_database(&self, _state_manager: &crate::db::StateManager, _key: &str) -> Result<Option<Variable>> {
        // TODO: Implement with proper SQL query
        // SELECT * FROM global_kv WHERE key = $1
        Err(anyhow!("Database global KV not yet implemented - needs schema migration"))
    }

    async fn set_to_database(&self, _state_manager: &crate::db::StateManager, _variable: &Variable) -> Result<()> {
        // TODO: Implement with proper SQL query
        // INSERT INTO global_kv (key, value, var_type, description, created_at, updated_at) 
        // VALUES ($1, $2, $3, $4, $5, $6)
        // ON CONFLICT (key) DO UPDATE SET ...
        Err(anyhow!("Database global KV not yet implemented - needs schema migration"))
    }

    async fn delete_from_database(&self, _state_manager: &crate::db::StateManager, _key: &str) -> Result<bool> {
        // TODO: Implement with proper SQL query
        // DELETE FROM global_kv WHERE key = $1
        Err(anyhow!("Database global KV not yet implemented - needs schema migration"))
    }

    async fn list_keys_from_database(&self, _state_manager: &crate::db::StateManager) -> Result<Vec<String>> {
        // TODO: Implement with proper SQL query
        // SELECT key FROM global_kv ORDER BY key
        Err(anyhow!("Database global KV not yet implemented - needs schema migration"))
    }

    async fn list_variables_from_database(&self, _state_manager: &crate::db::StateManager) -> Result<Vec<Variable>> {
        // TODO: Implement with proper SQL query
        // SELECT * FROM global_kv ORDER BY key
        Err(anyhow!("Database global KV not yet implemented - needs schema migration"))
    }
}

// REST API endpoints for global KV access
// These would be added to the main API router

/// REST API handler for getting a global variable
pub async fn get_global_var_handler(
    _store: GlobalKvStore,
    _key: String,
) -> Result<JsonValue> {
    // TODO: Implement REST endpoint
    // GET /api/v1/kv/global/{key}
    Err(anyhow!("Global KV REST API not yet implemented"))
}

/// REST API handler for setting a global variable
pub async fn set_global_var_handler(
    _store: GlobalKvStore,
    _key: String,
    _value: JsonValue,
) -> Result<()> {
    // TODO: Implement REST endpoint
    // PUT /api/v1/kv/global/{key}
    Err(anyhow!("Global KV REST API not yet implemented"))
}

/// REST API handler for listing global variables
pub async fn list_global_vars_handler(
    _store: GlobalKvStore,
    _prefix: Option<String>,
) -> Result<Vec<Variable>> {
    // TODO: Implement REST endpoint
    // GET /api/v1/kv/global?prefix={prefix}
    Err(anyhow!("Global KV REST API not yet implemented"))
}

/// REST API handler for deleting a global variable
pub async fn delete_global_var_handler(
    _store: GlobalKvStore,
    _key: String,
) -> Result<bool> {
    // TODO: Implement REST endpoint
    // DELETE /api/v1/kv/global/{key}
    Err(anyhow!("Global KV REST API not yet implemented"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_global_kv_store_in_memory() {
        let store = GlobalKvStore::new_in_memory();
        
        // Test setting and getting a global variable
        let var = Variable::new_global("global_key".to_string(), json!("global_value"), None);
        store.set(var).await.unwrap();
        
        let retrieved = store.get("global_key").await.unwrap().unwrap();
        assert_eq!(retrieved.value, json!("global_value"));
        assert_eq!(retrieved.var_type, VariableType::Global);
        
        // Test listing
        let keys = store.list_keys().await.unwrap();
        assert_eq!(keys, vec!["global_key"]);
        
        // Test deletion
        let deleted = store.delete("global_key").await.unwrap();
        assert!(deleted);
        
        let not_found = store.get("global_key").await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_global_kv_store_rejects_stack_variables() {
        let store = GlobalKvStore::new_in_memory();
        
        let stack_var = Variable::new_var("stack_key".to_string(), json!("value"), None);
        let result = store.set(stack_var).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cannot store stack variable"));
    }

    #[tokio::test]
    async fn test_global_kv_store_allows_constants() {
        let store = GlobalKvStore::new_in_memory();
        
        let const_var = Variable::new_const("const_key".to_string(), json!("const_value"), None);
        store.set(const_var).await.unwrap();
        
        let retrieved = store.get("const_key").await.unwrap().unwrap();
        assert_eq!(retrieved.value, json!("const_value"));
        assert_eq!(retrieved.var_type, VariableType::Const);
    }

    #[tokio::test]
    async fn test_bulk_set_from_json() {
        let store = GlobalKvStore::new_in_memory();
        
        let config = json!({
            "api": {
                "endpoint": "https://api.example.com",
                "version": "v1"
            },
            "database": {
                "host": "db.example.com",
                "port": 5432
            }
        });
        
        store.set_bulk(&config, Some("config")).await.unwrap();
        
        // Check nested keys were created
        let endpoint = store.get("config.api.endpoint").await.unwrap().unwrap();
        assert_eq!(endpoint.value, json!("https://api.example.com"));
        
        let port = store.get("config.database.port").await.unwrap().unwrap();
        assert_eq!(port.value, json!(5432));
    }

    #[tokio::test]
    async fn test_get_by_prefix() {
        let store = GlobalKvStore::new_in_memory();
        
        store.set_json("app.config.debug", json!(true), None).await.unwrap();
        store.set_json("app.config.port", json!(8080), None).await.unwrap();
        store.set_json("app.version", json!("1.0.0"), None).await.unwrap();
        store.set_json("other.value", json!("test"), None).await.unwrap();
        
        let app_vars = store.get_by_prefix("app.").await.unwrap();
        assert_eq!(app_vars.len(), 3);
        
        let config_vars = store.get_by_prefix("app.config.").await.unwrap();
        assert_eq!(config_vars.len(), 2);
    }

    #[tokio::test]
    async fn test_json_context_generation() {
        let store = GlobalKvStore::new_in_memory();
        
        store.set_json("api_key", json!("secret123"), None).await.unwrap();
        store.set_json("debug_mode", json!(true), None).await.unwrap();
        store.set_json("max_connections", json!(100), None).await.unwrap();
        
        let context = store.to_json_context().await.unwrap();
        
        assert_eq!(context["api_key"], json!("secret123"));
        assert_eq!(context["debug_mode"], json!(true));
        assert_eq!(context["max_connections"], json!(100));
    }
}