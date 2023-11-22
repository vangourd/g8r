use std::thread::sleep;
use std::time::Duration;

use tokio;

mod utils;
use utils::config::Config;
use utils::repo;

#[tokio::main]
async fn main() {
    println!("Starting snapper ...");

    let config = Config::from_file("config.yaml")
                    .expect("Failed to load config");

    println!("Initating reconciliation loop every {}",config.refresh);
    // Initialize repo
    
    loop{
        println!("Reconciling...");
        println!("Action...");
        sleep(Duration::new(5,0));
        println!("Done...");
        sleep(Duration::new(30,0));
    }

    // Evaluate IAC rules for host

}