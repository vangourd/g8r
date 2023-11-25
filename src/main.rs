use std::thread::sleep;
use std::time::Duration;

use tokio;

mod utils;

use log::{info, warn, error, log_enabled, Level};


#[tokio::main]
async fn main() {
    env_logger::init();
    //print!("{esc}[2J{esc}[1;1H", esc = 27 as char);
    info!("Starting snapper ...");

    let config = utils::config::Config::from_file("config.yaml")
                    .expect("Failed to load config");

    println!("Initating reconciliation loop every {}",config.refresh);

    let mut iac = utils::repo::IacSync::new(config);
    iac.init();

    loop{
        if iac.out_of_sync().unwrap() {
            iac.reset().unwrap();
        }
        sleep(Duration::new(30,0));
    }

    // Evaluate IAC rules for host

}