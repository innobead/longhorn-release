use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use clap::Args;
use lazy_static::lazy_static;
use maplit::hashmap;
use regex::Regex;
use tracing_log::log;

use crate::cmds::CliCommand;
use crate::common::execute;
use crate::git::{GitCli, GitOperationTrait};
use crate::github::{GithubCli, GithubOperationTrait};
use crate::{cmd, Cli};

lazy_static! {
    static ref VERSION_MANIFEST_PATTERNS: HashMap<&'static str, Vec<&'static str>> = hashmap! {
        "chart/Chart.yaml" => vec![
            r"(version: )(\S+)",
            r"(appVersion: v?)(\S+)"
        ],
        "chart/questions.yaml" => vec![
            r"(variable: image\.longhorn\.(manager|engine|ui|instanceManager|shareManager|backingImageManager)\.tag[\s\S]*?default:\s+)(\S+)",
        ],
        "chart/values.yaml" => vec![
            r"(repository: longhornio\/(longhorn|backing)[\s\S]*?tag:\s+)(\S+)"
        ],
        "uninstall/uninstall.yaml" => vec![
            r"(image: longhornio/\S+:)(\S+)"
        ]
    };
}

//TODO Keep repo but rename it to repos. Remove the original chart_repo and repos to make the command general
#[derive(Args)]
#[command(about = "Create PRs for a release")]
pub struct PrArgs {
    #[arg(long, help = "GitHub owner")]
    owner: String,

    #[arg(long, help = "GitHub repo")]
    repo: String,

    #[arg(long, help = "Branch")]
    branch: String,

    #[arg(long, help = "Tag")]
    tag: String,

    #[arg(long, help = "Commit message")]
    message: Option<String>,

    #[arg(long, help = "Dry run")]
    dryrun: bool,

    #[arg(long, help = "Merge the created PRs")]
    merge: bool,

    #[arg(long, help = "Script to update files in repo for PR")]
    hook: Option<String>,

    #[arg(long, hide = true, help = "Longhorn chart repo")]
    longhorn_chart_repo: Option<String>,

    #[arg(
        long,
        hide = true,
        requires = "longhorn_chart_repo",
        help = "Longhorn repos used to update in the PR"
    )]
    longhorn_repos: Option<Vec<String>>,
}

#[async_trait]
impl CliCommand for PrArgs {
    async fn run(&self, _: &Cli) -> anyhow::Result<()> {
        let git = GitCli::new(self.owner.clone(), self.repo.clone());
        let repo_dir_path = git.repo.repo_dir_path();

        git.clone_repo(&self.branch)?;

        execute(
            self.hook.as_ref(),
            Some(&vec![git
                .repo
                .repo_dir_path()
                .to_string_lossy()
                .to_string()]),
        )?;

        if self.longhorn_repos.is_some() {
            let longhorn_repos = self.longhorn_repos.as_ref().unwrap();

            update_version_manifests(repo_dir_path, &self.tag, longhorn_repos)?;
        }

        if self.longhorn_chart_repo.is_some() {
            let longhorn_chart_repo = self.longhorn_chart_repo.as_ref().unwrap();
            let git = GitCli::new(self.owner.clone(), longhorn_chart_repo.clone());
            git.clone_repo(&self.branch)?;
            let chart_repo_dir_path = git.repo.repo_dir_path();

            update_deploy_manifest(repo_dir_path, chart_repo_dir_path)?;
        }

        let mut changed_repos = vec![];

        if self.hook.is_some() || self.longhorn_repos.is_some() {
            changed_repos.push((self.owner.clone(), self.repo.clone()));
        }
        if let Some(repo) = self.longhorn_chart_repo.as_ref() {
            changed_repos.push((self.owner.clone(), repo.clone()));
        }

        if changed_repos.is_empty() {
            log::info!("No repositories have changed, so no PRs are required");
            return Ok(());
        }

        //TODO if nothing changed, also there is no need to create a PR
        if !self.dryrun {
            // let mut task_joiner = tokio::task::JoinSet::new();
            for (owner, repo) in changed_repos {
                let message = self.message.clone().unwrap_or_default();
                let tag = self.tag.clone();
                let branch = self.branch.clone();
                let merge = self.merge;

                // task_joiner.spawn(async move {
                let gh_client = GithubCli::new(owner, repo);

                gh_client
                    .create_pr(&message, &tag, &branch)
                    .and_then(|id| match id {
                        id if id.is_empty() => Ok(()),
                        _ => {
                            if merge {
                                gh_client.merge_pr(id.trim())
                            } else {
                                Ok(())
                            }
                        }
                    })?;
                // });
            }

            // while let Some(res) = task_joiner.join_next().await {
            //     let _ = res?;
            // }
        }

        Ok(())
    }
}

