use std::path::PathBuf;
use std::str::FromStr;

use anyhow::anyhow;
use is_executable::IsExecutable;
use std::process::Command;
use tracing_log::{log, LogTracer};

pub fn enable_logging(level: &str) -> anyhow::Result<()> {
    LogTracer::init()?;

    let level = tracing::Level::from_str(level)?;
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(level)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

pub fn check_runtime_dependencies() -> anyhow::Result<()> {
    let deps = ["git", "gh", "helm"];

    for dep in deps {
        which::which(dep).map_err(|err| anyhow!("{dep}, {}", err))?;
    }

    Ok(())
}

pub fn working_dir_path<'a>() -> &'a PathBuf {
    &crate::global::RELEASE_DIR_PATH
}

pub fn execute(script: Option<&String>, args: Option<&Vec<String>>) -> anyhow::Result<()> {
    match script
        .as_ref()
        .map(|str| PathBuf::from(&str))
        .iter()
        .find(|p| p.is_executable())
    {
        None => Ok(()),
        Some(script) => {
            log::info!("Running {:?}", script);

            let mut new_args = vec![script.to_str().unwrap()];
            if let Some(args) = args {
                let mut args = args.iter().map(|it| it.as_str()).collect();
                new_args.append(&mut args);
            }

            let script = script.canonicalize()?;
            let status = Command::new("bash").args(new_args).spawn()?.wait()?;
            if !status.success() {
                Err(anyhow!("failed to run {:?}: {}", script, status))
            } else {
                Ok(())
            }
        }
    }
}
