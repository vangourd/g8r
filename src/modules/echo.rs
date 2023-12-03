use serde::{Serialize, Deserialize};
use crate::utils::task::Task;

#[derive(Serialize,Deserialize)]
pub struct EchoTask {
    module: String,
    mutate: bool,
    context: serde_yaml::Value
}

impl Task for EchoTask {
    fn new(module: String, mutate: bool, context: serde_yaml::Value) -> Result<Self, std::io::Error> {
        Ok(EchoTask { module: module, mutate: mutate, context: context })
    }
}