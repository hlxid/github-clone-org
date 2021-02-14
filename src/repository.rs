use serde::Deserialize;
use std::path::Path;

use git2::{
    build::RepoBuilder, FetchOptions, Progress, RemoteCallbacks, Repository as GitRepository,
};

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
        let p = path.as_ref();
        p.exists() && GitRepository::open(p).is_ok()
    }

    pub fn clone<P: AsRef<Path>, F: Fn(Progress) + 'static>(
        &self,
        path: P,
        callback: F,
        bare: bool,
    ) -> Result<(), git2::Error> {
        let mut builder = RepoBuilder::new();
        builder.fetch_options(Repository::build_fetch_options(callback));
        builder.bare(bare);

        match builder.clone(&self.clone_url, path.as_ref()) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    pub fn fetch<P: AsRef<Path>, F: Fn(Progress) + 'static>(
        &self,
        path: P,
        callback: F,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = GitRepository::open(path)?;
        Repository::fetch_internal(&repo, callback)?;
        Ok(())
    }

    fn fetch_internal<F: Fn(Progress) + 'static>(
        repo: &GitRepository,
        callback: F,
    ) -> Result<git2::AnnotatedCommit, Box<dyn std::error::Error>> {
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

    // TODO: support fastforwards
}

#[cfg(test)]
mod tests {
    use super::*;

    mod clone {
        #[test]
        fn should_work() {
            let path = &tempfile::tempdir().unwrap();
            let r = super::test_repo();
            r.clone(path, |_p| {}, false).unwrap();

            assert!(r.is_at_path(path)); // is_at_path must be true, because we cloned it there
            assert!(path.path().join(".git").exists()); // .git directory should exist
        }

        #[test]
        fn should_work_bare() {
            let path = &tempfile::tempdir().unwrap();
            let r = super::test_repo();
            r.clone(path, |_p| {}, true).unwrap();

            assert!(r.is_at_path(path)); // is_at_path must be true, because we cloned it there
        }

        #[test]
        #[should_panic(expected = "unsupported URL protocol")]
        fn should_fail_on_invalid_clone_url() {
            let path = &tempfile::tempdir().unwrap();
            let mut r = super::test_repo();
            r.clone_url = "test123".to_owned();
            r.clone(path, |_p| {}, false).unwrap();
        }
    }

    mod fetch {
        #[test]
        fn should_work_on_valid_repo() {
            let path = &tempfile::tempdir().unwrap();
            let r = super::test_repo();
            r.clone(path, |_p| {}, false).unwrap();
            r.fetch(path, |_p| {}).unwrap();
        }

        #[test]
        #[should_panic(expected = "could not find repository")]
        fn should_fail_on_empty_dir() {
            let path = &tempfile::tempdir().unwrap();
            super::test_repo().fetch(path, |_p| {}).unwrap();
        }
    }

    #[test]
    fn empty_dir_is_not_repo() {
        let path = tempfile::tempdir().unwrap();
        let sub_dir = path.path().join("test"); // dir doesn't exist there => repo is not there
        assert!(!test_repo().is_at_path(sub_dir));
    }

    fn test_repo() -> Repository {
        Repository {
            name: "Hello-World".to_owned(),
            clone_url: "https://github.com/octocat/Hello-World.git".to_owned(),
        }
    }
}
