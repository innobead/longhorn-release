use clap::Args;

use crate::args::CliCommand;
use crate::Cli;

#[derive(Args)]
pub struct ReleaseArgs {}

impl CliCommand for ReleaseArgs {
    fn run(&self, _cli: &Cli) -> anyhow::Result<()> {
        todo!()
    }
}
