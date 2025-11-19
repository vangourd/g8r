use async_trait::async_trait;
use anyhow::{Result, anyhow};
use sqlx::PgPool;

use super::SecretResolver;

pub struct PostgresSecretResolver {
    pool: PgPool,
}

impl PostgresSecretResolver {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    
    pub async fn from_url(database_url: &str) -> Result<Self> {
        let pool = PgPool::connect(database_url).await?;
        Ok(Self { pool })
    }
}

#[async_trait]
impl SecretResolver for PostgresSecretResolver {
    fn scheme(&self) -> &str {
        "postgres"
    }
    
    async fn resolve(&self, reference: &str) -> Result<String> {
        let row: (String,) = sqlx::query_as(
            "SELECT value FROM secrets WHERE name = $1"
        )
        .bind(reference)
        .fetch_one(&self.pool)
        .await
        .map_err(|_| anyhow!("Secret '{}' not found in database", reference))?;
        
        Ok(row.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    
    async fn setup_test_db() -> PgPool {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://g8r:g8r_dev_password@localhost:5432/g8r_state".to_string());
        
        let pool = PgPool::connect(&database_url).await.unwrap();
        
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS secrets (
                id SERIAL PRIMARY KEY,
                name VARCHAR(255) UNIQUE NOT NULL,
                value TEXT NOT NULL,
                description TEXT,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )"
        )
        .execute(&pool)
        .await
        .unwrap();
        
        sqlx::query("TRUNCATE secrets CASCADE")
            .execute(&pool)
            .await
            .unwrap();
        
        pool
    }
    
    #[tokio::test]
    #[serial]
    async fn test_scheme() {
        let pool = setup_test_db().await;
        let resolver = PostgresSecretResolver::new(pool);
        assert_eq!(resolver.scheme(), "postgres");
    }
    
    #[tokio::test]
    #[serial]
    async fn test_resolve_existing_secret() {
        let pool = setup_test_db().await;
        
        sqlx::query("INSERT INTO secrets (name, value) VALUES ($1, $2)")
            .bind("aws_access_key")
            .bind("AKIAIOSFODNN7EXAMPLE")
            .execute(&pool)
            .await
            .unwrap();
        
        let resolver = PostgresSecretResolver::new(pool);
        let result = resolver.resolve("aws_access_key").await.unwrap();
        assert_eq!(result, "AKIAIOSFODNN7EXAMPLE");
    }
    
    #[tokio::test]
    #[serial]
    async fn test_resolve_missing_secret() {
        let pool = setup_test_db().await;
        let resolver = PostgresSecretResolver::new(pool);
        let result = resolver.resolve("nonexistent").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
