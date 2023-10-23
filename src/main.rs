use std::fs;

use anyhow::anyhow;
use clap::{Parser, Subcommand};
use octocrab::OctocrabBuilder;

use crate::args::changelog::ChangelogArgs;
use crate::args::CliCommand;
use crate::args::pr::PrArgs;
use crate::args::release::ReleaseArgs;
use crate::args::tag::TagArgs;
use crate::common::working_dir_path;
use crate::global::GITHUB_CLIENT;

mod args;
mod common;
mod global;
mod macros;

#[derive(Parser)]
#[command(author, version, about)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(global = true,
    short,
    long,
    default_value = "info",
    value_parser = ["error", "warn", "info", "debug", "trace"],
    help = "Log level")]
    log_level: String,

    #[arg(global = true, short, long, env, help = "Github Token")]
    github_token: Option<String>,
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
        return Err(anyhow!("Github Token is required"));
    }

    let octocrab = OctocrabBuilder::default().personal_token(cli.github_token.clone().unwrap()).build()?;
    if GITHUB_CLIENT.set(octocrab).is_err() {
        return Err(anyhow!("GitHub client has been initialized"));
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::try_parse()?;
    init(&cli)?;

    match &cli.command {
        Commands::Changelog(args) => args.run(&cli).await,
        Commands::Pr(args) => args.run(&cli).await,
        Commands::Release(args) => args.run(&cli).await,
        Commands::Tag(args) => args.run(&cli).await,
    }
}
