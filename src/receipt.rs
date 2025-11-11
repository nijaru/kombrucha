//! Install receipt generation and metadata.
//!
//! This module creates and manages installation receipts - JSON files that Homebrew
//! stores alongside each installed package. These receipts contain:
//! - **Installation metadata**: When it was installed, Homebrew version, etc.
//! - **Dependencies**: Runtime and build dependencies
//! - **Source info**: Which tap it came from and the source formula
//! - **Build info**: Architecture, compiler, build platform, etc.
//!
//! # Architecture
//!
//! Each installed package has an `INSTALL_RECEIPT.json` file:
//! ```text
//! /opt/homebrew/Cellar/ripgrep/13.0.0/
//!   INSTALL_RECEIPT.json     # Metadata about this installation
//!   bin/
//!   lib/
//! ```
//!
//! The receipt contains information that allows:
//! - Detecting which packages were installed on request vs as dependencies
//! - Reading runtime dependencies for uninstall operations
//! - Determining the source tap for upgrades
//! - Identifying the Homebrew version that performed the installation
//!
//! # Examples
//!
//! ```no_run
//! use kombrucha::receipt::InstallReceipt;
//! use std::path::Path;
//!
//! fn main() -> anyhow::Result<()> {
//!     let cellar_path = Path::new("/opt/homebrew/Cellar/ripgrep/13.0.0");
//!     let receipt = InstallReceipt::read(cellar_path)?;
//!
//!     println!("Installed with: {}", receipt.homebrew_version);
//!     println!("On request: {}", receipt.installed_on_request);
//!     println!("Dependencies: {}", receipt.runtime_dependencies.len());
//!
//!     Ok(())
//! }
//! ```

use crate::api::Formula;
use crate::cellar::RuntimeDependency;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Install receipt compatible with Homebrew
#[derive(Debug, Serialize, Deserialize)]
pub struct InstallReceipt {
    pub homebrew_version: String,
    #[serde(default)]
    pub used_options: Vec<String>,
    #[serde(default)]
    pub unused_options: Vec<String>,
    pub built_as_bottle: bool,
    pub poured_from_bottle: bool,
    pub loaded_from_api: bool,
    pub installed_as_dependency: bool,
    pub installed_on_request: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changed_files: Option<Vec<String>>,
    pub time: i64,
    #[serde(default)]
    pub source_modified_time: i64,
    #[serde(default)]
    pub compiler: Option<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub runtime_dependencies: Vec<RuntimeDependency>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub built_on: Option<BuiltOn>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdlib: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SourceInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub tap: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tap_git_head: Option<String>,
    pub spec: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub versions: Option<SourceVersions>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SourceVersions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stable: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head: Option<String>,
    #[serde(default)]
    pub version_scheme: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compatibility_version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuiltOn {
    pub os: String,
    pub os_version: String,
    pub cpu_family: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xcode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_perl: Option<String>,
}

impl InstallReceipt {
    /// Read an existing INSTALL_RECEIPT.json file from a Cellar directory.
    ///
    /// Loads the installation metadata for an installed package. This receipt contains
    /// critical information about when the package was installed, what dependencies it has,
    /// and which tap it came from.
    ///
    /// # Arguments
    ///
    /// * `cellar_path` - Path to the installed package directory in the Cellar
    ///   (e.g., `/opt/homebrew/Cellar/ripgrep/13.0.0`)
    ///
    /// # Returns
    ///
    /// A fully-parsed `InstallReceipt` struct with all installation metadata.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The `INSTALL_RECEIPT.json` file doesn't exist in the cellar directory
    /// - The file cannot be read (permission denied, etc.)
    /// - The JSON is malformed or incompatible with the receipt schema
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use kombrucha::receipt::InstallReceipt;
    /// use std::path::Path;
    ///
    /// fn main() -> anyhow::Result<()> {
    ///     let cellar_path = Path::new("/opt/homebrew/Cellar/ripgrep/13.0.0");
    ///     let receipt = InstallReceipt::read(cellar_path)?;
    ///
    ///     // Check installation metadata
    ///     println!("Installed with: {}", receipt.homebrew_version);
    ///     println!("Installed on request: {}", receipt.installed_on_request);
    ///     println!("Poured from bottle: {}", receipt.poured_from_bottle);
    ///     println!("Runtime dependencies: {}", receipt.runtime_dependencies.len());
    ///
    ///     // Check source information
    ///     if let Some(source) = &receipt.source {
    ///         println!("From tap: {}", source.tap);
    ///         if let Some(stable) = &source.versions {
    ///             println!("Stable version: {:?}", stable.stable);
    ///         }
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Use Cases
    ///
    /// - **Determine installation source**: Check `receipt.source.tap` to identify which
    ///   tap the package came from (core Homebrew or custom tap)
    /// - **Get dependencies**: Access `receipt.runtime_dependencies` to find what packages
    ///   this one depends on (needed for safe uninstallation)
    /// - **Check installation method**: `receipt.poured_from_bottle` indicates if it was
    ///   installed from a precompiled bottle vs built from source
    /// - **Upgrade detection**: Use `receipt.source.versions.stable` to compare against
    ///   available versions in the API
    pub fn read(cellar_path: &Path) -> Result<Self> {
        let receipt_path = cellar_path.join("INSTALL_RECEIPT.json");
        let contents = fs::read_to_string(&receipt_path)
            .with_context(|| format!("Failed to read receipt: {}", receipt_path.display()))?;

        let receipt: Self =
            serde_json::from_str(&contents).context("Failed to parse INSTALL_RECEIPT.json")?;

        Ok(receipt)
    }

