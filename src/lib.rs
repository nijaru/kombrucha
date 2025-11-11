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
//! - **receipt.rs**: Installation receipt generation and metadata
//! - **platform.rs**: Platform detection for bottle selection
//! - **cache.rs**: Persistent disk caching of API data
//! - **error.rs**: Unified error types
//!
//! # Key Concepts
//!
//! ## Cellar
//!
//! The Cellar is Homebrew's package directory (typically `/opt/homebrew/Cellar` on macOS).
//! Each installed package has a directory structure:
//! ```text
//! /opt/homebrew/Cellar/
//!   ripgrep/
//!     13.0.0/
//!       bin/
//!       lib/
//!       INSTALL_RECEIPT.json
//! ```
//!
//! ## Bottles
//!
//! Bottles are precompiled `.tar.gz` archives containing binaries for a specific platform.
//! The library can download, verify, and extract bottles from GitHub Container Registry (GHCR).
//!
//! ## API Client
//!
//! The [`BrewApi`] client queries Homebrew's public JSON API for formula metadata,
//! with in-memory and persistent disk caching.
//!
//! ## Symlinks
//!
//! After extraction, symlinks make binaries and libraries accessible from standard
//! directories (e.g., `/opt/homebrew/bin/ripgrep` → `../Cellar/ripgrep/13.0.0/bin/ripgrep`).
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
//!
//! # Common Tasks
//!
//! ## Query Package Information
//!
//! ```no_run
//! use kombrucha::BrewApi;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let api = BrewApi::new()?;
//!
//!     // Fetch metadata for a formula
//!     let formula = api.fetch_formula("python").await?;
//!
//!     println!("Name: {}", formula.name);
//!     println!("Version: {}", formula.versions.stable.unwrap_or_default());
//!     println!("Description: {}", formula.desc.unwrap_or_default());
//!     println!("Dependencies: {}", formula.dependencies.join(", "));
//!
//!     Ok(())
//! }
//! ```
//!
//! ## List Installed Packages
//!
//! ```no_run
//! use kombrucha::cellar;
//!
//! fn main() -> anyhow::Result<()> {
//!     // Get all installed packages
//!     let installed = cellar::list_installed()?;
//!     println!("Installed packages: {}", installed.len());
//!
//!     for pkg in installed {
//!         println!("  {} {}", pkg.name, pkg.version);
//!     }
//!
//!     // Get versions of a specific formula
//!     let versions = cellar::get_installed_versions("python")?;
//!     if !versions.is_empty() {
//!         println!("Python latest: {}", versions[0].version);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Search for Packages
//!
//! ```no_run
//! use kombrucha::BrewApi;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let api = BrewApi::new()?;
//!
//!     // Search across all formulae and casks
//!     let results = api.search("python").await?;
//!
//!     println!("Found {} formulae", results.formulae.len());
//!     for formula in &results.formulae {
//!         println!("  {} - {}", formula.name, formula.desc.as_deref().unwrap_or(""));
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Download and Extract a Bottle
//!
//! ```no_run
//! use kombrucha::{BrewApi, download, extract, symlink, cellar};
//! use std::fs;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let api = BrewApi::new()?;
//!     let client = reqwest::Client::new();
//!
//!     // Fetch formula metadata
//!     let formula = api.fetch_formula("ripgrep").await?;
//!
//!     // Step 1: Download bottle
//!     let bottle_path = download::download_bottle(&formula, None, &client).await?;
//!     println!("Downloaded bottle to: {}", bottle_path.display());
//!
//!     // Step 2: Extract to Cellar
//!     let version = formula.versions.stable.unwrap();
//!     let cellar_dir = extract::extract_bottle(&bottle_path, "ripgrep", &version)?;
//!     println!("Extracted to: {}", cellar_dir.display());
//!
//!     // Step 3: Create symlinks
//!     let linked = symlink::link_formula("ripgrep", &version)?;
//!     println!("Created {} symlinks", linked.len());
//!
//!     // Step 4: Create version-agnostic links
//!     symlink::optlink("ripgrep", &version)?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Read Installation Metadata
//!
//! ```no_run
//! use kombrucha::{receipt::InstallReceipt, cellar};
//! use std::path::Path;
//!
//! fn main() -> anyhow::Result<()> {
//!     // Read an existing installation's receipt
//!     let cellar_path = cellar::cellar_path().join("ripgrep").join("13.0.0");
//!     let receipt = InstallReceipt::read(&cellar_path)?;
//!
//!     println!("Installed with: {}", receipt.homebrew_version);
//!     println!("Installed on request: {}", receipt.installed_on_request);
//!     println!("Runtime dependencies: {}", receipt.runtime_dependencies.len());
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Work with Custom Taps
//!
//! ```no_run
//! use kombrucha::tap;
//!
//! fn main() -> anyhow::Result<()> {
//!     // List installed taps
//!     let taps = tap::list_taps()?;
//!     println!("Installed taps: {}", taps.join(", "));
//!
//!     // Parse formula metadata from a tap
//!     let formula_path = tap::tap_directory("user/repo")?
//!         .join("Formula")
//!         .join("mypackage.rb");
//!
//!     if let Some(version) = tap::parse_formula_version(&formula_path)? {
//!         println!("Formula version: {}", version);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! # Error Handling
//!
//! All fallible operations return [`Result<T>`], which is an alias for
//! `std::result::Result<T, BruError>`. Common error variants:
//!
//! - [`BruError::FormulaNotFound`] - Formula doesn't exist in Homebrew
//! - [`BruError::ApiError`] - Network request failed
//! - [`BruError::IoError`] - File system operation failed
//! - [`BruError::JsonError`] - JSON parsing failed
//!
//! ```no_run
//! use kombrucha::{BrewApi, BruError};
//!
//! #[tokio::main]
//! async fn main() {
//!     let api = BrewApi::new().unwrap();
//!     match api.fetch_formula("nonexistent-package").await {
//!         Ok(formula) => println!("Found: {}", formula.name),
//!         Err(BruError::FormulaNotFound(name)) => println!("'{}' not found", name),
//!         Err(e) => eprintln!("Error: {}", e),
//!     }
//! }
//! ```
//!
//! # Performance Characteristics
//!
//! - **API queries**: ~200-500ms per request (cached in-memory for session, disk cache 24h)
//! - **List installed**: 10-50ms on typical systems (depends on number of packages)
//! - **Download bottles**: Limited to 8 concurrent downloads; 500 Mbps connection downloads
//!   10 bottles in ~5-10 seconds
//! - **Extract bottles**: 50-200ms per bottle (depends on size and disk speed)
//! - **Symlink creation**: 10-50ms per formula (parallelized with rayon)

// Core library modules (no UI/CLI dependencies)
pub mod api;
pub mod cache;
pub mod cellar;
pub mod download;
pub mod error;
pub mod extract;
pub mod package_manager;
pub mod platform;
pub mod receipt;
pub mod symlink;
pub mod tap;

// Re-export commonly used types and functions
pub use api::{Bottle, BrewApi, Cask, Formula, SearchResults, Versions};
pub use cache::{get_cached_casks, get_cached_formulae, store_casks, store_formulae};
pub use cellar::{InstalledPackage, RuntimeDependency, cellar_path, detect_prefix, list_installed};
pub use download::cache_dir;
pub use error::{BruError, Result};
pub use extract::extract_bottle;
pub use package_manager::{
    CleanupResult, Dependencies, HealthCheck, InstallResult, OutdatedPackage, PackageManager,
    ReinstallResult, UninstallResult, UpgradeResult,
};
pub use receipt::InstallReceipt;
pub use symlink::{link_formula, normalize_path, optlink, unlink_formula, unoptlink};
pub use tap::{list_taps, parse_formula_info, parse_formula_version};
