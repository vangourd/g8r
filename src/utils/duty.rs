use std::error::Error;

pub trait Duty{
    fn id(&self) -> &str;
    fn parse(&self) -> Result<(), Box<dyn Error>>;
    fn execute(&self) -> Result<(), Box<dyn Error>>;
    fn out_of_spec(&self) -> Result<(), Box<dyn Error>>;
    fn apply(&self) -> Result<(), Box<dyn Error>>;
}

