use std::path::PathBuf;
use std::str::FromStr;

use anyhow::anyhow;
use octocrab::Octocrab;
use tracing_log::{log, LogTracer};

use crate::{cmd, cmd_ignore_err};

pub fn enable_logging(level: &str) -> anyhow::Result<()> {
    LogTracer::init()?;

    let level = tracing::Level::from_str(level)?;
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(level)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

pub fn check_runtime_dependencies() -> anyhow::Result<()> {
    let deps = ["git", "gh", "helm", "nix"];

    for dep in deps {
        which::which(dep).map_err(|err| anyhow!("{dep}, {}", err))?;
    }

    Ok(())
}

pub fn clone_repo(
    repo_path: &str,
    branch: &str,
    repo_dir_path: &PathBuf,
    working_dir_path: &PathBuf,
) -> anyhow::Result<()> {
    if repo_dir_path.exists() {
        log::info!("Fetching repo {repo_path} and reset to branch {branch}");

        for args in [
            vec!["fetch", "origin", branch],
            vec!["reset", "--hard", &format!("origin/{}", branch)],
            vec!["checkout", branch],
        ] {
            cmd!("git", &repo_dir_path, args);
        }
    } else {
        log::info!("Cloning repo {repo_path}");

        cmd_ignore_err!(
            "gh",
            &working_dir_path,
            ["repo", "clone", repo_path, "--", "--branch", branch]
        );
    }

    Ok(())
}

pub fn working_dir_path<'a>() -> &'a PathBuf {
    &crate::global::RELEASE_DIR_PATH
}

pub fn github_client<'a>() -> &'a Octocrab {
    crate::global::GITHUB_CLIENT.get().unwrap()
}
