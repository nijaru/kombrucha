//! Kombrucha Library - Rust API for package management
//!
//! This library provides a programmatic interface to Homebrew package management operations.
//! It enables downstream projects to interact with Homebrew packages, dependencies, and
//! installation metadata without shelling out to the CLI.
//!
//! # Architecture
//!
//! - **api.rs**: Homebrew JSON API client with caching
//! - **cellar.rs**: Local Cellar inspection (installed packages)
//! - **download.rs**: Parallel bottle downloads from GHCR
//! - **extract.rs**: Bottle extraction to Cellar
//! - **symlink.rs**: Symlink management for installed packages
//! - **tap.rs**: Custom tap management
//! - **error.rs**: Unified error types
//!
//! # Quick Start
//!
//! ```no_run
//! use kombrucha::{BrewApi, cellar};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // List installed packages
//!     let installed = cellar::list_installed()?;
//!     for pkg in installed {
//!         println!("{} {}", pkg.name, pkg.version);
//!     }
//!
//!     // Query package metadata
//!     let api = BrewApi::new()?;
//!     let formula = api.fetch_formula("ripgrep").await?;
//!     println!("{}: {}", formula.name, formula.desc.unwrap_or_default());
//!
//!     Ok(())
//! }
//! ```

// Core library modules (no UI/CLI dependencies)
pub mod api;
pub mod cache;
pub mod cellar;
pub mod download;
pub mod error;
pub mod extract;
pub mod platform;
pub mod receipt;
pub mod symlink;
pub mod tap;

// Re-export commonly used types and functions
pub use api::{Bottle, BrewApi, Cask, Formula, SearchResults, Versions};
pub use cache::{get_cached_casks, get_cached_formulae, store_casks, store_formulae};
pub use cellar::{InstallReceipt, InstalledPackage, RuntimeDependency, cellar_path, detect_prefix};
pub use download::cache_dir;
pub use error::{BruError, Result};
pub use extract::extract_bottle;
pub use receipt::write_receipt;
pub use symlink::{link_formula, normalize_path, optlink, unlink_formula, unoptlink};
pub use tap::{list_taps, parse_formula_info, parse_formula_version};
