use std::path::Path;

use clap::{Clap, crate_authors, crate_version};

use repository::Repository;

use crate::repository::RepositoryMetadata;
use std::fs;

mod github;
mod repository;

#[derive(Clap)]
#[clap(version = crate_version ! (), author = crate_authors ! ())]
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

fn clone_repositories(entity: &str, repositories: &[RepositoryMetadata], opts: &Opts) {
    for repo in repositories {
        process_repo(entity, repo, opts);
    }
}

fn process_repo(entity: &str, meta: &RepositoryMetadata, opts: &Opts) {
    let path_string = format!("{}/{}", entity, meta.name);
    let path: &Path = Path::new(&path_string);
    if meta.is_at_path(path) {
        match Repository::open(&meta, path) {
            Ok(repo) => fetch_repo(&repo),
            Err(err) => println!(
                "Couldn't open repository {} at {}: {}",
                meta.name, path_string, err
            ),
        };
    } else {
        if path.exists() {
            // Repo already exists, but is invalid in some way.
            // Deleting it so it can be re-cloned again.
            println!("Repository {} is invalid. Re-cloning it...", meta.name);
            fs::remove_dir_all(path).unwrap();
        }
        clone_repo(path, meta, opts);
    }
}

fn fetch_repo(repo: &Repository) {
    println!("Fetching {}...", repo.meta.name);
    match repo.fetch(handle_progress) {
        Ok(fetch_commit) => {
            println!("\nSuccessfully fetched {}.", repo.meta.clone_url);
            match repo.merge(&fetch_commit) {
                Err(err) => println!("Couldn't merge repo {}: {}", repo.meta.name, err),
                Ok(()) => (),
            }
        }
        Err(e) => panic!("{}", e),
    };
}

fn clone_repo(path: &Path, meta: &RepositoryMetadata, opts: &Opts) {
    println!("Cloning {} repository...", meta.name);
    if let Err(e) = Repository::clone(meta, &path, handle_progress, opts.bare) {
        panic!("Error while cloning: {}", e);
    }
    println!("\nSuccessfully cloned {}.", meta.clone_url)
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
