use std::{thread::sleep, fs};
use log::{info,debug};
use std::path::Path;
use tokio;
use hostname;

use crate::utils::{duty::Duty, task::TaskFactory};

mod utils;
mod modules;

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
        //info!("{}",roster);

        let current_hostname = hostname::get()
            .expect("Couldn't get hostname")
            .into_string()
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "Hostname is not valid UTF-8"))
            .unwrap();

        debug!("Detected hostname as {}",&current_hostname);
                // parse corresponding duty file
                    // pass configuration context to module for execution

        for (duty, hostnames) in roster.duties {
            let mut current_duty_ids: Vec<String> = Vec::new();
            if hostnames.contains(&current_hostname) {
                current_duty_ids.push(duty);
            }
            // perform all duties
            for id in current_duty_ids {
                let dpath = Path::new(&config.duties_path);
                let absolute_path = match fs::canonicalize(&dpath) {
                    Ok(abs_path) => abs_path,
                    Err(e) => panic!("Error converting path: {}", e),
                };
                let dpath_str = absolute_path
                    .as_path()
                    .to_str()
                    .expect("Unable to convert to string");
                debug!("duty path: {}", &dpath_str);
                let duty = Duty::new(&dpath_str).unwrap();
                let tf = TaskFactory::new();
                for task_config in duty.configs {
                    println!("{:?}", task_config);
                }
            }
        }
        

        sleep(config.refresh.into());

}


}

