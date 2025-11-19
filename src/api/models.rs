use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use sqlx::types::JsonValue;

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub database: String,
    pub timestamp: DateTime<Utc>,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateRosterRequest {
    pub name: String,
    pub roster_type: String,
    pub traits: Vec<String>,
    pub connection: JsonValue,
    pub auth: JsonValue,
    pub metadata: Option<JsonValue>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RosterResponse {
    pub id: i32,
    pub name: String,
    pub roster_type: String,
    pub traits: Vec<String>,
    pub connection: JsonValue,
    pub auth: JsonValue,
    pub metadata: Option<JsonValue>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateDutyRequest {
    pub name: String,
    pub duty_type: String,
    pub backend: String,
    pub roster_selector: JsonValue,
    pub spec: JsonValue,
    pub metadata: Option<JsonValue>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DutyResponse {
    pub id: i32,
    pub name: String,
    pub duty_type: String,
    pub backend: String,
    pub roster_selector: JsonValue,
    pub spec: JsonValue,
    pub status: Option<JsonValue>,
    pub metadata: Option<JsonValue>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReconcileResponse {
    pub duty_name: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateStackRequest {
    pub name: String,
    pub source_type: String,
    pub source_config: JsonValue,
    pub config_path: String,
    pub reconcile_interval: Option<i32>,
    pub metadata: Option<JsonValue>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StackResponse {
    pub id: i32,
    pub name: String,
    pub source_type: String,
    pub source_config: JsonValue,
    pub config_path: String,
    pub reconcile_interval: Option<i32>,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_sync_version: Option<String>,
    pub status: String,
    pub metadata: Option<JsonValue>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StackSyncResponse {
    pub stack_name: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateQueueRequest {
    pub name: String,
    pub queue_type: String,
    pub queue_config: JsonValue,
    pub message_handler: String,
    pub handler_config: Option<JsonValue>,
    pub metadata: Option<JsonValue>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueueResponse {
    pub id: i32,
    pub name: String,
    pub queue_type: String,
    pub queue_config: JsonValue,
    pub message_handler: String,
    pub handler_config: Option<JsonValue>,
    pub status: String,
    pub metadata: Option<JsonValue>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueueControlResponse {
    pub queue_name: String,
    pub status: String,
    pub message: String,
}
