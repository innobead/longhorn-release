use async_trait::async_trait;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use clap::Args;
use lazy_static::lazy_static;
use maplit::hashmap;
use regex::Regex;
use tracing_log::log;

use crate::args::CliCommand;
use crate::common::working_dir_path;
use crate::{cmd, cmd_ignore_err, common, Cli};

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
            r"(image: longhornio/\S+:v?)(\S+)"
        ]
    };
}

#[derive(Args)]
#[command(about = "Create PRs for a release")]
pub struct PrArgs {
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

    #[arg(
        short,
        long,
        default_value = "longhorn",
        hide = true,
        help = "GitHub Repo for release"
    )]
    repo: String,

    #[arg(
        short,
        long,
        default_value = "charts",
        hide = true,
        help = "Github Repo for helm chart"
    )]
    chart_repo: String,

    #[arg(short, long, help = "Git commit message")]
    message: Option<String>,

    #[arg(long,
    default_values = [
    "longhorn-ui",
    "longhorn-manager",
    "longhorn-engine",
    "longhorn-instance-manager",
    "longhorn-share-manager",
    "backing-image-manager",
    ],
    hide = true,
    help = "Components to update")]
    components: Vec<String>,
}

#[async_trait]
impl CliCommand for PrArgs {
    async fn run(&self, _: &Cli) -> anyhow::Result<()> {
        let repo_path = format!("{}/{}", self.owner, self.repo);
        let repo_dir_path = working_dir_path().join(&self.repo);
        let chart_repo_dir_path = working_dir_path().join(&self.chart_repo);

        common::clone_repo(&repo_path, &self.branch, &repo_dir_path, working_dir_path())?;
        common::clone_repo(
            &repo_path,
            &self.branch,
            &chart_repo_dir_path,
            working_dir_path(),
        )?;

        update_version_manifests(&repo_dir_path, &self.tag, &self.components)?;
        update_deploy_manifest(&repo_dir_path, &chart_repo_dir_path)?;

        for p in [&repo_dir_path, &chart_repo_dir_path] {
            create_pr(
                p,
                &self.message.clone().unwrap_or_default(),
                &self.tag,
                &self.branch,
            )?;
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
        let mut new_version = version.clone();

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

    fs_extra::dir::copy(
        repo_dir_path.join("chart"),
        chart_dir,
        &fs_extra::dir::CopyOptions::new(),
    )?;

    Ok(())
}

fn create_pr(repo_dir_path: &PathBuf, msg: &str, tag: &str, branch: &str) -> anyhow::Result<()> {
    log::info!("Creating PR for tag {}, branch {}", tag, branch);

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
        cmd!("git", &repo_dir_path, args);
    }

    cmd!(
        "gh",
        &repo_dir_path,
        ["pr", "create", "-B", branch, "-f", "-t", &msg]
    );

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
