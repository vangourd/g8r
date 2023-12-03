use serde_yaml::Value;

pub trait Task {
    fn new(module: String, mutate: bool, config: Value) -> Result<Self, std::io::Error>
    where
        Self: Sized; 
}

enum TaskType {
    EchoTask, //modules//echo.rs
}