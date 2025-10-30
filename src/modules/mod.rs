pub mod aws;
pub mod powerdns;
pub mod echo;

use async_trait::async_trait;
use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::utils::{Duty, Roster};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DutyState {
    NotDeployed,
    Deployed,
    Drifted,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub message: String,
    pub resources: Vec<ResourceInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceInfo {
    pub resource_type: String,
    pub resource_id: String,
    pub arn: Option<String>,
}

#[async_trait]
pub trait AutomationModule: Send + Sync {
    fn name(&self) -> &str;
    
    fn supported_duty_types(&self) -> Vec<&str>;
    
    fn required_roster_traits(&self) -> Vec<&str>;
    
    async fn validate(&self, roster: &Roster, duty: &Duty) -> Result<()>;
    
    async fn apply(&self, roster: &Roster, duty: &Duty) -> Result<serde_json::Value>;
    
    async fn destroy(&self, roster: &Roster, duty: &Duty) -> Result<()>;
}