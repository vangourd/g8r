use std::{error::Error, collections::HashMap};
use serde_derive::{Deserialize, Serialize};
use serde_yaml::Value;
use crate::modules::echo::Echo;

pub trait Task {
    fn new(module: &str, mutate: bool, config: Value) -> Result<Self, Box<dyn Error>>
    where
        Self: Sized; 
}