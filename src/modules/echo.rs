use log::info;
use serde::{Serialize, Deserialize};
use crate::utils::task::TaskModule;

#[derive(Serialize,Deserialize)]
pub struct Echo {
    config: serde_yaml::Value,
    message: Option<String>
}

#[derive(Serialize,Deserialize)]
pub struct EchoConfig {
    message: String,
}

impl TaskModule for Echo {
    fn new(config: &serde_yaml::Value) -> Result<Self, std::io::Error> {
        Ok(Echo {config: config.clone(), message: None })
    }

    fn parse(&mut self) -> Result<(), std::io::Error> {
        let module_yaml = &self.config;
        let y: EchoConfig = serde_yaml::from_value(module_yaml.clone())
                .expect("Unable to parse module configuration data for Echo");
        self.message = Some(y.message);
        Ok(())
    }

    fn apply(&self) -> Result<(), std::io::Error> {
        info!("{:?}",self.message.clone().unwrap());
        Ok(())
    }
}