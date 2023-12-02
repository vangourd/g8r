
use crate::utils::task::Task;
use core::fmt;
use std::collections::HashMap;
use std::error::Error;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Serialize,Deserialize)]
pub struct Duty{
    pub base: String,
    pub mutate: bool,
    pub tasks: serde_yaml::Value
}

impl Duty {
    pub fn new(file_path: &str) -> Result<Self, Box<dyn Error>> {
        let file_content = fs::read_to_string(file_path)?;
        let duty: Duty = serde_yaml::from_str(&file_content)?;
        Ok(duty)
    }
}

impl fmt::Display for Duty {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Base: {}, Configs: {:?}", &self.base, &self.tasks)
    }
}