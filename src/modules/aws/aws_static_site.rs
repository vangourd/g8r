use async_trait::async_trait;
use anyhow::{Result, anyhow};
use serde_json::{json, Value as JsonValue};
use tracing::instrument;

use crate::utils::{Duty, Roster};
use crate::modules::AutomationModule;

pub struct AwsStaticSiteModule;

impl AwsStaticSiteModule {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl AutomationModule for AwsStaticSiteModule {
    fn name(&self) -> &str {
        "aws-static-site"
    }
    
    fn supported_duty_types(&self) -> Vec<&str> {
        vec!["StaticSite"]
    }
    
    fn required_roster_traits(&self) -> Vec<&str> {
        vec!["cloud-provider", "aws"]
    }
    
    #[instrument(skip(self, _roster, duty), fields(duty_name = %duty.name, duty_type = %duty.duty_type))]
    async fn validate(&self, _roster: &Roster, duty: &Duty) -> Result<()> {
        if !self.supported_duty_types().contains(&duty.duty_type.as_str()) {
            return Err(anyhow!(
                "Duty type '{}' not supported by {}. Supported types: {:?}",
                duty.duty_type,
                self.name(),
                self.supported_duty_types()
            ));
        }
        
        let spec = &duty.spec;
        
        if spec.get("site").is_none() {
            return Err(anyhow!("Missing required field: 'site' in spec"));
        }
        
        let site = spec.get("site").unwrap();
        
        if site.get("domain").is_none() {
            return Err(anyhow!("Missing required field: 'site.domain' in spec"));
        }
        
        Ok(())
    }
    
    #[instrument(skip(self, roster, duty), fields(roster_name = %roster.name, duty_name = %duty.name))]
    async fn apply(&self, roster: &Roster, duty: &Duty) -> Result<JsonValue> {
        let domain = duty.spec["site"]["domain"]
            .as_str()
            .ok_or_else(|| anyhow!("domain must be a string"))?;
        
        Ok(json!({
            "phase": "deployed",
            "message": format!(
                "Would deploy static site '{}' to domain '{}' using roster '{}'",
                duty.name,
                domain,
                roster.name
            ),
            "resources": [
                {
                    "resource_type": "s3_bucket",
                    "resource_id": format!("{}-bucket", duty.name),
                    "arn": format!("arn:aws:s3:::{}-bucket", duty.name),
                },
                {
                    "resource_type": "cloudfront_distribution",
                    "resource_id": "E1234567890ABC",
                    "arn": null,
                },
            ],
        }))
    }
    
    async fn validate_duty(&self, duty: &Duty) -> Result<()> {
        let spec = &duty.spec;
        
        if spec.get("site").is_none() {
            return Err(anyhow!("Missing required field: 'site' in spec"));
        }
        
        let site = spec.get("site").unwrap();
        
        if site.get("domain").is_none() {
            return Err(anyhow!("Missing required field: 'site.domain' in spec"));
        }
        
        Ok(())
    }
    
    async fn check_state(&self, _roster: &Roster, _duty: &Duty) -> Result<crate::modules::DutyState> {
        // For static sites, we'll assume they're deployed if they have a domain
        // More sophisticated checking would query AWS resources
        Ok(crate::modules::DutyState::Deployed)
    }

    #[instrument(skip(self, _roster, duty))]
    async fn destroy(&self, _roster: &Roster, duty: &Duty) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    fn create_test_roster() -> Roster {
        Roster {
            id: Some(1),
            name: "test-aws-account".to_string(),
            roster_type: "aws-account".to_string(),
            traits: vec!["cloud-provider".to_string(), "aws".to_string()],
            connection: json!({"region": "us-east-1"}),
            auth: json!({"type": "iam-user"}),
            metadata: None,
            created_at: None,
            updated_at: None,
        }
    }
    
    fn create_test_duty() -> Duty {
        Duty {
            id: Some(1),
            name: "test-site".to_string(),
            duty_type: "StaticSite".to_string(),
            backend: "aws".to_string(),
            roster_selector: json!({"traits": ["cloud-provider", "aws"]}),
            spec: json!({
                "site": {
                    "domain": "test.example.com"
                }
            }),
            status: None,
            metadata: None,
            created_at: None,
            updated_at: None,
        }
    }
    
    #[tokio::test]
    async fn test_module_name() {
        let module = AwsStaticSiteModule::new();
        assert_eq!(module.name(), "aws-static-site");
    }
    
    #[tokio::test]
    async fn test_supported_duty_types() {
        let module = AwsStaticSiteModule::new();
        assert_eq!(module.supported_duty_types(), vec!["StaticSite"]);
    }
    
    #[tokio::test]
    async fn test_required_roster_traits() {
        let module = AwsStaticSiteModule::new();
        assert_eq!(module.required_roster_traits(), vec!["cloud-provider", "aws"]);
    }
    
    #[tokio::test]
    async fn test_validate_duty_success() {
        let module = AwsStaticSiteModule::new();
        let duty = create_test_duty();
        
        let roster = create_test_roster();
        let result = module.validate(&roster, &duty).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_validate_duty_missing_site() {
        let module = AwsStaticSiteModule::new();
        let mut duty = create_test_duty();
        duty.spec = json!({});
        
        let roster = create_test_roster();
        let result = module.validate(&roster, &duty).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing required field: 'site'"));
    }
    
    #[tokio::test]
    async fn test_validate_duty_missing_domain() {
        let module = AwsStaticSiteModule::new();
        let mut duty = create_test_duty();
        duty.spec = json!({"site": {}});
        
        let roster = create_test_roster();
        let result = module.validate(&roster, &duty).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing required field: 'site.domain'"));
    }
    
    #[tokio::test]
    async fn test_validate_duty_wrong_type() {
        let module = AwsStaticSiteModule::new();
        let mut duty = create_test_duty();
        duty.duty_type = "Database".to_string();
        
        let roster = create_test_roster();
        let result = module.validate(&roster, &duty).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not supported"));
    }
    
    #[tokio::test]
    async fn test_apply() {
        let module = AwsStaticSiteModule::new();
        let roster = create_test_roster();
        let duty = create_test_duty();
        
        let result = module.apply(&roster, &duty).await.unwrap();
        assert!(result.get("phase").is_some());
        assert_eq!(result["phase"].as_str().unwrap(), "deployed");
    }
    
    #[tokio::test]
    async fn test_destroy() {
        let module = AwsStaticSiteModule::new();
        let roster = create_test_roster();
        let duty = create_test_duty();
        
        let result = module.destroy(&roster, &duty).await;
        assert!(result.is_ok());
    }
}
