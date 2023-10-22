use std::collections::HashMap;
use std::io::Write;

use async_trait::async_trait;
use clap::Args;
use filepath::FilePath;
use maplit::{hashmap, hashset};
use octocrab::models::issues::Issue;
use octocrab::params::State;
use tempfile::tempfile;

use crate::args::CliCommand;
use crate::common::{github_client, working_dir_path};
use crate::{cmd, Cli};

#[derive(Args)]
#[command(about = "Create a GitHub release")]
pub struct ReleaseArgs {
    #[arg(
        short,
        long,
        default_value = "longhorn",
        hide = true,
        help = "GitHub Owner"
    )]
    owner: String,

    #[arg(
        short,
        long,
        default_value = "longhorn",
        hide = true,
        help = "GitHub Repo for release"
    )]
    repo: String,

    #[arg(short, long, help = "Tag")]
    tag: String,

    #[arg(short, long, help = "Branch name")]
    branch: String,

    #[arg(short, long, help = "GitHub Milestone")]
    milestone: String,

    #[arg(long, help = "GitHub labels")]
    labels: Option<Vec<String>>,

    #[arg(long, help = "GitHub release title")]
    note_title: Option<String>,

    #[arg(long, help = "GitHub release note template file")]
    note_template: String,

    #[arg(long, help = "GitHub release section labels")]
    note_section_labels: Option<Vec<String>>,

    #[arg(long, help = "GitHub release extra contributors")]
    note_extra_contributors: Option<Vec<String>>,

    #[arg(
        long,
        default_value = "true",
        help = "Generate release note only instead of creating a GitHub release"
    )]
    note_only: bool,
}

#[async_trait]
impl CliCommand for ReleaseArgs {
    async fn run(&self, _: &Cli) -> anyhow::Result<()> {
        self.check_tag_exist()?;

        let issues = self.search_issues().await?;
        let release_note = self.create_draft_release(&issues)?;

        if self.note_only {
            println!("{}", release_note);
        }

        Ok(())
    }
}

impl ReleaseArgs {
    fn check_tag_exist(&self) -> anyhow::Result<()> {
        let repo_dir_path = working_dir_path().join(&self.repo);
        cmd!(
            "git",
            &repo_dir_path,
            ["rev-parse", &format!("refs/tags/{}", &self.tag)]
        );

        Ok(())
    }

    async fn search_issues(&self) -> anyhow::Result<Vec<Issue>> {
        Ok(github_client()
            .issues(&self.owner, &self.repo)
            .list()
            .labels(&self.labels.clone().unwrap_or_default())
            .per_page(255)
            .page(10u32)
            .state(State::All)
            .send()
            .await?
            .items)
    }

    fn create_draft_release(&self, issues: &[Issue]) -> anyhow::Result<String> {
        let mut note = self.note_template.clone();
        let mut sections: HashMap<String, Vec<&Issue>> = hashmap! {};
        let mut contributors = hashset! {};

        for contributor in self.note_extra_contributors.clone().unwrap_or_default() {
            contributors.insert(contributor);
        }

        for issue in issues {
            for assignee in &issue.assignees {
                contributors.insert(assignee.login.clone());
            }

            for label in self.note_section_labels.clone().unwrap_or_default() {
                let key = if issue.labels.iter().any(|it| it.name == label) {
                    &label[label.rfind('/').unwrap_or_default()..]
                } else {
                    "misc"
                };

                if sections.get(key).is_none() {
                    sections.insert(key.to_owned(), vec![]);
                }
                sections.get_mut(key).unwrap().push(issue);
            }
        }

        for (title, issues) in &sections {
            note += &format!("\n ## {title}\n");

            for issue in issues {
                let contributors: Vec<String> =
                    issue.assignees.iter().map(|it| it.login.clone()).collect();
                note += &format!(
                    "{}[{}]({}) - {}\n",
                    issue.title,
                    issue.number,
                    issue.url,
                    contributors.join(" ")
                );
            }
        }

        if !self.note_only {
            let repo_dir_path = working_dir_path().join(&self.repo);

            let mut note_file = tempfile()?;
            note_file.write_all(note.as_bytes())?;

            let release_title = self
                .note_title
                .clone()
                .unwrap_or(format!("{} {} release", self.repo, self.tag));

            cmd!(
                "gh",
                &repo_dir_path,
                [
                    "release",
                    "create",
                    &self.tag,
                    "--verify-tag",
                    "--notes-file",
                    note_file.path()?.as_os_str().to_str().unwrap(),
                    "-d",
                    "-p",
                    "--target",
                    &self.branch,
                    "-t",
                    &release_title
                ]
            );
        }

        Ok(note)
    }
}
