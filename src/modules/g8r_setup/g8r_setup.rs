use std::fs::{self, Permissions};
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use log::info;
use serde::{Serialize, Deserialize};
use crate::utils::task::TaskModule;

#[derive(Serialize, Deserialize)]
pub struct G8rSetup {
    systemd: Option<bool>,
    // other fields if necessary
}

impl TaskModule for G8rSetup {
    fn new(config: &serde_yaml::Value) -> Result<Self, io::Error> {
        let setup = serde_yaml::from_value::<G8rSetup>(config.clone())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        setup.setup_application_directories()?;
        if setup.systemd.unwrap_or(false) {
            setup.setup_systemd_service()?;
        }

        Ok(setup)
    }

    fn apply(&self) -> Result<(), io::Error> {
        info!("G8rSetup applied with systemd: {:?}", self.systemd);
        // Your application logic here
        Ok(())
    }
}

impl G8rSetup {
    fn setup_application_directories(&self) -> io::Result<()> {
        self.create_dir_if_not_exists("/usr/local/bin/myapp")?;
        self.create_dir_if_not_exists("/etc/myapp/")?;
        self.create_dir_if_not_exists("/var/log/myapp/")?;
        self.create_dir_if_not_exists("/var/lib/myapp/")?;
        Ok(())
    }

    fn create_dir_if_not_exists(&self, dir: &str) -> io::Result<()> {
        let path = Path::new(dir);
        if !path.exists() {
            fs::create_dir_all(path)?;
            fs::set_permissions(path, Permissions::from_mode(0o755))?;
        }
        Ok(())
    }

    fn enable_and_start_systemd_service(&self) -> io::Result<(), std::io::Error>{
        let service_name = "g8r.service";

        let enable_output = Command::new("systemctl")
            .args(&["enable", service_name])
            .output()?;

        if !enable_output.status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other, "Failed to enable the systemd service"
            ));

        let start_output = Command::new("systemctl")
            .args(&["start"], service_name)
            .output()?;

        if !start_output.status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other, "Failed to start start the systemd service"
            ));
        }

        }
    }

    fn setup_systemd_service(&self) -> io::Result<()> {
        let service_path = "/etc/systemd/system/myapp.service";
        let service_content = r#"
                [Unit]
                Description=g8r is a powerful configuration management and event-driven automation engine
                [Service]
                ExecStart=/usr/local/bin/g8r
                [Install]
                WantedBy=multi-user.target
                "#;
        
        if !Path::new(service_path).exists() {
            let mut file = fs::File::create(service_path)?;
            file.write_all(service_content.as_bytes())?;
        }
        // Further logic to enable and start the service can be added here
        Ok(())
    }
}
