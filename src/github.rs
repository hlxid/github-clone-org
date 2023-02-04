use futures::future::{BoxFuture, FutureExt};
use reqwest::header::USER_AGENT;
use reqwest::StatusCode;

use crate::repository::RepositoryMetadata;

const GITHUB_API: &str = "https://api.github.com";
const PAGE_SIZE: usize = 100;

// region get_repos

pub async fn get_repos<S: AsRef<str>>(
    entity: S,
    filter_forks: bool,
) -> Result<Vec<RepositoryMetadata>, String> {
    let entity_ref = entity.as_ref();
    let repos = match get_repos_internal(entity_ref, true, 0).await {
        Ok(repos) => Ok(repos),
        Err(_) => get_repos_internal(entity_ref, false, 0).await,
    }?;

    Ok(repos
        .into_iter()
        .filter(|r| !r.fork || !filter_forks)
        .collect())
}

fn get_repos_internal(
    entity: &str,
    is_user: bool,
    current_page: usize,
) -> BoxFuture<Result<Vec<RepositoryMetadata>, String>> {
    async move {
        let response = match build_request(entity, is_user, current_page).send().await {
            Ok(response) => Ok(response),
            Err(err) => Err(err.to_string()),
        }?;

        match response.status() {
            StatusCode::OK => handle_received_repos(entity, is_user, current_page, response).await,
            StatusCode::NOT_FOUND => Err("entity is not valid".into()),
            _ => Err("unknown error".into()),
        }
    }
    .boxed()
}

async fn handle_received_repos(
    entity: &str,
    is_user: bool,
    current_page: usize,
    response: reqwest::Response,
) -> Result<Vec<RepositoryMetadata>, String> {
    match response.json::<Vec<RepositoryMetadata>>().await {
        Ok(mut repos) => {
            if repos.len() == PAGE_SIZE {
                let next_repos = get_repos_internal(entity, is_user, current_page + 1).await?;
                repos.extend(next_repos.iter().cloned())
            }
            Ok(repos)
        }
        Err(err) => Err(err.to_string()),
    }
}

fn build_request(entity: &str, is_user: bool, current_page: usize) -> reqwest::RequestBuilder {
    let descriptor = if is_user { "users" } else { "orgs" };
    let url = format!(
        "{GITHUB_API}/{descriptor}/{entity}/repos?per_page={PAGE_SIZE}&page={current_page}"
    );
    println!("url: {url}");

    reqwest::Client::new()
        .get(&url)
        .header(USER_AGENT, "github-clone-org")
}

// endregion get_repos

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn works_with_user() {
        let repos = get_repos("daniel0611", false);
        internal_test("daniel0611", repos.await.unwrap());
    }

    #[tokio::test]
    async fn works_with_org() {
        let repos = get_repos("kubernetes", false);
        internal_test("kubernetes", repos.await.unwrap());
    }

    #[tokio::test]
    #[should_panic(expected = "entity is not valid")]
    async fn fails_with_nonexistent_entity() {
        let entity = "abnkklvmdlkdklvvfdslkjdsfjldfslkdsalksadmlk";
        let repos = get_repos(entity, false); // Probably nobody will use this name, at least I hope
        internal_test(entity, repos.await.unwrap());
    }

    #[tokio::test]
    async fn filter_forks_works() {
        // Ensure that DT repo is there if forks aren't filtered.
        let repos = get_repos("daniel0611", false).await.unwrap();
        assert_eq!(
            repos.iter().find(|r| r.name == "DefinitelyTyped").is_some(),
            true
        );

        // but it should exist if forks are filtered out.
        let repos = get_repos("daniel0611", true).await.unwrap();
        assert_eq!(
            repos.iter().find(|r| r.name == "DefinitelyTyped").is_some(),
            false
        );
    }

    #[tokio::test]
    async fn pagination_works() {
        let repos = get_repos("github", false).await.unwrap();
        // GitHub has 373 repositories. Without doing anything the API would give us 30.
        // With page_size extended to 100 we would get 100 repositories.
        // But if pagination works and it follows the pages until it is finished we would
        // get way more, assuming GitHub won't delete too many repositories.
        assert!(repos.len() > 100);
    }

    fn internal_test(entity: &str, repos: Vec<RepositoryMetadata>) {
        assert!(repos.len() > 0); // Should find at least one repo

        let r = &repos[0];
        assert!(!r.name.is_empty()); // must have a name
        assert!(r.clone_url.contains(&r.name)); // repo name is part of clone url
        assert!(r.clone_url.contains(entity)); // entity is part of clone url
        assert!(r.clone_url.ends_with(".git")); // clone url must end with .git to be valid
    }
}
