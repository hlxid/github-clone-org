use crate::repository::RepositoryMetadata;

// TODO: use GraphQL API

const GITHUB_API: &str = "https://api.github.com";

use reqwest::header::USER_AGENT;
use reqwest::StatusCode;

pub async fn get_repos<S: AsRef<str>>(entity: S) -> Result<Vec<RepositoryMetadata>, String> {
    let entity_ref = entity.as_ref();
    match get_repos_internal(entity_ref, true).await {
        Ok(repos) => Ok(repos),
        Err(_) => get_repos_internal(entity_ref, false).await,
    }
}

async fn get_repos_internal(entity: &str, is_user: bool) -> Result<Vec<RepositoryMetadata>, String> {
    let descriptor = if is_user { "users" } else { "orgs" };
    let url = format!("{}/{}/{}/repos", GITHUB_API, descriptor, entity);
    let client = reqwest::Client::new();
    let response = match client
        .get(&url)
        .header(USER_AGENT, "github-clone-org")
        .send()
        .await
    {
        Ok(response) => Ok(response),
        Err(err) => Err(format!("{}", err)),
    }?;

    match response.status() {
        StatusCode::OK => match response.json::<Vec<RepositoryMetadata>>().await {
            Ok(repos) => Ok(repos),
            Err(err) => Err(format!("{}", err)),
        },
        StatusCode::NOT_FOUND => Err("entity is not valid".into()),
        _ => Err("unknown error".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn works_with_user() {
        let repos = get_repos("daniel0611");
        internal_test("daniel0611", repos.await.unwrap());
    }

    #[tokio::test]
    async fn works_with_org() {
        let repos = get_repos("kubernetes");
        internal_test("kubernetes", repos.await.unwrap());
    }

    #[tokio::test]
    #[should_panic(expected = "entity is not valid")]
    async fn fails_with_nonexistent_entity() {
        let entity = "abnkklvmdlkdklvvfdslkjdsfjldfslkdsalksadmlk";
        let repos = get_repos(entity); // Probably nobody will use this name, at least I hope
        internal_test(entity, repos.await.unwrap());
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
