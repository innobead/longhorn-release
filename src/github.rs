use crate::git::GitRepo;
use crate::{cmd, cmd_ignore_err};
use anyhow::anyhow;
use async_trait::async_trait;
use octocrab::models::repos::Tag;
use octocrab::Octocrab;
use tracing_log::log;

pub fn github_client<'a>() -> &'a Octocrab {
    crate::global::GITHUB_CLIENT.get().unwrap()
}

#[async_trait]
pub trait GithubOperationTrait {
    fn create_pr(&self, msg: &str, tag: &str, branch: &str) -> anyhow::Result<String>;

    fn merge_pr(&self, id: &str) -> anyhow::Result<()>;

    async fn get_tag(&self, owner: &str, repo: &str, tag: &str) -> anyhow::Result<Tag>;
}

pub struct GithubCli {
    repo: GitRepo,
}

impl GithubCli {
    pub fn new(owner: String, repo: String) -> Self {
        Self {
            repo: GitRepo::new(owner, repo),
        }
    }
}

#[async_trait]
impl GithubOperationTrait for GithubCli {
    fn create_pr(&self, msg: &str, tag: &str, branch: &str) -> anyhow::Result<String> {
        log::info!("Creating PR for tag {}, branch {}", tag, branch);

        let repo_dir_path = self.repo.repo_dir_path();

        let msg = if msg.is_empty() {
            format!("release: {}", tag)
        } else {
            msg.to_string()
        };
        let fork_branch = format!("pr-{}", tag);

        cmd_ignore_err!("git", &repo_dir_path, ["branch", "-D", &fork_branch]);

        for args in [
            vec!["checkout", "-b", &fork_branch],
            vec!["commit", "-am", &msg, "-s"],
            vec!["push", "-u", "--force", "origin", &fork_branch],
        ] {
            cmd!("git", &repo_dir_path, &args);
        }

        let id = String::from_utf8(
            cmd!(
                "gh",
                &repo_dir_path,
                ["pr", "create", "--base", branch, "--fill", "--title", &msg]
            )
            .stdout,
        )?;

        Ok(id)
    }

    fn merge_pr(&self, id: &str) -> anyhow::Result<()> {
        cmd!(
            "gh",
            &self.repo.repo_dir_path(),
            ["pr", "merge", "--admin", "--rebase", "--delete-branch", id]
        );

        Ok(())
    }

    async fn get_tag(&self, owner: &str, repo: &str, tag: &str) -> anyhow::Result<Tag> {
        let tag: Tag = github_client()
            .get(
                format!("/repos/{}/{}/git/tags/{}", owner, repo, tag),
                None::<&()>,
            )
            .await?;

        Ok(tag)
    }
}
