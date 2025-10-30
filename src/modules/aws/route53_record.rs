use anyhow::{Context, Result};
use async_trait::async_trait;
use aws_config::BehaviorVersion;
use serde_json::{json, Value as JsonValue};
use tracing::info;

use crate::modules::AutomationModule;
use crate::utils::{Duty, Roster};
use crate::db::StateManager;
use crate::modules::aws::clients::route53::Route53Module;
use crate::modules::aws::clients::traits::Route53Operations;

pub struct Route53RecordModule {
    state: StateManager,
}

impl Route53RecordModule {
    pub fn new(state: StateManager) -> Self {
        Self { state }
    }

    async fn get_route53_client(&self, roster: &Roster) -> Result<Route53Module> {
        let region = roster.connection.get("region")
            .and_then(|v| v.as_str())
            .unwrap_or("us-east-1");

        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(aws_config::Region::new(region.to_string()))
            .load()
            .await;

        let client = aws_sdk_route53::Client::new(&config);
        Ok(Route53Module::new(client))
    }
}

#[async_trait]
impl AutomationModule for Route53RecordModule {
    fn name(&self) -> &str {
        "route53-record"
    }

    fn supported_duty_types(&self) -> Vec<&str> {
        vec!["Route53Record"]
    }

    fn required_roster_traits(&self) -> Vec<&str> {
        vec!["cloud-provider", "aws"]
    }

    async fn validate(&self, _roster: &Roster, duty: &Duty) -> Result<()> {
        let spec = &duty.spec;
        
        if spec.get("hosted_zone_id").and_then(|v| v.as_str()).is_none() {
            anyhow::bail!("Route53Record duty requires 'hosted_zone_id' in spec");
        }
        
        if spec.get("name").and_then(|v| v.as_str()).is_none() {
            anyhow::bail!("Route53Record duty requires 'name' in spec");
        }
        
        if spec.get("record_type").and_then(|v| v.as_str()).is_none() {
            anyhow::bail!("Route53Record duty requires 'record_type' in spec");
        }

        Ok(())
    }

    async fn apply(&self, roster: &Roster, duty: &Duty) -> Result<JsonValue> {
        let spec = &duty.spec;
        let hosted_zone_id = spec["hosted_zone_id"].as_str()
            .ok_or_else(|| anyhow::anyhow!("hosted_zone_id is required"))?;
        let name = spec["name"].as_str()
            .ok_or_else(|| anyhow::anyhow!("name is required"))?;
        let record_type = spec["record_type"].as_str()
            .ok_or_else(|| anyhow::anyhow!("record_type is required"))?;

        let route53 = self.get_route53_client(roster).await?;

        let record_id = if let Some(alias_spec) = spec.get("alias").and_then(|v| v.as_object()) {
            let dns_name = alias_spec.get("dns_name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("alias.dns_name is required for alias records"))?;
            let target_zone_id = alias_spec.get("hosted_zone_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("alias.hosted_zone_id is required for alias records"))?;

            route53.create_alias_record(
                hosted_zone_id,
                name,
                dns_name,
                target_zone_id
            ).await.context("Failed to create alias record")?;

            format!("{}/{}/A-ALIAS", hosted_zone_id, name)
        } else {
            let value = spec.get("value")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("value is required for standard records"))?;
            
            let ttl = spec.get("ttl")
                .and_then(|v| v.as_i64())
                .unwrap_or(300);

            route53.create_record(
                hosted_zone_id,
                name,
                record_type,
                value,
                ttl
            ).await.context("Failed to create DNS record")?;

            format!("{}/{}/{}", hosted_zone_id, name, record_type)
        };

        Ok(json!({
            "phase": "deployed",
            "outputs": {
                "record_id": record_id,
            }
        }))
    }

    async fn destroy(&self, roster: &Roster, duty: &Duty) -> Result<()> {
        let hosted_zone_id = duty.spec["hosted_zone_id"].as_str()
            .ok_or_else(|| anyhow::anyhow!("hosted_zone_id is required"))?;
        let name = duty.spec["name"].as_str()
            .ok_or_else(|| anyhow::anyhow!("name is required"))?;
        let record_type = duty.spec["record_type"].as_str()
            .ok_or_else(|| anyhow::anyhow!("record_type is required"))?;
        
        info!("Destroying Route53 record: {} ({}) in zone {}", name, record_type, hosted_zone_id);
        
        let route53 = self.get_route53_client(roster).await?;
        
        if let Some(_alias_spec) = duty.spec.get("alias").and_then(|v| v.as_object()) {
            info!("Skipping deletion of alias record (not yet implemented)");
        } else {
            let value = duty.spec.get("value")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("value is required for standard records"))?;
            
            route53.delete_record(
                hosted_zone_id,
                name,
                record_type,
                value
            ).await.context("Failed to delete DNS record")?;
            
            info!("Successfully destroyed Route53 record: {} ({})", name, record_type);
        }
        
        Ok(())
    }
}
