use anyhow::{Context, Result};
use octocrab::Octocrab;

pub struct GitHubModule {
    client: Octocrab,
}

impl GitHubModule {
    pub fn new(token: &str) -> Result<Self> {
        let client = Octocrab::builder()
            .personal_token(token.to_string())
            .build()
            .context("Failed to build GitHub client")?;

        Ok(Self { client })
    }

    pub async fn create_repository(
        &self,
        owner: &str,
        name: &str,
        private: bool,
        _auto_init: bool,
        _has_issues: bool,
        _has_projects: bool,
        _has_wiki: bool,
    ) -> Result<String> {
        use octocrab::models;
        
        let _: () = self.client
            .post(
                format!("/orgs/{}/repos", owner),
                Some(&serde_json::json!({
                    "name": name,
                    "private": private,
                    "auto_init": true,
                    "has_issues": true,
                    "has_projects": true,
                    "has_wiki": true,
                }))
            )
            .await
            .with_context(|| format!("Failed to create GitHub repository: {}/{}", owner, name))?;

        let repo_data: models::Repository = self.client
            .get(format!("/repos/{}/{}", owner, name), None::<&()>)
            .await
            .context("Failed to get repository details")?;
        
        Ok(repo_data.html_url.map(|u| u.to_string()).unwrap_or_default())
    }

    pub async fn repository_exists(&self, owner: &str, name: &str) -> Result<bool> {
        match self.client.repos(owner, name).get().await {
            Ok(_) => Ok(true),
            Err(octocrab::Error::GitHub { source, .. }) if source.message.contains("Not Found") => Ok(false),
            Err(e) => Err(anyhow::anyhow!("Failed to check repository existence: {}", e)),
        }
    }

    pub async fn create_secret(
        &self,
        owner: &str,
        repo: &str,
        secret_name: &str,
        secret_value: &str,
    ) -> Result<()> {
        use serde::Deserialize;
        
        #[derive(Deserialize)]
        struct PublicKey {
            key_id: String,
            key: String,
        }
        
        let public_key: PublicKey = self.client
            .get(format!("/repos/{}/{}/actions/secrets/public-key", owner, repo), None::<&()>)
            .await
            .context("Failed to get repository public key")?;

        let encrypted = self.encrypt_secret(secret_value, &public_key.key)?;

        let _: () = self.client
            .put(
                format!("/repos/{}/{}/actions/secrets/{}", owner, repo, secret_name),
                Some(&serde_json::json!({
                    "encrypted_value": encrypted,
                    "key_id": public_key.key_id,
                }))
            )
            .await
            .with_context(|| format!("Failed to create secret: {}", secret_name))?;

        Ok(())
    }

    fn encrypt_secret(&self, value: &str, public_key: &str) -> Result<String> {
        use base64::{Engine as _, engine::general_purpose};
        
        let key_bytes = general_purpose::STANDARD.decode(public_key)
            .context("Failed to decode public key")?;

        let encrypted = sodiumoxide::crypto::sealedbox::seal(
            value.as_bytes(),
            &sodiumoxide::crypto::box_::PublicKey::from_slice(&key_bytes)
                .context("Invalid public key")?,
        );

        Ok(general_purpose::STANDARD.encode(&encrypted))
    }
}
