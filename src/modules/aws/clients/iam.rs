use anyhow::{Context, Result};
use async_trait::async_trait;
use aws_sdk_iam::Client as IamClient;

use super::traits::IAMOperations;

pub struct IAMModule {
    client: IamClient,
}

impl IAMModule {
    pub fn new(client: IamClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl IAMOperations for IAMModule {
    async fn create_user(&self, name: &str) -> Result<String> {
        let result = self.client
            .create_user()
            .user_name(name)
            .send()
            .await
            .with_context(|| format!("Failed to create IAM user: {}", name))?;

        let user = result.user().context("No user in response")?;
        let arn = user.arn().to_string();

        Ok(arn)
    }

    async fn create_access_key(&self, user: &str) -> Result<(String, String)> {
        let result = self.client
            .create_access_key()
            .user_name(user)
            .send()
            .await
            .with_context(|| format!("Failed to create access key for user: {}", user))?;

        let access_key = result.access_key().context("No access key in response")?;
        let access_key_id = access_key.access_key_id().to_string();
        let secret_access_key = access_key.secret_access_key().to_string();

        Ok((access_key_id, secret_access_key))
    }

    async fn put_user_policy(&self, user: &str, policy_name: &str, policy: &str) -> Result<()> {
        self.client
            .put_user_policy()
            .user_name(user)
            .policy_name(policy_name)
            .policy_document(policy)
            .send()
            .await
            .with_context(|| format!("Failed to put user policy for: {}", user))?;

        Ok(())
    }

    async fn user_exists(&self, name: &str) -> Result<bool> {
        match self.client.get_user().user_name(name).send().await {
            Ok(_) => Ok(true),
            Err(e) => {
                let err_string = format!("{:?}", e);
                if err_string.contains("NoSuchEntity") || err_string.contains("404") {
                    Ok(false)
                } else {
                    log::error!("Failed to check user existence for '{}': {:?}", name, e);
                    Err(anyhow::anyhow!("Failed to check user existence for '{}': {:?}", name, e))
                }
            }
        }
    }

    async fn delete_access_keys(&self, user: &str) -> Result<()> {
        let keys = self.client
            .list_access_keys()
            .user_name(user)
            .send()
            .await
            .with_context(|| format!("Failed to list access keys for user: {}", user))?;

        for key in keys.access_key_metadata() {
            if let Some(key_id) = key.access_key_id() {
                self.client
                    .delete_access_key()
                    .user_name(user)
                    .access_key_id(key_id)
                    .send()
                    .await
                    .with_context(|| format!("Failed to delete access key {} for user: {}", key_id, user))?;
            }
        }

        Ok(())
    }

    async fn delete_user_policies(&self, user: &str) -> Result<()> {
        let policies = self.client
            .list_user_policies()
            .user_name(user)
            .send()
            .await
            .with_context(|| format!("Failed to list policies for user: {}", user))?;

        for policy_name in policies.policy_names() {
            self.client
                .delete_user_policy()
                .user_name(user)
                .policy_name(policy_name)
                .send()
                .await
                .with_context(|| format!("Failed to delete policy {} for user: {}", policy_name, user))?;
        }

        Ok(())
    }

    async fn delete_user(&self, name: &str) -> Result<()> {
        self.delete_access_keys(name).await?;
        self.delete_user_policies(name).await?;

        self.client
            .delete_user()
            .user_name(name)
            .send()
            .await
            .with_context(|| format!("Failed to delete IAM user: {}", name))?;

        Ok(())
    }
}