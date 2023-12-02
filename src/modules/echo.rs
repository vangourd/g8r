use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::ops::RangeBounds;

use crate::utils::task::Task;

pub struct EchoTask {
    module: String,
    mutate: bool,
    config: serde_yaml::Value
}

impl Task for EchoTask {
    fn new(module: &str, mutate: bool, config: serde_yaml::Value) -> Result<Self, Box<dyn Error>> {
        Ok(EchoTask { module: module, mutate: mutate, config: config })
    }
}