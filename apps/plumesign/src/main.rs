mod commands;

use clap::Parser;
use commands::{Commands, Cli};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Sign(args) => commands::sign::execute(args).await?,
        Commands::MachO(args) => commands::macho::execute(args).await?,
        Commands::Account(args) => commands::account::execute(args).await?,
        Commands::Device(args) => commands::device::execute(args).await?,
    }

    Ok(())
}
