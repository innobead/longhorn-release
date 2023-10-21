use clap::Args;

use crate::args::CliCommand;
use crate::Cli;

#[derive(Args)]
pub struct ReleaseArgs {}

impl CliCommand for ReleaseArgs {
    fn run(&self, _: &Cli) -> anyhow::Result<()> {
        //TODO
        // 1. Create a draft release with a release note
        // 2. Create a release note PR

        todo!()
    }
}
