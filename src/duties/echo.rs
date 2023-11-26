struct EchoDuty {
    count: i32,
}

impl Duty for EchoDuty {
    fn validate(&self) -> Result<(), String> {
        if self.message.is_empty() {
            Err("Message cannot be empty".to_string())
        } else {
            Ok(())
        }
    }

    fn execute(&self) {
        println!("{}", self.message);
    }
}

