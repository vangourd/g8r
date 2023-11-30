
use crate::utils::task::Task;
use core::fmt;
use std::collections::HashMap;
use std::error::Error;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use super::task::TaskFactory;

#[derive(Serialize,Deserialize)]
pub struct Duty{
    pub name: String,
    pub base: String,
    pub configs: HashMap<String, String>,
}

impl Duty {
    pub fn new(file_path: &str) -> Result<Self, Box<dyn Error>> {
        let file_content = fs::read_to_string(file_path)?;
        let duty: Duty = serde_yaml::from_str(&file_content)?;
        Ok(duty)
    }

    pub fn id(&self) -> &str {
        return &self.name
    }

}

impl fmt::Display for Duty {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Name: {}, Base: {}, Configs: {:?}", &self.name, &self.base, &self.configs)
    }
}