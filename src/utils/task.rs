use std::{error::Error, collections::HashMap};
use serde_derive::{Deserialize, Serialize};
use crate::modules::echo::Echo;

pub trait Task{
    fn validate(&self,config: &HashMap<String,String>) -> Result<(), Box<dyn Error>>;
    fn apply(&self,config: &HashMap<String,String>) -> Result<(), Box<dyn Error>>;
}

pub struct TaskConfig{
    module: String,
    vars: HashMap<String, String>
}

pub struct TaskFactory {
    task_queue: Vec<(Box<dyn Task>, HashMap<String, String>)>,
}

impl TaskFactory {
    pub fn new() -> TaskFactory  {
        TaskFactory {
            task_queue: Vec::new(),
        }
    }

    fn add_task(&mut self, config: TaskConfig) {
        match config.module.as_str() {
            "echo" => {
                let echo_task = Echo;
                if let Ok(_) = echo_task.validate(&config.vars) {
                    self.task_queue.push((Box::new(echo_task), config.vars));
                }
            }
            _ => println!("Unknown module"),
        }
    }
}