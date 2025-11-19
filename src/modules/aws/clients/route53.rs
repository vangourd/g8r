use anyhow::{Context, Result};
use async_trait::async_trait;
use aws_sdk_route53::Client as Route53Client;
use aws_sdk_route53::types::{Change, ChangeAction, ChangeBatch, ResourceRecordSet, RrType, ResourceRecord, AliasTarget};

use super::traits::Route53Operations;

pub struct Route53Module {
    client: Route53Client,
}

impl Route53Module {
    pub fn new(client: Route53Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Route53Operations for Route53Module {
    async fn create_hosted_zone(&self, domain: &str) -> Result<String> {
        let caller_reference = format!("g8r-{}", chrono::Utc::now().timestamp());
        
        let result = self.client
            .create_hosted_zone()
            .name(domain)
            .caller_reference(&caller_reference)
            .send()
            .await
            .context("Failed to create hosted zone")?;

        let zone = result.hosted_zone()
            .context("No hosted zone in response")?;
        
        let zone_id = zone.id().to_string();
        
        Ok(zone_id)
    }

    async fn get_zone_id(&self, domain: &str) -> Result<Option<String>> {
        let result = self.client
            .list_hosted_zones_by_name()
            .dns_name(domain)
            .max_items(1)
            .send()
            .await
            .context("Failed to list hosted zones")?;

        let zones = result.hosted_zones();
        if let Some(zone) = zones.first() {
            let name = zone.name();
            if name.trim_end_matches('.') == domain.trim_end_matches('.') {
                return Ok(Some(zone.id().to_string()));
            }
        }

        Ok(None)
    }

    async fn create_record(&self, zone_id: &str, name: &str, record_type: &str, value: &str, ttl: i64) -> Result<()> {
        let rr = ResourceRecord::builder()
            .value(value)
            .build()
            .context("Failed to build resource record")?;

        let rr_type = match record_type {
            "A" => RrType::A,
            "AAAA" => RrType::Aaaa,
            "CNAME" => RrType::Cname,
            "TXT" => RrType::Txt,
            _ => return Err(anyhow::anyhow!("Unsupported record type: {}", record_type)),
        };

        let rr_set = ResourceRecordSet::builder()
            .name(name)
            .r#type(rr_type)
            .ttl(ttl)
            .resource_records(rr)
            .build()
            .context("Failed to build resource record set")?;

        let change = Change::builder()
            .action(ChangeAction::Upsert)
            .resource_record_set(rr_set)
            .build()
            .context("Failed to build change")?;

        let change_batch = ChangeBatch::builder()
            .changes(change)
            .build()
            .context("Failed to build change batch")?;

        self.client
            .change_resource_record_sets()
            .hosted_zone_id(zone_id)
            .change_batch(change_batch)
            .send()
            .await
            .context(format!(
                "Failed to create DNS record: zone={}, name={}, type={}, value={}",
                zone_id, name, record_type, value
            ))?;

        Ok(())
    }

    async fn create_alias_record(&self, zone_id: &str, name: &str, target_domain: &str, target_zone_id: &str) -> Result<()> {
        let alias_target = AliasTarget::builder()
            .hosted_zone_id(target_zone_id)
            .dns_name(target_domain)
            .evaluate_target_health(false)
            .build()
            .context("Failed to build alias target")?;

        let rr_set = ResourceRecordSet::builder()
            .name(name)
            .r#type(RrType::A)
            .alias_target(alias_target)
            .build()
            .context("Failed to build alias record set")?;

        let change = Change::builder()
            .action(ChangeAction::Upsert)
            .resource_record_set(rr_set)
            .build()
            .context("Failed to build change")?;

        let change_batch = ChangeBatch::builder()
            .changes(change)
            .build()
            .context("Failed to build change batch")?;

        self.client
            .change_resource_record_sets()
            .hosted_zone_id(zone_id)
            .change_batch(change_batch)
            .send()
            .await
            .context("Failed to create alias record")?;

        Ok(())
    }

    async fn delete_record(&self, zone_id: &str, name: &str, record_type: &str, value: &str) -> Result<()> {
        let rr = ResourceRecord::builder()
            .value(value)
            .build()
            .context("Failed to build resource record")?;

        let rr_set = ResourceRecordSet::builder()
            .name(name)
            .r#type(record_type.parse().context("Invalid record type")?)
            .ttl(300)
            .resource_records(rr)
            .build()
            .context("Failed to build resource record set")?;

        let change = Change::builder()
            .action(ChangeAction::Delete)
            .resource_record_set(rr_set)
            .build()
            .context("Failed to build change")?;

        let change_batch = ChangeBatch::builder()
            .changes(change)
            .build()
            .context("Failed to build change batch")?;

        self.client
            .change_resource_record_sets()
            .hosted_zone_id(zone_id)
            .change_batch(change_batch)
            .send()
            .await
            .context("Failed to delete record")?;

        Ok(())
    }
}