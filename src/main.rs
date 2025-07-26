use anyhow::Result;
use clap::Parser;

mod cli;
mod daemon;
mod scheduler;
mod storage;

use cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        cli::Commands::Add {
            slug,
            cron,
            command,
        } => {
            cli::handle_add(slug, cron, command).await?;
        }
        cli::Commands::Remove { slug } => {
            cli::handle_remove(slug).await?;
        }
        cli::Commands::List => {
            cli::handle_list().await?;
        }
        cli::Commands::Start { slugs, all } => {
            cli::handle_start(slugs, all).await?;
        }
        cli::Commands::Stop { slugs, all } => {
            cli::handle_stop(slugs, all).await?;
        }
    }

    Ok(())
}
