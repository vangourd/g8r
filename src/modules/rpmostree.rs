use serde::{Serialize, Deserialize};
use crate::utils::task::TaskModule;
use log::info;

#[derive(Serialize,Deserialize)]
pub struct RpmOstreeConfig {
    packages: Vec<String>,
    mode: String,
    tag: String
}

#[derive(Serialize, Deserialize)]
pub struct PackageState {
    package_name: String,
    state: PackageStateType,
}

#[derive(Serialize, Deserialize)]
enum PackageStateType {
    Installed,
    Removed,
}

impl TaskModule for RpmOstreeConfig {

    fn new(config: &serde_yaml::Value) -> Result<Self, std::io::Error> {
        serde_yaml::from_value(config.clone()).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to parse config: {}", e))
        })
    }

    fn parse(&mut self) -> Result<(), std::io::Error> {
        // Additional parsing logic if needed
        Ok(())
    }

    fn apply(&self) -> Result<(), std::io::Error> {
        for package in &self.packages {
            match package.state {
                PackageStateType::Installed => {
                    install_package(&package.package_name)?;
                },
                PackageStateType::Removed => {
                    remove_package(&package.package_name)?;
                },
            }
        }
        Ok(())
    }

    fn install_package(package_name: &str) -> Result<(), std::io::Error> {
        let output = Command::new("rpm-ostree")
                            .args(["install", package_name])
                            .output()?;
    
        if output.status.success() {
            info!("Installed package: {}", package_name);
            Ok(())
        } else {
            let err = String::from_utf8_lossy(&output.stderr);
            error!("Failed to install package {}: {}", package_name, err);
            Err(std::io::Error::new(std::io::ErrorKind::Other, err.to_string()))
        }
    }
    
    
    fn remove_package(package_name: &str) -> Result<(), std::io::Error> {
        let output = Command::new("rpm-ostree")
                            .args(["uninstall", package_name])
                            .output()?;

        if output.status.success() {
            info!("Removed package: {}", package_name);
            Ok(())
        } else {
            let err = String::from_utf8_lossy(&output.stderr);
            error!("Failed to remove package {}: {}", package_name, err);
            Err(std::io::Error::new(std::io::ErrorKind::Other, err.to_string()))
        }
    }

}