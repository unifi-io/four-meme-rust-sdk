use clap::{Parser, Subcommand};
use eyre::Result;

mod create_token;
mod export_private_key;

use create_token::CreateTokenArgs;
use export_private_key::ExportPrivateKeyArgs;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create Token
    CreateToken(CreateTokenArgs),
    ExportPrivateKey(ExportPrivateKeyArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::CreateToken(args) => {
            args.execute().await?;
        }
        Commands::ExportPrivateKey(args) => {
            args.execute().await?;
        }
    }

    Ok(())
}
