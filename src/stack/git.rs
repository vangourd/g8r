use anyhow::{Context, Result};
use async_trait::async_trait;
use git2::{Repository, ObjectType, ResetType};
use log::info;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tracing::instrument;
use url::Url;

use super::source::StackSource;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitSourceConfig {
    pub url: String,
    pub branch: String,
    pub token: Option<String>,
    pub local_path: String,
}

pub struct GitSource {
    config: GitSourceConfig,
    config_file_path: String,
    repo: std::sync::Mutex<Option<Repository>>,
}

impl GitSource {
    pub fn new(mut config: GitSourceConfig, config_file_path: String) -> Self {
        if config.token.is_none() {
            config.token = std::env::var("GITHUB_TOKEN").ok();
        }
        
        Self {
            config,
            config_file_path,
            repo: std::sync::Mutex::new(None),
        }
    }
    
    #[instrument(
        skip(self),
        fields(
            git.url = %self.config.url,
            git.branch = %self.config.branch,
            git.local_path = %self.config.local_path,
            git.operation = tracing::field::Empty,
        )
    )]
    pub async fn init(&self) -> Result<()> {
        let span = tracing::Span::current();
        let repo_path = &self.config.local_path;
        
        if !Path::exists(Path::new(&repo_path)) {
            span.record("git.operation", "clone");
            self.clone_repo().await?;
        } else {
            span.record("git.operation", "open_and_fetch");
            let repo = Repository::open(&self.config.local_path)
                .context("Unable to open existing repository path")?;
            *self.repo.lock().unwrap() = Some(repo);
            self.fetch_repo().await?;
            self.reset_repo().await?;
        }
        
        Ok(())
    }
    
    #[instrument(
        skip(self),
        fields(
            git.url = %self.config.url,
            git.branch = %self.config.branch,
            git.local_path = %self.config.local_path,
            git.clone_duration_ms = tracing::field::Empty,
        )
    )]
    async fn clone_repo(&self) -> Result<()> {
        let start = Instant::now();
        let span = tracing::Span::current();
        
        info!("Cloning repository: {}", self.config.url);
        
        let mut callbacks = git2::RemoteCallbacks::new();
        
        if let Some(ref token) = self.config.token {
            let token_clone = token.clone();
            callbacks.credentials(move |_url, _username_from_url, _allowed_types| {
                git2::Cred::userpass_plaintext("oauth2", &token_clone)
            });
        }
        
        let mut fetch_options = git2::FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);
        
        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fetch_options);
        
        let repo = match builder.clone(&self.config.url, std::path::Path::new(&self.config.local_path)) {
            Ok(r) => r,
            Err(e) => {
                log::error!("Git clone failed: {} (code: {:?}, class: {:?})", 
                    e.message(), e.code(), e.class());
                return Err(anyhow::anyhow!("Git clone failed: {}", e.message()));
            }
        };
        
        *self.repo.lock().unwrap() = Some(repo);
        span.record("git.clone_duration_ms", start.elapsed().as_millis() as i64);
        Ok(())
    }
    
    #[instrument(
        skip(self),
        fields(
            git.branch = %self.config.branch,
            git.fetch_duration_ms = tracing::field::Empty,
        )
    )]
    async fn fetch_repo(&self) -> Result<()> {
        let start = Instant::now();
        let span = tracing::Span::current();
        
        info!("Fetching from remote");
        let mut repo_guard = self.repo.lock().unwrap();
        let repo = repo_guard.as_mut()
            .context("Repository not initialized")?;
        
        let mut callbacks = git2::RemoteCallbacks::new();
        
        if let Some(ref token) = self.config.token {
            let token_clone = token.clone();
            callbacks.credentials(move |_url, _username_from_url, _allowed_types| {
                git2::Cred::userpass_plaintext("oauth2", &token_clone)
            });
        }
        
        let mut fetch_options = git2::FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);
        
        repo.find_remote("origin")
            .context("Unable to find remote 'origin'")?
            .fetch(&[&self.config.branch], Some(&mut fetch_options), None)
            .context("Unable to fetch from remote")?;
        
        span.record("git.fetch_duration_ms", start.elapsed().as_millis() as i64);
        Ok(())
    }
    
    async fn reset_repo(&self) -> Result<()> {
        info!("Resetting repository to FETCH_HEAD");
        let mut repo_guard = self.repo.lock().unwrap();
        let repo = repo_guard.as_mut()
            .context("Repository not initialized")?;
        
        let commit = repo.find_reference("FETCH_HEAD")
            .context("Unable to find FETCH_HEAD")?
            .peel(ObjectType::Commit)
            .context("Unable to peel FETCH_HEAD to commit")?;
        
        repo.reset(&commit, ResetType::Hard, None)
            .context("Unable to reset repository")?;
        
        Ok(())
    }
    
    fn get_current_commit_sha(&self) -> Result<String> {
        let repo_guard = self.repo.lock().unwrap();
        let repo = repo_guard.as_ref()
            .context("Repository not initialized")?;
        
        let head = repo.head()
            .context("Unable to get HEAD")?;
        
        let commit = head.peel_to_commit()
            .context("Unable to peel HEAD to commit")?;
        
        Ok(commit.id().to_string())
    }
}

