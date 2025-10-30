use anyhow::{Context, Result};
use async_trait::async_trait;
use aws_sdk_acm::Client as AcmClient;
use serde_json::{json, Value as JsonValue};

use super::traits::ACMOperations;
use crate::modules::aws::utils::retry_with_backoff;

pub struct ACMModule {
    client: AcmClient,
}

impl ACMModule {
    pub fn new(client: AcmClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl ACMOperations for ACMModule {
    async fn request_certificate(&self, domain: &str, sans: Vec<String>) -> Result<String> {
        let mut request = self.client
            .request_certificate()
            .domain_name(domain)
            .validation_method(aws_sdk_acm::types::ValidationMethod::Dns);

        for san in sans {
            request = request.subject_alternative_names(san);
        }

        let result = request
            .send()
            .await
            .context("Failed to request ACM certificate")?;

        let arn = result.certificate_arn()
            .context("No certificate ARN in response")?
            .to_string();

        Ok(arn)
    }

    async fn get_certificate(&self, arn: &str) -> Result<Option<JsonValue>> {
        match self.client.describe_certificate().certificate_arn(arn).send().await {
            Ok(result) => {
                let _cert = result.certificate().context("No certificate in response")?;
                Ok(Some(json!({"status": "ok"})))
            }
            Err(e) if e.to_string().contains("ResourceNotFoundException") => Ok(None),
            Err(e) => Err(anyhow::anyhow!("Failed to get certificate: {}", e)),
        }
    }

    async fn get_certificate_validation_records(&self, arn: &str) -> Result<Vec<(String, String)>> {
        retry_with_backoff(
            || async {
                let result = self.client
                    .describe_certificate()
                    .certificate_arn(arn)
                    .send()
                    .await
                    .context("Failed to describe certificate")?;

                let cert = result.certificate().context("No certificate in response")?;
                let validation_options = cert.domain_validation_options();

                let mut records = Vec::new();
                for option in validation_options {
                    if let Some(record) = option.resource_record() {
                        let name = record.name().to_string();
                        let value = record.value().to_string();
                        records.push((name, value));
                    }
                }

                if records.is_empty() {
                    anyhow::bail!("No DNS validation records available yet");
                }

                Ok(records)
            },
            10,
            "fetch ACM validation records",
        ).await
    }

    async fn wait_for_validation(&self, arn: &str, timeout_secs: u64) -> Result<()> {
        use std::time::{Duration, Instant};
        
        let start = Instant::now();
        let timeout = Duration::from_secs(timeout_secs);
        let mut wait_secs = 5;

        loop {
            if start.elapsed() > timeout {
                anyhow::bail!("Certificate validation timed out after {} seconds", timeout_secs);
            }

            let result = self.client
                .describe_certificate()
                .certificate_arn(arn)
                .send()
                .await
                .context("Failed to describe certificate")?;

            let cert = result.certificate().context("No certificate in response")?;
            let status = cert.status().context("No certificate status")?;

            match status {
                aws_sdk_acm::types::CertificateStatus::Issued => {
                    log::info!("Certificate validated successfully");
                    return Ok(());
                }
                aws_sdk_acm::types::CertificateStatus::Failed => {
                    anyhow::bail!("Certificate validation failed");
                }
                aws_sdk_acm::types::CertificateStatus::PendingValidation => {
                    log::info!("Waiting for certificate validation... ({}s elapsed)", start.elapsed().as_secs());
                    tokio::time::sleep(Duration::from_secs(wait_secs)).await;
                    wait_secs = std::cmp::min(wait_secs * 2, 30); // exponential backoff, max 30s
                }
                _ => {
                    log::warn!("Unexpected certificate status: {:?}", status);
                    tokio::time::sleep(Duration::from_secs(wait_secs)).await;
                }
            }
        }
    }

    async fn delete_certificate(&self, arn: &str) -> Result<()> {
        self.client
            .delete_certificate()
            .certificate_arn(arn)
            .send()
            .await
            .context("Failed to delete ACM certificate")?;
        
        Ok(())
    }
}