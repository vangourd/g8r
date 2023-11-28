
use crate::utils::task::Task;
use std::error::Error;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Serialize,Deserialize)]
pub struct Duty{
    pub name: String,
    pub base: String,
}

impl Duty {
    pub fn new(file_path: &str) -> Result<Self, Box<dyn Error>> {
        let file_content = fs::read_to_string(file_path)?;
        let duty: Duty = serde_yaml::from_str(&file_content)?;
        Ok(duty)
    }

    pub fn id(&self) -> &str {
        return &self.name
    }

    // pub fn load_tasks(&self, duty_path: &str) -> Result<Vec<Box<dyn Task>>,Box<dyn Error>>  {
    //     let path = Path::new("duties").join(format!("{}/{}.yaml", duty_path, &self.name));
    //     let content = fs::read_to_string(path)?;
    //     let duty_yaml: DutyYaml = serde_yaml::from_str(&content)?;

    //     let mut tasks = Vec::new();

    //     for task_def in duty_yaml.tasks {
    //         let task = Task::create(task_def.module, task_def.context)?;
    //         tasks.push(task);
    //     }

    //     Ok(tasks)

    // }
}