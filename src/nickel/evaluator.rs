use anyhow::{Context, Result};
use regex::Regex;
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use crate::utils::{Duty, Roster, InstructionContext};
use crate::kv::{VariableContext, VariableType};

pub struct NickelEvaluator {
    config_path: String,
}

impl NickelEvaluator {
    pub fn new(config_path: impl Into<String>) -> Self {
        Self {
            config_path: config_path.into(),
        }
    }

    pub fn from_path(path: &Path) -> Result<Self> {
        Ok(Self::new(path.to_string_lossy().to_string()))
    }
    
    fn json_to_nickel(json: &HashMap<String, JsonValue>) -> Result<String> {
        fn value_to_nickel(val: &JsonValue) -> String {
            match val {
                JsonValue::Null => "null".to_string(),
                JsonValue::Bool(b) => b.to_string(),
                JsonValue::Number(n) => n.to_string(),
                JsonValue::String(s) => format!("\"{}\"", s.replace('\"', "\\\"")),
                JsonValue::Array(arr) => {
                    let items: Vec<String> = arr.iter().map(value_to_nickel).collect();
                    format!("[{}]", items.join(", "))
                }
                JsonValue::Object(obj) => {
                    let pairs: Vec<String> = obj
                        .iter()
                        .map(|(k, v)| {
                            let key = if k.contains('-') || k.contains('.') {
                                format!("\"{}\"", k)
                            } else {
                                k.clone()
                            };
                            format!("{} = {}", key, value_to_nickel(v))
                        })
                        .collect();
                    format!("{{{}}}", pairs.join(", "))
                }
            }
        }
        
        Ok(value_to_nickel(&serde_json::to_value(json)?))
    }

    pub fn load_config(&self) -> Result<JsonValue> {
        let config_path = Path::new(&self.config_path);
        let config_dir = config_path.parent()
            .ok_or_else(|| anyhow::anyhow!("Config path has no parent directory"))?;
        let config_file = config_path.file_name()
            .ok_or_else(|| anyhow::anyhow!("Config path has no filename"))?;
        
        let output = Command::new("nickel")
            .arg("export")
            .arg(config_file)
            .current_dir(config_dir)
            .output()
            .context("Failed to execute nickel CLI - is nickel installed?")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Nickel evaluation failed: {}", stderr));
        }

        let json_str = String::from_utf8(output.stdout)
            .context("Nickel output is not valid UTF-8")?;

