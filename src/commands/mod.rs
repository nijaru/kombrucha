//! Command implementations for the bru CLI
//!
//! This module contains all command implementations organized by functional area:
//!
//! - **analytics**: Analytics and telemetry control
//! - **bundle**: Brewfile operations
//! - **cask**: Cask (GUI application) management
//! - **developer**: Homebrew-internal developer commands
//! - **development**: Formula development and testing
//! - **git**: Git-related operations
//! - **install**: Package installation, upgrade, and removal
//! - **linking**: Symlink management (link, unlink, pin, unpin)
//! - **list**: Package listing and status
//! - **maintenance**: System maintenance (cleanup, update, etc.)
//! - **paths**: Path and environment information
//! - **query**: Package search and information retrieval
//! - **services**: Background service management
//! - **tap**: Tap (repository) management
//! - **utilities**: Miscellaneous utility commands
//! - **utils**: Shared utility functions (internal)

// Module declarations
pub mod analytics;
pub mod bundle;
pub mod cask;
pub mod developer;
pub mod development;
pub mod git;
pub mod install;
pub mod linking;
pub mod list;
pub mod maintenance;
pub mod paths;
pub mod pkg;
pub mod query;
pub mod services;
pub mod tap;
pub mod utilities;
pub(crate) mod utils;

use clap::{CommandFactory, Parser, Subcommand};
use colored::Colorize;
use crate::error::Result;
use crate::api::BrewApi;

#[derive(Parser)]
#[command(name = "bru")]
#[command(author, version, about = "A fast Homebrew-compatible package manager", long_about = None)]
#[command(disable_help_subcommand = true)]
#[command(help_template = "\
Example usage:
  bru search TEXT
  bru info [FORMULA|CASK...]
  bru install FORMULA|CASK...
  bru update
  bru upgrade [FORMULA|CASK...]
  bru uninstall FORMULA|CASK...
  bru list [FORMULA|CASK...]

Troubleshooting:
  bru config
  bru doctor
  bru install --verbose FORMULA|CASK

Further help:
  bru commands
  bru help [COMMAND]
  bru --version
")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress non-essential output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Disable colored output (same as NO_COLOR=1)
    #[arg(long, global = true)]
    pub no_color: bool,
}

#[derive(Subcommand)]
pub enum Commands {
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

