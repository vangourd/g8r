use std::{thread::sleep, fs};
use log::{info,debug};
use std::path::Path;
use tokio;
use hostname;

use crate::utils::duty::Duty;

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

        // Iterate all duties in the roster
        for (duty_name, hostnames) in roster.duties {
            if hostnames.contains(&current_hostname) {
                let duty_file_path = format!("{}/{}{}.yaml", &config.local_path, &config.duties_path, &duty_name);
                let absolute_path = get_absolute_path(&duty_file_path);

                let duty = Duty::new(&absolute_path)
                    .expect("Failed to create Duty from file");

                let _ = duty.schedule_tasks();
            }
        }


        sleep(config.refresh.into());

}


}



fn get_absolute_path(relative_path: &String) -> String {
    debug!("{}",relative_path);
    fs::canonicalize(Path::new(relative_path))
        .expect("Error converting path")
        .to_str()
        .expect("Unable to convert to string")
        .to_owned()
}