mod repository;
use repository::Repository;

use reqwest::header::USER_AGENT;
use reqwest::{blocking, StatusCode};

use std::result::Result;

const GITHUB_API: &str = "https://api.github.com";

// TODO: support auth
// TODO: use tokio runtime

fn main() {
    println!("Hello, world!");
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 1 {
        eprintln!("No entity argument provided");
        return;
    }
    let entity = &args[1];
    println!("Entity: {}", entity);

    match get_repos(entity) {
        Ok(repositories) => {
            println!("Repos: {:#?}", repositories);
            clone_repositories(entity, &repositories);
        }
        Err(msg) => eprintln!("Error getting repositories: {}", msg),
    }
}

fn get_repos(entity: &String) -> Result<Vec<Repository>, String> {
    match get_repos_internal(entity, true) {
        Ok(repos) => Ok(repos),
        Err(_) => get_repos_internal(entity, false),
    }
}

fn get_repos_internal(entity: &String, is_user: bool) -> Result<Vec<Repository>, String> {
    let descriptor = if is_user { "users" } else { "orgs" };
    let url = format!("{}/{}/{}/repos", GITHUB_API, descriptor, entity);
    let client = blocking::Client::new();
    let response = match client
        .get(&url)
        .header(USER_AGENT, "github-clone-org")
        .send()
    {
        Ok(response) => Ok(response),
        Err(err) => Err(format!("{}", err)),
    }?;

    match response.status() {
        StatusCode::OK => match response.json::<Vec<Repository>>() {
            Ok(repos) => Ok(repos),
            Err(err) => Err(format!("{}", err)),
        },
        StatusCode::NOT_FOUND => Err("entity is not valid".into()),
        _ => Err("unknown error".into()),
    }
}

fn clone_repositories(entity: &String, repositories: &Vec<Repository>) {
    for repo in repositories {
        let path = format!("{}/{}", entity, repo.name);
        if repo.is_at_path(&path) {
            println!("Repo {} already cloned.", repo.name);
        } else {
            println!("Cloning {} repository...", repo.name);
            repo.clone(&path, |progress| {
                let rec = progress.received_objects();
                let tot = progress.total_objects();
                let percentage = 100 * rec / tot;
                print!(
                    "\r{}/{} ({}%)",
                    progress.received_objects(),
                    progress.total_objects(),
                    percentage
                );
            });
            println!("\nSuccessfully cloned {}.", repo.clone_url)
        }
    }
}
