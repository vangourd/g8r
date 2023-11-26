use std::thread::sleep;
use log::{info,debug};
use tokio;
use hostname;

mod utils;
mod duties;

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
        // Load roster file
        let roster_path = format!("{}/{}",&config.local_path,&config.roster_path);
        info!("Loading roster file {}", &roster_path);
        let roster = utils::roster::Roster::new(&roster_path)
            .expect("Unable to locate roster file");
        info!("{}",roster);

        let current_hostname = hostname::get()
            .expect("Couldn't get hostname")
            .into_string()
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "Hostname is not valid UTF-8"))
            .unwrap();

        debug!("Detected hostname as {}",&current_hostname);

        let duties = roster.get_duties(&current_hostname);

        for duty in duties {
            info!("Duty: {}", duty.id());
            duty.parse();
            duty.execute();
        }
                // parse corresponding duty file
                    // pass configuration context to module for execution
        sleep(config.refresh.into());
    }

}

