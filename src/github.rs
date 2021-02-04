use crate::repository::Repository;

// TODO: use GraphQL API

const GITHUB_API: &str = "https://api.github.com";

use reqwest::header::USER_AGENT;
use reqwest::{blocking, StatusCode};

pub fn get_repos(entity: &String) -> Result<Vec<Repository>, String> {
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
