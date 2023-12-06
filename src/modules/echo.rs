use log::info;
use serde::{Serialize, Deserialize};
use crate::utils::task::{Task, TaskModule};

#[derive(Serialize,Deserialize)]
pub struct Echo {
    config: serde_yaml::Value,
}

impl TaskModule for Echo {
    fn new(config: &serde_yaml::Value) -> Result<Self, std::io::Error> {
        Ok(Echo {config: config.clone() })
    }

    fn parse(&self) -> Result<(), std::io::Error> {
        info!("Hey look we're parsing in echo!");
        Ok(())
    }

    fn apply(&self) -> Result<(), std::io::Error> {
        info!("Hey look we're applying in echo!");
        Ok(())
    }
}