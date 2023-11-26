use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use log::{info};
use std::fs;
use std::fmt;

use crate::utils::task::Task;

#[derive(Serialize, Deserialize, Debug)]
pub struct Roster{
    duties: HashMap<String, Vec<String>>,
}

impl Roster {
    pub fn new(file_path: &str) -> Result<Self, Box<dyn Error>> {
        let file_content = fs::read_to_string(file_path)?;
        let roster: Roster = serde_yaml::from_str(&file_content)?;
        Ok(roster)
    }

    pub fn get_duties(&self, hostname: &str) {
        let mut duties_vec: Vec<Box<dyn Task>> = Vec::new();
        for (name, _list) in &self.duties {
            info!("{}",name);
        }
    }
}