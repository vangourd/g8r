use anyhow::{Result, Context};
use async_trait::async_trait;
use serde_json::{json, Value as JsonValue};
use log::info;

use crate::modules::AutomationModule;
use crate::utils::{Duty, Roster};
use crate::db::StateManager;
use crate::modules::aws::clients::acm::ACMModule as AwsACMModule;
use crate::modules::aws::clients::traits::ACMOperations;
use aws_sdk_acm::Client as AcmClient;

pub struct ACMCertificateModule {
    state: StateManager,
}

impl ACMCertificateModule {
    pub fn new(state: StateManager) -> Self {
        Self { state }
    }

    async fn get_acm_client(&self, _roster: &Roster) -> Result<AwsACMModule> {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new("us-east-1".to_string()))
            .load()
            .await;

        let client = AcmClient::new(&config);
        Ok(AwsACMModule::new(client))
    }
}

#[async_trait]
impl AutomationModule for ACMCertificateModule {
    fn name(&self) -> &str {
        "acm-certificate"
    }

    fn supported_duty_types(&self) -> Vec<&str> {
        vec!["ACMCertificate"]
    }

    fn required_roster_traits(&self) -> Vec<&str> {
        vec!["cloud-provider", "aws"]
    }

    async fn validate(&self, _roster: &Roster, duty: &Duty) -> Result<()> {
        let spec = &duty.spec;
        
        if spec.get("domain_name").and_then(|v| v.as_str()).is_none() {
            anyhow::bail!("ACMCertificate duty requires 'domain_name' in spec");
        }

        Ok(())
    }

    async fn apply(&self, roster: &Roster, duty: &Duty) -> Result<JsonValue> {
        let spec = &duty.spec;
        let domain_name = spec["domain_name"].as_str()
            .ok_or_else(|| anyhow::anyhow!("domain_name is required"))?;
        
        let sans = spec.get("subject_alternative_names")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter()
                .filter_map(|v| v.as_str())
                .map(String::from)
                .collect::<Vec<_>>())
            .unwrap_or_default();

        let acm = self.get_acm_client(roster).await?;

        let existing_arn = duty.status.as_ref()
            .and_then(|s| s.get("outputs"))
            .and_then(|o| o.get("arn"))
            .and_then(|v| v.as_str());

        let certificate_arn = if let Some(arn) = existing_arn {
            info!("Certificate already exists: {}, checking validation status", arn);
            arn.to_string()
        } else {
            info!("Requesting new ACM certificate for domain '{}'", domain_name);
            let arn = acm.request_certificate(domain_name, sans.clone()).await
                .context("Failed to request certificate")?;
            info!("Certificate requested: {}", arn);
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            arn
        };

        info!("Fetching DNS validation records");
        let validation_records = acm.get_certificate_validation_records(&certificate_arn).await
            .context("Failed to get validation records")?;

        let validation_records_json: Vec<_> = validation_records.iter()
            .map(|(name, value)| json!({
                "name": name,
                "value": value,
                "type": "CNAME"
            }))
            .collect();

        if existing_arn.is_some() {
            info!("Certificate already requested, waiting for validation (up to 5 minutes)");
            match acm.wait_for_validation(&certificate_arn, 300).await {
                Ok(_) => {
                    info!("Certificate validated successfully");
                    return Ok(json!({
                        "phase": "deployed",
                        "message": format!("ACM certificate validated for '{}'", domain_name),
                        "outputs": {
                            "arn": certificate_arn,
                            "domain_name": domain_name,
                            "validation_records": validation_records_json,
                            "status": "ISSUED",
                        }
                    }));
                }
                Err(e) => {
                    info!("Certificate validation still pending: {}", e);
                }
            }
        } else {
            info!("Certificate requested, returning validation records for DNS setup");
        }

        Ok(json!({
            "phase": "pending_validation",
            "message": format!("ACM certificate requested for '{}'. Create DNS validation record and sync again.", domain_name),
            "outputs": {
                "arn": certificate_arn,
                "domain_name": domain_name,
                "validation_records": validation_records_json,
                "status": "PENDING_VALIDATION",
            }
        }))
    }

    async fn destroy(&self, roster: &Roster, duty: &Duty) -> Result<()> {
        let certificate_arn = duty.status.as_ref()
            .and_then(|s| s.get("outputs"))
            .and_then(|o| o.get("arn"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("No certificate ARN found in duty status"))?;
        
        info!("Destroying ACM certificate: {}", certificate_arn);
        
        let acm = self.get_acm_client(roster).await?;
        
        if acm.get_certificate(certificate_arn).await?.is_none() {
            info!("ACM certificate '{}' does not exist, skipping deletion", certificate_arn);
            return Ok(());
        }
        
        info!("Deleting ACM certificate '{}'", certificate_arn);
        acm.delete_certificate(certificate_arn).await
            .context("Failed to delete ACM certificate")?;
        
        info!("Successfully destroyed ACM certificate: {}", certificate_arn);
        Ok(())
    }
}
