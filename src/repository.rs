use serde::Deserialize;
use std::path::Path;

use git2::{build::RepoBuilder, FetchOptions, Progress, RemoteCallbacks};

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

    pub fn clone<P: AsRef<Path>, F: Fn(Progress) + 'static>(&self, path: P, callback: F) {
        let mut cbs = RemoteCallbacks::new();
        cbs.transfer_progress(|progress| {
            callback(progress);
            true
        });

        let mut fetch_opts = FetchOptions::new();
        fetch_opts.remote_callbacks(cbs);

        let mut builder = RepoBuilder::new();
        builder.fetch_options(fetch_opts);

        match builder.clone(&self.clone_url, path.as_ref()) {
            Ok(_) => (),
            Err(e) => panic!(e),
        }
    }
}
