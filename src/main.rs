use std::thread::sleep;
use std::time::Duration;
use log::{info};
use tokio;

mod utils;



#[tokio::main]
async fn main() {
    env_logger::init();
    //print!("{esc}[2J{esc}[1;1H", esc = 27 as char);
    info!("Starting snapper ...");

    let config = utils::config::Config::from_file("config.yaml")
                    .expect("Failed to load config");

    println!("Initating reconciliation loop every {}",config.refresh);

    let mut iac = utils::repo::IacSync::new(&config);
    iac.init();

    loop{
        if iac.out_of_sync().unwrap() {
            iac.reset().unwrap();
        }
        sleep(config.refresh.into());
        println!("New loop")
    }

    // Evaluate IAC rules for host

}