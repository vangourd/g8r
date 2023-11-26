
use crate::utils::task::Task;
use std::error:Error;

pub struct Duty{
    name: &str,
    tasks: <Option<Vec<Task>>
}

impl Duty {
    pub fn new(name: &str) -> Result<Duty, Error> {
        Duty{
            name: &name,
            tasks: None
        }
    }

    pub fn load_tasks() -> Result<(), Error> {
        Ok(())
    }
}