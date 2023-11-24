use git2::build::CheckoutBuilder;
use git2::{Repository, Remote, ObjectType, Error, Config};
use git2::{Cred, RemoteCallbacks, ResetType};
use log::{info, warn, error, log_enabled, Level, debug};
use std::{path::Path};
use url::{Url, ParseError};

use crate::utils;

pub struct IacSync {
    local: Option<git2::Repository>,
    config: utils::config::Config,
}

impl IacSync {
    pub fn new(config: utils::config::Config) -> IacSync {
        return IacSync {
            config: config,
            local: None,
        }
    }


    pub fn init(&self) {

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


    pub fn check_local(&self){
        // Check if local repository exists
        if !Path::exists(Path::new(&self.config.local_path)) {
            let repo = clone_repo(&self.config)?;
            
        } else {
            let repo = Repository::open("./iac/")
                .expect("Unable to open existing repository path");
            fetch(&repo).expect("Unable to fetch from repo");

        }
    }
    pub fn clone_repo(&self) -> Result<Repository, Error> {

        let mut configured_url = Url::parse(&self.config.repo)
            .expect("Unable to parse URL");
    
        configured_url.set_username(&self.config.username)
            .expect("Unable to set username");
        configured_url.set_password(Some(&self.config.token))
            .expect("Unable to set password");
        
        error!("Configured URL: {}",&configured_url);
    
        let repo= Repository::clone(&configured_url.as_str(), &self.config.local_path)
            .expect("Unable to clone repository");
        info!("Cloned repository {}",&self.config.repo);
        Ok(repo)
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
    
}