use anyhow::Result;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use super::{KvStore, Variable, VariableType};

/// In-memory KV store for variables within a stack execution context
/// This provides local variable storage that persists across duty executions
/// within the same stack reconciliation cycle
#[derive(Debug, Clone)]
pub struct StackContext {
    stack_name: String,
    variables: Arc<RwLock<HashMap<String, Variable>>>,
}

impl StackContext {
    /// Create a new stack context for the given stack
    pub fn new(stack_name: String) -> Self {
        Self {
            stack_name,
            variables: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get the stack name
    pub fn stack_name(&self) -> &str {
        &self.stack_name
    }

    /// Clear all variables in this stack context
    pub fn clear(&self) -> Result<()> {
        let mut vars = self.variables.write()
            .map_err(|_| anyhow::anyhow!("Failed to acquire write lock on stack variables"))?;
        vars.clear();
        Ok(())
    }

    /// Get all variables as a JSON object for Nickel context injection
    pub fn to_json_context(&self) -> Result<JsonValue> {
        let vars = self.variables.read()
            .map_err(|_| anyhow::anyhow!("Failed to acquire read lock on stack variables"))?;
        
        let mut context = serde_json::Map::new();
        for (key, variable) in vars.iter() {
            context.insert(key.clone(), variable.value.clone());
        }
        
        Ok(JsonValue::Object(context))
    }

    /// Set multiple variables from a JSON object (used for duty outputs)
    pub fn set_from_json(&self, key_prefix: &str, json: &JsonValue) -> Result<()> {
        match json {
            JsonValue::Object(obj) => {
                for (key, value) in obj {
                    let full_key = if key_prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", key_prefix, key)
                    };
                    
                    let variable = Variable::new_var(full_key.clone(), value.clone(), None);
                    self.set(variable)?;
                }
            }
            _ => {
                let variable = Variable::new_var(key_prefix.to_string(), json.clone(), None);
                self.set(variable)?;
            }
        }
        Ok(())
    }

    /// Get duty outputs in the runtime.duties format expected by Nickel
    pub fn get_duties_context(&self) -> Result<JsonValue> {
        let vars = self.variables.read()
            .map_err(|_| anyhow::anyhow!("Failed to acquire read lock on stack variables"))?;
        
        let mut duties_map = serde_json::Map::new();
        
        for (key, variable) in vars.iter() {
            // Parse keys in the format "duties.{duty_name}.outputs.{field}"
            if let Some(rest) = key.strip_prefix("duties.") {
                if let Some((duty_name, field_path)) = rest.split_once('.') {
                    // Ensure duty entry exists
                    if !duties_map.contains_key(duty_name) {
                        duties_map.insert(duty_name.to_string(), JsonValue::Object(serde_json::Map::new()));
                    }
                    
                    // Set the field value in nested structure
                    if let Some(duty_obj) = duties_map.get_mut(duty_name) {
                        if let JsonValue::Object(duty_map) = duty_obj {
                            Self::set_nested_field(duty_map, field_path, variable.value.clone())?;
                        }
                    }
                }
            }
        }
        
        Ok(JsonValue::Object(duties_map))
    }

    /// Set a nested field in a JSON map using dot notation
    fn set_nested_field(map: &mut serde_json::Map<String, JsonValue>, path: &str, value: JsonValue) -> Result<()> {
        let parts: Vec<&str> = path.split('.').collect();
        
        if parts.is_empty() {
            return Err(anyhow::anyhow!("Empty field path"));
        }
        
        if parts.len() == 1 {
            map.insert(parts[0].to_string(), value);
            return Ok(());
        }
        
        let key = parts[0];
        let rest = parts[1..].join(".");
        
        // Ensure the intermediate object exists
        if !map.contains_key(key) {
            map.insert(key.to_string(), JsonValue::Object(serde_json::Map::new()));
        }
        
        if let Some(JsonValue::Object(nested)) = map.get_mut(key) {
            Self::set_nested_field(nested, &rest, value)?;
        } else {
            return Err(anyhow::anyhow!("Expected object at key '{}'", key));
        }
        
        Ok(())
    }
}

#[async_trait::async_trait]
impl KvStore for StackContext {
    async fn get(&self, key: &str) -> Result<Option<Variable>> {
        let vars = self.variables.read()
            .map_err(|_| anyhow::anyhow!("Failed to acquire read lock on stack variables"))?;
        Ok(vars.get(key).cloned())
    }

