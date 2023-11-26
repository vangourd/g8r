
use crate::utils::task::Task;
use std::error::Error;

pub struct Duty{
    name: String,
    tasks: Option<Vec<Box<dyn Task>>>,
}

impl Duty {
    pub fn new(name: &str) -> Result<Self, Box<dyn Error>>  {
        Ok(Duty{
            name: name.to_string(),
            tasks: None
        })
    }

    pub fn load_tasks() -> Result<(), Box<dyn Error>>  {
        Ok(())
    }
}