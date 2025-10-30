use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value as JsonValue;

#[async_trait]
pub trait S3Operations {
    async fn create_bucket(&self, name: &str, region: &str) -> Result<String>;
    async fn configure_website(&self, bucket: &str, index: &str, error: &str) -> Result<()>;
    async fn enable_versioning(&self, bucket: &str) -> Result<()>;
    async fn set_public_access_block(&self, bucket: &str, block: bool) -> Result<()>;
    async fn set_bucket_policy(&self, bucket: &str, policy: &str) -> Result<()>;
    async fn get_website_endpoint(&self, bucket: &str, region: &str) -> String;
    async fn bucket_exists(&self, bucket: &str) -> Result<bool>;
    async fn delete_bucket(&self, bucket: &str) -> Result<()>;
    async fn empty_bucket(&self, bucket: &str) -> Result<()>;
}

#[async_trait]
pub trait CloudFrontOperations {
    async fn create_distribution(&self, config: JsonValue) -> Result<(String, String)>;
    async fn get_distribution(&self, id: &str) -> Result<Option<JsonValue>>;
    async fn delete_distribution(&self, id: &str) -> Result<()>;
    async fn disable_distribution(&self, id: &str) -> Result<()>;
}

#[async_trait]
pub trait ACMOperations {
    async fn request_certificate(&self, domain: &str, sans: Vec<String>) -> Result<String>;
    async fn get_certificate(&self, arn: &str) -> Result<Option<JsonValue>>;
    async fn get_certificate_validation_records(&self, arn: &str) -> Result<Vec<(String, String)>>;
    async fn wait_for_validation(&self, arn: &str, timeout_secs: u64) -> Result<()>;
    async fn delete_certificate(&self, arn: &str) -> Result<()>;
}

#[async_trait]
pub trait IAMOperations {
    async fn create_user(&self, name: &str) -> Result<String>;
    async fn create_access_key(&self, user: &str) -> Result<(String, String)>;
    async fn put_user_policy(&self, user: &str, policy_name: &str, policy: &str) -> Result<()>;
    async fn user_exists(&self, name: &str) -> Result<bool>;
    async fn delete_user(&self, name: &str) -> Result<()>;
    async fn delete_access_keys(&self, user: &str) -> Result<()>;
    async fn delete_user_policies(&self, user: &str) -> Result<()>;
}

#[async_trait]
pub trait Route53Operations {
    async fn create_hosted_zone(&self, domain: &str) -> Result<String>;
    async fn get_zone_id(&self, domain: &str) -> Result<Option<String>>;
    async fn create_record(&self, zone_id: &str, name: &str, record_type: &str, value: &str, ttl: i64) -> Result<()>;
    async fn create_alias_record(&self, zone_id: &str, name: &str, target_domain: &str, target_zone_id: &str) -> Result<()>;
    async fn delete_record(&self, zone_id: &str, name: &str, record_type: &str, value: &str) -> Result<()>;
}
