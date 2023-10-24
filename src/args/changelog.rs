use async_trait::async_trait;
use clap::Args;
use indoc::formatdoc;

use crate::args::CliCommand;
use crate::common::working_dir_path;
use crate::{cmd, common, Cli};

#[derive(Args)]
#[command(about = "Create a Changelog for repos after a tag")]
pub struct ChangelogArgs {
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
}

#[async_trait]
impl CliCommand for ChangelogArgs {
    async fn run(&self, _: &Cli) -> anyhow::Result<()> {
        let mut result = String::new();

        for repo in &self.repos {
            let repo_path = format!("{}/{}", self.owner, repo);
            let repo_dir_path = working_dir_path().join(repo);

            common::clone_repo(&repo_path, &self.branch, &repo_dir_path, working_dir_path())?;

            let output = cmd!(
                "git",
                &repo_dir_path,
                ["log", &format!("{}..HEAD", &self.tag), "--oneline"]
            );

            if output.stdout.is_empty() {
                continue;
            }

            let log = String::from_utf8(output.stdout)?;

            result += &formatdoc! {"

                    ## {}
                    -----
            ", repo};

            for l in log.lines() {
                let (commit, msg) = l.split_once(' ').unwrap();

                result += &formatdoc! {"
                    - {} [{}]({})
                ", msg, commit, format!("https://github.com/{}/{}/commit/{}", self.owner, repo, commit)};
            }
        }

        println!("{}", result);

        Ok(())
    }
}
