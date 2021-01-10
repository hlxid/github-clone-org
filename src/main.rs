use reqwest::header::USER_AGENT;
use reqwest::{blocking, StatusCode};
use serde::Deserialize;
use std::result::Result;

const GITHUB_API: &str = "https://api.github.com";

// TODO: support auth

// #[derive(Debug)]
#[derive(Deserialize, Debug)]
struct Repository {
    name: String,
    clone_url: String,
}

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
        Ok(repositories) => println!("Repos: {:#?}", repositories),
        Err(msg) => eprintln!("Error getting repositories: {}", msg),
    }
    // TODO: actually clone the repositories
}

fn get_repos(entity: &String) -> Result<Vec<Repository>, String> {
    // TODO: support users too
    let url = format!("{}/orgs/{}/repos", GITHUB_API, entity);
    let client = blocking::Client::new();
    let response = match client.get(&url).header(USER_AGENT, "github-clone-org").send() {
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
