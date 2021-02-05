use serde::Deserialize;
use std::path::Path;

use git2::{build::RepoBuilder, FetchOptions, Progress, RemoteCallbacks, Repository as GitRepository};

const FETCH_HEAD_REF: &str = "FETCH_HEAD";

#[derive(Deserialize, Debug)]
pub struct Repository {
    pub name: String,
    pub clone_url: String,
}

impl Repository {
    pub fn is_at_path<P: AsRef<Path>>(&self, path: P) -> bool {
        // TODO: do other checks to verify it is this repository that is located at this path
        // e.g. check remote
        path.as_ref().exists()
    }

    pub fn clone<P: AsRef<Path>, F: Fn(Progress) + 'static>(&self, path: P, callback: F) -> Result<(), git2::Error> {
        let mut builder = RepoBuilder::new();
        builder.fetch_options(Repository::build_fetch_options(callback));

        match builder.clone(&self.clone_url, path.as_ref()) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    pub fn fetch<P: AsRef<Path>, F: Fn(Progress) + 'static>(&self, path: P, callback: F) -> Result<(), Box<dyn std::error::Error>> {
        let repo = GitRepository::open(path)?;
        Repository::fetch_internal(&repo, callback)?;
        Ok(())
    }

    fn fetch_internal<F: Fn(Progress) + 'static>(repo: &GitRepository, callback: F) -> Result<git2::AnnotatedCommit, Box<dyn std::error::Error>> {
        let mut fetch_opts = Repository::build_fetch_options(callback);
        // Always fetch all tags.
        // Perform a download and also update tips
        fetch_opts.download_tags(git2::AutotagOption::All);

        let remote_name = "origin";
        let mut remote = repo.find_remote(remote_name)?;
        remote.fetch(&["master"], Some(&mut fetch_opts), None)?;

        let fetch_head_ref = repo.find_reference(FETCH_HEAD_REF)?;
        let fetch_head_commit = repo.reference_to_annotated_commit(&fetch_head_ref)?;
        Ok(fetch_head_commit)
    }

    fn build_fetch_options<'a, F: Fn(Progress) + 'static>(callback: F) -> FetchOptions<'a> {
        let mut cbs = RemoteCallbacks::new();
        cbs.transfer_progress(move |progress| {
            callback(progress);
            true
        });

        let mut fetch_opts = FetchOptions::new();
        fetch_opts.remote_callbacks(cbs);
        fetch_opts
    }
}
