use std::path::Path;

use git2::{
    build::RepoBuilder, FetchOptions, MergeAnalysis, Progress, RemoteCallbacks,
    Repository as GitRepository,
};
use serde::Deserialize;

const FETCH_HEAD_REF: &str = "FETCH_HEAD";
const REMOTE_NAME: &str = "origin";
const DEFAULT_BRANCH: &str = "master";
const DEFAULT_BRANCH_REF: &str = "refs/heads/master";

#[derive(Deserialize, Debug, Clone)]
pub struct RepositoryMetadata {
    pub name: String,
    pub clone_url: String,
}

impl RepositoryMetadata {
    pub fn is_at_path<P: AsRef<Path>>(&self, path: P) -> bool {
        let p = path.as_ref();
        if !p.exists() {
            return false;
        }

        let repo = GitRepository::open(p);
        if repo.is_err() {
            return false;
        }
        let repo = repo.unwrap();

        let remote = repo.find_remote(REMOTE_NAME);
        if remote.is_err() {
            return false;
        }
        let remote = remote.unwrap();

        match remote.url() {
            Some(url) => url == self.clone_url,
            None => false
        }
    }
}

pub struct Repository {
    pub meta: RepositoryMetadata,
    git: GitRepository,
}

impl Repository {
    pub fn open<P: AsRef<Path>>(
        meta: &RepositoryMetadata,
        path: P,
    ) -> Result<Repository, git2::Error> {
        Ok(Repository {
            meta: meta.clone(),
            git: git2::Repository::open(path)?,
        })
    }

    // region clone & fetch

    pub fn clone<P: AsRef<Path>, F: Fn(Progress) + 'static>(
        meta: &RepositoryMetadata,
        path: P,
        callback: F,
        bare: bool,
    ) -> Result<Repository, git2::Error> {
        let mut builder = RepoBuilder::new();
        builder.fetch_options(Repository::build_fetch_options(callback));
        builder.bare(bare);

        match builder.clone(&meta.clone_url, path.as_ref()) {
            Ok(repo) => Ok(Repository {
                meta: meta.clone(),
                git: repo,
            }),
            Err(e) => Err(e),
        }
    }

    pub fn fetch<F: Fn(Progress) + 'static>(
        &self,
        callback: F,
    ) -> Result<git2::AnnotatedCommit, Box<dyn std::error::Error>> {
        let mut fetch_opts = Repository::build_fetch_options(callback);
        // Always fetch all tags.
        // Perform a download and also update tips
        fetch_opts.download_tags(git2::AutotagOption::All);

        let mut remote = self.git.find_remote(REMOTE_NAME)?;
        remote.fetch(&[DEFAULT_BRANCH], Some(&mut fetch_opts), None)?;

        let fetch_head_ref = self.git.find_reference(FETCH_HEAD_REF)?;
        let fetch_head_commit = self.git.reference_to_annotated_commit(&fetch_head_ref)?;
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

    // endregion

    // region merge

    pub fn merge(&self, fetch_commit: &git2::AnnotatedCommit) -> Result<(), git2::Error> {
        // Bare repositories don't have a working copy of the code that we would need to update.
        if self.git.is_bare() {
            return Ok(());
        }

        let (analysis, _) = self.git.merge_analysis(&[fetch_commit])?;
        if analysis.is_up_to_date() {
            Ok(()) // Nothing to do, already up to date.
        } else if analysis.is_fast_forward() {
            self.fast_forward(fetch_commit)
        } else {
            // Normal merge is also not supported because this tool is mainly for archival purposes
            // and if you modify it you can also do the merge yourself.
            self.merge_unsupported(&analysis);
            Ok(())
        }
    }

    fn merge_unsupported(&self, analysis: &MergeAnalysis) {
        println!(
            "Can't merge changes in {} repository: {:?}",
            self.meta.name, analysis
        );
        println!("You may wish to merge these changes manually.");
    }

    fn fast_forward(&self, fetch_commit: &git2::AnnotatedCommit) -> Result<(), git2::Error> {
        println!("Performing a fast forward in {}", self.meta.name);

        match self.git.find_reference(DEFAULT_BRANCH_REF) {
            Ok(mut r) => Repository::fast_forward_to_branch(&self.git, &fetch_commit, &mut r),
            Err(_) => Repository::set_head_directly_to_commit(
                &self.git,
                &fetch_commit,
                DEFAULT_BRANCH_REF,
            ),
        }
    }

    fn set_head_directly_to_commit(
        repo: &GitRepository,
        fetch_commit: &git2::AnnotatedCommit,
        ref_name: &str,
    ) -> Result<(), git2::Error> {
        // The branch doesn't exist so just set the reference to the
        // commit directly. Usually this is because you are pulling
        // into an empty repository.
        repo.reference(
            &ref_name,
            fetch_commit.id(),
            true,
            &format!("Setting {} to {}", "master", fetch_commit.id()),
        )?;
        repo.set_head(&ref_name)?;
        repo.checkout_head(Some(
            git2::build::CheckoutBuilder::default()
                .allow_conflicts(true)
                .conflict_style_merge(true)
                .force(),
        ))
    }

    fn fast_forward_to_branch(
        repo: &GitRepository,
        rc: &git2::AnnotatedCommit,
        lb: &mut git2::Reference,
    ) -> Result<(), git2::Error> {
        let name = match lb.name() {
            Some(s) => s.to_string(),
            None => String::from_utf8_lossy(lb.name_bytes()).to_string(),
        };
        let msg = format!("Fast-Forward: Setting {} to id: {}", name, rc.id());
        lb.set_target(rc.id(), &msg)?;
        repo.set_head(&name)?;
        repo.checkout_head(Some(
            git2::build::CheckoutBuilder::default()
                // For some reason force is required to make the working directory actually get updated
                // I don't know yet but it would be nice if we could do it without force.
                .force(),
        ))?;
        Ok(())
    }

    // endregion
}

