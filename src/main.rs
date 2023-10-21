use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::args::CliCommand;
use crate::args::release::ReleaseArgs;
use crate::args::pr::PrArgs;
use crate::args::tag::TagArgs;

mod args;
mod macros;
mod common;

#[derive(Parser)]
#[command(author, version, about)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(global = true, short, long)]
    working_dir: Option<PathBuf>,

    #[arg(global = true, short, long, default_value = "info", value_parser = ["error", "warn", "info", "debug", "trace"])]
    log_level: String,
}

#[derive(Subcommand)]
enum Commands {
    Tag(TagArgs),
    Pr(PrArgs),
    Release(ReleaseArgs),
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    common::enable_logging(&cli.log_level)?;
    common::check_runtime_dependencies()?;

    match &cli.command {
        Commands::Tag(args) => args.run(&cli),
        Commands::Pr(args) => args.run(&cli),
        Commands::Release(args) => args.run(&cli),
    }
}
