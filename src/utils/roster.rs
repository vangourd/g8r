use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::fmt;

use crate::utils::duty::Duty;

#[derive(Serialize, Deserialize, Debug)]
pub struct Roster{
    duties: HashMap<String, Vec<String>>,
}

impl Roster {
    pub fn new(file_path: &str) -> Result<Roster, Box<dyn Error>> {
        let file_content = fs::read_to_string(file_path)?;
        let roster: Roster = serde_yaml::from_str(&file_content)?;
        Ok(roster)
    }

    pub fn get_duties(&self, hostname: &str) -> Vec<Duty> {
        self.duties.iter()
            .filter(|(_, hostnames) |hostnames.contains(&String::from(hostname)))
            .map(|(duty_name, _)| Duty{&duty_name})
            .collect()
    }
}

impl fmt::Display for Roster {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut duties_str = String::new();
        for (duty, hostnames) in &self.duties {
            let hostnames_str = hostnames.join(", ");
            duties_str.push_str(&format!("{}: [{}]\n", duty, hostnames_str));
        }
        write!(f, "{}", duties_str)
    }
}