# github-clone-org

This is a small rust tool to clone all public GitHub repositories of an organization or a user.

## Installation

First you'll need `cargo` which should be installed with most rust installations. If you have cargo installed you can
run this to install `github-clone-org``:

```shell
$ cargo install --git https://github.com/daniel0611/github-clone-org.git
```

## Usage

Give it the name of the user or organization as an argument. There are also two optional parameters `--bare`
and `--no-forks` which should be self-explanatory.

As an example this would clone all my public repos into `./daniel0611/...`:

```shell
$ github-clone-org daniel0611
```

It will create a directory named after the org or user that you want to clone all repositories from. In this directory
there will be a sub-directory for each repository.

### Fetching repositories
If a repository is already cloned at the target location (maybe because you re-ran this tool) it will perform a fetch
instead of cloning. If you have a normal (non-bare) clone it will also merge it if possible and advance `HEAD` to the
newest fetched commit.