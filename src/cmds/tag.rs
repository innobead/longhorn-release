use async_trait::async_trait;
use clap::Args;
use tracing::log;

use crate::cmds::CliCommand;

use crate::git::{GitCli, GitOperationTrait};
use crate::Cli;

#[derive(Args)]
#[command(about = "Create a tag into repos")]
pub struct TagArgs {
    #[arg(long, help = "GitHub owner")]
    owner: String,

    #[arg(long, help = "GitHub repos")]
    repos: Vec<String>,

    #[arg(long, help = "Branch")]
    branch: String,

    #[arg(long, help = "Tag")]
    tag: String,

    #[arg(long, help = "Commit message")]
    message: Option<String>,

    #[arg(long, help = "Create a ")]
    create_version_file: bool,

    #[arg(short, long, help = "Force to delete the existing tag")]
    force: bool,
}

#[async_trait]
impl CliCommand for TagArgs {
    async fn run(&self, _: &Cli) -> anyhow::Result<()> {
        for repo in &self.repos {
            let git = GitCli::new(self.owner.clone(), repo.clone());
            git.clone_repo(&self.branch)?;

            match git.delete_tag(&self.tag, self.force) {
                Ok(_) => {}
                Err(err) => {
                    if !self.force {
                        log::warn!("Skipped to force creating tag {}: {}", self.tag, err);
                        return Ok(());
                    }
                }
            }

            git.create_tag(&self.tag, self.message.clone())?;
        }

        Ok(())
    }
}
