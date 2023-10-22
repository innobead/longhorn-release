use crate::Cli;
use async_trait::async_trait;

pub mod pr;
pub mod release;
pub mod tag;

#[async_trait]
pub trait CliCommand {
    async fn run(&self, cli: &Cli) -> anyhow::Result<()>;
}