    async fn set(&self, variable: Variable) -> Result<()> {
        // Only allow Var and Const types in stack context
        match variable.var_type {
            VariableType::Global => {
                return Err(anyhow::anyhow!(
                    "Cannot store global variable '{}' in stack context", 
                    variable.key
                ));
            }
            VariableType::Const => {
                return Err(anyhow::anyhow!(
                    "Cannot store constant '{}' in stack context - constants are read-only", 
                    variable.key
                ));
            }
            VariableType::Var => {
                // Allow stack variables
            }
        }

        let mut vars = self.variables.write()
            .map_err(|_| anyhow::anyhow!("Failed to acquire write lock on stack variables"))?;
        
        vars.insert(variable.key.clone(), variable);
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<bool> {
        let mut vars = self.variables.write()
            .map_err(|_| anyhow::anyhow!("Failed to acquire write lock on stack variables"))?;
        Ok(vars.remove(key).is_some())
    }

    async fn list_keys(&self) -> Result<Vec<String>> {
        let vars = self.variables.read()
            .map_err(|_| anyhow::anyhow!("Failed to acquire read lock on stack variables"))?;
        Ok(vars.keys().cloned().collect())
    }

    async fn list_variables(&self) -> Result<Vec<Variable>> {
        let vars = self.variables.read()
            .map_err(|_| anyhow::anyhow!("Failed to acquire read lock on stack variables"))?;
        Ok(vars.values().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_stack_context_basic_operations() {
        let context = StackContext::new("test-stack".to_string());
        
        // Test setting and getting a variable
        let var = Variable::new_var("test_key".to_string(), json!("test_value"), None);
        context.set(var).await.unwrap();
        
        let retrieved = context.get("test_key").await.unwrap().unwrap();
        assert_eq!(retrieved.value, json!("test_value"));
        assert_eq!(retrieved.var_type, VariableType::Var);
        
        // Test listing keys
        let keys = context.list_keys().await.unwrap();
        assert_eq!(keys, vec!["test_key"]);
        
        // Test deletion
        let deleted = context.delete("test_key").await.unwrap();
        assert!(deleted);
        
        let not_found = context.get("test_key").await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_stack_context_rejects_global_variables() {
        let context = StackContext::new("test-stack".to_string());
        
        let global_var = Variable::new_global("global_key".to_string(), json!("value"), None);
        let result = context.set(global_var).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cannot store global variable"));
    }

    #[tokio::test]
    async fn test_stack_context_rejects_constants() {
        let context = StackContext::new("test-stack".to_string());
        
        let const_var = Variable::new_const("const_key".to_string(), json!("value"), None);
        let result = context.set(const_var).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cannot store constant"));
    }

    #[tokio::test]
    async fn test_duties_context_generation() {
        let context = StackContext::new("test-stack".to_string());
        
        // Set some duty outputs
        let bucket_endpoint = Variable::new_var(
            "duties.bucket.outputs.website_endpoint".to_string(),
            json!("bucket.s3-website.us-east-1.amazonaws.com"),
            None
        );
        context.set(bucket_endpoint).await.unwrap();
        
        let cert_arn = Variable::new_var(
            "duties.cert.outputs.certificate_arn".to_string(),
            json!("arn:aws:acm:us-east-1:123456789012:certificate/abcd1234"),
            None
        );
        context.set(cert_arn).await.unwrap();
        
        // Get duties context
        let duties_context = context.get_duties_context().unwrap();
        
        // Verify structure
        assert_eq!(
            duties_context["bucket"]["outputs"]["website_endpoint"],
            json!("bucket.s3-website.us-east-1.amazonaws.com")
        );
        assert_eq!(
            duties_context["cert"]["outputs"]["certificate_arn"],
            json!("arn:aws:acm:us-east-1:123456789012:certificate/abcd1234")
        );
    }

    #[tokio::test]
    async fn test_set_from_json() {
        let context = StackContext::new("test-stack".to_string());
        
        let outputs = json!({
            "website_endpoint": "bucket.s3-website.us-east-1.amazonaws.com",
            "bucket_arn": "arn:aws:s3:::test-bucket",
            "nested": {
                "value": "deep_value"
            }
        });
        
        context.set_from_json("duties.bucket.outputs", &outputs).unwrap();
        
        // Check that nested structure was created
        let endpoint = context.get("duties.bucket.outputs.website_endpoint").await.unwrap().unwrap();
        assert_eq!(endpoint.value, json!("bucket.s3-website.us-east-1.amazonaws.com"));
        
        let nested = context.get("duties.bucket.outputs.nested").await.unwrap().unwrap();
        assert_eq!(nested.value, json!({"value": "deep_value"}));
    }

    #[test]
    fn test_nested_field_setting() {
        let mut map = serde_json::Map::new();
        
        StackContext::set_nested_field(&mut map, "outputs.website_endpoint", json!("test-endpoint")).unwrap();
        StackContext::set_nested_field(&mut map, "outputs.bucket_arn", json!("test-arn")).unwrap();
        StackContext::set_nested_field(&mut map, "metadata.version", json!("1.0")).unwrap();
        
        let result = JsonValue::Object(map);
        
        assert_eq!(result["outputs"]["website_endpoint"], json!("test-endpoint"));
        assert_eq!(result["outputs"]["bucket_arn"], json!("test-arn"));
        assert_eq!(result["metadata"]["version"], json!("1.0"));
    }
}