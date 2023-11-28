use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::ops::RangeBounds;

use crate::utils::task::Task;

pub struct Echo;

impl Task for Echo {

    fn validate(&self, vars: &HashMap<String, String>) -> Result<(), Box<dyn Error>> {
        if vars.contains_key("echo_message") {
            Ok(())
        } else {
            Err("Validation failed: echo_message key not found".into())
        }
    }

    fn apply(&self, vars: &HashMap<String, String>) -> Result<(), Box<dyn Error>> {
        if let Some(message) = vars.get("echo_message") {
            println!("Echo: {}", message);
            Ok(())
        } else {
            Err("Application failed: echo_message key not found".into())
        }
    }

}