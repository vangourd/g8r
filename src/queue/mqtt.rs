use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use super::source::{QueueSource, QueueMessage};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttSourceConfig {
    pub broker_url: String,
    pub topic: String,
    pub client_id: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub qos: u8,
}

pub struct MqttSource {
    config: MqttSourceConfig,
}

impl MqttSource {
    pub fn new(config: MqttSourceConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl QueueSource for MqttSource {
    #[instrument(skip(self))]
    async fn init(&self) -> Result<()> {
        Ok(())
    }
    
    #[instrument(skip(self))]
    async fn connect(&self) -> Result<()> {
        Ok(())
    }
    
    #[instrument(skip(self))]
    async fn subscribe(&self) -> Result<()> {
        Ok(())
    }
    
    #[instrument(skip(self))]
    async fn receive_message(&self) -> Result<Option<QueueMessage>> {
        Ok(None)
    }
    
    #[instrument(skip(self))]
    async fn acknowledge(&self, _message_id: &str) -> Result<()> {
        Ok(())
    }
    
    #[instrument(skip(self))]
    async fn disconnect(&self) -> Result<()> {
        Ok(())
    }
}