#[async_trait]
impl StackSource for GitSource {
    async fn init(&self) -> Result<()> {
        let repo_path = &self.config.local_path;
        
        if !Path::exists(Path::new(&repo_path)) {
            self.clone_repo().await?;
        } else {
            let repo = Repository::open(&self.config.local_path)
                .context("Unable to open existing repository path")?;
            *self.repo.lock().unwrap() = Some(repo);
            self.fetch_repo().await?;
            self.reset_repo().await?;
        }
        
        Ok(())
    }
    
    async fn fetch(&self) -> Result<()> {
        let mut repo_guard = self.repo.lock().unwrap();
        let repo = repo_guard.as_mut()
            .context("Repository not initialized")?;
        
        let mut callbacks = git2::RemoteCallbacks::new();
        
        if let Some(ref token) = self.config.token {
            let token_clone = token.clone();
            callbacks.credentials(move |_url, _username_from_url, _allowed_types| {
                git2::Cred::userpass_plaintext("oauth2", &token_clone)
            });
        }
        
        let mut fetch_options = git2::FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);
        
        repo.find_remote("origin")
            .context("Unable to find remote 'origin'")?
            .fetch(&[&self.config.branch], Some(&mut fetch_options), None)
            .context("Unable to fetch from remote")?;
        
        Ok(())
    }
    
    async fn get_version(&self) -> Result<String> {
        self.get_current_commit_sha()
    }
    
    async fn get_config_path(&self) -> Result<PathBuf> {
        let mut path = PathBuf::from(&self.config.local_path);
        path.push(&self.config_file_path);
        Ok(path)
    }
    
    async fn has_updates(&self, last_version: &str) -> Result<bool> {
        let mut repo_guard = self.repo.lock().unwrap();
        let repo = repo_guard.as_mut()
            .context("Repository not initialized")?;
        
        let mut callbacks = git2::RemoteCallbacks::new();
        
        if let Some(ref token) = self.config.token {
            let token_clone = token.clone();
            callbacks.credentials(move |_url, _username_from_url, _allowed_types| {
                git2::Cred::userpass_plaintext("oauth2", &token_clone)
            });
        }
        
        let mut fetch_options = git2::FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);
        
        repo.find_remote("origin")
            .context("Unable to find remote 'origin'")?
            .fetch(&[&self.config.branch], Some(&mut fetch_options), None)
            .context("Unable to fetch from remote")?;
        
        let local_commit = repo.revparse_single("HEAD")
            .context("Unable to resolve HEAD")?
            .id()
            .to_string();
        
        let remote_commit = repo.revparse_single(&format!("refs/remotes/origin/{}", self.config.branch))
            .context("Unable to resolve remote branch")?
            .id()
            .to_string();
        
        Ok(local_commit != remote_commit || local_commit != last_version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_git_source_config_serialization() {
        let config = GitSourceConfig {
            url: "https://github.com/test/repo".to_string(),
            branch: "main".to_string(),
            token: Some("test-token".to_string()),
            local_path: "/tmp/test-repo".to_string(),
        };
        
        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(json["url"], "https://github.com/test/repo");
        assert_eq!(json["branch"], "main");
        
        let deserialized: GitSourceConfig = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized.url, config.url);
    }
}
