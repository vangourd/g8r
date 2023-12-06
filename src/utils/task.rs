use std::collections::HashMap;

use serde_derive::Deserialize;
use serde_yaml::Value;

use crate::modules::echo::Echo;

pub trait TaskModule {
    fn new(config: &serde_yaml::Value) -> Result<Self, std::io::Error>
    where
        Self: Sized; 
    fn parse(&self, config: &serde_yaml::Value) -> Result<(), std::io::Error>;
    fn apply(&self) -> Result<(), std::io::Error>;
}

pub struct ModuleFactory;
impl ModuleFactory {
    pub fn create_module(name: &str, config: &serde_yaml::Value) -> Result<Box<dyn TaskModule>, std::io::Error> {
        match name {
            "echo" => Ok(Box::new(Echo::new(config)?)),
            _ => Err(std::io::Error::new(std::io::ErrorKind::Other, "Unknown module type")),
        }
    }
}


pub struct Task {
    config: &serde_yaml::Value,
    module: Box<dyn TaskModule>,
}

impl Task {
    pub fn new(name: &str, config: &serde_yaml::Value) -> Result<Task, std::io::Error> {
        let module = ModuleFactory::create_module(name, config)?;
        Ok(Task {config,module})
    }

    pub fn parse(&self) -> Result<(), std::io::Error> {
        self.module.parse(&self.config)
    }

    pub fn apply(&self) -> Result<(), std::io::Error> {
        self.module.apply()
    }
}