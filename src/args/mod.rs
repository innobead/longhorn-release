use crate::Cli;

pub mod tag;
pub mod pr;
pub mod release;

pub trait CliCommand {
    fn run(&self, cli: &Cli) -> anyhow::Result<()>;
}