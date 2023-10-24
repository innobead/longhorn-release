use std::fs;
use std::fs::File;
use std::io::Write;
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

pub fn create_tag(
    tag: &str,
    message: Option<String>,
    repo_path: &str,
    repo_dir_path: &PathBuf,
    version_file_created: bool,
) -> anyhow::Result<()> {
    let msg = message.unwrap_or(format!("release: {}", tag));
    let version_file_path = repo_dir_path.join("version");

    if version_file_created
        && (!version_file_path.exists() || fs::read_to_string(&version_file_path)? != tag)
    {
        log::info!(
            "Updating the version file {:?} with {}, and making the release commit",
            version_file_path,
            tag
        );

        let mut version_file = File::create(&version_file_path)?;
        version_file.write_all(tag.as_bytes())?;

        cmd!("git", &repo_dir_path, ["commit", "-am", &msg, "-s"]);
        cmd!("git", &repo_dir_path, ["push"]);
    }

    log::info!("Creating tag {repo_path}/{}", tag);
    cmd!("git", &repo_dir_path, ["tag", tag]);

    log::info!("Pushing tag {repo_path}/{} to remote repo", tag);
    cmd!("git", &repo_dir_path, ["push", "origin", tag]);

    Ok(())
}

pub fn delete_tag(
    tag: &str,
    repo_path: &str,
    repo_dir_path: &PathBuf,
    force: bool,
) -> anyhow::Result<()> {
    log::info!("Checking if tag {repo_path}/{} exists", tag);

    cmd_ignore_err!(
        "git",
        &repo_dir_path,
        ["rev-parse", &format!("refs/tags/{}", tag)],
        {
            if !force {
                return Err(anyhow!("Tag {}/{} already exits", repo_path, tag));
            }

            log::info!("Deleting existing tag {repo_path}/{}", tag);
            cmd!("git", &repo_dir_path, ["tag", "-d", tag]);
            cmd!("git", &repo_dir_path, ["push", "--delete", "origin", tag]);
        }
    );

    Ok(())
}
