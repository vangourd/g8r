use git2::{Repository, Error};


pub fn clone(url: &str,token: &str) -> Result<Repository, Error> {
    let repo = Repository::clone(&url, "/iac")
        .expect("Failed to clone GIT URL {}",&url)
    

}