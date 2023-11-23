use git2::build::CheckoutBuilder;
use git2::{Repository, Remote, ObjectType, Error, Config};
use git2::{Cred, RemoteCallbacks, ResetType};
use log::{info, warn, error, log_enabled, Level, debug};
use std::{path::Path};
use url::{Url, ParseError};

use crate::utils::config;

pub struct IacSync {
    remote: git2::Repository,
    local: git2::Repository,
    config: config::Config,
}

impl IacSync {
    pub fn new(config: Config) -> Result<Repository, Error> {

        // Use parsed config to populate an IacSync configuration
        this.config = config;

        if !Path::exists(Path::new(&path)) {
            self.clone_repo()
        } else {
            let repo = Repository::open("./iac/")
                .expect("Unable to open existing repository path");
            fetch(&repo).expect("Unable to fetch from repo");
    
            return Ok(repo)
        }
    }

    pub fn clone_repo(path: &str, repo: &str,username: &str, token: &str) -> Result<Repository, Error> {
            let mut configured_url = Url::parse(&repo)
                .expect("Unable to parse URL");
    
            configured_url.set_username(&username)
                .expect("Unable to set username");
            configured_url.set_password(Some(&token))
                .expect("Unable to set password");
            
            error!("Configured URL: {}",&configured_url);
    
            let repo= Repository::clone(&configured_url.as_str(), &repo_path)
                .expect("Unable to clone repository");
            info!("Cloned repository {}",&url);
            Ok(repo)
    }

    pub fn sync(){

    }


}

pub fn initialize(url: String, token: String, branch: String ,tag: String, username: String) -> Result<Repository, std::fmt::Error> {

    let repo_path = "./iac/";

    if !Path::exists(Path::new(&repo_path)) {

        let mut configured_url = Url::parse(&url)
            .expect("Unable to parse URL");

        configured_url.set_username(&username)
            .expect("Unable to set username");
        configured_url.set_password(Some(&token))
            .expect("Unable to set password");
        

        error!("Configured URL: {}",&configured_url);

        let repo= Repository::clone(&configured_url.as_str(), &repo_path)
            .expect("Unable to clone repository");
        info!("Cloned repository {}",&url);
        Ok(repo)
    } else {
        let repo = Repository::open("./iac/")
            .expect("Unable to open existing repository path");
        fetch(&repo).expect("Unable to fetch from repo");

        return Ok(repo)
    }

    
}

pub fn fetch(repo: &Repository) -> Result<(), git2::Error> {

    info!("Fetching remote");
    repo.find_remote("origin")
        .expect("Unable to find remote")
        .fetch(&["main"], None, None)
    
}

// Function to hard reset the current branch to 'origin/main
pub fn reset(repo: &Repository) -> Result<(), git2::Error> {

    info!("Resetting repository");
    
    // Locate the commit object for 'origin/main'; 
    let target_commit = repo.find_reference("origin/main")?.peel(ObjectType::Commit)?;

    // Create a CheckoutBuilder for configuring the hard reset
    // 'force()' ensures that all changes in the working directory are overritten
    let mut checkout_opts = CheckoutBuilder::new();

    // Perform the hard reset
    // This moves HEAD to 'origin/main', resets the index, and updates the working directory
    repo.reset(
        &target_commit,
        ResetType::Hard,
        Some(&mut checkout_opts.force()),
    )
}