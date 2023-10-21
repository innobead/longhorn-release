use crate::Cli;

pub mod pr;
pub mod release;
pub mod tag;

pub trait CliCommand {
    fn run(&self, cli: &Cli) -> anyhow::Result<()>;
}
