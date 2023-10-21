use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use anyhow::anyhow;
use clap::Args;
use tracing_log::log;

use crate::args::CliCommand;
use crate::common::RELEASE_DIR_PATH;
use crate::{cmd, cmd_ignore_err, common, Cli};

#[derive(Args)]
pub struct TagArgs {
    #[arg(short, long)]
    branch: String,

    #[arg(short, long)]
    tag: String,

    #[arg(short, long, default_value = "longhorn", hide = true)]
    group: Option<String>,

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
    hide = true)]
    repos: Option<Vec<String>>,

    #[arg(short, long)]
    message: Option<String>,

    #[arg(short, long, default_value = "false")]
    force: bool,
}

impl CliCommand for TagArgs {
    fn run(&self, _: &Cli) -> anyhow::Result<()> {
        let group = self.group.as_ref().expect("required");
        let repos = self.repos.as_ref().expect("required");

        for repo in repos {
            let repo_path = format!("{}/{}", group, repo);
            let repo_dir_path = RELEASE_DIR_PATH.join(repo);

            common::clone_repo(&repo_path, &self.branch, &repo_dir_path, &RELEASE_DIR_PATH)?;

            self.delete_existing_tag(&repo_path, &repo_dir_path)?;
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

    fn delete_existing_tag(&self, repo_path: &str, repo_dir_path: &PathBuf) -> anyhow::Result<()> {
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