    /// Create a new receipt for a bottle installation.
    ///
    /// Generates a complete `InstallReceipt` for a formula that was installed from a
    /// precompiled bottle. The receipt contains installation metadata, dependencies, and
    /// source information that matches Homebrew's format.
    ///
    /// This receipt should be written to the Cellar directory after bottle extraction
    /// to track the installation for future operations (upgrades, uninstallation, etc.).
    ///
    /// # Arguments
    ///
    /// * `formula` - The formula being installed (provides version and API metadata)
    /// * `runtime_deps` - Vector of runtime dependencies for this package
    ///   (extracted from the formula's dependency list)
    /// * `installed_on_request` - Whether the user explicitly requested this package
    ///   (true) or it's being installed as a dependency (false)
    ///
    /// # Returns
    ///
    /// A new `InstallReceipt` with:
    /// - Current timestamp
    /// - Kombrucha/bru version identifier
    /// - Build environment detected from the system
    /// - Architecture detected from current system
    /// - Provided runtime dependencies
    /// - Source information (tap: "homebrew/core", spec: "stable")
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use kombrucha::{BrewApi, receipt::InstallReceipt, cellar::RuntimeDependency};
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let api = BrewApi::new()?;
    ///     let formula = api.fetch_formula("ripgrep").await?;
    ///
    ///     // Create runtime dependencies from formula
    ///     let runtime_deps: Vec<RuntimeDependency> = formula
    ///         .dependencies
    ///         .iter()
    ///         .enumerate()
    ///         .map(|(idx, dep)| RuntimeDependency {
    ///             full_name: dep.clone(),
    ///             version: "1.0.0".to_string(),
    ///             revision: 0,
    ///             bottle_rebuild: 0,
    ///             pkg_version: "1.0.0".to_string(),
    ///             declared_directly: idx == 0,
    ///         })
    ///         .collect();
    ///
    ///     // Generate receipt for explicit installation
    ///     let receipt = InstallReceipt::new_bottle(&formula, runtime_deps, true);
    ///
    ///     println!("Receipt version: {}", receipt.homebrew_version);
    ///     println!("Installed on request: {}", receipt.installed_on_request);
    ///     println!("Dependencies: {}", receipt.runtime_dependencies.len());
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Receipt Fields
    ///
    /// The generated receipt includes:
    /// - `homebrew_version`: Set to `"bru/{version}"` (Kombrucha version)
    /// - `built_as_bottle`: Always `true` (this is for bottle installations)
    /// - `poured_from_bottle`: Always `true` (indicates precompiled installation)
    /// - `loaded_from_api`: Always `true` (metadata came from Homebrew API)
    /// - `installed_as_dependency`: Set to the opposite of `installed_on_request`
    /// - `time`: Current Unix timestamp
    /// - `arch`: Detected system architecture ("arm64" or "x86_64")
    /// - `built_on`: Detected build environment (macOS version, CPU family, etc.)
    pub fn new_bottle(
        formula: &Formula,
        runtime_deps: Vec<RuntimeDependency>,
        installed_on_request: bool,
    ) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        Self {
            homebrew_version: format!("bru/{}", env!("CARGO_PKG_VERSION")),
            used_options: vec![],
            unused_options: vec![],
            built_as_bottle: true,
            poured_from_bottle: true,
            loaded_from_api: true,
            installed_as_dependency: !installed_on_request,
            installed_on_request,
            changed_files: Some(vec![]),
            time: now,
            source_modified_time: now,
            compiler: Some("clang".to_string()),
            aliases: vec![],
            runtime_dependencies: runtime_deps,
            source: Some(SourceInfo {
                path: Some(format!(
                    "{}/Library/Caches/Homebrew/api/formula.jws.json",
                    std::env::var("HOME").unwrap_or_else(|_| "/Users/USER".to_string())
                )),
                tap: "homebrew/core".to_string(),
                tap_git_head: None,
                spec: "stable".to_string(),
                versions: Some(SourceVersions {
                    stable: formula.versions.stable.clone(),
                    head: None,
                    version_scheme: 0,
                    compatibility_version: None,
                }),
            }),
            arch: Some(homebrew_arch().to_string()),
            built_on: detect_build_environment(),
            stdlib: Some("libc++".to_string()),
        }
    }

