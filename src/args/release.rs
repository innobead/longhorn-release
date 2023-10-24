use std::{fs, vec};
use std::path::PathBuf;

use anyhow::anyhow;
use async_trait::async_trait;
use clap::Args;
use convert_case::{Case, Casing};
use indexmap::{indexmap, indexset};
use octocrab::models::issues::Issue;
use octocrab::models::Milestone;
use octocrab::params::State;
use tracing::log;

use crate::{Cli, cmd, common};
use crate::args::CliCommand;
use crate::common::{github_client, working_dir_path};

#[derive(Args)]
#[command(about = "Create a GitHub release")]
pub struct ReleaseArgs {
    #[arg(short,
    long,
    default_value = "longhorn",
    hide = true,
    help = "GitHub Owner")]
    owner: String,

    #[arg(short,
    long,
    default_value = "longhorn",
    hide = true,
    help = "GitHub Repo for release")]
    repo: String,

    #[arg(short, long, help = "Tag")]
    tag: String,

    #[arg(long, help = "Milestone")]
    milestone: String,

    #[arg(short, long, help = "Branch name")]
    branch: String,

    #[arg(short = 'a', long, help = "GitHub labels to search issues")]
    labels: Option<Vec<String>>,

    #[arg(short, long, help = "GitHub labels to exclude issues")]
    exclude_labels: Option<Vec<String>>,

    #[arg(long, help = "GitHub release title")]
    note_title: Option<String>,

    #[arg(long, help = "GitHub release note template file")]
    note_template: Option<String>,

    #[arg(short = 's', long, help = "GitHub release section labels")]
    note_section_labels: Option<Vec<String>>,

    #[arg(short = 'c', long, help = "GitHub release extra contributors")]
    note_extra_contributors: Option<Vec<String>>,

    #[arg(short, long, help = "Create a GitHub release with a generated release note")]
    publish: bool,

    #[arg(short, long, help = "Git commit message")]
    message: Option<String>,

    #[arg(short,
    long,
    default_value = "false",
    help = "Force to delete the existing tag")]
    force: bool,
}

#[async_trait]
impl CliCommand for ReleaseArgs {
    async fn run(&self, _: &Cli) -> anyhow::Result<()> {
        let repo_path = format!("{}/{}", self.owner, self.repo);
        let repo_dir_path = working_dir_path().join(&self.repo);

        common::clone_repo(&repo_path, &self.branch, &repo_dir_path, working_dir_path())?;
        common::delete_tag(&self.tag, &repo_path, &repo_dir_path, self.force)?;

        let issues = self.search_issues().await?;
        let note = if let Some(path) = self.note_template.clone() {
            let path = PathBuf::from(path);
            if !path.exists() {
                return Err(anyhow!("note template not found {:?}", path));
            }

            fs::read_to_string(path)?
        } else {
            String::default()
        };

        let note = self.create_draft_release(&issues, &note)?;
        println!("{}", note);

        Ok(())
    }
}

impl ReleaseArgs {
    async fn search_issues(&self) -> anyhow::Result<Vec<Issue>> {
        let labels = self.labels.clone().unwrap();
        let exclude_labels = self.exclude_labels.clone().unwrap();

        let milestones: Vec<Milestone> = github_client().get(
            format!("/repos/{}/{}/milestones", self.owner, self.repo),
            None::<&()>,
        ).await?;
        let milestone = if let Some(milestone) = milestones.iter().find(|m| m.title == self.milestone) {
            milestone
        } else {
            return Err(anyhow!("{} milestone not found", self.milestone));
        };

        log::info!(
            "Searching issues by labels or milestone {}, {:?}",
            milestone.title,
            labels
        );

        let mut issues: Vec<Issue> = vec![];
        let issue_handler = github_client().issues(&self.owner, &self.repo);

        for search_type in ["label", "milestone"] {
            let mut page: u32 = 1;

            loop {
                let builder = match search_type {
                    "label" => issue_handler.list().labels(&labels).state(State::All),
                    "milestone" => issue_handler.list().milestone(milestone.number as u64).state(State::All),
                    _ => return Err(anyhow!("invalid search type")),
                };

                let mut results = builder.page(page).send().await?.items;

                results.retain(|issue| {
                    !issue.labels.iter().any(|label| exclude_labels.contains(&label.name))
                });

                if results.is_empty() {
                    break;
                }

                issues.append(&mut results);
                page += 1;
            }
        }

        Ok(issues)
    }

    fn create_draft_release(&self, issues: &[Issue], note: &str) -> anyhow::Result<String> {
        log::info!("Creating a draft release for {}", self.tag);

        let mut note = note.to_string();
        let mut sections: indexmap::IndexMap<String, Vec<&Issue>> = indexmap! {};
        let mut contributors = indexset! {};
        let note_section_labels = self.note_section_labels.clone().unwrap_or_default();

        for contributor in self.note_extra_contributors.clone().unwrap_or_default() {
            contributors.insert(contributor);
        }

        let get_section_key = |label: &str| {
            if label.contains('/') {
                label[label.rfind('/').unwrap_or_default() + 1..].to_owned()
            } else {
                label.to_owned()
            }
        };

        for label in &note_section_labels {
            sections.insert(get_section_key(label), vec![]);
        }
        sections.insert("misc".to_owned(), vec![]);

        for issue in issues {
            for assignee in &issue.assignees {
                contributors.insert(assignee.login.clone());
            }

            let mut section_key = "misc".to_string();
            for label in &note_section_labels {
                if issue.labels.iter().any(|it| it.name == *label) {
                    section_key = get_section_key(label);
                    break;
                };
            }

            sections.get_mut(&section_key).unwrap().push(issue);
        }

        for (title, issues) in &sections {
            if issues.is_empty() {
                continue;
            }

            note += &format!("\n### {}\n", title.to_case(Case::Title));

            for issue in issues {
                let contributors: Vec<String> = issue.assignees.iter().map(|it| it.login.clone()).collect();
                note += &format!(
                    "- {} [{}]({}) - {}\n",
                    issue.title,
                    issue.number,
                    issue.url,
                    contributors.iter().map(|it| format!("@{it}")).collect::<Vec<String>>().join(" ")
                );
            }
        }

        note += "\n## Contributors\n";
        contributors.sort();
        for c in &contributors {
            note += &format!(
                "- @{} \n",
                c
            );
        }

        if self.publish {
            let repo_dir_path = working_dir_path().join(&self.repo);
            let release_title = self.note_title.clone().unwrap_or(format!(
                "{} {} release",
                self.repo.to_case(Case::Title),
                self.tag
            ));

            cmd!(
                "gh",
                &repo_dir_path,
                [
                    "release",
                    "create",
                    &self.tag,
                    "-n",
                    &note,
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
