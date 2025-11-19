use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value as JsonValue;

#[async_trait]
pub trait QueueSource: Send + Sync {
    async fn init(&self) -> Result<()>;
    
    async fn connect(&self) -> Result<()>;
    
    async fn subscribe(&self) -> Result<()>;
    
    async fn receive_message(&self) -> Result<Option<QueueMessage>>;
    
    async fn acknowledge(&self, message_id: &str) -> Result<()>;
    
    async fn disconnect(&self) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct QueueMessage {
    pub id: String,
    pub payload: JsonValue,
    pub attributes: JsonValue,
    pub received_at: chrono::DateTime<chrono::Utc>,
}
