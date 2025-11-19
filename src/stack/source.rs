use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;

#[async_trait]
pub trait StackSource: Send + Sync {
    async fn init(&self) -> Result<()>;
    
    async fn fetch(&self) -> Result<()>;
    
    async fn get_version(&self) -> Result<String>;
    
    async fn get_config_path(&self) -> Result<PathBuf>;
    
    async fn has_updates(&self, last_version: &str) -> Result<bool>;
}
