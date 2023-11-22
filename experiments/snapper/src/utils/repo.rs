use git2::{Repository, Remote};
use git2::{Cred, RemoteCallbacks};
use log::{info, warn, error, log_enabled, Level, debug};
use std::{path::Path};
use url::{Url, ParseError};

use crate::utils::config;


// pub fn clone(url: &str,token: &str) -> Result<Repository, Error> {
//     let repo = Repository::clone(&url, "/iac")
//         .expect("Failed to clone GIT URL {}",&url)
    

// }

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

    debug!("Fetching remote...");
    repo.find_remote("origin")
        .expect("Unable to find remote")
        .fetch(&["main"], None, None)
    
}