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
mod tap;

use clap::{CommandFactory, Parser, Subcommand};
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

        /// Only search formulae
        #[arg(long)]
        formula: bool,

        /// Only search casks
        #[arg(long)]
        cask: bool,
    },

    /// Show information about a formula or cask
    Info {
        /// Formula/cask name
        formula: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show dependencies for a formula
    Deps {
        /// Formula name
        formula: String,

        /// Show as tree
        #[arg(long)]
        tree: bool,

        /// Only show installed dependencies
        #[arg(long)]
        installed: bool,
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

    /// Uninstall formulae
    Uninstall {
        /// Formula names
        formulae: Vec<String>,

        /// Ignore dependencies (force uninstall)
        #[arg(long)]
        force: bool,
    },

    /// Remove unused dependencies
    Autoremove {
        /// Show what would be removed without actually removing
        #[arg(short = 'n', long)]
        dry_run: bool,
    },

    /// Add a tap (third-party repository)
    Tap {
        /// Tap name (user/repo format, or empty to list all taps)
        tap: Option<String>,
    },

    /// Remove a tap
    Untap {
        /// Tap name (user/repo format)
        tap: String,
    },

    /// Update Homebrew and all taps
    Update,

    /// Remove old versions of installed formulae
    Cleanup {
        /// Formula names (or all if empty)
        formulae: Vec<String>,

        /// Show what would be removed without actually removing
        #[arg(short = 'n', long)]
        dry_run: bool,
    },

    /// Manage download cache
    Cache {
        /// Clean cache (remove all downloaded bottles)
        #[arg(short, long)]
        clean: bool,
    },

    /// Show system configuration
    Config,

    /// Check system for potential problems
    Doctor,

    /// Open formula homepage in browser
    Home {
        /// Formula name
        formula: String,
    },

    /// List installed packages that are not dependencies of others
    Leaves,

    /// Pin formulae to prevent upgrades
    Pin {
        /// Formula names
        formulae: Vec<String>,
    },

    /// Unpin formulae to allow upgrades
    Unpin {
        /// Formula names
        formulae: Vec<String>,
    },

    /// Show formula description
    Desc {
        /// Formula names
        formulae: Vec<String>,
    },

    /// Link a formula
    Link {
        /// Formula names
        formulae: Vec<String>,
    },

    /// Unlink a formula
    Unlink {
        /// Formula names
        formulae: Vec<String>,
    },

    /// List all available commands
    #[allow(clippy::enum_variant_names)]
    Commands,

    /// Generate shell completion scripts
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
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
        Some(Commands::Search { query, formula, cask }) => {
            commands::search(&api, &query, formula, cask).await?;
        }
        Some(Commands::Info { formula, json }) => {
            commands::info(&api, &formula, json).await?;
        }
        Some(Commands::Deps { formula, tree, installed }) => {
            commands::deps(&api, &formula, tree, installed).await?;
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
        Some(Commands::Uninstall { formulae, force }) => {
            commands::uninstall(&api, &formulae, force).await?;
        }
        Some(Commands::Autoremove { dry_run }) => {
            commands::autoremove(dry_run)?;
        }
        Some(Commands::Tap { tap }) => {
            commands::tap(tap.as_deref())?;
        }
        Some(Commands::Untap { tap }) => {
            commands::untap(&tap)?;
        }
        Some(Commands::Update) => {
            commands::update()?;
        }
        Some(Commands::Cleanup { formulae, dry_run }) => {
            commands::cleanup(&formulae, dry_run)?;
        }
        Some(Commands::Cache { clean }) => {
            commands::cache(clean)?;
        }
        Some(Commands::Config) => {
            commands::config()?;
        }
        Some(Commands::Doctor) => {
            commands::doctor()?;
        }
        Some(Commands::Home { formula }) => {
            commands::home(&api, &formula).await?;
        }
        Some(Commands::Leaves) => {
            commands::leaves()?;
        }
        Some(Commands::Pin { formulae }) => {
            commands::pin(&formulae)?;
        }
        Some(Commands::Unpin { formulae }) => {
            commands::unpin(&formulae)?;
        }
        Some(Commands::Desc { formulae }) => {
            commands::desc(&api, &formulae).await?;
        }
        Some(Commands::Link { formulae }) => {
            commands::link(&formulae)?;
        }
        Some(Commands::Unlink { formulae }) => {
            commands::unlink(&formulae)?;
        }
        Some(Commands::Commands) => {
            commands::commands()?;
        }
        Some(Commands::Completions { shell }) => {
            let mut cmd = Cli::command();
            clap_complete::generate(
                shell,
                &mut cmd,
                "bru",
                &mut std::io::stdout(),
            );
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
