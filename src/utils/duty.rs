use serde::de::DeserializeOwned;

use crate::utils;
use crate::duties;

#[derive(Debug, Clone)]
pub struct Duty {
    pub name: String
}

impl Duty {

    pub fn parse(&self) -> Result<Duty, Box<dyn std::error::Error>> {
        let file_path = format!("/duties/{}.yaml", &self.name);
        let contents = fs::read_to_string(file_path)?;
        let duty: Duty = serde_yaml::from_str(&contents)?;
        Ok(duty)
    }

    pub fn exec(duty: &Duty) {
        let c_duty = match duty.name.as_str() {
            "echo" => {
                duties::echo::EchoDuty::new()
            },
            _ => {
                error!("Invalid module: {}", duty.name);
            }
        }
    }
}

trait Duty{
    fn validate(&self) -> Result<(), String>;
    fn execute(&self);
    fn out_of_state(&self) -> Result<bool, Error>;
    fn apply(&self) -> 
}

