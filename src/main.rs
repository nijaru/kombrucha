mod api;
mod cellar;
mod commands;
mod download;
mod error;
mod extract;
mod platform;
mod receipt;
mod relocate;
mod symlink;

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

    /// Show formulae that depend on a formula
    Uses {
        /// Formula name
        formula: String,
    },

    /// List installed packages
    List {
        /// Show all installed versions
        #[arg(long)]
        versions: bool,
    },

    /// Show outdated installed packages
    Outdated,

    /// Download bottles for formulae
    Fetch {
        /// Formula names
        formulae: Vec<String>,
    },

    /// Install formulae from bottles
    Install {
        /// Formula names
        formulae: Vec<String>,

        /// Skip installing dependencies
        #[arg(long)]
        only_dependencies: bool,
    },

    /// Upgrade installed formulae
    Upgrade {
        /// Formula names (or all if empty)
        formulae: Vec<String>,
    },

    /// Reinstall formulae
    Reinstall {
        /// Formula names
        formulae: Vec<String>,
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
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
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
        Some(Commands::Uses { formula }) => {
            commands::uses(&api, &formula).await?;
        }
        Some(Commands::List { versions }) => {
            commands::list(&api, versions).await?;
        }
        Some(Commands::Outdated) => {
            commands::outdated(&api).await?;
        }
        Some(Commands::Fetch { formulae }) => {
            commands::fetch(&api, &formulae).await?;
        }
        Some(Commands::Install {
            formulae,
            only_dependencies,
        }) => {
            commands::install(&api, &formulae, only_dependencies).await?;
        }
        Some(Commands::Upgrade { formulae }) => {
            commands::upgrade(&api, &formulae).await?;
        }
        Some(Commands::Reinstall { formulae }) => {
            commands::reinstall(&api, &formulae).await?;
        }
        None => {
            println!(
                "{} Welcome to bru - a fast Homebrew-compatible package manager!",
                "👋".bold()
            );
            println!("\nRun {} to see available commands.", "bru --help".cyan());
            println!("\n{} Built with Rust for maximum performance", "⚡".bold());
        }
    }

    Ok(())
}
