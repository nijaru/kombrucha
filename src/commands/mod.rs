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
pub mod query;
pub mod services;
pub mod tap;
pub mod utilities;
pub(crate) mod utils;

// Re-export commonly used commands for convenience
// This allows using `commands::search()` instead of `commands::query::search()`

// Query commands
pub use query::formula_info;
pub use query::{
    casks, cat, deps, desc, formula, formulae, info, options, search, unbottled, uses,
};

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
pub use git::{gist_logs, log};

// Utilities
pub use utilities::{
    alias, command, command_not_found_init, commands, docs, man, unalias, which_formula,
};

// Development
pub use development::{
    audit, bottle, create, edit, extract, linkage, livecheck, migrate, readall, style, test, unpack,
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
