mod api;
mod commands;
mod error;

use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;

#[derive(Parser)]
#[command(name = "bru")]
#[command(author, version, about = "A fast Homebrew-compatible package manager", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Search for formulae and casks
    Search {
        /// Query string
        query: String,
    },

    /// Show information about a formula or cask
    Info {
        /// Formula/cask name
        formula: String,
    },

    /// Show dependencies for a formula
    Deps {
        /// Formula name
        formula: String,

        /// Show as tree
        #[arg(long)]
        tree: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    if std::env::var("RUST_LOG").is_err() {
        unsafe {
            std::env::set_var("RUST_LOG", "warn");
        }
    }
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"))
        )
        .init();

    let cli = Cli::parse();

    // Create API client
    let api = api::BrewApi::new()?;

    match cli.command {
        Some(Commands::Search { query }) => {
            commands::search(&api, &query).await?;
        }
        Some(Commands::Info { formula }) => {
            commands::info(&api, &formula).await?;
        }
        Some(Commands::Deps { formula, tree }) => {
            commands::deps(&api, &formula, tree).await?;
        }
        None => {
            println!("{} Welcome to bru - a fast Homebrew-compatible package manager!", "👋".bold());
            println!("\nRun {} to see available commands.", "bru --help".cyan());
            println!("\n{} Built with Rust for maximum performance", "⚡".bold());
        }
    }

    Ok(())
}