    /// Write receipt to INSTALL_RECEIPT.json in the Cellar directory.
    ///
    /// Persists the installation metadata to disk as a JSON file. This receipt is essential
    /// for Homebrew to track the package installation and enable future operations like
    /// upgrades and uninstallation.
    ///
    /// # Arguments
    ///
    /// * `cellar_path` - Path to the installed package directory in the Cellar
    ///   (e.g., `/opt/homebrew/Cellar/ripgrep/13.0.0`)
    ///
    /// # Returns
    ///
    /// `Ok(())` if the receipt was successfully written.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The Cellar directory doesn't exist and can't be created
    /// - Write permission is denied on the Cellar directory
    /// - The receipt cannot be serialized to JSON (shouldn't happen with valid data)
    /// - Disk is full or other I/O error occurs
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use kombrucha::{BrewApi, receipt::InstallReceipt, cellar};
    /// use std::path::Path;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let api = BrewApi::new()?;
    ///     let formula = api.fetch_formula("ripgrep").await?;
    ///
    ///     // Create and write receipt
    ///     let receipt = InstallReceipt::new_bottle(&formula, vec![], true);
    ///     let cellar_path = cellar::cellar_path().join("ripgrep").join("13.0.0");
    ///
    ///     // Ensure directory exists
    ///     std::fs::create_dir_all(&cellar_path)?;
    ///
    ///     // Write receipt to INSTALL_RECEIPT.json
    ///     receipt.write(&cellar_path)?;
    ///     println!("Receipt written to: {}", cellar_path.display());
    ///
    ///     // Verify it can be read back
    ///     let receipt_copy = InstallReceipt::read(&cellar_path)?;
    ///     assert_eq!(receipt_copy.installed_on_request, true);
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Installation Workflow
    ///
    /// Typical bottle installation sequence:
    /// 1. Download bottle (using [`download::download_bottle`](crate::download::download_bottle))
    /// 2. Extract bottle (using [`extract::extract_bottle`](crate::extract::extract_bottle))
    /// 3. Create receipt with [`InstallReceipt::new_bottle`]
    /// 4. **Write receipt with this function** ← You are here
    /// 5. Create symlinks (using [`symlink::link_formula`](crate::symlink::link_formula))
    ///
    /// # File Format
    ///
    /// The receipt is written as pretty-printed JSON (indented for readability):
    /// ```json
    /// {
    ///   "homebrew_version": "bru/0.1.0",
    ///   "installed_on_request": true,
    ///   "poured_from_bottle": true,
    ///   "time": 1699564800,
    ///   "runtime_dependencies": [],
    ///   ...
    /// }
    /// ```
    pub fn write(&self, cellar_path: &Path) -> Result<()> {
        let receipt_path = cellar_path.join("INSTALL_RECEIPT.json");
        let json =
            serde_json::to_string_pretty(self).context("Failed to serialize install receipt")?;

        fs::write(&receipt_path, json)
            .with_context(|| format!("Failed to write receipt: {}", receipt_path.display()))?;

        Ok(())
    }
}

/// Convert Rust target architecture to Homebrew platform name
/// Homebrew uses "arm64" for Apple Silicon, while Rust uses "aarch64"
fn homebrew_arch() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "arm64",
        arch => arch,
    }
}

/// Detect build environment for receipt
fn detect_build_environment() -> Option<BuiltOn> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        let os_version = Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        Some(BuiltOn {
            os: "Macintosh".to_string(),
            os_version: format!("macOS {}", os_version),
            cpu_family: homebrew_arch().to_string(),
            xcode: None,
            clt: None,
            preferred_perl: Some("5.34".to_string()),
        })
    }

    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}
