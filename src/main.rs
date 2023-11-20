use std::fs;
use std::sync::{Arc, Mutex};
use toml::Value;
use tokio::fs as tokio_fs;
use tokio::sync::Semaphore;


#[tokio::main]
async fn main() {

    let semaphore = Arc::new(Semaphore::new(1));
    
    let shared_toml = Arc::new(Mutex::new(None));

    // Spawn a tokio task to read and parse the toml file
    let read_task = tokio::spawn(({
        let semaphore = semaphore.clone();
        let shared_toml = shared_toml.clone();
        async move {
            // Acquire the semaphore to gain exclusive access to shared_toml
            let _permit = semaphore.acquire().await.unwrap();

            // Read the TOML file into a string (use your actual file path)
            let toml_str = tokio_fs::read_to_string("config.toml")
                .await
                .expect("Failed to read configuration file toml");
            
            let toml_value: Value = toml::de::from_str(&toml_str).expect("Failed to parse TOML");

            // Store the parsed TOML data in the shared data structure
            *shared_toml.lock().unwrap() = Some(toml_value);
        }
    }));

    let access_task = tokio::spawn({
        let semaphore = semaphore.clone();
        let shared_toml = shared_toml.clone();
        async move {
            // Acquire the semaphore to gain access to shared_toml
            let _permit = semaphore.acquire().await.unwrap();
            
            // Access the parsed TOML data and print the value
            let mutex_guard = shared_toml.lock().unwrap();
            let toml_value = mutex_guard.as_ref().unwrap();
            let mode = toml_value["mode"].as_str().expect("Missing or invalid mode field");
            println!("mode: {}", mode);
        }
    });

    tokio::join!(read_task, access_task);


}