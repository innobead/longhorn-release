use async_trait::async_trait;
use clap::Args;

use crate::args::CliCommand;
use crate::common::working_dir_path;
use crate::{common, Cli};

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

            common::delete_tag(&self.tag, &repo_path, &repo_dir_path, self.force)?;
            common::create_tag(
                &self.tag,
                self.message.clone(),
                &repo_path,
                &repo_dir_path,
                true,
            )?;
        }

        Ok(())
    }
}
