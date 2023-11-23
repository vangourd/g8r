use serde_derive::Deserialize;
use core::fmt;
use std::fs;
use std::path::Path;
use std::error::Error;

#[derive(Debug, Deserialize)]
pub struct Config {
    // Define your configuration fields here
    pub repo: String,
    pub branch: String,
    pub refresh: String,
    pub token: String,
    pub tag: String,
    pub username: String,
    pub local_path: String,
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Configured to {}",self.repo)
    }
}

impl Config {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let contents = fs::read_to_string(path)?;
        let config = serde_yaml::from_str(&contents)?;
        Ok(config)
    }
}