        /// Show only direct dependencies (default: show all transitive dependencies)
        #[arg(long)]
        direct: bool,
    },

    /// Show formulae that depend on a formula
    Uses {
        /// Formula name
        formula: String,

        /// Only show formulae that are currently installed
        #[arg(long)]
        installed: bool,
    },

    /// List installed packages
    List {
        /// Show all installed versions
        #[arg(long)]
        versions: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// List formulae only
        #[arg(long)]
        formula: bool,

        /// List casks only
        #[arg(long)]
        cask: bool,

        /// Show only names (one per line, no versions or headers)
        #[arg(short = '1', long)]
        quiet: bool,

        /// Display packages in columns (names only, no versions)
        #[arg(short = 'C', long)]
        columns: bool,
    },

    /// Show outdated installed packages
    Outdated {
        /// Check outdated casks instead of formulae
        #[arg(long)]
        cask: bool,

        /// Show only names (one per line, no versions or headers)
        #[arg(short, long)]
        quiet: bool,
    },

    /// Download bottles for formulae
    Fetch {
        /// Formula names
        formulae: Vec<String>,
    },

    /// Install formulae from bottles
    #[command(alias = "i")]
    Install {
        /// Formula/cask names
        formulae: Vec<String>,

        /// Install cask instead of formula
        #[arg(long)]
        cask: bool,

        /// Show what would be installed without actually installing
        #[arg(short = 'n', long)]
        dry_run: bool,

        /// Install even if already installed
        #[arg(short, long)]
        force: bool,
    },

    /// Upgrade installed formulae
    #[command(alias = "up")]
    Upgrade {
        /// Formula names (or all if empty)
        formulae: Vec<String>,

        /// Upgrade casks instead of formulae
        #[arg(long)]
        cask: bool,

        /// Show what would be upgraded without actually upgrading
        #[arg(short = 'n', long)]
        dry_run: bool,

        /// Upgrade even if already at latest version
        #[arg(short, long)]
        force: bool,
    },

    /// Reinstall formulae
    #[command(alias = "re")]
    Reinstall {
        /// Formula names
        formulae: Vec<String>,

        /// Reinstall casks instead of formulae
        #[arg(long)]
        cask: bool,
    },

    /// Uninstall formulae
    #[command(aliases = ["rm", "remove"])]
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

    /// Open Homebrew man page
    Man,

    /// Reset Homebrew/homebrew-core tap to latest
    #[command(hide = true)]
    UpdateReset {
        /// Tap name (or homebrew/core if empty)
        tap: Option<String>,
    },

    /// Check formula style with RuboCop
    Style {
        /// Formula names
        formulae: Vec<String>,

        /// Fix style violations automatically
        #[arg(long)]
        fix: bool,
    },

    /// Run formula test suite
    Test {
        /// Formula name
        formula: String,
    },

    /// Generate bottle from formula
    Bottle {
        /// Formula names
        formulae: Vec<String>,

        /// Write bottle block to formula file
        #[arg(long)]
        write: bool,
    },

    /// Pin a tap to prevent updates
    TapPin {
        /// Tap name (user/repo format)
        tap: String,
    },

    /// Unpin a tap to allow updates
    TapUnpin {
        /// Tap name (user/repo format)
        tap: String,
    },

    /// Install Homebrew's vendored gems
    VendorGems,

    /// Run Homebrew's Ruby instance
    Ruby {
        /// Ruby code or script file
        args: Vec<String>,
    },

    /// Enter Homebrew's interactive Ruby shell
    Irb,

    /// Profile a brew command
    Prof {
        /// Command to profile
        command: String,

        /// Arguments for the command
        args: Vec<String>,
    },

    /// Synchronize nodenv with Homebrew formulae
    NodenvSync,

    /// Synchronize pyenv with Homebrew formulae
    PyenvSync,

    /// Synchronize rbenv with Homebrew formulae
    RbenvSync,

    /// Update Python resources for a formula
    UpdatePythonResources {
        /// Formula name
        formula: String,

        /// Print resources but don't update formula
        #[arg(long)]
        print_only: bool,
    },

    /// Pull a pull request from GitHub
    PrPull {
        /// PR number or URL
        pr: String,
    },

    /// Upload bottles to GitHub
    PrUpload {
        /// Use Bintray instead of GitHub
        #[arg(long)]
        use_bintray: bool,
    },

    /// Run Homebrew's test-bot
    TestBot {
        /// Formula names
        formulae: Vec<String>,

        /// Skip cleanup after test
        #[arg(long)]
        skip_cleanup: bool,
    },

    /// Bump revision for a formula
    BumpRevision {
        /// Formula names
        formulae: Vec<String>,

        /// Message for the revision bump
        #[arg(short, long)]
        message: Option<String>,
    },

    /// Automatically merge pull requests
    PrAutomerge {
        /// Strategy for merging (merge, rebase, squash)
        #[arg(long)]
        strategy: Option<String>,
    },

    /// Show contributions to Homebrew
    Contributions {
        /// GitHub username
        user: Option<String>,

        /// Show contributions from this date
        #[arg(long)]
        from: Option<String>,
    },

    /// Update Homebrew license data
    UpdateLicenseData,

    /// Install Homebrew formula API data
    InstallFormulaApi,

    /// Setup developer environment
    Setup,

    /// Fix bottle tags in formula files
    FixBottleTags {
        /// Formula names
        formulae: Vec<String>,
    },

    /// Generate man page completions
    GenerateManCompletions,

    /// Merge bottle files
    BottleMerge {
        /// Bottle files to merge
        files: Vec<String>,
    },

    /// Install Homebrew's bundler
    InstallBundler,

    /// Bump version for a formula
    Bump {
        /// Formula name
        formula: String,

        /// Skip audit after bump
        #[arg(long)]
        no_audit: bool,
    },

    /// Sponsor Homebrew development
    Sponsor {
        /// Target to sponsor
        target: Option<String>,
    },

    /// Homebrew's internal setup-ruby command
    SetupRuby,

    /// Open formula tab in browser
    Tab {
        /// Formula name
        formula: String,
    },

    /// Homebrew's internal update-if-needed command
    #[command(hide = true)]
    UpdateIfNeeded,

    /// Show help for a command
    Help {
        /// Command name
        command: Option<String>,
    },

    /// Show tap README
    #[command(hide = true)]
    TapReadme {
        /// Tap name (user/repo format)
        tap: String,
    },

    /// Homebrew's internal analytics-state command
    #[command(hide = true)]
    AnalyticsState,

    /// Update Python resources for a formula
    #[command(hide = true)]
    UpdatePythonResourcesInternal {
        /// Formula name
        formula: String,

        /// Print resources but don't update formula
        #[arg(long)]
        print_only: bool,
    },

    /// Determine which test runners to use for a set of formulae
    #[command(hide = true)]
    DetermineTestRunners {
        /// Formula names
        formulae: Vec<String>,
    },

    /// Dispatch build bottle job
    #[command(hide = true)]
    DispatchBuildBottle {
        /// Formula name
        formula: String,

        /// Target platform
        #[arg(long)]
        platform: Option<String>,
    },

    /// Bump formula pull request
    #[command(hide = true)]
    BumpFormulaPr {
        /// Formula name
        formula: String,

        /// New version
        #[arg(long)]
        version: Option<String>,

        /// New source URL
        #[arg(long)]
        url: Option<String>,
    },

    /// Bump cask pull request
    #[command(hide = true)]
    BumpCaskPr {
        /// Cask name
        cask: String,

        /// New version
        #[arg(long)]
        version: Option<String>,
    },

    /// Generate Homebrew formula API data
    #[command(hide = true)]
    GenerateFormulaApi {
        /// Formula names
        formulae: Vec<String>,
    },

    /// Generate Homebrew cask API data
    #[command(hide = true)]
    GenerateCaskApi {
        /// Cask names
        casks: Vec<String>,
    },

    /// Homebrew's internal install-bundler-gems command
    #[command(hide = true)]
    InstallBundlerGems,

    /// Internal developer command
    #[command(hide = true)]
    Developer {
        /// Action to perform
        action: Option<String>,
    },

    /// Homebrew's internal typecheck command
    #[command(hide = true)]
    Typecheck {
        /// Files to typecheck
        files: Vec<String>,
    },

    /// Homebrew's internal update-report command
    #[command(hide = true)]
    UpdateReport,

    /// Homebrew's internal nodeenv-sync command
    #[command(hide = true)]
    NodeenvSync,

    /// Homebrew's internal formula-info command
    #[command(hide = true)]
    FormulaInfo {
        /// Formula name
        formula: String,
    },

    /// Run a command from a tap
    #[command(hide = true)]
    TapCmd {
        /// Tap name
        tap: String,

        /// Command name
        command: String,

        /// Arguments for the command
        args: Vec<String>,
    },

    /// Homebrew's internal command command
    #[command(hide = true)]
    Command {
        /// Subcommand name
        subcommand: String,

        /// Arguments for the subcommand
        args: Vec<String>,
    },

    /// Homebrew's internal unalias command
    #[command(hide = true)]
    Unalias {
        /// Alias name
        alias_name: String,
    },

    /// Find which formula provides a command
    #[command(hide = true)]
    WhichFormulaInternal {
        /// Command name
        command: String,
    },

    /// Show build options for a formula
    #[command(hide = true)]
    OptionsInternal {
        /// Formula name
        formula: String,
    },

    /// Show formula description
    #[command(hide = true)]
    DescInternal {
        /// Formula names
        formulae: Vec<String>,
    },

    /// Show contributions to Homebrew
    #[command(hide = true)]
    ContributionsInternal {
        /// GitHub username
        user: Option<String>,

        /// Show contributions from this date
        #[arg(long)]
        from: Option<String>,
    },

    /// Show formula aliases
    #[command(hide = true)]
    AliasInternal {
        /// Formula name (optional)
        formula: Option<String>,
    },

    /// Show outdated installed packages
    #[command(hide = true)]
    OutdatedInternal {
        /// Check outdated casks instead of formulae
        #[arg(long)]
        cask: bool,

        /// Show only names (one per line, no versions or headers)
        #[arg(short, long)]
        quiet: bool,
    },

    /// Show formulae that depend on a formula
    #[command(hide = true)]
    UsesInternal {
        /// Formula name
        formula: String,

        /// Only show formulae that are currently installed
        #[arg(long)]
        installed: bool,
    },

    /// Show dependencies for a formula
    #[command(hide = true)]
    DepsInternal {
        /// Formula name
        formula: String,

        /// Show as tree
        #[arg(long)]
        tree: bool,

        /// Only show installed dependencies
        #[arg(long)]
        installed: bool,

        /// Show only direct dependencies (default: show all transitive dependencies)
        #[arg(long)]
        direct: bool,
    },

    /// Show information about a formula or cask
    #[command(hide = true)]
    InfoInternal {
        /// Formula/cask name
        formula: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Search for formulae and casks
    #[command(hide = true)]
    SearchInternal {
        /// Query string
        query: String,

        /// Only search formulae
        #[arg(long)]
        formula: bool,

        /// Only search casks
        #[arg(long)]
        cask: bool,
    },

    /// Show formula tab in browser
    #[command(hide = true)]
    TabInternal {
        /// Formula name
        formula: String,
    },

    /// Show Homebrew formula API data
    #[command(hide = true)]
    UsesCask {
        /// Cask name
        cask: String,
    },

    /// Show information about a cask
    #[command(hide = true)]
    AbvCask {
        /// Cask name
        cask: String,
    },

    /// Test the embedded Ruby interpreter
    #[command(hide = true)]
    TestRuby,
}

