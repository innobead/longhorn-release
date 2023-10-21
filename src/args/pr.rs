use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use clap::Args;
use lazy_static::lazy_static;
use maplit::hashmap;
use regex::Regex;
use tracing_log::log;

use crate::args::CliCommand;
use crate::common::RELEASE_DIR_PATH;
use crate::{cmd, cmd_ignore_err, common, Cli};

lazy_static! {
    static ref MANIFEST_VERSION_REG_PATTERNS: HashMap<&'static str, Vec<&'static str>> = hashmap! {
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

impl CliCommand for PrArgs {
    fn run(&self, _: &Cli) -> anyhow::Result<()> {
        let group = self.group.as_ref().expect("required");
        let repo = self.repo.as_ref().expect("required");
        let components = self.components.as_ref().expect("required");

        let repo_path = format!("{}/{}", group, repo);
        let repo_dir_path = RELEASE_DIR_PATH.join(repo);

        common::clone_repo(&repo_path, &self.branch, &repo_dir_path, &RELEASE_DIR_PATH)?;

        update_manifests(&repo_dir_path, &self.tag, components)?;
        update_deploy_manifest(&repo_dir_path)?;
        create_prs(
            &repo_dir_path,
            &self.message.clone().unwrap_or_default(),
            &self.tag,
            &self.branch,
        )?;

        Ok(())
    }
}

fn update_manifests(
    repo_dir_path: &Path,
    version: &str,
    components: &[String],
) -> anyhow::Result<()> {
    log::info!("Updating manifests");

    for (f, reg_pats) in MANIFEST_VERSION_REG_PATTERNS.iter() {
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

fn update_deploy_manifest(repo_dir_path: &PathBuf) -> anyhow::Result<()> {
    cmd!(
        "scripts/generate-longhorn-yaml.sh",
        &repo_dir_path,
        [] as [&str; 0]
    );

    Ok(())
}

fn create_prs(repo_dir_path: &PathBuf, msg: &str, tag: &str, branch: &str) -> anyhow::Result<()> {
    let msg = if msg.is_empty() {
        format!("release: {}", tag)
    } else {
        msg.to_string()
    };
    let fork_branch = format!("pr-{}", tag);

    log::info!("Creating PRs for {}", tag);

    cmd_ignore_err!("git", &repo_dir_path, ["branch", "-D", &fork_branch]);

    for args in [
        vec!["checkout", "-b", &fork_branch],
        vec!["commit", "-am", &msg, "-s"],
        vec!["push", "-u", "--force", "origin", &fork_branch],
        vec!["pr", "create", "-B", branch, "-f", "-t", &msg],
    ] {
        cmd!("git", &repo_dir_path, args);
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
