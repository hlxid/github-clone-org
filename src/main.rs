mod github;
mod repository;

use clap::{crate_authors, crate_version, Clap};
use repository::Repository;

#[derive(Clap)]
#[clap(version = crate_version!(), author = crate_authors!())]
pub struct Opts {
    #[clap(long, about = "Creates bare Git repositories")]
    bare: bool,
    #[clap(about = "User or Org of which all repositories shall be cloned")]
    entity: String,
}

// TODO: support auth

#[tokio::main]
async fn main() {
    let opts = Opts::parse();

    match github::get_repos(&opts.entity).await {
        Ok(repositories) => clone_repositories(&opts.entity, &repositories, &opts),
        Err(msg) => eprintln!("Error getting repositories: {}", msg),
    }
}

fn clone_repositories(entity: &String, repositories: &Vec<Repository>, opts: &Opts) {
    for repo in repositories {
        process_repo(entity, repo, opts);
    }
}

fn process_repo(entity: &String, repo: &Repository, opts: &Opts) {
    let path = format!("{}/{}", entity, repo.name);
    if repo.is_at_path(&path) {
        fetch_repo(&path, repo);
    } else {
        clone_repo(&path, repo, opts);
    }
}

fn fetch_repo(path: &String, repo: &Repository) {
    println!("Fetching {}...", repo.name);
    match repo.fetch(&path, handle_progress) {
        Ok(()) => println!("\nSuccessfully fetched {}.", repo.clone_url),
        Err(e) => panic!("{}", e),
    };
}

fn clone_repo(path: &String, repo: &Repository, opts: &Opts) {
    println!("Cloning {} repository...", repo.name);
    match repo.clone(&path, handle_progress, opts.bare) {
        Err(e) => panic!("Error while cloning: {}", e),
        Ok(()) => (),
    };
    println!("\nSuccessfully cloned {}.", repo.clone_url)
}

fn handle_progress(progress: git2::Progress) {
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