fn error_exit(msg: &str, usage: &str) -> ! {
    eprintln!("{} {}", "Error:".red().bold(), msg);
    eprintln!("Usage: {}", usage);
    std::process::exit(1);
}

/// Execute the command specified in the CLI arguments
pub async fn run(cli: Cli, ruby_handle: &magnus::Ruby) -> Result<()> {
    // Create API client
    let api = BrewApi::new()?;

    match cli.command {
        Some(Commands::Search { query, formula, cask }) | Some(Commands::SearchInternal { query, formula, cask }) => {
            search(&api, &query, formula, cask).await?;
        }
        Some(Commands::Info { formula, json }) | Some(Commands::InfoInternal { formula, json }) => {
            info(&api, &formula, json).await?;
        }
        Some(Commands::Deps { formula, tree, installed, direct }) | Some(Commands::DepsInternal { formula, tree, installed, direct }) => {
            deps(&api, &formula, tree, installed, direct).await?;
        }
        Some(Commands::Uses { formula, installed }) | Some(Commands::UsesInternal { formula, installed }) => {
            uses(&api, &formula, installed).await?;
        }
        Some(Commands::List {
            versions,
            json,
            formula,
            cask,
            quiet,
            columns,
        }) => {
            list(&api, versions, json, formula, cask, quiet, columns).await?;
        }
        Some(Commands::Outdated { cask, quiet }) | Some(Commands::OutdatedInternal { cask, quiet }) => {
            outdated(&api, cask, quiet).await?;
        }
        Some(Commands::Fetch { formulae }) => {
            if formulae.is_empty() {
                error_exit("No formulae specified", "bru fetch [FORMULAE]...");
            }
            fetch(&api, &formulae).await?;
        }
        Some(Commands::Install {
            formulae,
            cask,
            dry_run,
            force,
        }) => {
            if formulae.is_empty() {
                error_exit(
                    "No formulae specified",
                    "bru install [OPTIONS] [FORMULAE]...",
                );
            }
            if cask {
                self::cask::install_cask(&api, &formulae).await?;
            } else {
                install(&api, ruby_handle, &formulae, dry_run, force).await?;
            }
        }
        Some(Commands::Upgrade {
            formulae,
            cask,
            dry_run,
            force,
        }) => {
            upgrade(&api, ruby_handle, &formulae, cask, dry_run, force).await?;
        }
        Some(Commands::Reinstall { formulae, cask }) => {
            if formulae.is_empty() {
                error_exit("No formulae specified", "bru reinstall [OPTIONS] [FORMULAE]...");
            }
            reinstall(&api, ruby_handle, &formulae, cask).await?;
        }
        Some(Commands::Uninstall {
            formulae,
            force,
            cask,
        }) => {
            if formulae.is_empty() {
                error_exit("No formulae specified", "bru uninstall [OPTIONS] [FORMULAE]...");
            }
            if cask {
                self::cask::uninstall_cask(&formulae)?;
            }
            else {
                uninstall(&api, ruby_handle, &formulae, force).await?;
            }
        }
        Some(Commands::Autoremove { dry_run }) => {
            maintenance::autoremove(dry_run)?;
        }
        Some(Commands::Tap { tap }) => {
            self::tap::tap(tap.as_deref())?;
        }
        Some(Commands::Untap { tap }) => {
            self::tap::untap(&tap)?;
        }
        Some(Commands::TapInfo { tap }) => {
            self::tap::tap_info(&tap)?;
        }
        Some(Commands::Update) => {
            maintenance::update()?;
        }
        Some(Commands::Cleanup {
            formulae,
            dry_run,
            cask,
        }) => {
            maintenance::cleanup(&formulae, dry_run, cask)?;
        }
        Some(Commands::Cache { clean }) => {
            maintenance::cache(clean)?;
        }
        Some(Commands::Config) => {
            config()?;
        }
        Some(Commands::Doctor) => {
            maintenance::doctor()?;
        }
        Some(Commands::Env) => {
            env()?;
        }
        Some(Commands::Home { formula }) => {
            self::query::home(&api, &formula).await?;
        }
        Some(Commands::Leaves) => {
            leaves()?;
        }
        Some(Commands::Pin { formulae }) => {
            if formulae.is_empty() {
                error_exit("No formulae specified", "bru pin [FORMULAE]...");
            }
            pin(&formulae)?;
        }
        Some(Commands::Unpin { formulae }) => {
            if formulae.is_empty() {
                error_exit("No formulae specified", "bru unpin [FORMULAE]...");
            }
            unpin(&formulae)?;
        }
        Some(Commands::Desc { formulae }) | Some(Commands::DescInternal { formulae }) => {
            if formulae.is_empty() {
                error_exit("No formulae specified", "bru desc [FORMULAE]...");
            }
            desc(&api, &formulae).await?;
        }
        Some(Commands::Link { formulae }) => {
            if formulae.is_empty() {
                error_exit("No formulae specified", "bru link [FORMULAE]...");
            }
            link(&api, &formulae).await?;
        }
        Some(Commands::Unlink { formulae }) => {
            if formulae.is_empty() {
                error_exit("No formulae specified", "bru unlink [FORMULAE]...");
            }
            unlink(&formulae)?;
        }
        Some(Commands::Commands) => {
            self::utilities::commands()?;
        }
        Some(Commands::Completions { shell }) => {
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, "bru", &mut std::io::stdout());
        }
        Some(Commands::Missing { formulae }) => {
            missing(&formulae)?;
        }
        Some(Commands::Analytics { action }) => {
            analytics(action.as_deref())?;
        }
        Some(Commands::Cat { formulae }) => {
            cat(&api, &formulae).await?;
        }
        Some(Commands::Shellenv { shell }) => {
            shellenv(shell.as_deref())?;
        }
        Some(Commands::GistLogs { formula }) => {
            gist_logs(&api, formula.as_deref()).await?;
        }
        Some(Commands::Alias { formula }) | Some(Commands::AliasInternal { formula }) => {
            alias(&api, formula.as_deref()).await?;
        }
        Some(Commands::Log { formula }) => {
            self::git::log(&formula)?;
        }
        Some(Commands::WhichFormula { command }) | Some(Commands::WhichFormulaInternal { command }) => {
            which_formula(&command)?;
        }
        Some(Commands::Options { formula }) | Some(Commands::OptionsInternal { formula }) => {
            options(&api, &formula).await?;
        }
        Some(Commands::Bundle { dump, file }) => {
            bundle(&api, ruby_handle, dump, file.as_deref()).await?;
        }
        Some(Commands::Services { action, formula }) => {
            services(action.as_deref(), formula.as_deref())?;
        }
        Some(Commands::Edit { formula }) => {
            edit(&api, &formula).await?;
        }
        Some(Commands::Create { url, name }) => {
            create(&url, name.as_deref())?;
        }
        Some(Commands::Livecheck { formula }) => {
            livecheck(&api, &formula).await?;
        }
        Some(Commands::Audit { formulae }) => {
            audit(&api, &formulae).await?;
        }
        Some(Commands::Prefix { formula }) => {
            prefix(formula.as_deref())?;
        }
        Some(Commands::Cellar { formula }) => {
            cellar_cmd(formula.as_deref())?;
        }
        Some(Commands::Repository { tap }) => {
            repository(tap.as_deref())?;
        }
        Some(Commands::Formula { name }) => {
            formula(&name)?;
        }
        Some(Commands::Postinstall { formulae }) => {
            postinstall(&formulae)?;
        }
        Some(Commands::Formulae) => {
            self::query::formulae(&api).await?;
        }
        Some(Commands::Casks) => {
            self::query::casks(&api).await?;
        }
        Some(Commands::Unbottled { formulae }) => {
            unbottled(&api, &formulae).await?;
        }
        Some(Commands::Docs) => {
            docs()?;
        }
        Some(Commands::TapNew { tap }) => {
            self::tap::tap_new(&tap)?;
        }
        Some(Commands::Migrate { formula, tap }) => {
            migrate(&formula, tap.as_deref())?;
        }
        Some(Commands::Linkage { formulae, all }) => {
            linkage(&formulae, all)?;
        }
        Some(Commands::Readall { tap }) => {
            readall(tap.as_deref())?;
        }
        Some(Commands::Extract { formula, tap }) => {
            development::extract(&formula, &tap)?;
        }
        Some(Commands::Unpack { formula, destdir }) => {
            unpack(&api, &formula, destdir.as_deref()).await?;
        }
        Some(Commands::CommandNotFoundInit { shell }) => {
            command_not_found_init(shell.as_deref())?;
        }
        Some(Commands::Man) => {
            man()?;
        }
        Some(Commands::UpdateReset { tap }) => {
            maintenance::update_reset(tap.as_deref())?;
        }
        Some(Commands::Style { formulae, fix }) => {
            style(&formulae, fix)?;
        }
        Some(Commands::Test { formula }) => {
            test(&formula)?;
        }
        Some(Commands::Bottle { formulae, write }) => {
            bottle(&formulae, write)?;
        }
        Some(Commands::TapPin { tap }) => {
            self::tap::tap_pin(&tap)?;
        }
        Some(Commands::TapUnpin { tap }) => {
            self::tap::tap_unpin(&tap)?;
        }
        Some(Commands::VendorGems) => {
            vendor_gems()?;
        }
        Some(Commands::Ruby { args }) => {
            ruby(&args)?;
        }
        Some(Commands::Irb) => {
            irb()?;
        }
        Some(Commands::Prof { command: _, args }) => {
            prof(&args)?;
        }
        Some(Commands::TapReadme { tap }) => {
            self::tap::tap_readme(&tap)?;
        }
        Some(Commands::InstallBundlerGems) => {
            install_bundler_gems()?;
        }
        Some(Commands::Developer { action }) => {
            developer(action.as_deref())?;
        }
        Some(Commands::Typecheck { files }) => {
            typecheck(&files)?;
        }
        Some(Commands::UpdateReport) => {
            maintenance::update_report()?;
        }
        Some(Commands::UpdatePythonResources { formula, print_only }) | Some(Commands::UpdatePythonResourcesInternal { formula, print_only }) => {
            update_python_resources(&formula, print_only)?;
        }
        Some(Commands::DetermineTestRunners { formulae }) => {
            determine_test_runners(&formulae)?;
        }
        Some(Commands::DispatchBuildBottle { formula, platform }) => {
            dispatch_build_bottle(&formula, platform.as_deref())?;
        }
        Some(Commands::BumpFormulaPr {
            formula,
            version,
            url,
        }) => {
            bump_formula_pr(&formula, version.as_deref(), url.as_deref())?;
        }
        Some(Commands::BumpCaskPr { cask, version }) => {
            bump_cask_pr(&cask, version.as_deref())?;
        }
        Some(Commands::GenerateFormulaApi { formulae }) => {
            generate_formula_api(&formulae).await?;
        }
        Some(Commands::GenerateCaskApi { casks }) => {
            generate_cask_api(&casks).await?;
        }
        Some(Commands::PrPull { pr }) => {
            pr_pull(&pr)?;
        }
        Some(Commands::PrUpload { use_bintray }) => {
            pr_upload(use_bintray)?;
        }
        Some(Commands::TestBot {
            formulae,
            skip_cleanup,
        }) => {
            test_bot(&formulae, skip_cleanup)?;
        }
        Some(Commands::BumpRevision { formulae, message }) => {
            bump_revision(&formulae, message.as_deref())?;
        }
        Some(Commands::PrAutomerge { strategy }) => {
            pr_automerge(strategy.as_deref())?;
        }
        Some(Commands::Contributions { user, from }) | Some(Commands::ContributionsInternal { user, from }) => {
            contributions(user.as_deref(), from.as_deref())?;
        }
        Some(Commands::UpdateLicenseData) => {
            update_license_data()?;
        }
        Some(Commands::FormulaInfo { formula }) => {
            formula_info(&api, &formula).await?;
        }
        Some(Commands::TapCmd { tap, command, args }) => {
            self::tap::tap_cmd(&tap, &command, &args)?;
        }
        Some(Commands::InstallFormulaApi) => {
            install_formula_api()?;
        }
        Some(Commands::UsesCask { cask }) => {
            self::cask::uses_cask(&cask)?;
        }
        Some(Commands::AbvCask { cask }) => {
            self::cask::abv_cask(&api, &cask).await?;
        }
        Some(Commands::TestRuby) => {
            self::pkg::ruby::test_ruby(ruby_handle)?;
        }
        Some(Commands::Setup) => {
            setup()?;
        }
        Some(Commands::FixBottleTags { formulae }) => {
            fix_bottle_tags(&formulae)?;
        }
        Some(Commands::GenerateManCompletions) => {
            generate_man_completions()?;
        }
        Some(Commands::BottleMerge { files }) => {
            bottle_merge(&files)?;
        }
        Some(Commands::InstallBundler) => {
            install_bundler()?;
        }
        Some(Commands::Bump { formula, no_audit }) => {
            bump(&formula, no_audit)?;
        }
        Some(Commands::AnalyticsState) => {
            analytics_state()?;
        }
        Some(Commands::Sponsor { target }) => {
            sponsor(target.as_deref())?;
        }
        Some(Commands::Command { subcommand, args }) => {
            command(&subcommand, &args)?;
        }
        Some(Commands::NodenvSync) | Some(Commands::NodeenvSync) => {
            nodenv_sync()?;
        }
        Some(Commands::PyenvSync) => {
            pyenv_sync()?;
        }
        Some(Commands::RbenvSync) => {
            rbenv_sync()?;
        }
        Some(Commands::SetupRuby) => {
            setup_ruby()?;
        }
        Some(Commands::Tab { formula }) | Some(Commands::TabInternal { formula }) => {
            tab(&api, &formula).await?;
        }
        Some(Commands::Unalias { alias_name }) => {
            unalias(&alias_name)?;
        }
        Some(Commands::Help { command }) => {
            match command {
                None => {
                    // Show general help
                    Cli::command().print_help()?;
                }
                Some(cmd) => {
                    // Try to show help for the specific command using bru (suppress errors)
                    let status = std::process::Command::new(std::env::current_exe()?)
                        .arg(&cmd)
                        .arg("--help")
                        .stderr(std::process::Stdio::null())
                        .status();

                    match status {
                        Ok(s) if s.success() => {
                            // Command help succeeded
                        }
                        _ => {
                            // Fall back to brew for unsupported commands
                            let fallback_status = std::process::Command::new("brew")
                                .arg("help")
                                .arg(&cmd)
                                .status();

                            if let Ok(s) = fallback_status {
                                if !s.success() {
                                    eprintln!("{} Unknown command: {}", "Error:".red().bold(), cmd);
                                    std::process::exit(1);
                                }
                            } else {
                                eprintln!(
                                    "{} Command '{}' not supported by bru or brew",
                                    "Error:".red().bold(),
                                    cmd
                                );
                                std::process::exit(1);
                            }
                        }
                    }
                }
            }
        }
        Some(Commands::UpdateIfNeeded) => {
            maintenance::update_if_needed()?;
        }
        None => {
            // Show help when no command is provided (brew-compatible behavior)
            Cli::command().print_help()?;
        }
    }

    Ok(())
}

