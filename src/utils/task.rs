use std::collections::HashMap;

use serde_derive::Deserialize;
use serde_yaml::Value;

use crate::modules::echo::Echo;

pub trait TaskModule {
    fn new(config: &serde_yaml::Value) -> Result<Self, std::io::Error>
    where
        Self: Sized; 
    fn parse(&self) -> Result<(), std::io::Error>;
    fn apply(&self) -> Result<(), std::io::Error>;
}

pub struct ModuleFactory;
impl ModuleFactory {
    pub fn create_module(config: &serde_yaml::Value) -> Result<Box<dyn TaskModule>, std::io::Error> {
        if let serde_yaml::Value::Mapping(ref map) = *config {
            for (key, value) in map {
                if let serde_yaml::Value::String(ref module_type) = *key {
                    return match module_type.as_str() {
                        "echo" => Ok(Box::new(Echo::new(value)?)),
                        _ => Err(std::io::Error::new(std::io::ErrorKind::Other, "Unknown module type")),
                    }
                }
            }
        }
        Err(std::io::Error::new(std::io::ErrorKind::Other, "Invalid config format"))
    }
}


pub struct Task {
    module: Box<dyn TaskModule>,
}

impl Task {
    pub fn new(config: &serde_yaml::Value) -> Result<Task, std::io::Error> {
        let module = ModuleFactory::create_module(config)?;
        Ok(Task { module })
    }

    pub fn parse(&self) -> Result<(), std::io::Error> {
        self.module.parse()
    }

    pub fn apply(&self) -> Result<(), std::io::Error> {
        self.module.apply()
    }
}