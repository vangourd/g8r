use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Roster {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,
    pub name: String,
    pub roster_type: String,
    #[sqlx(json)]
    pub traits: Vec<String>,
    #[sqlx(json)]
    pub connection: JsonValue,
    #[sqlx(json)]
    pub auth: JsonValue,
    #[sqlx(default)]
    pub metadata: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewRoster {
    pub name: String,
    pub roster_type: String,
    pub traits: Vec<String>,
    pub connection: JsonValue,
    pub auth: JsonValue,
    pub metadata: Option<JsonValue>,
}

impl Roster {
    pub fn has_trait(&self, trait_name: &str) -> bool {
        self.traits.iter().any(|t| t == trait_name)
    }
    
    pub fn has_all_traits(&self, required_traits: &[&str]) -> bool {
        required_traits.iter().all(|t| self.has_trait(t))
    }
    
    pub fn matches_selector(&self, selector: &RosterSelector) -> bool {
        if let Some(required_traits) = &selector.traits {
            if !self.has_all_traits(required_traits.iter().map(|s| s.as_str()).collect::<Vec<_>>().as_slice()) {
                return false;
            }
        }
        
        if let Some(roster_type) = &selector.roster_type {
            if &self.roster_type != roster_type {
                return false;
            }
        }
        
        true
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RosterSelector {
    pub traits: Option<Vec<String>>,
    pub roster_type: Option<String>,
}
