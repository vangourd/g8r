use anyhow::{Context, Result};
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use crate::utils::{Duty, Roster};

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
}
