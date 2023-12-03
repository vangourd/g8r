use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs;

#[derive(Serialize, Deserialize, Debug)]
pub struct Roster{
    pub duties: HashMap<String, Vec<String>>,
}

impl Roster {
    pub fn new(file_path: &str) -> Result<Self, Box<dyn Error>> {
        let file_content = fs::read_to_string(file_path)?;
        let roster: Roster = serde_yaml::from_str(&file_content)?;
        Ok(roster)
    }
}