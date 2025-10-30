use anyhow::{Context, Result};
use async_trait::async_trait;
use aws_config::BehaviorVersion;
use log::info;
use serde_json::{json, Value as JsonValue};

use crate::modules::AutomationModule;
use crate::utils::{Duty, Roster};
use crate::db::StateManager;
use crate::modules::aws::clients::iam::IAMModule;
use crate::modules::aws::clients::traits::IAMOperations;

pub struct IAMUserModule {
    state: StateManager,
}

impl IAMUserModule {
    pub fn new(state: StateManager) -> Self {
        Self { state }
    }

    async fn get_iam_client(&self, roster: &Roster) -> Result<IAMModule> {
        let region = roster.connection.get("region")
            .and_then(|v| v.as_str())
            .unwrap_or("us-east-1");

        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(aws_config::Region::new(region.to_string()))
            .load()
            .await;

        let client = aws_sdk_iam::Client::new(&config);
        Ok(IAMModule::new(client))
    }
}

#[async_trait]
impl AutomationModule for IAMUserModule {
    fn name(&self) -> &str {
        "iam-user"
    }

    fn supported_duty_types(&self) -> Vec<&str> {
        vec!["IAMUser"]
    }

    fn required_roster_traits(&self) -> Vec<&str> {
        vec!["cloud-provider", "aws"]
    }

    async fn validate(&self, _roster: &Roster, duty: &Duty) -> Result<()> {
        let spec = &duty.spec;
        
        if spec.get("user_name").and_then(|v| v.as_str()).is_none() {
            anyhow::bail!("IAMUser duty requires 'user_name' in spec");
        }

        Ok(())
    }

    async fn apply(&self, roster: &Roster, duty: &Duty) -> Result<JsonValue> {
        let spec = &duty.spec;
        let user_name = spec["user_name"].as_str()
            .ok_or_else(|| anyhow::anyhow!("user_name is required"))?;

        let iam = self.get_iam_client(roster).await?;

        let user_arn = if !iam.user_exists(user_name).await? {
            iam.create_user(user_name)
                .await
                .context("Failed to create IAM user")?
        } else {
            format!(
                "arn:aws:iam::{}:user/{}",
                roster.connection.get("account_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("123456789012"),
                user_name
            )
        };

        if let Some(policies) = spec.get("inline_policies").and_then(|v| v.as_object()) {
            for (policy_name, policy_doc) in policies {
                let policy_json = serde_json::to_string(policy_doc)
                    .context("Failed to serialize policy document")?;
                iam.put_user_policy(user_name, policy_name, &policy_json)
                    .await
                    .context("Failed to put inline policy")?;
            }
        }

        let create_access_key = spec.get("create_access_key")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut outputs = json!({
            "user_arn": user_arn,
            "user_name": user_name,
        });

        if create_access_key {
            let (access_key_id, secret_access_key) = iam.create_access_key(user_name)
                .await
                .context("Failed to create access key")?;

            let secret_key_ref = format!("postgres://secrets/iam/{}/secret-access-key", user_name);
            
            self.state.store_secret(
                &secret_key_ref,
                &secret_access_key,
                Some("IAM user secret access key")
            ).await?;

            outputs["access_key_id"] = json!(access_key_id);
            outputs["secret_access_key_ref"] = json!(secret_key_ref);
        }

        Ok(json!({
            "phase": "deployed",
            "outputs": outputs,
        }))
    }

    async fn destroy(&self, roster: &Roster, duty: &Duty) -> Result<()> {
        let user_name = duty.spec["user_name"].as_str()
            .ok_or_else(|| anyhow::anyhow!("user_name is required"))?;
        
        info!("Destroying IAM user: {}", user_name);
        
        let iam = self.get_iam_client(roster).await?;
        
        if !iam.user_exists(user_name).await? {
            info!("IAM user '{}' does not exist, skipping deletion", user_name);
            return Ok(());
        }
        
        info!("Deleting IAM user '{}'", user_name);
        iam.delete_user(user_name).await
            .context("Failed to delete IAM user")?;
        
        info!("Successfully destroyed IAM user: {}", user_name);
        Ok(())
    }
}
