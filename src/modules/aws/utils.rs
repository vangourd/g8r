use anyhow::{Result, Context};
use std::future::Future;
use std::time::Duration;
use log::info;

use crate::utils::Roster;
use super::clients::s3::S3Module;
use super::clients::acm::ACMModule;
use super::clients::route53::Route53Module;
use super::clients::iam::IAMModule;
use super::clients::cloudfront::CloudFrontModule;

pub async fn retry_with_backoff<F, Fut, T>(
    operation: F,
    max_attempts: u32,
    operation_name: &str,
) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut delay = Duration::from_secs(2);
    let max_delay = Duration::from_secs(30);
    
    for attempt in 1..=max_attempts {
        match operation().await {
            Ok(result) => {
                if attempt > 1 {
                    info!("{} succeeded on attempt {}", operation_name, attempt);
                }
                return Ok(result);
            }
            Err(e) => {
                if attempt < max_attempts {
                    info!(
                        "{} failed (attempt {}/{}), retrying in {:?}: {}",
                        operation_name, attempt, max_attempts, delay, e
                    );
                    tokio::time::sleep(delay).await;
                    delay = std::cmp::min(delay * 2, max_delay);
                } else {
                    return Err(e).context(format!(
                        "{} failed after {} attempts",
                        operation_name, max_attempts
                    ));
                }
            }
        }
    }
    
    unreachable!("retry loop should always return or error");
}

pub async fn get_aws_config(
    roster: &Roster,
    region_override: Option<&str>,
) -> Result<aws_config::SdkConfig> {
    let region = if let Some(override_region) = region_override {
        override_region.to_string()
    } else {
        roster
            .connection
            .get("region")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Roster missing 'region' in connection"))?
            .to_string()
    };

    let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new(region))
        .load()
        .await;

    Ok(config)
}

pub async fn aws_s3_client(roster: &Roster) -> Result<S3Module> {
    let config = get_aws_config(roster, None).await?;
    let client = aws_sdk_s3::Client::new(&config);
    Ok(S3Module::new(client))
}

pub async fn aws_acm_client(roster: &Roster, region_override: Option<&str>) -> Result<ACMModule> {
    let config = get_aws_config(roster, region_override).await?;
    let client = aws_sdk_acm::Client::new(&config);
    Ok(ACMModule::new(client))
}

pub async fn aws_route53_client(roster: &Roster) -> Result<Route53Module> {
    let config = get_aws_config(roster, None).await?;
    let client = aws_sdk_route53::Client::new(&config);
    Ok(Route53Module::new(client))
}

pub async fn aws_iam_client(roster: &Roster) -> Result<IAMModule> {
    let config = get_aws_config(roster, None).await?;
    let client = aws_sdk_iam::Client::new(&config);
    Ok(IAMModule::new(client))
}

pub async fn aws_cloudfront_client(roster: &Roster) -> Result<CloudFrontModule> {
    let config = get_aws_config(roster, None).await?;
    let client = aws_sdk_cloudfront::Client::new(&config);
    Ok(CloudFrontModule::new(client))
}