// Re-export commonly used commands for convenience
// This allows using `commands::search()` instead of `commands::query::search()`

// Query commands
pub use query::formula_info;
pub use query::{cat, deps, desc, formula, info, options, search, unbottled, uses};

// Install commands
pub use install::{fetch, install, reinstall, uninstall, upgrade};

// Cask commands

// List commands
pub use list::{leaves, list, missing, outdated};

// Maintenance commands

// Tap commands

// Linking commands
pub use linking::{link, pin, postinstall, unlink, unpin};

// Paths commands
pub use paths::{cellar_cmd, config, env, prefix, repository, shellenv};

// Services
pub use services::services;

// Bundle
pub use bundle::bundle;

// Analytics
pub use analytics::{analytics, analytics_state};

// Git
pub use git::gist_logs;

// Utilities
pub use utilities::{
    alias, command, command_not_found_init, docs, man, unalias, which_formula,
};

// Development
pub use development::{
    audit, bottle, create, edit, linkage, livecheck, migrate, readall, style, test, unpack,
};

// Developer commands
pub use developer::{
    bottle_merge, bump, bump_cask_pr, bump_formula_pr, bump_revision, contributions,
    determine_test_runners, developer, dispatch_build_bottle, fix_bottle_tags, generate_cask_api,
    generate_formula_api, generate_man_completions, install_bundler, install_bundler_gems,
    install_formula_api, irb, nodenv_sync, pr_automerge, pr_pull, pr_upload, prof, pyenv_sync,
    rbenv_sync, ruby, setup, setup_ruby, sponsor, tab, test_bot, typecheck, update_license_data,
    update_python_resources, vendor_gems,
};