        serde_json::from_str(&json_str)
            .context("Failed to parse Nickel output as JSON")
    }

    pub fn load_roster(&self) -> Result<Roster> {
        let config = self.load_config()?;
        let roster_json = config
            .get("roster")
            .ok_or_else(|| anyhow::anyhow!("Config missing 'roster' field"))?;

        serde_json::from_value(roster_json.clone())
            .context("Failed to deserialize roster from Nickel config")
    }

    pub fn load_rosters(&self) -> Result<Vec<Roster>> {
        let config = self.load_config()?;
        
        if let Some(rosters_json) = config.get("rosters") {
            serde_json::from_value(rosters_json.clone())
                .context("Failed to deserialize rosters array from Nickel config")
        } else if let Some(roster_json) = config.get("roster") {
            let roster: Roster = serde_json::from_value(roster_json.clone())
                .context("Failed to deserialize roster object from Nickel config")?;
            Ok(vec![roster])
        } else {
            Ok(Vec::new())
        }
    }

    pub fn load_duties(&self) -> Result<Vec<Duty>> {
        let config = self.load_config()?;
        let duties_json = config
            .get("duties")
            .ok_or_else(|| anyhow::anyhow!("Config missing 'duties' field"))?;

        if let Some(duties_obj) = duties_json.as_object() {
            let mut duties = Vec::new();
            for (name, duty_config) in duties_obj {
                let mut duty: Duty = serde_json::from_value(duty_config.clone())
                    .with_context(|| format!("Failed to deserialize duty '{}'", name))?;
                
                if duty.name.is_empty() {
                    duty.name = name.clone();
                }
                
                if let Some(depends_on) = duty_config.get("depends_on") {
                    let mut metadata = duty.metadata.unwrap_or_else(|| json!({}));
                    if let Some(meta_obj) = metadata.as_object_mut() {
                        meta_obj.insert("depends_on".to_string(), depends_on.clone());
                    }
                    duty.metadata = Some(metadata);
                }
                
                duties.push(duty);
            }
            Ok(duties)
        } else if let Some(duties_arr) = duties_json.as_array() {
            serde_json::from_value(duties_json.clone())
                .context("Failed to deserialize duties array from Nickel config")
        } else {
            Err(anyhow::anyhow!("'duties' must be an object or array"))
        }
    }

    /// Load duties with variable context support for KV stores
    pub async fn load_duties_with_variable_context(
        &self,
        var_context: &VariableContext,
    ) -> Result<Vec<Duty>> {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let source = std::fs::read_to_string(&self.config_path)
            .with_context(|| format!("Failed to read {}", self.config_path))?;

        // Process g8r variable functions
        let (processed_source, instruction_context) = self.process_g8r_variable_functions(&source, var_context).await?;

        // Build runtime context for backward compatibility
        let mut runtime_context = HashMap::new();
        
        // Add duties context from stack context
        let duties_context = var_context.stack_context.get_duties_context()?;
        runtime_context.insert("duties".to_string(), duties_context);
        
        // Add global variables context
        let global_context = var_context.global_store.to_json_context().await?;
        runtime_context.insert("global".to_string(), global_context);
        
        // Add constants context
        let constants_json = serde_json::to_value(&var_context.constants)?;
        runtime_context.insert("const".to_string(), constants_json);

        let runtime_nickel = Self::json_to_nickel(&runtime_context)?;

        let augmented_source = format!(
            r#"
let runtime = {} in
{}
"#,
            runtime_nickel, processed_source
        );

        let config_path = Path::new(&self.config_path);
        let config_dir = config_path.parent()
            .ok_or_else(|| anyhow::anyhow!("Config path has no parent directory"))?;
        
        let mut temp_file = NamedTempFile::new_in(config_dir)
            .context("Failed to create temporary file for augmented source")?;
        temp_file.write_all(augmented_source.as_bytes())
            .context("Failed to write augmented source to temp file")?;
        temp_file.flush()?;

        let temp_path = temp_file.path();
        let temp_filename = temp_path.file_name()
            .ok_or_else(|| anyhow::anyhow!("Temp file has no filename"))?;
        
        let output = Command::new("nickel")
            .arg("export")
            .arg(temp_filename)
            .current_dir(config_dir)
            .output()
            .context("Failed to execute nickel CLI with variable context")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Nickel evaluation with variable context failed: {}", stderr));
        }

        let json_str = String::from_utf8(output.stdout)
            .context("Nickel output is not valid UTF-8")?;

        let config: JsonValue = serde_json::from_str(&json_str)
            .context("Failed to parse Nickel output as JSON")?;

        let duties_json = config
            .get("duties")
            .ok_or_else(|| anyhow::anyhow!("Config missing 'duties' field"))?;

        let mut duties = if let Some(duties_obj) = duties_json.as_object() {
            let mut duties = Vec::new();
            for (name, duty_config) in duties_obj {
                let mut duty: Duty = serde_json::from_value(duty_config.clone())
                    .with_context(|| format!("Failed to deserialize duty '{}'", name))?;
                
                if duty.name.is_empty() {
                    duty.name = name.clone();
                }
                
                if let Some(depends_on) = duty_config.get("depends_on") {
                    let mut metadata = duty.metadata.unwrap_or_else(|| serde_json::json!({}));
                    if let Some(meta_obj) = metadata.as_object_mut() {
                        meta_obj.insert("depends_on".to_string(), depends_on.clone());
                    }
                    duty.metadata = Some(metadata);
                }
                
                duties.push(duty);
            }
            Ok(duties)
        } else if let Some(_duties_arr) = duties_json.as_array() {
            serde_json::from_value(duties_json.clone())
                .context("Failed to deserialize duties array from Nickel config")
        } else {
            Err(anyhow::anyhow!("'duties' must be an object or array"))
        }?;

        // Attach instructions to duties that have them
        for duty in &mut duties {
            let duty_instructions: Vec<_> = instruction_context.instructions.iter()
                .filter(|inst| inst.target_path.starts_with(&format!("duties.{}", duty.name)))
                .cloned()
                .collect();

            if !duty_instructions.is_empty() {
                let mut metadata = duty.metadata.clone().unwrap_or_else(|| serde_json::json!({}));
                if let Some(meta_obj) = metadata.as_object_mut() {
                    meta_obj.insert("instructions".to_string(), serde_json::json!(duty_instructions));
                }
                duty.metadata = Some(metadata);
            }
        }

        Ok(duties)
    }

    pub fn load_duties_with_runtime_context(
        &self,
        runtime_context: &HashMap<String, JsonValue>,
    ) -> Result<Vec<Duty>> {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let source = std::fs::read_to_string(&self.config_path)
            .with_context(|| format!("Failed to read {}", self.config_path))?;

        let runtime_nickel = Self::json_to_nickel(runtime_context)?;

        let augmented_source = format!(
            r#"
let runtime = {} in
{}
"#,
            runtime_nickel, source
        );

        let config_path = Path::new(&self.config_path);
        let config_dir = config_path.parent()
            .ok_or_else(|| anyhow::anyhow!("Config path has no parent directory"))?;
        
        let mut temp_file = NamedTempFile::new_in(config_dir)
            .context("Failed to create temporary file for augmented source")?;
        temp_file.write_all(augmented_source.as_bytes())
            .context("Failed to write augmented source to temp file")?;
        temp_file.flush()?;

        let temp_path = temp_file.path();
        let temp_filename = temp_path.file_name()
            .ok_or_else(|| anyhow::anyhow!("Temp file has no filename"))?;
        
        let output = Command::new("nickel")
            .arg("export")
            .arg(temp_filename)
            .current_dir(config_dir)
            .output()
            .context("Failed to execute nickel CLI with runtime context")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Nickel evaluation with runtime context failed: {}", stderr));
        }

        let json_str = String::from_utf8(output.stdout)
            .context("Nickel output is not valid UTF-8")?;

        let config: JsonValue = serde_json::from_str(&json_str)
            .context("Failed to parse Nickel output as JSON")?;

        let duties_json = config
            .get("duties")
            .ok_or_else(|| anyhow::anyhow!("Config missing 'duties' field"))?;

        if let Some(duties_obj) = duties_json.as_object() {
            let mut duties = Vec::new();
            for (name, duty_config) in duties_obj {
                let mut duty: Duty = serde_json::from_value(duty_config.clone())
                    .with_context(|| format!("Failed to deserialize duty '{}'", name))?;
                
                if duty.name.is_empty() {
                    duty.name = name.clone();
                }
                
                if let Some(depends_on) = duty_config.get("depends_on") {
                    let mut metadata = duty.metadata.unwrap_or_else(|| json!({}));
                    if let Some(meta_obj) = metadata.as_object_mut() {
                        meta_obj.insert("depends_on".to_string(), depends_on.clone());
                    }
                    duty.metadata = Some(metadata);
                }
                
                duties.push(duty);
            }
            Ok(duties)
        } else if let Some(_duties_arr) = duties_json.as_array() {
            serde_json::from_value(duties_json.clone())
                .context("Failed to deserialize duties array from Nickel config")
        } else {
            Err(anyhow::anyhow!("'duties' must be an object or array"))
        }
    }

    pub fn load_duties_with_instructions(&self) -> Result<Vec<Duty>> {
        let source = std::fs::read_to_string(&self.config_path)
            .with_context(|| format!("Failed to read {}", self.config_path))?;

        let (processed_source, instruction_context) = self.process_g8r_functions(&source)?;

        // Evaluate the processed Nickel source
        let config = self.evaluate_nickel_source(&processed_source)?;
        
        let duties_json = config
            .get("duties")
            .ok_or_else(|| anyhow::anyhow!("Config missing 'duties' field"))?;

        let mut duties = if let Some(duties_obj) = duties_json.as_object() {
            let mut duties = Vec::new();
            for (name, duty_config) in duties_obj {
                let mut duty: Duty = serde_json::from_value(duty_config.clone())
                    .with_context(|| format!("Failed to deserialize duty '{}'", name))?;
                
                if duty.name.is_empty() {
                    duty.name = name.clone();
                }
                
                // Add dependency metadata if present
                if let Some(depends_on) = duty_config.get("depends_on") {
                    let mut metadata = duty.metadata.unwrap_or_else(|| json!({}));
                    if let Some(meta_obj) = metadata.as_object_mut() {
                        meta_obj.insert("depends_on".to_string(), depends_on.clone());
                    }
                    duty.metadata = Some(metadata);
                }
                
                duties.push(duty);
            }
            Ok(duties)
        } else if let Some(_duties_arr) = duties_json.as_array() {
            serde_json::from_value(duties_json.clone())
                .context("Failed to deserialize duties array from Nickel config")
        } else {
            Err(anyhow::anyhow!("'duties' must be an object or array"))
        }?;

        // Attach instructions to duties that have them
        for duty in &mut duties {
            let duty_instructions: Vec<_> = instruction_context.instructions.iter()
                .filter(|inst| inst.target_path.starts_with(&format!("duties.{}", duty.name)))
                .cloned()
                .collect();

            if !duty_instructions.is_empty() {
                let mut metadata = duty.metadata.clone().unwrap_or_else(|| json!({}));
                if let Some(meta_obj) = metadata.as_object_mut() {
                    meta_obj.insert("instructions".to_string(), json!(duty_instructions));
                }
                duty.metadata = Some(metadata);
            }
        }

        Ok(duties)
    }

    /// Process g8r variable functions with KV store support
    async fn process_g8r_variable_functions(&self, source: &str, var_context: &VariableContext) -> Result<(String, InstructionContext)> {
        let mut instruction_context = InstructionContext::new();
        let mut processed_source = source.to_string();

        // Regex patterns for g8r variable functions
        let patterns = vec![
            // Existing functions
            (r#"g8r_output\s*\(\s*"([^"]+)"\s*,\s*"([^"]+)"\s*\)"#, "g8r_output"),
            (r#"g8r_secret\s*\(\s*"([^"]+)"\s*\)"#, "g8r_secret"),
            (r#"g8r_env\s*\(\s*"([^"]+)"\s*\)"#, "g8r_env"),
            
            // New variable functions
            (r#"g8r_get\s*\(\s*"([^"]+)"\s*\)"#, "g8r_get"),
            (r#"g8r_set\s*\(\s*"([^"]+)"\s*,\s*([^)]+)\s*\)"#, "g8r_set"),
            (r#"g8r_global\s*\(\s*"([^"]+)"\s*\)"#, "g8r_global"),
            (r#"g8r_const\s*\(\s*"([^"]+)"\s*\)"#, "g8r_const"),
        ];

        for (pattern, function_type) in patterns {
            let regex = Regex::new(pattern).context("Failed to compile regex")?;
            
            // Find all matches and replace them
            let mut replacements = Vec::new();
            for captures in regex.captures_iter(&processed_source) {
                let full_match = captures.get(0).unwrap();
                let args: Vec<String> = captures.iter()
                    .skip(1)
                    .filter_map(|m| m.map(|m| m.as_str().to_string()))
                    .collect();

                let replacement_value = match function_type {
                    "g8r_get" => {
                        // Resolve variable from KV context
                        let key = &args[0];
                        match var_context.resolve(key).await? {
                            Some(value) => serde_json::to_string(&value)
                                .context("Failed to serialize resolved value")?,
                            None => {
                                return Err(anyhow::anyhow!("Variable '{}' not found in any context", key));
                            }
                        }
                    }
                    "g8r_global" => {
                        // Get global variable directly
                        let key = &args[0];
                        match var_context.global_store.get(key).await? {
                            Some(var) => serde_json::to_string(&var.value)
                                .context("Failed to serialize global variable")?,
                            None => {
                                return Err(anyhow::anyhow!("Global variable '{}' not found", key));
                            }
                        }
                    }
                    "g8r_const" => {
                        // Get constant value
                        let key = &args[0];
                        match var_context.constants.get(key) {
                            Some(value) => serde_json::to_string(value)
                                .context("Failed to serialize constant")?,
                            None => {
                                return Err(anyhow::anyhow!("Constant '{}' not found", key));
                            }
                        }
                    }
                    "g8r_set" => {
                        // g8r_set should not be used in evaluation - it's for runtime updates
                        return Err(anyhow::anyhow!(
                            "g8r_set() cannot be used in configuration - use runtime variable setting instead"
                        ));
                    }
                    _ => {
                        // Handle existing functions with instruction tokens
                        let token = instruction_context.add_instruction(
                            function_type.to_string(),
                            args,
                            "".to_string(), // Will be filled in later when we know the JSON path
                            None,
                        );
                        format!("\"{}\"", token)
                    }
                };

                replacements.push((full_match.start(), full_match.end(), replacement_value));
            }

            // Apply replacements in reverse order to maintain indices
            replacements.sort_by(|a, b| b.0.cmp(&a.0));
            for (start, end, replacement) in replacements {
                processed_source.replace_range(start..end, &replacement);
            }
        }

        Ok((processed_source, instruction_context))
    }

    fn process_g8r_functions(&self, source: &str) -> Result<(String, InstructionContext)> {
        let mut instruction_context = InstructionContext::new();
        let mut processed_source = source.to_string();

        // Regex patterns for g8r function calls
        let patterns = vec![
            (r#"g8r_output\s*\(\s*"([^"]+)"\s*,\s*"([^"]+)"\s*\)"#, "g8r_output"),
            (r#"g8r_secret\s*\(\s*"([^"]+)"\s*\)"#, "g8r_secret"),
            (r#"g8r_env\s*\(\s*"([^"]+)"\s*\)"#, "g8r_env"),
        ];

        for (pattern, function_type) in patterns {
            let regex = Regex::new(pattern).context("Failed to compile regex")?;
            
            // Find all matches and replace them
            let mut replacements = Vec::new();
            for captures in regex.captures_iter(&processed_source) {
                let full_match = captures.get(0).unwrap();
                let args: Vec<String> = captures.iter()
                    .skip(1)
                    .filter_map(|m| m.map(|m| m.as_str().to_string()))
                    .collect();

                // Generate a placeholder token
                let token = instruction_context.add_instruction(
                    function_type.to_string(),
                    args,
                    "".to_string(), // Will be filled in later when we know the JSON path
                    None,
                );

                replacements.push((full_match.start(), full_match.end(), format!("\"{}\"", token)));
            }

            // Apply replacements in reverse order to maintain indices
            replacements.sort_by(|a, b| b.0.cmp(&a.0));
            for (start, end, replacement) in replacements {
                processed_source.replace_range(start..end, &replacement);
            }
        }

        Ok((processed_source, instruction_context))
    }

    fn evaluate_nickel_source(&self, source: &str) -> Result<JsonValue> {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let config_path = Path::new(&self.config_path);
        let config_dir = config_path.parent()
            .ok_or_else(|| anyhow::anyhow!("Config path has no parent directory"))?;
        
        let mut temp_file = NamedTempFile::new_in(config_dir)
            .context("Failed to create temporary file for processed source")?;
        temp_file.write_all(source.as_bytes())
            .context("Failed to write processed source to temp file")?;
        temp_file.flush()?;

        let temp_path = temp_file.path();
        let temp_filename = temp_path.file_name()
            .ok_or_else(|| anyhow::anyhow!("Temp file has no filename"))?;
        
        let output = Command::new("nickel")
            .arg("export")
            .arg(temp_filename)
            .current_dir(config_dir)
            .output()
            .context("Failed to execute nickel CLI with processed source")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Nickel evaluation with processed source failed: {}", stderr));
        }

        let json_str = String::from_utf8(output.stdout)
            .context("Nickel output is not valid UTF-8")?;

        serde_json::from_str(&json_str)
            .context("Failed to parse Nickel output as JSON")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_simple_config() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
{{
  roster = {{
    name = "test-roster",
    roster_type = "aws-account",
    traits = ["cloud-provider", "aws"],
    connection = {{ region = "us-east-1" }},
  }},
  duties = {{
    test-duty = {{
      type = "S3Bucket",
      spec = {{ bucket_name = "test-bucket" }},
    }}
  }}
}}
"#
        )
        .unwrap();

        let evaluator = NickelEvaluator::from_path(file.path()).unwrap();
        let config = evaluator.load_config().unwrap();

        assert!(config.get("roster").is_some());
        assert!(config.get("duties").is_some());
    }

    #[test]
    fn test_load_roster() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
{{
  roster = {{
    name = "prod-aws",
    roster_type = "aws-account",
    traits = ["cloud-provider", "aws"],
    connection = {{ region = "us-west-2" }},
  }},
  duties = {{}}
}}
"#
        )
        .unwrap();

        let evaluator = NickelEvaluator::from_path(file.path()).unwrap();
        let roster = evaluator.load_roster().unwrap();

        assert_eq!(roster.name, "prod-aws");
        assert_eq!(roster.roster_type, "aws-account");
        assert!(roster.traits.contains(&"aws".to_string()));
    }

    #[test]
    fn test_load_duties() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
{{
  roster = {{ name = "test", roster_type = "aws-account", traits = [], connection = {{}} }},
  duties = {{
    bucket = {{
      type = "S3Bucket",
      roster = {{ traits = ["aws"] }},
      spec = {{ bucket_name = "my-bucket" }},
    }}
  }}
}}
"#
        )
        .unwrap();

        let evaluator = NickelEvaluator::from_path(file.path()).unwrap();
        let duties = evaluator.load_duties().unwrap();

        assert_eq!(duties.len(), 1);
        assert_eq!(duties[0].name, "bucket");
        assert_eq!(duties[0].duty_type, "S3Bucket");
    }

    #[test]
    fn test_runtime_context_injection() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
{{
  roster = {{ name = "test", roster_type = "aws-account", traits = [], connection = {{}} }},
  duties = {{
    cdn = {{
      type = "CloudFrontDistribution",
      roster = {{ traits = ["aws"] }},
      spec = {{
        origin = {{
          domain_name = runtime.duties.bucket.outputs.website_endpoint
        }}
      }},
    }}
  }}
}}
"#
        )
        .unwrap();

        let evaluator = NickelEvaluator::from_path(file.path()).unwrap();

        let mut runtime_context = HashMap::new();
        runtime_context.insert(
            "duties".to_string(),
            serde_json::json!({
                "bucket": {
                    "outputs": {
                        "website_endpoint": "my-bucket.s3-website.us-east-1.amazonaws.com"
                    }
                }
            }),
        );

        let duties = evaluator
            .load_duties_with_runtime_context(&runtime_context)
            .unwrap();

        assert_eq!(duties.len(), 1);
        let cdn_spec = &duties[0].spec;
        assert_eq!(
            cdn_spec["origin"]["domain_name"].as_str().unwrap(),
            "my-bucket.s3-website.us-east-1.amazonaws.com"
        );
    }

    #[tokio::test]
    async fn test_variable_context_resolution() {
        use crate::kv::{GlobalKvStore, VariableContext};
        use std::collections::HashMap;

        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
{{
  roster = {{ name = "test", roster_type = "aws-account", traits = [], connection = {{}} }},
  duties = {{
    cdn = {{
      type = "CloudFrontDistribution",
      roster = {{ traits = ["aws"] }},
      spec = {{
        origin = {{
          domain_name = g8r_get("bucket_endpoint")
        }},
        api_key = g8r_global("api_key"),
        debug_mode = g8r_const("debug")
      }},
    }}
  }}
}}
"#
        )
        .unwrap();

        let evaluator = NickelEvaluator::from_path(file.path()).unwrap();
        
        // Set up variable context
        let global_store = GlobalKvStore::new_in_memory();
        global_store.set_json("api_key", serde_json::json!("secret123"), None).await.unwrap();
        
        let mut var_context = VariableContext::new("test-stack".to_string(), global_store);
        
        // Add constants
        let mut constants = HashMap::new();
        constants.insert("debug".to_string(), serde_json::json!(true));
        var_context.add_constants(constants);
        
        // Add stack variable
        var_context.stack_context.set_from_json("bucket_endpoint", 
            &serde_json::json!("bucket.s3-website.us-east-1.amazonaws.com")).unwrap();

        let duties = evaluator.load_duties_with_variable_context(&var_context).await.unwrap();

        assert_eq!(duties.len(), 1);
        let cdn_spec = &duties[0].spec;
        
        // Verify that variables were resolved
        assert_eq!(
            cdn_spec["origin"]["domain_name"].as_str().unwrap(),
            "bucket.s3-website.us-east-1.amazonaws.com"
        );
        assert_eq!(
            cdn_spec["api_key"].as_str().unwrap(),
            "secret123"
        );
        assert_eq!(
            cdn_spec["debug_mode"].as_bool().unwrap(),
            true
        );
    }

    #[test]
    fn test_instruction_detection() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
{{
  roster = {{ name = "test", roster_type = "aws-account", traits = [], connection = {{}} }},
  duties = {{
    cdn = {{
      type = "CloudFrontDistribution",
      roster = {{ traits = ["aws"] }},
      spec = {{
        origin = {{
          domain_name = g8r_output("bucket", "website_endpoint")
        }},
        api_key = g8r_secret("api_key")
      }},
    }}
  }}
}}
"#
        )
        .unwrap();

        let evaluator = NickelEvaluator::from_path(file.path()).unwrap();
        let duties = evaluator.load_duties_with_instructions().unwrap();

        assert_eq!(duties.len(), 1);
        assert_eq!(duties[0].name, "cdn");
        
        // Check that instructions were attached to the duty
        let metadata = duties[0].metadata.as_ref().unwrap();
        let instructions = metadata.get("instructions").unwrap().as_array().unwrap();
        assert_eq!(instructions.len(), 2);

        // Check that the spec contains instruction tokens
        let cdn_spec = &duties[0].spec;
        let domain_name = cdn_spec["origin"]["domain_name"].as_str().unwrap();
        let api_key = cdn_spec["api_key"].as_str().unwrap();
        
        assert!(domain_name.starts_with("__INSTRUCTION_"));
        assert!(api_key.starts_with("__INSTRUCTION_"));
    }
}
