use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use anyhow::anyhow;
use async_trait::async_trait;
use clap::Args;
use tracing_log::log;

use crate::args::CliCommand;
use crate::common::working_dir_path;
use crate::{cmd, cmd_ignore_err, common, Cli};

#[derive(Args)]
#[command(about = "Create a tag to owner's repos for a release")]
pub struct TagArgs {
    #[arg(short, long, help = "Branch name")]
    branch: String,

    #[arg(short, long, help = "Tag")]
    tag: String,

    #[arg(
        short,
        long,
        default_value = "longhorn",
        hide = true,
        help = "GitHub Owner"
    )]
    owner: String,

    #[arg(short,
    long,
    default_values = [
    "longhorn-ui",
    "longhorn-manager",
    "longhorn-engine",
    "longhorn-instance-manager",
    "longhorn-share-manager",
    "backing-image-manager",
    ],
    hide = true,
    help = "GitHub Repos to tag")]
    repos: Vec<String>,

    #[arg(short, long, help = "Git commit message")]
    message: Option<String>,

    #[arg(
        short,
        long,
        default_value = "false",
        help = "Force to delete the existing tag"
    )]
    force: bool,
}

#[async_trait]
impl CliCommand for TagArgs {
    async fn run(&self, _: &Cli) -> anyhow::Result<()> {
        for repo in &self.repos {
            let repo_path = format!("{}/{}", self.owner, repo);
            let repo_dir_path = working_dir_path().join(repo);

            common::clone_repo(&repo_path, &self.branch, &repo_dir_path, working_dir_path())?;

            self.delete_tag(&repo_path, &repo_dir_path)?;
            self.create_tag(&repo_path, &repo_dir_path)?;
        }

        Ok(())
    }
}

impl TagArgs {
    fn create_tag(&self, repo_path: &str, repo_dir_path: &PathBuf) -> anyhow::Result<()> {
        let msg = self
            .message
            .clone()
            .unwrap_or(format!("release: {}", self.tag));
        let version_file_path = repo_dir_path.join("version");

        if !version_file_path.exists() || fs::read_to_string(&version_file_path)? != self.tag {
            log::info!(
                "Updating the version file {:?} with {}, and making the release commit",
                version_file_path,
                self.tag
            );

            let mut version_file = File::create(&version_file_path)?;
            version_file.write_all(self.tag.as_bytes())?;

            cmd!("git", &repo_dir_path, ["commit", "-am", &msg, "-s"]);
            cmd!("git", &repo_dir_path, ["push"]);
        }

        log::info!("Creating tag {repo_path}/{}", self.tag);
        cmd!("git", &repo_dir_path, ["tag", &self.tag]);

        log::info!("Pushing tag {repo_path}/{} to remote repo", self.tag);
        cmd!("git", &repo_dir_path, ["push", "origin", &self.tag]);

        Ok(())
    }

    fn delete_tag(&self, repo_path: &str, repo_dir_path: &PathBuf) -> anyhow::Result<()> {
        log::info!("Checking if tag {repo_path}/{} exists", self.tag);

        cmd_ignore_err!(
            "git",
            &repo_dir_path,
            ["rev-parse", &format!("refs/tags/{}", &self.tag)],
            {
                if !self.force {
                    return Err(anyhow!("Tag {}/{} already exits", repo_path, self.tag));
                }

                log::info!("Deleting existing tag {repo_path}/{}", self.tag);
                cmd!("git", &repo_dir_path, ["tag", "-d", &self.tag]);
                cmd!(
                    "git",
                    &repo_dir_path,
                    ["push", "--delete", "origin", &self.tag]
                );
            }
        );

        Ok(())
    }
}
