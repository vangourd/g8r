use anyhow::{Context, Result};
use async_trait::async_trait;
use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::types::{
    BucketVersioningStatus, VersioningConfiguration,
    PublicAccessBlockConfiguration, WebsiteConfiguration,
    IndexDocument, ErrorDocument,
};

use super::traits::S3Operations;

pub struct S3Module {
    client: S3Client,
}

impl S3Module {
    pub fn new(client: S3Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl S3Operations for S3Module {
    async fn create_bucket(&self, name: &str, region: &str) -> Result<String> {
        let mut request = self.client
            .create_bucket()
            .bucket(name);

        if region != "us-east-1" {
            let constraint = aws_sdk_s3::types::BucketLocationConstraint::from(region);
            let cfg = aws_sdk_s3::types::CreateBucketConfiguration::builder()
                .location_constraint(constraint)
                .build();
            request = request.create_bucket_configuration(cfg);
        }

        request
            .send()
            .await
            .with_context(|| format!("Failed to create S3 bucket: {}", name))?;

        Ok(name.to_string())
    }

    async fn configure_website(&self, bucket: &str, index: &str, error: &str) -> Result<()> {
        let index_doc = IndexDocument::builder()
            .suffix(index)
            .build()
            .context("Failed to build index document")?;

        let error_doc = ErrorDocument::builder()
            .key(error)
            .build()
            .context("Failed to build error document")?;

        let config = WebsiteConfiguration::builder()
            .index_document(index_doc)
            .error_document(error_doc)
            .build();

        self.client
            .put_bucket_website()
            .bucket(bucket)
            .website_configuration(config)
            .send()
            .await
            .with_context(|| format!("Failed to configure website for bucket: {}", bucket))?;

        Ok(())
    }

    async fn enable_versioning(&self, bucket: &str) -> Result<()> {
        let config = VersioningConfiguration::builder()
            .status(BucketVersioningStatus::Enabled)
            .build();

        self.client
            .put_bucket_versioning()
            .bucket(bucket)
            .versioning_configuration(config)
            .send()
            .await
            .with_context(|| format!("Failed to enable versioning for bucket: {}", bucket))?;

        Ok(())
    }

    async fn set_public_access_block(&self, bucket: &str, block: bool) -> Result<()> {
        let config = PublicAccessBlockConfiguration::builder()
            .block_public_acls(block)
            .block_public_policy(block)
            .ignore_public_acls(block)
            .restrict_public_buckets(block)
            .build();

        self.client
            .put_public_access_block()
            .bucket(bucket)
            .public_access_block_configuration(config)
            .send()
            .await
            .with_context(|| format!("Failed to set public access block for bucket: {}", bucket))?;

        Ok(())
    }

    async fn set_bucket_policy(&self, bucket: &str, policy: &str) -> Result<()> {
        self.client
            .put_bucket_policy()
            .bucket(bucket)
            .policy(policy)
            .send()
            .await
            .with_context(|| format!("Failed to set bucket policy for: {}", bucket))?;

        Ok(())
    }

    async fn get_website_endpoint(&self, bucket: &str, region: &str) -> String {
        format!("{}.s3-website.{}.amazonaws.com", bucket, region)
    }

    async fn bucket_exists(&self, bucket: &str) -> Result<bool> {
        match self.client.head_bucket().bucket(bucket).send().await {
            Ok(_) => {
                log::debug!("Bucket {} exists", bucket);
                Ok(true)
            },
            Err(e) => {
                let err_str = format!("{:?}", e);
                log::debug!("HeadBucket error for {}: {}", bucket, err_str);
                
                if err_str.contains("NotFound") || err_str.contains("404") {
                    log::debug!("Bucket {} does not exist", bucket);
                    Ok(false)
                } else if err_str.contains("301") {
                    log::debug!("Bucket {} exists (got 301 redirect, likely in different region)", bucket);
                    Ok(true)
                } else {
                    log::error!("Failed to check bucket existence for {}: {:?}", bucket, e);
                    Err(anyhow::anyhow!("Failed to check bucket existence: {:?}", e))
                }
            }
        }
    }

    async fn empty_bucket(&self, bucket: &str) -> Result<()> {
        loop {
            let objects = self.client
                .list_objects_v2()
                .bucket(bucket)
                .send()
                .await
                .with_context(|| format!("Failed to list objects in bucket: {}", bucket))?;

            let contents = objects.contents();
            if contents.is_empty() {
                break;
            }

            for obj in contents {
                if let Some(key) = obj.key() {
                    self.client
                        .delete_object()
                        .bucket(bucket)
                        .key(key)
                        .send()
                        .await
                        .with_context(|| format!("Failed to delete object: {} from bucket: {}", key, bucket))?;
                }
            }
        }

        Ok(())
    }

    async fn delete_bucket(&self, bucket: &str) -> Result<()> {
        self.client
            .delete_bucket()
            .bucket(bucket)
            .send()
            .await
            .with_context(|| format!("Failed to delete bucket: {}", bucket))?;

        Ok(())
    }
}
