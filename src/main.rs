mod api;
mod cache;
mod cask;
mod cellar;
mod commands;
mod download;
mod error;
mod extract;
mod platform;
mod receipt;
mod relocate;
mod services;
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

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// List casks instead of formulae
        #[arg(long)]
        cask: bool,
    },

    /// Show outdated installed packages
    Outdated {
        /// Check outdated casks instead of formulae
        #[arg(long)]
        cask: bool,
    },

    /// Download bottles for formulae
    Fetch {
        /// Formula names
        formulae: Vec<String>,
    },

    /// Install formulae from bottles
    Install {
        /// Formula/cask names
        formulae: Vec<String>,

        /// Skip installing dependencies
        #[arg(long)]
        only_dependencies: bool,

        /// Install cask instead of formula
        #[arg(long)]
        cask: bool,
    },

    /// Upgrade installed formulae
    Upgrade {
        /// Formula names (or all if empty)
        formulae: Vec<String>,

        /// Upgrade casks instead of formulae
        #[arg(long)]
        cask: bool,
    },

    /// Reinstall formulae
    Reinstall {
        /// Formula names
        formulae: Vec<String>,

        /// Reinstall casks instead of formulae
        #[arg(long)]
        cask: bool,
    },

    /// Uninstall formulae
    Uninstall {
        /// Formula/cask names
        formulae: Vec<String>,

        /// Ignore dependencies (force uninstall)
        #[arg(long)]
        force: bool,

        /// Uninstall cask instead of formula
        #[arg(long)]
        cask: bool,
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

    /// Show tap information
    TapInfo {
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

        /// Clean up casks instead of formulae
        #[arg(long)]
        cask: bool,
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

    /// Show Homebrew environment variables
    Env,

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

    /// Check for missing dependencies
    Missing {
        /// Formula names (or all if empty)
        formulae: Vec<String>,
    },

    /// Control analytics (on/off/state)
    Analytics {
        /// Action: on, off, or state
        action: Option<String>,
    },

    /// Print formula source code
    Cat {
        /// Formula names
        formulae: Vec<String>,
    },

    /// Print shell configuration
    Shellenv {
        /// Shell type (bash, zsh, fish)
        #[arg(long)]
        shell: Option<String>,
    },

    /// Create diagnostic gist for debugging
    GistLogs {
        /// Formula name (optional)
        formula: Option<String>,
    },

    /// Show formula aliases
    Alias {
        /// Formula name (optional)
        formula: Option<String>,
    },

    /// Show install logs
    Log {
        /// Formula name
        formula: String,
    },

    /// Find which formula provides a command
    WhichFormula {
        /// Command name
        command: String,
    },

    /// Show build options for a formula
    Options {
        /// Formula name
        formula: String,
    },

    /// Install or dump Brewfile dependencies
    Bundle {
        /// Generate Brewfile from installed packages
        #[arg(long)]
        dump: bool,

        /// Path to Brewfile (default: ./Brewfile)
        #[arg(long)]
        file: Option<String>,
    },

    /// Manage background services
    Services {
        /// Service action (list/start/stop/restart)
        action: Option<String>,

        /// Formula name (for start/stop/restart)
        formula: Option<String>,
    },

    /// Edit a formula in your editor
    Edit {
        /// Formula name
        formula: String,
    },

    /// Create a new formula from URL
    Create {
        /// URL to download source
        url: String,

        /// Formula name (optional, inferred from URL)
        #[arg(long)]
        name: Option<String>,
    },

    /// Check for newer versions of formulae
    Livecheck {
        /// Formula name
        formula: String,
    },

    /// Check formulae for issues
    Audit {
        /// Formula names
        formulae: Vec<String>,
    },

    /// Display Homebrew install path
    Prefix {
        /// Formula name (show formula prefix)
        formula: Option<String>,
    },

    /// Display Homebrew Cellar path
    Cellar {
        /// Formula name (show formula cellar)
        formula: Option<String>,
    },

    /// Display Homebrew repository path
    Repository {
        /// Tap name (optional)
        tap: Option<String>,
    },

    /// Display formula file path
    Formula {
        /// Formula name
        name: String,
    },

    /// Run post-install script
    Postinstall {
        /// Formula names
        formulae: Vec<String>,
    },

    /// List all available formulae
    Formulae,

    /// List all available casks
    Casks,

    /// List formulae that don't have bottles
    Unbottled {
        /// Formula names (or all if empty)
        formulae: Vec<String>,
    },

    /// Open Homebrew documentation
    Docs,

    /// Create a new tap
    TapNew {
        /// Tap name (user/repo format)
        tap: String,
    },

    /// Migrate formulae between taps
    Migrate {
        /// Formula name
        formula: String,

        /// New tap name
        #[arg(long)]
        tap: Option<String>,
    },

    /// Check library linkages for installed formulae
    Linkage {
        /// Formula names (or all if empty)
        formulae: Vec<String>,

        /// Show all files
        #[arg(long)]
        all: bool,
    },

    /// Read and validate all formulae in a tap
    Readall {
        /// Tap name (or homebrew/core if empty)
        tap: Option<String>,
    },

    /// Extract formula to a tap
    Extract {
        /// Formula name
        formula: String,

        /// Target tap name
        tap: String,
    },

    /// Unpack source code for a formula
    Unpack {
        /// Formula name
        formula: String,

        /// Destination directory
        #[arg(long)]
        destdir: Option<String>,
    },

    /// Print shell integration for command-not-found
    CommandNotFoundInit {
        /// Shell type
        shell: Option<String>,
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
        Some(Commands::List { versions, json, cask }) => {
            commands::list(&api, versions, json, cask).await?;
        }
        Some(Commands::Outdated { cask }) => {
            commands::outdated(&api, cask).await?;
        }
        Some(Commands::Fetch { formulae }) => {
            commands::fetch(&api, &formulae).await?;
        }
        Some(Commands::Install {
            formulae,
            only_dependencies,
            cask,
        }) => {
            if cask {
                commands::install_cask(&api, &formulae).await?;
            } else {
                commands::install(&api, &formulae, only_dependencies).await?;
            }
        }
        Some(Commands::Upgrade { formulae, cask }) => {
            commands::upgrade(&api, &formulae, cask).await?;
        }
        Some(Commands::Reinstall { formulae, cask }) => {
            commands::reinstall(&api, &formulae, cask).await?;
        }
        Some(Commands::Uninstall { formulae, force, cask }) => {
            if cask {
                commands::uninstall_cask(&formulae)?;
            } else {
                commands::uninstall(&api, &formulae, force).await?;
            }
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
        Some(Commands::TapInfo { tap }) => {
            commands::tap_info(&tap)?;
        }
        Some(Commands::Update) => {
            commands::update()?;
        }
        Some(Commands::Cleanup { formulae, dry_run, cask }) => {
            commands::cleanup(&formulae, dry_run, cask)?;
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
        Some(Commands::Env) => {
            commands::env()?;
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
        Some(Commands::Missing { formulae }) => {
            commands::missing(&formulae)?;
        }
        Some(Commands::Analytics { action }) => {
            commands::analytics(action.as_deref())?;
        }
        Some(Commands::Cat { formulae }) => {
            commands::cat(&api, &formulae).await?;
        }
        Some(Commands::Shellenv { shell }) => {
            commands::shellenv(shell.as_deref())?;
        }
        Some(Commands::GistLogs { formula }) => {
            commands::gist_logs(&api, formula.as_deref()).await?;
        }
        Some(Commands::Alias { formula }) => {
            commands::alias(&api, formula.as_deref()).await?;
        }
        Some(Commands::Log { formula }) => {
            commands::log(&formula)?;
        }
        Some(Commands::WhichFormula { command }) => {
            commands::which_formula(&command)?;
        }
        Some(Commands::Options { formula }) => {
            commands::options(&api, &formula).await?;
        }
        Some(Commands::Bundle { dump, file }) => {
            commands::bundle(&api, dump, file.as_deref()).await?;
        }
        Some(Commands::Services { action, formula }) => {
            commands::services(action.as_deref(), formula.as_deref())?;
        }
        Some(Commands::Edit { formula }) => {
            commands::edit(&api, &formula).await?;
        }
        Some(Commands::Create { url, name }) => {
            commands::create(&url, name.as_deref())?;
        }
        Some(Commands::Livecheck { formula }) => {
            commands::livecheck(&api, &formula).await?;
        }
        Some(Commands::Audit { formulae }) => {
            commands::audit(&api, &formulae).await?;
        }
        Some(Commands::Prefix { formula }) => {
            commands::prefix(formula.as_deref())?;
        }
        Some(Commands::Cellar { formula }) => {
            commands::cellar_cmd(formula.as_deref())?;
        }
        Some(Commands::Repository { tap }) => {
            commands::repository(tap.as_deref())?;
        }
        Some(Commands::Formula { name }) => {
            commands::formula(&name)?;
        }
        Some(Commands::Postinstall { formulae }) => {
            commands::postinstall(&formulae)?;
        }
        Some(Commands::Formulae) => {
            commands::formulae(&api).await?;
        }
        Some(Commands::Casks) => {
            commands::casks(&api).await?;
        }
        Some(Commands::Unbottled { formulae }) => {
            commands::unbottled(&api, &formulae).await?;
        }
        Some(Commands::Docs) => {
            commands::docs()?;
        }
        Some(Commands::TapNew { tap }) => {
            commands::tap_new(&tap)?;
        }
        Some(Commands::Migrate { formula, tap }) => {
            commands::migrate(&formula, tap.as_deref())?;
        }
        Some(Commands::Linkage { formulae, all }) => {
            commands::linkage(&formulae, all)?;
        }
        Some(Commands::Readall { tap }) => {
            commands::readall(tap.as_deref())?;
        }
        Some(Commands::Extract { formula, tap }) => {
            commands::extract(&formula, &tap)?;
        }
        Some(Commands::Unpack { formula, destdir }) => {
            commands::unpack(&api, &formula, destdir.as_deref()).await?;
        }
        Some(Commands::CommandNotFoundInit { shell }) => {
            commands::command_not_found_init(shell.as_deref())?;
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
