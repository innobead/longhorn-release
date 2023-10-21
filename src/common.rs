use std::{env, fs};
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::anyhow;
use tracing_log::{log, LogTracer};

use crate::{Cli, cmd, cmd_ignore_err};

pub(crate) fn get_release_dir_path(cli: &Cli) -> anyhow::Result<PathBuf> {
    let path = cli.working_dir.clone().unwrap_or(env::current_dir()?).join(".release");
    fs::create_dir_all(&path)?;

    Ok(path)
}

pub fn enable_logging(level: &str) -> anyhow::Result<()> {
    LogTracer::init()?;

    let level = tracing::Level::from_str(level)?;
    let subscriber = tracing_subscriber::FmtSubscriber::builder().with_max_level(level).finish();

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

pub fn clone_repo(repo_path: &str, branch: &str, repo_dir_path: &PathBuf, rel_dir_path: &PathBuf) -> anyhow::Result<()> {
    if repo_dir_path.exists() {
        log::info!("Fetching repo {repo_path} and reset to branch {branch}");

        cmd!("git", &repo_dir_path, ["fetch", "origin", branch]);
        cmd!("git", &repo_dir_path, ["reset", "--hard", &format!("origin/{}", branch)]);
        cmd!("git", &repo_dir_path, ["checkout", branch]);
    } else {
        log::info!("Cloning repo {repo_path}");
        cmd_ignore_err!(
            "gh",
            &rel_dir_path,
            ["repo", "clone", repo_path, "--", "--branch", branch]
        );
    }

    Ok(())
}