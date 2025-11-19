use async_trait::async_trait;
use anyhow::{Result, anyhow};
use std::env;

use super::SecretResolver;

pub struct EnvSecretResolver;

impl EnvSecretResolver {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EnvSecretResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SecretResolver for EnvSecretResolver {
    fn scheme(&self) -> &str {
        "env"
    }
    
    async fn resolve(&self, reference: &str) -> Result<String> {
        env::var(reference)
            .map_err(|_| anyhow!("Environment variable '{}' not found", reference))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_scheme() {
        let resolver = EnvSecretResolver::new();
        assert_eq!(resolver.scheme(), "env");
    }
    
    #[tokio::test]
    async fn test_resolve_existing_var() {
        env::set_var("TEST_SECRET", "secret_value");
        
        let resolver = EnvSecretResolver::new();
        let result = resolver.resolve("TEST_SECRET").await.unwrap();
        assert_eq!(result, "secret_value");
        
        env::remove_var("TEST_SECRET");
    }
    
    #[tokio::test]
    async fn test_resolve_missing_var() {
        let resolver = EnvSecretResolver::new();
        let result = resolver.resolve("NONEXISTENT_VAR").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
