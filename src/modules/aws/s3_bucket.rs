use anyhow::{Result, Context};
use async_trait::async_trait;
use serde_json::{json, Value as JsonValue};
use log::info;
use std::time::Duration;
use tokio::time::sleep;

use crate::modules::AutomationModule;
use crate::utils::{Duty, Roster};
use crate::db::StateManager;
use crate::modules::aws::clients::s3::S3Module;
use crate::modules::aws::clients::traits::S3Operations;
use aws_sdk_s3::Client as S3Client;

pub struct S3BucketModule {
    state: StateManager,
}

impl S3BucketModule {
    pub fn new(state: StateManager) -> Self {
        Self { state }
    }

    async fn get_s3_client(&self, roster: &Roster) -> Result<S3Module> {
        let region = roster.connection.get("region")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Roster missing 'region' in connection"))?;

        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(region.to_string()))
            .load()
            .await;

        let client = S3Client::new(&config);
        Ok(S3Module::new(client))
    }
}

#[async_trait]
impl AutomationModule for S3BucketModule {
    fn name(&self) -> &str {
        "s3-bucket"
    }

    fn supported_duty_types(&self) -> Vec<&str> {
        vec!["S3Bucket"]
    }

    fn required_roster_traits(&self) -> Vec<&str> {
        vec!["cloud-provider", "aws"]
    }

    async fn validate(&self, _roster: &Roster, duty: &Duty) -> Result<()> {
        let spec = &duty.spec;
        
        if spec.get("bucket_name").and_then(|v| v.as_str()).is_none() {
            anyhow::bail!("S3Bucket duty requires 'bucket_name' in spec");
        }

        Ok(())
    }

    async fn apply(&self, roster: &Roster, duty: &Duty) -> Result<JsonValue> {
        let spec = &duty.spec;
        let bucket_name = spec["bucket_name"].as_str()
            .ok_or_else(|| anyhow::anyhow!("bucket_name is required"))?;
        
        let region = roster.connection.get("region")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Roster missing 'region' in connection"))?;

        let s3 = self.get_s3_client(roster).await?;

        info!("Checking if S3 bucket '{}' exists", bucket_name);
        let exists = s3.bucket_exists(bucket_name).await
            .context("Failed to check bucket existence")?;

        if !exists {
            info!("Creating S3 bucket '{}' in region '{}'", bucket_name, region);
            s3.create_bucket(bucket_name, region).await
                .context("Failed to create bucket")?;
            
            info!("Waiting for bucket to be ready (AWS eventual consistency)");
            sleep(Duration::from_secs(3)).await;
        } else {
            info!("S3 bucket '{}' already exists", bucket_name);
        }

        let website_config = spec.get("website_config");
        if let Some(config) = website_config {
            let index_document = config.get("index_document")
                .and_then(|v| v.as_str())
                .unwrap_or("index.html");
            let error_document = config.get("error_document")
                .and_then(|v| v.as_str())
                .unwrap_or("404.html");

            info!("Configuring website hosting for bucket '{}'", bucket_name);
            
            let max_retries = 5;
            let mut delay = Duration::from_secs(2);
            let mut last_error = None;
            
            for attempt in 1..=max_retries {
                match s3.configure_website(bucket_name, index_document, error_document).await {
                    Ok(_) => {
                        if attempt > 1 {
                            info!("Website configuration succeeded on attempt {}", attempt);
                        }
                        break;
                    }
                    Err(e) => {
                        last_error = Some(e);
                        if attempt < max_retries {
                            info!("Website configuration failed (attempt {}/{}), retrying in {:?}", 
                                  attempt, max_retries, delay);
                            sleep(delay).await;
                            delay = std::cmp::min(delay * 2, Duration::from_secs(30));
                        }
                    }
                }
            }
            
            if let Some(e) = last_error {
                return Err(e).context("Failed to configure website after retries");
            }
        }

        if let Some(true) = spec.get("versioning").and_then(|v| v.as_bool()) {
            info!("Enabling versioning for bucket '{}'", bucket_name);
            s3.enable_versioning(bucket_name).await
                .context("Failed to enable versioning")?;
        }

        if let Some(true) = spec.get("public_access").and_then(|v| v.as_bool()) {
            info!("Disabling public access block for bucket '{}'", bucket_name);
            s3.set_public_access_block(bucket_name, false).await
                .context("Failed to set public access block")?;

            let policy = json!({
                "Version": "2012-10-17",
                "Statement": [{
                    "Effect": "Allow",
                    "Principal": "*",
                    "Action": "s3:GetObject",
                    "Resource": format!("arn:aws:s3:::{}/*", bucket_name)
                }]
            }).to_string();

            info!("Setting public read policy for bucket '{}'", bucket_name);
            s3.set_bucket_policy(bucket_name, &policy).await
                .context("Failed to set bucket policy")?;
        }

        let website_endpoint = if website_config.is_some() {
            Some(s3.get_website_endpoint(bucket_name, region).await)
        } else {
            None
        };

        Ok(json!({
            "phase": "deployed",
            "message": format!("S3 bucket '{}' deployed in region '{}'", bucket_name, region),
            "resources": [{
                "resource_type": "s3_bucket",
                "resource_id": bucket_name,
                "arn": format!("arn:aws:s3:::{}", bucket_name),
                "properties": {
                    "region": region,
                    "website_enabled": website_config.is_some(),
                    "website_endpoint": website_endpoint,
                    "roster": &roster.name,
                }
            }],
            "outputs": {
                "bucket_name": bucket_name,
                "arn": format!("arn:aws:s3:::{}", bucket_name),
                "website_endpoint": website_endpoint,
            }
        }))
    }

    async fn destroy(&self, roster: &Roster, duty: &Duty) -> Result<()> {
        let bucket_name = duty.spec["bucket_name"].as_str()
            .ok_or_else(|| anyhow::anyhow!("bucket_name is required"))?;
        
        info!("Destroying S3 bucket: {}", bucket_name);
        
        let s3 = self.get_s3_client(roster).await?;
        
        if !s3.bucket_exists(bucket_name).await? {
            info!("Bucket '{}' does not exist, skipping deletion", bucket_name);
            return Ok(());
        }
        
        info!("Emptying bucket '{}'", bucket_name);
        s3.empty_bucket(bucket_name).await
            .context("Failed to empty bucket")?;
        
        info!("Deleting bucket '{}'", bucket_name);
        s3.delete_bucket(bucket_name).await
            .context("Failed to delete bucket")?;
        
        info!("Successfully destroyed S3 bucket: {}", bucket_name);
        Ok(())
    }
}
