use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use clap::Args;
use lazy_static::lazy_static;
use maplit::hashmap;
use regex::Regex;
use tracing_log::log;

use crate::{Cli, cmd, cmd_ignore_err, common};
use crate::args::CliCommand;

#[derive(Args)]
pub struct PrArgs {
    #[arg(short, long)]
    branch: String,

    #[arg(short, long)]
    tag: String,

    #[arg(short, long, default_value = "longhorn", hide = true)]
    group: Option<String>,

    #[arg(short, long, default_value = "longhorn", hide = true)]
    repo: Option<String>,

    #[arg(short, long)]
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
    hide = true)]
    components: Option<Vec<String>>,
}

lazy_static! {
    static ref VERSION_FILES: HashMap<&'static str, Vec<&'static str>> = hashmap! {
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

impl CliCommand for PrArgs {
    fn run(&self, cli: &Cli) -> anyhow::Result<()> {
        let group = self.group.as_ref().expect("required");
        let repo = self.repo.as_ref().expect("required");
        let components = self.components.as_ref().expect("required");

        let repo_path = format!("{}/{}", group, repo);
        let rel_dir_path = common::get_release_dir_path(cli)?;
        let repo_dir_path = rel_dir_path.join(repo);

        common::clone_repo(&repo_path, &self.branch, &repo_dir_path, &rel_dir_path)?;

        update_manifests(&repo_dir_path, &self.tag, components)?;
        update_deploy_manifest(&repo_dir_path)?;

        let commit_msg = self.message.clone().unwrap_or(format!("release: {}", self.tag));
        let fork_branch = format!("pr-{}", self.tag);

        log::info!("Creating PRs for {}", self.tag);

        cmd_ignore_err!("git", &repo_dir_path, ["branch", "-D", &fork_branch]);
        cmd!("git", &repo_dir_path, ["checkout", "-b", &fork_branch]);
        cmd!("git", &repo_dir_path, ["commit", "-am", &commit_msg, "-s"]);
        cmd!("git", &repo_dir_path, ["push", "-u", "--force", "origin", &fork_branch]);
        cmd!("gh", &repo_dir_path, ["pr", "create", "-B", &self.branch, "-f", "-t", &commit_msg]);

        Ok(())
    }
}

fn update_deploy_manifest(repo_dir_path: &PathBuf) -> anyhow::Result<()> {
    cmd!(
        "scripts/generate-longhorn-yaml.sh",
        &repo_dir_path,
        [] as [&str; 0]
    );

    Ok(())
}

fn update_manifests(repo_dir_path: &Path, version: &str, components: &[String]) -> anyhow::Result<()> {
    log::info!("Updating manifests");

    for (f, reg_pats) in VERSION_FILES.iter() {
        let mut new_version = version.clone();

        // A workaround for chart.yaml
        if *f == "chart/Chart.yaml" {
            new_version = version.trim_matches('v');
        }

        let f = repo_dir_path.join(f);
        let mut str = fs::read_to_string(&f)?;

        log::info!("Updating {:?}", &f);

        replace_str_with_version_by_reg(
            &mut str,
            reg_pats,
            new_version,
        )?;
        fs::write(&f, str)?;
    }

    {
        let f = repo_dir_path.join("deploy").join("longhorn-images.txt");
        let mut str = fs::read_to_string(&f)?;

        log::info!("Updating {:?}", &f);

        let mut reg_pats = vec![];
        for com in components {
            reg_pats.push(format!(r"(longhornio/{}:)(\S+)", com));
        }

        let reg_pats: Vec<&str> = reg_pats.iter().map(|it| it.as_str()).collect();
        replace_str_with_version_by_reg(
            &mut str,
            &reg_pats,
            version,
        )?;
        fs::write(&f, str)?;
    }

    Ok(())
}

fn replace_str_with_version_by_reg(str: &mut String, reg_pats: &[&str], version: &str) -> anyhow::Result<()> {
    for pat in reg_pats {
        let reg = Regex::new(pat)?;

        if reg.captures(str).is_some() {
            *str = reg.replace_all(str, format!("${{1}}{version}")).to_string();
        }
    }

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