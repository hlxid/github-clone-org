mod github;
mod repository;

use repository::Repository;

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

    match github::get_repos(entity) {
        Ok(repositories) => {
            println!("Repos: {:#?}", repositories);
            clone_repositories(entity, &repositories);
        }
        Err(msg) => eprintln!("Error getting repositories: {}", msg),
    }
}

fn clone_repositories(entity: &String, repositories: &Vec<Repository>) {
    for repo in repositories {
        process_repo(entity, repo);
    }
}

fn process_repo(entity: &String, repo: &Repository) {
    let path = format!("{}/{}", entity, repo.name);
    if repo.is_at_path(&path) {
        println!("Repo {} already cloned.", repo.name);
    } else {
        clone_repo(&path, repo);
    }
}

fn clone_repo(path: &String, repo: &Repository) {
    println!("Cloning {} repository...", repo.name);
    repo.clone(&path, handle_clone_progress);
    println!("\nSuccessfully cloned {}.", repo.clone_url)
}

fn handle_clone_progress(progress: git2::Progress) {
    let rec = progress.received_objects();
    let tot = progress.total_objects();
    let percentage = 100 * rec / tot;
    print!(
        "\r{}/{} ({}%)",
        progress.received_objects(),
        progress.total_objects(),
        percentage
    );
}