use async_trait::async_trait;

use crate::Cli;

pub mod pr;
pub mod release;
pub mod tag;
pub mod changelog;

#[async_trait]
pub trait CliCommand {
    async fn run(&self, cli: &Cli) -> anyhow::Result<()>;
}
