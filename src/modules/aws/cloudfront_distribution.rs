use anyhow::{Context, Result};
use async_trait::async_trait;
use aws_config::BehaviorVersion;
use serde_json::{json, Value as JsonValue};
use tracing::info;

use crate::modules::AutomationModule;
use crate::utils::{Duty, Roster};
use crate::db::StateManager;
use crate::modules::aws::clients::cloudfront::CloudFrontModule;
use crate::modules::aws::clients::traits::CloudFrontOperations;

pub struct CloudFrontDistributionModule {
    state: StateManager,
}

impl CloudFrontDistributionModule {
    pub fn new(state: StateManager) -> Self {
        Self { state }
    }

    async fn get_cloudfront_client(&self, roster: &Roster) -> Result<CloudFrontModule> {
        let region = roster.connection.get("region")
            .and_then(|v| v.as_str())
            .unwrap_or("us-east-1");

        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(aws_config::Region::new(region.to_string()))
            .load()
            .await;

        let client = aws_sdk_cloudfront::Client::new(&config);
        Ok(CloudFrontModule::new(client))
    }
}

#[async_trait]
impl AutomationModule for CloudFrontDistributionModule {
    fn name(&self) -> &str {
        "cloudfront-distribution"
    }

    fn supported_duty_types(&self) -> Vec<&str> {
        vec!["CloudFrontDistribution"]
    }

    fn required_roster_traits(&self) -> Vec<&str> {
        vec!["cloud-provider", "aws"]
    }

    async fn validate(&self, _roster: &Roster, duty: &Duty) -> Result<()> {
        let spec = &duty.spec;
        
        if spec.get("origin").is_none() {
            anyhow::bail!("CloudFrontDistribution duty requires 'origin' in spec");
        }

        Ok(())
    }

    async fn apply(&self, roster: &Roster, duty: &Duty) -> Result<JsonValue> {
        let spec = &duty.spec;
        let origin = spec.get("origin")
            .ok_or_else(|| anyhow::anyhow!("origin is required"))?;
        
        let domain_name = origin.get("domain_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("origin.domain_name is required"))?;

        let certificate_arn = spec.get("certificate_arn")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("certificate_arn is required"))?;

        let aliases = spec.get("aliases")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("aliases is required"))?;

        // Check if certificate is validated before creating CloudFront distribution
        let cert_region = "us-east-1"; // CloudFront requires us-east-1
        let acm_config = aws_config::defaults(BehaviorVersion::latest())
            .region(aws_config::Region::new(cert_region.to_string()))
            .load()
            .await;
        let acm_client = aws_sdk_acm::Client::new(&acm_config);
        
        let cert_result = acm_client
            .describe_certificate()
            .certificate_arn(certificate_arn)
            .send()
            .await
            .context("Failed to describe ACM certificate")?;
        
        let cert = cert_result.certificate()
            .context("No certificate in response")?;
        let cert_status = cert.status()
            .context("Certificate has no status")?;
        
        if cert_status.as_str() != "ISSUED" {
            info!("CloudFront waiting for certificate validation (current status: {})", cert_status.as_str());
            return Ok(json!({
                "phase": "pending",
                "message": format!("Waiting for ACM certificate to be validated (current status: {})", cert_status.as_str()),
                "outputs": {}
            }));
        }

        // Check if distribution already exists (idempotency)
        let existing_distribution_id = duty.status.as_ref()
            .and_then(|s| s.get("outputs"))
            .and_then(|o| o.get("distribution_id"))
            .and_then(|v| v.as_str());

        if let Some(dist_id) = existing_distribution_id {
            info!("CloudFront distribution already exists: {}", dist_id);
            let cloudfront = self.get_cloudfront_client(roster).await?;
            if let Some(domain) = cloudfront.get_distribution(dist_id).await? {
                let arn = format!(
                    "arn:aws:cloudfront::{}:distribution/{}",
                    roster.connection.get("account_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("123456789012"),
                    dist_id
                );
                
                return Ok(json!({
                    "phase": "deployed",
                    "outputs": {
                        "distribution_id": dist_id,
                        "distribution_domain": domain,
                        "arn": arn,
                    }
                }));
            }
        }

        let cloudfront = self.get_cloudfront_client(roster).await?;

        let config = json!({
            "origin_domain": domain_name,
            "origin_id": format!("s3-{}", domain_name),
            "certificate_arn": certificate_arn,
            "aliases": aliases,
        });

        match cloudfront.create_distribution(config).await {
            Ok((distribution_id, cloudfront_domain)) => {
                let arn = format!(
                    "arn:aws:cloudfront::{}:distribution/{}",
                    roster.connection.get("account_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("123456789012"),
                    distribution_id
                );

                Ok(json!({
                    "phase": "deployed",
                    "outputs": {
                        "distribution_id": distribution_id,
                        "distribution_domain": cloudfront_domain,
                        "arn": arn,
                    }
                }))
            }
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("CertificateNotFound") || error_msg.contains("InvalidViewerCertificate") {
                    info!("CloudFront creation waiting for certificate validation");
                    Ok(json!({
                        "phase": "pending_validation",
                        "message": "Waiting for ACM certificate to be validated. Run sync again after certificate is issued.",
                        "outputs": {}
                    }))
                } else {
                    Err(e).context("Failed to create CloudFront distribution")
                }
            }
        }
    }

    async fn destroy(&self, roster: &Roster, duty: &Duty) -> Result<()> {
        let distribution_id = duty.status.as_ref()
            .and_then(|s| s.get("outputs"))
            .and_then(|o| o.get("distribution_id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("No distribution_id found in duty status"))?;
        
        info!("Destroying CloudFront distribution: {}", distribution_id);
        
        let cloudfront = self.get_cloudfront_client(roster).await?;
        
        if cloudfront.get_distribution(distribution_id).await?.is_none() {
            info!("CloudFront distribution '{}' does not exist, skipping deletion", distribution_id);
            return Ok(());
        }
        
        info!("Disabling CloudFront distribution '{}'", distribution_id);
        cloudfront.disable_distribution(distribution_id).await
            .context("Failed to disable CloudFront distribution")?;
        
        info!("Waiting for distribution '{}' to be disabled (this may take several minutes)...", distribution_id);
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        
        info!("Deleting CloudFront distribution '{}'", distribution_id);
        cloudfront.delete_distribution(distribution_id).await
            .context("Failed to delete CloudFront distribution")?;
        
        info!("Successfully destroyed CloudFront distribution: {}", distribution_id);
        Ok(())
    }

    async fn validate_duty(&self, duty: &Duty) -> Result<()> {
        let spec = &duty.spec;
        
        if spec.get("origin").is_none() {
            anyhow::bail!("CloudFrontDistribution duty requires 'origin' in spec");
        }

        Ok(())
    }
    
    async fn check_state(&self, roster: &Roster, duty: &Duty) -> Result<crate::modules::DutyState> {
        let distribution_id = duty.status.as_ref()
            .and_then(|s| s.get("outputs"))
            .and_then(|o| o.get("distribution_id"))
            .and_then(|v| v.as_str());
            
        if let Some(dist_id) = distribution_id {
            let cloudfront = self.get_cloudfront_client(roster).await?;
            if cloudfront.get_distribution(dist_id).await?.is_some() {
                Ok(crate::modules::DutyState::Deployed)
            } else {
                Ok(crate::modules::DutyState::NotExists)
            }
        } else {
            Ok(crate::modules::DutyState::NotExists)
        }
    }
}
