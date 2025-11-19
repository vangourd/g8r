pub mod env;
pub mod postgres;

use async_trait::async_trait;
use anyhow::Result;

#[async_trait]
pub trait SecretResolver: Send + Sync {
    fn scheme(&self) -> &str;
    
    async fn resolve(&self, reference: &str) -> Result<String>;
}

pub struct SecretManager {
    resolvers: Vec<Box<dyn SecretResolver>>,
}

impl SecretManager {
    pub fn new() -> Self {
        Self {
            resolvers: Vec::new(),
        }
    }
    
    pub fn register_resolver(&mut self, resolver: Box<dyn SecretResolver>) {
        self.resolvers.push(resolver);
    }
    
    pub async fn resolve(&self, reference: &str) -> Result<String> {
        if !reference.contains("://") {
            return Ok(reference.to_string());
        }
        
        let parts: Vec<&str> = reference.splitn(2, "://").collect();
        if parts.len() != 2 {
            return Ok(reference.to_string());
        }
        
        let scheme = parts[0];
        let path = parts[1];
        
        for resolver in &self.resolvers {
            if resolver.scheme() == scheme {
                return resolver.resolve(path).await;
            }
        }
        
        anyhow::bail!("No resolver found for scheme: {}", scheme)
    }
}

impl Default for SecretManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    struct TestResolver;
    
    #[async_trait]
    impl SecretResolver for TestResolver {
        fn scheme(&self) -> &str {
            "test"
        }
        
        async fn resolve(&self, reference: &str) -> Result<String> {
            Ok(format!("resolved:{}", reference))
        }
    }
    
    #[tokio::test]
    async fn test_secret_manager_creation() {
        let manager = SecretManager::new();
        assert_eq!(manager.resolvers.len(), 0);
    }
    
    #[tokio::test]
    async fn test_register_resolver() {
        let mut manager = SecretManager::new();
        manager.register_resolver(Box::new(TestResolver));
        assert_eq!(manager.resolvers.len(), 1);
    }
    
    #[tokio::test]
    async fn test_resolve_with_scheme() {
        let mut manager = SecretManager::new();
        manager.register_resolver(Box::new(TestResolver));
        
        let result = manager.resolve("test://my-secret").await.unwrap();
        assert_eq!(result, "resolved:my-secret");
    }
    
    #[tokio::test]
    async fn test_resolve_plain_string() {
        let manager = SecretManager::new();
        let result = manager.resolve("plain-value").await.unwrap();
        assert_eq!(result, "plain-value");
    }
    
    #[tokio::test]
    async fn test_resolve_unknown_scheme() {
        let manager = SecretManager::new();
        let result = manager.resolve("unknown://secret").await;
        assert!(result.is_err());
    }
}
