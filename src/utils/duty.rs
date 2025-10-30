use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Duty {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,
    pub name: String,
    pub duty_type: String,
    pub backend: String,
    #[sqlx(json)]
    pub roster_selector: JsonValue,
    #[sqlx(json)]
    pub spec: JsonValue,
    #[sqlx(default)]
    pub status: Option<JsonValue>,
    #[sqlx(default)]
    pub metadata: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewDuty {
    pub name: String,
    pub duty_type: String,
    pub backend: String,
    pub roster_selector: JsonValue,
    pub spec: JsonValue,
    pub status: Option<JsonValue>,
    pub metadata: Option<JsonValue>,
}

impl Duty {
    pub fn get_phase(&self) -> String {
        self.status
            .as_ref()
            .and_then(|s| s.get("phase"))
            .and_then(|p| p.as_str())
            .unwrap_or("pending")
            .to_string()
    }
    
    pub fn is_pending(&self) -> bool {
        self.get_phase() == "pending"
    }
    
    pub fn is_deployed(&self) -> bool {
        self.get_phase() == "deployed" || self.get_phase() == "active"
    }
    
    pub fn is_failed(&self) -> bool {
        self.get_phase() == "failed"
    }
}
