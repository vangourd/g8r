use git2::{Repository, ObjectType, ResetType};
use log::{info};
use std::{path::Path};
use url::{Url};

use super::config;

pub struct IacSync {
    local: Option<git2::Repository>,
    config: config::Config,
}

impl IacSync {
    pub fn new(config: &config::Config) -> IacSync {
        return IacSync {
            config: config.clone(),
            local: None,
        }
    }


    pub fn init(&mut self) {

        // Set where repo should be locally
        let repo_path = &self.config.local_path;

        // Check if repo already initialized
        if !Path::exists(Path::new(&repo_path)) {

            // Parse the repo url from file
            let mut configured_url = Url::parse(&self.config.repo)
                .expect("Unable to parse URL");

            // Interpolate values to authenticate via oauth token
            configured_url.set_username(&self.config.username)
                .expect("Unable to set username");
            configured_url.set_password(Some(&self.config.token))
                .expect("Unable to set password");

            // Clone the repository with the authenticated API call
            Repository::clone(&configured_url.as_str(), &repo_path)
                .expect("Unable to clone repository");
            info!("Cloned repository {}",&self.config.repo);

        } else {
            //
            let repo = Repository::open(&self.config.local_path)
                .expect("Unable to open existing repository path");
            self.local = Some(repo);
            self.fetch().expect("Unable to fetch from repo");
            self.reset().expect("Unable to reset repository");
        }

        
    }

    pub fn out_of_sync(&mut self) -> Result<bool, git2::Error> {

        let repo = self.local.as_mut().unwrap();

        repo.find_remote("origin").unwrap().fetch(&["main"], None, None)?;

        
        let local_branch_commit = repo.revparse_single("refs/heads/main").unwrap().id();
        let remote_branch_commit = repo.revparse_single("refs/remotes/origin/main").unwrap().id();

        if local_branch_commit != remote_branch_commit {
            Ok(true)
        } else {
            Ok(false)
        }

    }


    pub fn fetch(&mut self) -> Result<(), git2::Error> {
        info!("Fetching remote");
        self.local.as_mut()
            .expect("Unable to access local git repo")
            .find_remote("origin")
            .expect("Unable to find remote")
            .fetch(&["main"], None, None)
        
    }


    // Function to hard reset the current branch to 'origin/main
    pub fn reset(&mut self) -> Result<(), git2::Error> {

        info!("Resetting repository");
        
        // Locate the commit object for 'origin/main'; 
        let repo = self.local.as_mut().unwrap();
        let commit = repo.find_reference(&format!("FETCH_HEAD"))?.peel(ObjectType::Commit)?;
        //let branch = repo.find_branch("main", git2::BranchType::Local)?;

        // Perform the hard reset
        // This moves HEAD to 'origin/main', resets the index, and updates the working directory
        repo.reset(
            &commit,
            ResetType::Hard,
            None,
        )
    }
    
}