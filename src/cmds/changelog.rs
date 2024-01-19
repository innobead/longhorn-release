use async_trait::async_trait;
use chrono::{Duration, NaiveDateTime, Utc};
use clap::Args;
use indoc::formatdoc;
use octocrab::models::commits::Commit;
use tracing_log::log;

use crate::cmds::CliCommand;
use crate::git::{GitCli, GitOperationTrait};

use crate::github::github_client;
use crate::{cmd, Cli};

#[derive(Args)]
#[command(about = "Create a Changelog for repos between tags")]
pub struct ChangelogArgs {
    #[arg(long, help = "GitHub owner")]
    owner: String,

    #[arg(long, help = "GitHub repos")]
    repos: Vec<String>,

    #[arg(long, help = "Branch")]
    branch: String,

    #[arg(long, help = "Tag")]
    tag: String,

    #[arg(long, help = "Previous tag")]
    prev_tag: Option<String>,

    #[arg(long, help = "Find previous tag automatically")]
    find_prev_tag: bool,

    #[arg(long, default_value = "14", help = "Search changes since days")]
    since_days: i64,

    #[arg(
        long,
        help = "Create logs from the last public release, not pre release"
    )]
    public: bool,

    #[arg(long, help = "Fold changelog of each repo")]
    markdown_folding: bool,
}

#[async_trait]
impl CliCommand for ChangelogArgs {
    async fn run(&self, _cli: &Cli) -> anyhow::Result<()> {
        let mut changelog = String::new();
        let mut task_joiner = tokio::task::JoinSet::new();

        for repo in &self.repos {
            task_joiner.spawn(generate_repo_report(
                self.owner.clone(),
                repo.clone(),
                self.branch.clone(),
                self.tag.clone(),
                self.prev_tag.clone(),
                self.find_prev_tag,
                self.since_days,
                self.public,
                self.markdown_folding,
            ));
        }

        while let Some(res) = task_joiner.join_next().await {
            let str = res??;
            changelog += &str;
        }

        println!("{}", changelog);

        Ok(())
    }
}

async fn generate_repo_report(
    owner: String,
    repo: String,
    branch: String,
    tag: String,
    prev_tag: Option<String>,
    is_find_prev_tag: bool,
    since_days: i64,
    is_public: bool,
    is_markdown_folding: bool,
) -> anyhow::Result<String> {
    let git = GitCli::new(owner.clone(), repo.clone());
    git.clone_repo(&branch)?;

    let mut prev_tag = prev_tag.unwrap_or_default();
    if prev_tag.is_empty() {
        prev_tag = git.previous_tag(&tag, is_public)?;
    }

    let tag_hash = git.tag_hash(&tag)?;
    let prev_tag_hash = git.tag_hash(&prev_tag)?;

    let output = cmd!(
        "git",
        git.repo.repo_dir_path(),
        ["log", "-1", "--format=%at", &prev_tag]
    );
    let prev_tag_timestamp = String::from_utf8(output.stdout)?
        .trim()
        .parse::<i64>()
        .unwrap();
    let prev_tag_datetime =
        NaiveDateTime::from_timestamp_opt(prev_tag_timestamp, 0).map(|it| it.and_utc());

    let today = Utc::now();
    let since_date = if let Some(it) = prev_tag_datetime {
        it
    } else {
        today - Duration::days(since_days)
    }
    .format("%Y-%m-%dT%H:%M:%SZ")
    .to_string();

    let mut changelog = String::new();
    let mut tag_found = false;
    let mut page = 1;

    'outer: loop {
        let result: Result<Vec<Commit>, octocrab::Error> = github_client()
            .get(
                format!("/repos/{}/commits", git.repo.repo_ref()),
                Some(&[
                    ("sha", &branch),
                    ("page", &page.to_string()),
                    ("since", &since_date),
                ]),
            )
            .await;
        page += 1;

        match result {
            Ok(commits) => {
                if commits.is_empty() {
                    break;
                }

                for commit in &commits {
                    if !tag_found {
                        if commit.sha.starts_with(&tag_hash) {
                            tag_found = true;
                        } else {
                            continue;
                        }
                    }

                    if !is_find_prev_tag || tag_found {
                        if tag_found {
                            if commit.sha.starts_with(&prev_tag_hash) {
                                break 'outer;
                            }
                        }

                        changelog += &formatdoc! {"
                                    - {} [{}]({}) {}
                                    ",
                            commit.commit.message.lines().next().unwrap(),
                            &commit.sha[0..8],
                            commit.html_url,
                            commit.author.as_ref().map(|it| String::from("by @") + it.login.as_str()).unwrap_or(String::from("")),
                        };
                    }
                }
            }
            Err(err) => {
                log::debug!("Failed to get commits {:?}", err);
                break;
            }
        }
    }

    changelog = if is_markdown_folding {
        formatdoc! {"
            <details>
            <summary>{repo}</summary>

            {changelog}
            </details>
            ",
            repo=git.repo.repo_ref(),
            changelog=changelog,
        }
    } else {
        formatdoc! {"
            ### {repo}
            {changelog}
            ",
            repo=git.repo.repo_ref(),
            changelog=changelog,
        }
    };

    Ok(changelog)
}
