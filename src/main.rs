mod banner;
mod create;
mod github;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "motia-cli")]
#[command(version)]
#[command(about = "Motia project scaffolding CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Create {
        #[arg(help = "Project folder name")]
        name: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Create { name } => create::run(name).await?,
    }

    Ok(())
}
