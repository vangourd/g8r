#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    async fn setup_test_db() -> StateManager {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://g8r:g8r_dev_password@localhost:5432/g8r_state".to_string());
        
        StateManager::new(&database_url).await.expect("Failed to connect to test database")
    }

    #[tokio::test]
    async fn test_create_client() {
        let state = setup_test_db().await;

        let client = state.upsert_client(NewClient {
            name: "test-client".to_string(),
            domain: "test.example.com".to_string(),
            config: json!({"test": "data"}),
        }).await.expect("Failed to create client");

        assert_eq!(client.name, "test-client");
        assert_eq!(client.domain, "test.example.com");
    }

    #[tokio::test]
    async fn test_create_environment() {
        let state = setup_test_db().await;

        let client = state.upsert_client(NewClient {
            name: "test-client-2".to_string(),
            domain: "test2.example.com".to_string(),
            config: json!({}),
        }).await.expect("Failed to create client");

        let env = state.upsert_environment(NewEnvironment {
            client_id: client.id,
            name: "prod".to_string(),
            enabled: true,
        }).await.expect("Failed to create environment");

        assert_eq!(env.name, "prod");
        assert_eq!(env.client_id, client.id);
        assert!(env.enabled);
    }

    #[tokio::test]
    async fn test_upsert_resource() {
        let state = setup_test_db().await;

        let client = state.upsert_client(NewClient {
            name: "test-client-3".to_string(),
            domain: "test3.example.com".to_string(),
            config: json!({}),
        }).await.expect("Failed to create client");

        let env = state.upsert_environment(NewEnvironment {
            client_id: client.id,
            name: "staging".to_string(),
            enabled: true,
        }).await.expect("Failed to create environment");

        let resource = state.upsert_resource(NewResource {
            environment_id: env.id,
            resource_type: "s3_bucket".to_string(),
            resource_name: "website".to_string(),
            resource_id: Some("my-bucket".to_string()),
            arn: Some("arn:aws:s3:::my-bucket".to_string()),
            state: json!({"bucket_name": "my-bucket"}),
        }).await.expect("Failed to create resource");

        assert_eq!(resource.resource_type, "s3_bucket");
        assert_eq!(resource.resource_id.unwrap(), "my-bucket");

        let updated = state.upsert_resource(NewResource {
            environment_id: env.id,
            resource_type: "s3_bucket".to_string(),
            resource_name: "website".to_string(),
            resource_id: Some("my-bucket-updated".to_string()),
            arn: Some("arn:aws:s3:::my-bucket-updated".to_string()),
            state: json!({"bucket_name": "my-bucket-updated"}),
        }).await.expect("Failed to update resource");

        assert_eq!(updated.id, resource.id);
        assert_eq!(updated.resource_id.unwrap(), "my-bucket-updated");
    }
}