// region test

#[cfg(test)]
mod tests {
    use super::*;

    mod is_at_path {
        use std::process::Command;

        #[test]
        fn newly_cloned_repo() {
            let path = &tempfile::tempdir().unwrap();
            let meta = super::test_repo();
            super::Repository::clone(&meta, path, |_p| {}, false).unwrap();
            assert!(meta.is_at_path(path)); // is_at_path must be true, because we cloned it there
        }

        #[test]
        fn no_remote() {
            let path = &tempfile::tempdir().unwrap();
            let meta = super::test_repo();
            super::Repository::clone(&meta, path, |_p| {}, false).unwrap();

            Command::new("git")
                .arg("remote")
                .arg("remove")
                .arg("origin")
                .current_dir(path)
                .spawn()
                .unwrap();

            assert_eq!(meta.is_at_path(path), false) // false since the remote is missing
        }
    }

    mod clone {
        #[test]
        fn normal() {
            let path = &tempfile::tempdir().unwrap();
            let meta = super::test_repo();
            super::Repository::clone(&meta, path, |_p| {}, false).unwrap();

            assert!(path.path().join(".git").exists()); // .git directory should exist
        }

        #[test]
        fn bare() {
            let path = &tempfile::tempdir().unwrap();
            let meta = super::test_repo();
            super::Repository::clone(&meta, path, |_p| {}, true).unwrap();

            assert!(meta.is_at_path(path)); // is_at_path must be true, because we cloned it there
        }

        #[test]
        #[should_panic(expected = "unsupported URL protocol")]
        fn invalid_clone_url() {
            let path = &tempfile::tempdir().unwrap();
            let mut meta = super::test_repo();
            meta.clone_url = "test123".to_owned();
            super::Repository::clone(&meta, path, |_p| {}, false).unwrap();
        }
    }

    mod fetch {
        #[test]
        fn valid_repo() {
            let path = &tempfile::tempdir().unwrap();
            let meta = super::test_repo();
            let repo = super::Repository::clone(&meta, path, |_p| {}, false).unwrap();
            repo.fetch(|_p| {}).unwrap();
        }

        #[test]
        #[should_panic(expected = "could not find repository")]
        fn empty_dir() {
            let path = &tempfile::tempdir().unwrap();
            let meta = super::test_repo();
            let repo = super::Repository::open(&meta, path).unwrap();
            repo.fetch(|_p| {}).unwrap();
        }
    }

    mod merge {
        use std::process::Command;

        #[test]
        fn up_to_date() {
            let path = &tempfile::tempdir().unwrap();
            let meta = super::test_repo();
            let repo = super::Repository::clone(&meta, path, |_p| {}, false).unwrap();

            let fetch_commit = repo.fetch(|_p| {}).unwrap();
            repo.merge(&fetch_commit).unwrap();
        }

        #[test]
        fn fast_forward() {
            let path = &tempfile::tempdir().unwrap();
            let meta = super::test_repo();
            let repo = super::Repository::clone(&meta, path, |_p| {}, false).unwrap();

            // Walk back to genesis commit
            Command::new("git")
                .arg("reset")
                .arg("553c2077f0edc3d5dc5d17262f6aa498e69d6f8e")
                .arg("--hard")
                .current_dir(path)
                .spawn()
                .unwrap();

            let fetch_commit = repo.fetch(|_p| {}).unwrap();
            repo.merge(&fetch_commit).unwrap();

            let out = Command::new("git")
                .arg("rev-parse")
                .arg("HEAD")
                .current_dir(path)
                .output()
                .unwrap();
            let resulting_commit_hash = String::from_utf8_lossy(&out.stdout);
            assert_eq!(
                resulting_commit_hash.trim(),
                "7fd1a60b01f91b314f59955a4e4d4e80d8edf11d"
            ) // commit has changed
        }
    }

    #[test]
    fn empty_dir_is_not_repo() {
        let path = tempfile::tempdir().unwrap();
        let sub_dir = path.path().join("test"); // dir doesn't exist there => repo is not there
        assert!(!test_repo().is_at_path(sub_dir));
    }

    fn test_repo() -> RepositoryMetadata {
        RepositoryMetadata {
            name: "Hello-World".to_owned(),
            clone_url: "https://github.com/octocat/Hello-World.git".to_owned(),
        }
    }
}

// endregion
