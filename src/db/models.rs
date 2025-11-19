use serde::{Deserialize, Serialize};
use sqlx::types::chrono::{DateTime, Utc};
use sqlx::types::JsonValue;


#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DutyExecution {
    pub id: i32,
    pub duty_id: i32,
    pub roster_id: Option<i32>,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub result: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Stack {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,
    pub name: String,
    pub source_type: String,
    #[sqlx(json)]
    pub source_config: JsonValue,
    pub config_path: String,
    pub reconcile_interval: Option<i32>,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_sync_version: Option<String>,
    pub status: String,
    #[sqlx(json)]
    pub metadata: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewStack {
    pub name: String,
    pub source_type: String,
    pub source_config: JsonValue,
    pub config_path: String,
    pub reconcile_interval: Option<i32>,
    pub metadata: Option<JsonValue>,
}

impl Stack {
    pub fn is_pending(&self) -> bool {
        self.status == "pending"
    }
    
    pub fn is_synced(&self) -> bool {
        self.status == "synced"
    }
    
    pub fn is_error(&self) -> bool {
        self.status == "error"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Queue {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,
    pub name: String,
    pub queue_type: String,
    #[sqlx(json)]
    pub queue_config: JsonValue,
    pub message_handler: String,
    #[sqlx(json)]
    pub handler_config: Option<JsonValue>,
    pub status: String,
    #[sqlx(json)]
    pub metadata: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

impl Queue {
    pub fn is_active(&self) -> bool {
        self.status == "active"
    }
    
    pub fn is_paused(&self) -> bool {
        self.status == "paused"
    }
    
    pub fn is_error(&self) -> bool {
        self.status == "error"
    }
}