fn update_version_manifests(
    repo_dir_path: &Path,
    version: &str,
    components: &[String],
) -> anyhow::Result<()> {
    log::info!("Updating manifests");

    for (f, reg_pats) in VERSION_MANIFEST_PATTERNS.iter() {
        let mut new_version = version;

        // A workaround for chart.yaml
        if *f == "chart/Chart.yaml" {
            new_version = version.trim_matches('v');
        }

        let f = repo_dir_path.join(f);
        let mut str = fs::read_to_string(&f)?;

        log::info!("Updating manifest {:?}", &f);

        replace_str_with_version_by_reg(&mut str, reg_pats, new_version)?;
        fs::write(&f, str)?;
    }

    let f = repo_dir_path.join("deploy").join("longhorn-images.txt");
    let mut str = fs::read_to_string(&f)?;

    log::info!("Updating {:?}", &f);

    for c in components {
        replace_str_with_version_by_reg(
            &mut str,
            &[&format!(r"(longhornio/{}:)(\S+)", c)],
            version,
        )?;
    }
    fs::write(&f, str)?;

    Ok(())
}

fn replace_str_with_version_by_reg(
    str: &mut String,
    pats: &[&str],
    version: &str,
) -> anyhow::Result<()> {
    for pat in pats {
        let reg = Regex::new(pat)?;

        if reg.captures(str).is_some() {
            *str = reg.replace_all(str, format!("${{1}}{version}")).to_string();
        }
    }

    Ok(())
}

fn update_deploy_manifest(
    repo_dir_path: &PathBuf,
    chart_repo_dir_path: &PathBuf,
) -> anyhow::Result<()> {
    log::info!("Updating deploy manifest in {:?}", repo_dir_path);
    cmd!(
        "scripts/generate-longhorn-yaml.sh",
        &repo_dir_path,
        [] as [&str; 0]
    );

    log::info!(
        "Updating chart {:?} from {:?}",
        chart_repo_dir_path,
        repo_dir_path
    );

    let chart_dir = chart_repo_dir_path.join("charts").join("longhorn");
    fs_extra::dir::remove(&chart_dir)?;
    fs::create_dir_all(&chart_dir)?;

    fs_extra::dir::copy(
        repo_dir_path.join("chart"),
        chart_dir,
        &fs_extra::dir::CopyOptions::new().content_only(true),
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_version_related_files() {
        let data = vec![
            hashmap! {
                "str" => "version: v1.1.1-rc1",
                "pattern" => r"(version:\s+)(\S+)",
                "version" => "v1.5.1-rc2",
                "expected" => "version: v1.5.1-rc2",
            },
            hashmap! {
                "str" => r"repository: longhornio/longhorn-engine
            tag: master-head",
                "pattern" => r"(repository:\s+longhornio[\s\S]*tag:\s+)(\S+)",
                "version" => "v1.5.1-rc2",
                "expected" => r"repository: longhornio/longhorn-engine
            tag: v1.5.1-rc2",
            },
        ];

        for d in data {
            let mut str = d["str"].to_string();
            let result = replace_str_with_version_by_reg(&mut str, &[d["pattern"]], d["version"]);

            assert!(result.is_ok());
            assert_eq!(str, d["expected"]);
        }
    }
}
