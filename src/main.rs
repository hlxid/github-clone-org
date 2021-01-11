use reqwest::header::USER_AGENT;
use reqwest::{blocking, StatusCode};
use serde::Deserialize;

use std::fs;
use std::result::Result;
use std::path::Path;

use git2::{build::RepoBuilder, FetchOptions, RemoteCallbacks};

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
        Ok(repositories) => {
            println!("Repos: {:#?}", repositories);
            clone_repositories(entity, &repositories);
        }
        Err(msg) => eprintln!("Error getting repositories: {}", msg),
    }
    // TODO: actually clone the repositories
}

fn get_repos(entity: &String) -> Result<Vec<Repository>, String> {
    // TODO: support users too
    let url = format!("{}/orgs/{}/repos", GITHUB_API, entity);
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

fn create_entity_directory(entity: &String) {
    match fs::create_dir_all(entity) {
        Err(err) => panic!(err),
        _ => (),
    }
}

fn clone_repositories(entity: &String, repositories: &Vec<Repository>) {
    create_entity_directory(entity);
    for repo in repositories {
        let path = format!("{}/{}", entity, repo.name);
        clone_repository(path, repo);
    }
}

fn clone_repository(path: String, repo: &Repository) {
    // TODO: support bare only repositories
    println!("Cloning {} repository...", repo.clone_url);

    let mut cbs = RemoteCallbacks::new();
    cbs.transfer_progress(|progress| {
        let rec = progress.received_objects();
        let tot = progress.total_objects();
        let percentage = 100 * rec / tot;

        print!(
            "\r{}/{} ({}%)",
            progress.received_objects(),
            progress.total_objects(),
            percentage
        );
        true
    });

    let mut fetch_opts = FetchOptions::new();
    fetch_opts.remote_callbacks(cbs);

    let mut builder = RepoBuilder::new();
    builder.fetch_options(fetch_opts);

    match builder.clone(&repo.clone_url, Path::new(&path)) {
        Ok(_) => println!("\nSuccessfully cloned {}.", repo.clone_url),
        Err(e) => panic!(e),
    }
}
