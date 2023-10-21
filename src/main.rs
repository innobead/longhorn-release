use std::fs;

use clap::{Parser, Subcommand};

use crate::args::pr::PrArgs;
use crate::args::release::ReleaseArgs;
use crate::args::tag::TagArgs;
use crate::args::CliCommand;
use crate::common::RELEASE_DIR_PATH;

mod args;
mod common;
mod macros;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    init(&cli)?;

    match &cli.command {
        Commands::Tag(args) => args.run(&cli),
        Commands::Pr(args) => args.run(&cli),
        Commands::Release(args) => args.run(&cli),
    }
}

fn init(cli: &Cli) -> anyhow::Result<()> {
    fs::create_dir_all(RELEASE_DIR_PATH.as_path())?;

    common::enable_logging(&cli.log_level)?;
    common::check_runtime_dependencies()?;

    Ok(())
}

#[derive(Parser)]
#[command(author, version, about)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(global = true, short, long, default_value = "info", value_parser = ["error", "warn", "info", "debug", "trace"])]
    log_level: String,
}

#[derive(Subcommand)]
enum Commands {
    Tag(TagArgs),
    Pr(PrArgs),
    Release(ReleaseArgs),
}
