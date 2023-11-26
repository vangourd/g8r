use serde::de::DeserializeOwned;

use crate::utils;

#[derive(Debug, Clone)]
pub struct Duty {
    pub name: String
}

impl Duty {

    pub fn new(&config: utils::config::Config) -> Duty{
        Duty{
            name: 
        }
    } 

    pub fn parse(&self) -> Result<Duty, Box<dyn std::error::Error>> {
        let file_path = format!("/duties/{}.yaml", &self.name);
        let contents = fs::read_to_string(file_path)?;
        let duty: Duty = serde_yaml::from_str(&contents)?;
        Ok(duty)
    }

    pub fn exec(duty: &Duty) {
        
    }
}

trait Duty{
    fn validate(&self) -> Result<(), String>;
    fn execute(&self);
}

