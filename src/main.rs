use clap::{Clap, crate_authors, crate_version};

use repository::Repository;

use crate::repository::RepositoryMetadata;

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
    let path = format!("{}/{}", entity, meta.name);
    if Repository::is_at_path(&meta, &path) {
        match Repository::open(&meta, &path) {
            Ok(repo) => fetch_repo(&repo),
            Err(err) => println!("Couldn't open repository {} at {}: {}", meta.name, &path, err)
        };
    } else {
        clone_repo(&path, meta, opts);
    }
}

fn fetch_repo(repo: &Repository) {
    println!("Fetching {}...", repo.meta.name);
    match repo.fetch(handle_progress) {
        Ok(fetch_commit) => {
            println!("\nSuccessfully fetched {}.", repo.meta.clone_url);
            match repo.merge(&fetch_commit) {
                Err(err) => println!("Couldn't merge repo {}: {}", repo.meta.name, err),
                Ok(()) => ()
            }
        },
        Err(e) => panic!("{}", e),
    };
}

fn clone_repo(path: &str, meta: &RepositoryMetadata, opts: &Opts) {
    println!("Cloning {} repository...", meta.name);
    match Repository::clone(meta, &path, handle_progress, opts.bare) {
        Err(e) => panic!("Error while cloning: {}", e),
        Ok(_) => (),
    };
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
