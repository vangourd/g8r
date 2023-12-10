use std::io;

use log::info;
use serde::{Serialize, Deserialize};
use crate::utils::task::TaskModule;

#[derive(Serialize,Deserialize)]
pub struct Echo {
    message: Option<String>
}

impl TaskModule for Echo {
    fn new(config: &serde_yaml::Value) -> Result<Self, std::io::Error> {
        serde_yaml::from_value::<Echo>(config.clone())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    fn apply(&self) -> Result<(), std::io::Error> {
        info!("{:?}",self.message.clone().unwrap());
        Ok(())
    }
}