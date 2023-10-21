use std::{env, fs};

use anyhow::anyhow;
use clap::{Parser, Subcommand};

use octocrab::OctocrabBuilder;

use crate::cmds::changelog::ChangelogArgs;
use crate::cmds::pr::PrArgs;
use crate::cmds::release::ReleaseArgs;
use crate::cmds::tag::TagArgs;
use crate::cmds::CliCommand;
use crate::common::{execute, working_dir_path};
use crate::global::GITHUB_CLIENT;

mod cmds;
mod common;
mod git;
mod github;
mod global;
mod macros;

#[derive(Parser)]
#[command(author, version = env!("VERSION"), about)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(
        global = true,
        short,
        long,
        default_value = "error",
        value_parser = ["error", "warn", "info", "debug", "trace"],
        help = "Log level"
    )]
    log_level: String,

    #[arg(global = true, long, env, help = "GitHub Token")]
    github_token: Option<String>,

    #[arg(global = true, long, help = "Script to run before command")]
    pre_hook: Option<String>,

    #[arg(global = true, long, help = "Args for pre-hook")]
    pre_hook_args: Option<Vec<String>>,

    #[arg(global = true, long, help = "Script to run after command")]
    post_hook: Option<String>,

    #[arg(global = true, long, help = "Args for post-hook")]
    post_hook_args: Option<Vec<String>>,
}

#[derive(Subcommand)]
enum Commands {
    Changelog(ChangelogArgs),
    Pr(PrArgs),
    Release(ReleaseArgs),
    Tag(TagArgs),
}

fn init(cli: &Cli) -> anyhow::Result<()> {
    fs::create_dir_all(working_dir_path().as_path())?;

    common::enable_logging(&cli.log_level)?;
    common::check_runtime_dependencies()?;

    if cli.github_token.is_none() {
        return Err(anyhow!("GitHub Token is required"));
    }

    let octocrab = OctocrabBuilder::default()
        .personal_token(cli.github_token.clone().unwrap())
        .build()?;
    if GITHUB_CLIENT.set(octocrab).is_err() {
        return Err(anyhow!("GitHub client has been initialized"));
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::try_parse()?;
    init(&cli)?;

    execute(cli.pre_hook.as_ref(), cli.pre_hook_args.as_ref())?;

    match &cli.command {
        Commands::Changelog(args) => args.run(&cli).await,
        Commands::Pr(args) => args.run(&cli).await,
        Commands::Release(args) => args.run(&cli).await,
        Commands::Tag(args) => args.run(&cli).await,
    }?;

    execute(cli.post_hook.as_ref(), cli.post_hook_args.as_ref())?;

    Ok(())
}
