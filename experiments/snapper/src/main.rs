use tokio;

mod utils;
use utils::config::Config;

#[tokio::main]
async fn main() {
    println!("Starting snapper ...");

    let config = Config::from_file("config.yaml")
                    .expect("Failed to load config");

    println!("Printing {}", config);
}