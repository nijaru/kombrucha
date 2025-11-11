//! Homebrew Cellar inspection - reading and analyzing installed packages.
//!
//! This module provides functions to inspect the Homebrew Cellar directory, which is where
//! Homebrew stores all installed packages. It allows you to:
//! - List all installed packages and versions
//! - Detect the system's Homebrew prefix
//! - Read installation metadata from INSTALL_RECEIPT.json files
//! - Query dependencies of installed packages
//!
//! # Architecture
//!
//! The Cellar is located at:
//! - **macOS (Apple Silicon)**: `/opt/homebrew/Cellar/`
//! - **macOS (Intel)**: `/usr/local/Cellar/`
//! - **Linux**: Varies, usually `/opt/homebrew/Cellar/` or `/home/user/.linuxbrew/Cellar/`
//!
//! Each installed package has the structure:
//! ```text
//! /opt/homebrew/Cellar/
//!   formula-name/
//!     1.0.0/                    # Version directory
//!       bin/                    # Executable files
//!       lib/                    # Library files
//!       INSTALL_RECEIPT.json    # Installation metadata
//!     1.1.0/
//!       ...
//! ```
//!
//! # Examples
//!
//! ```no_run
//! use kombrucha::cellar;
//!
//! fn main() -> anyhow::Result<()> {
//!     // List all installed packages
//!     let installed = cellar::list_installed()?;
//!     for pkg in installed {
//!         println!("{} {}", pkg.name, pkg.version);
//!     }
//!
//!     // Find a specific formula's versions
//!     let versions = cellar::get_installed_versions("python")?;
//!     for pkg in versions {
//!         println!("  {}", pkg.version);
//!     }
//!
//!     // Get system info
//!     let prefix = cellar::detect_prefix();
//!     println!("Homebrew prefix: {}", prefix.display());
//!
//!     Ok(())
//! }
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Detect the Homebrew prefix on this system.
///
/// Returns the root directory where Homebrew is installed:
/// - **macOS (Apple Silicon)**: `/opt/homebrew`
/// - **macOS (Intel)**: `/usr/local`
/// - **Linux**: Usually `/opt/homebrew` or `/home/user/.linuxbrew`
///
/// The detection order is:
/// 1. `HOMEBREW_PREFIX` environment variable (if set)
/// 2. Architecture-based detection (aarch64 → `/opt/homebrew`, x86_64 → `/usr/local`)
///
/// # Examples
///
/// ```no_run
/// use kombrucha::cellar;
///
/// let prefix = cellar::detect_prefix();
/// println!("Homebrew prefix: {}", prefix.display());
/// // Output: "/opt/homebrew" (on Apple Silicon)
/// // Output: "/usr/local" (on Intel)
/// ```
pub fn detect_prefix() -> PathBuf {
    // First check environment variable
    if let Ok(prefix) = std::env::var("HOMEBREW_PREFIX") {
        return PathBuf::from(prefix);
    }

    // Detect by architecture
    #[cfg(target_arch = "aarch64")]
    {
        PathBuf::from("/opt/homebrew")
    }
    #[cfg(target_arch = "x86_64")]
    {
        PathBuf::from("/usr/local")
    }
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    {
        PathBuf::from("/usr/local")
    }
}

/// Get the Cellar directory path.
///
/// Returns the path to Homebrew's Cellar directory where all installed packages are stored.
/// This is equivalent to `{prefix}/Cellar`.
///
/// # Examples
///
/// ```no_run
/// use kombrucha::cellar;
///
/// let cellar = cellar::cellar_path();
/// println!("Cellar location: {}", cellar.display());
/// // Output: "/opt/homebrew/Cellar" (on Apple Silicon)
/// ```
///
/// # Directory Structure
///
/// The Cellar contains installed packages in this structure:
/// ```text
/// /opt/homebrew/Cellar/
///   ripgrep/
///     13.0.0/
///       bin/ripgrep
///       INSTALL_RECEIPT.json
///     12.1.1/
///       bin/ripgrep
///       INSTALL_RECEIPT.json
///   python/
///     3.13.0/
///       bin/python3
///       INSTALL_RECEIPT.json
/// ```
pub fn cellar_path() -> PathBuf {
    detect_prefix().join("Cellar")
}

/// Runtime dependency from install receipt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeDependency {
    pub full_name: String,
    pub version: String,
    pub revision: u32,
    #[serde(default)]
    pub bottle_rebuild: u32,
    pub pkg_version: String,
    #[serde(default)]
    pub declared_directly: bool,
}

/// Source information from install receipt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceInfo {
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub tap: Option<String>,
    #[serde(default)]
    pub tap_git_head: Option<String>,
    #[serde(default)]
    pub spec: Option<String>,
}

/// Install receipt from Homebrew
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallReceipt {
    pub homebrew_version: String,
    #[serde(default)]
    pub built_as_bottle: bool,
    #[serde(default)]
    pub poured_from_bottle: bool,
    #[serde(default)]
    pub loaded_from_api: bool,
    #[serde(default)]
    pub installed_as_dependency: bool,
    #[serde(default)]
    pub installed_on_request: bool,
    #[serde(default)]
    pub time: Option<i64>,
    #[serde(default)]
    pub runtime_dependencies: Vec<RuntimeDependency>,
    #[serde(default)]
    pub source: Option<SourceInfo>,
}

/// An installed package in the Cellar
#[derive(Debug, Clone)]
pub struct InstalledPackage {
    pub name: String,
    pub version: String,
    #[allow(dead_code)]
    pub path: PathBuf,
    #[allow(dead_code)]
    pub receipt: Option<InstallReceipt>,
}

impl InstalledPackage {
    /// Create from a Cellar version directory.
    ///
    /// Reads the package metadata and INSTALL_RECEIPT.json if available.
    pub fn from_path(name: String, version: String, path: PathBuf) -> Self {
        let receipt = Self::read_receipt(&path).ok();
        Self {
            name,
            version,
            path,
            receipt,
        }
    }

    /// Read the INSTALL_RECEIPT.json file
    fn read_receipt(path: &Path) -> Result<InstallReceipt> {
        let receipt_path = path.join("INSTALL_RECEIPT.json");
        let contents = fs::read_to_string(&receipt_path)
            .with_context(|| format!("Failed to read receipt: {}", receipt_path.display()))?;
        let receipt: InstallReceipt = serde_json::from_str(&contents)
            .with_context(|| format!("Failed to parse receipt: {}", receipt_path.display()))?;
        Ok(receipt)
    }

    /// Check if this was installed on request (vs as dependency).
    ///
    /// Returns `true` if the user explicitly requested this package installation,
    /// `false` if it was installed as a dependency of another package.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use kombrucha::cellar;
    ///
    /// fn main() -> anyhow::Result<()> {
    ///     let installed = cellar::list_installed()?;
    ///     for pkg in installed {
    ///         if pkg.installed_on_request() {
    ///             println!("{} was explicitly installed", pkg.name);
    ///         }
    ///     }
    ///     Ok(())
    /// }
    /// ```
    #[allow(dead_code)]
    pub fn installed_on_request(&self) -> bool {
        self.receipt
            .as_ref()
            .map(|r| r.installed_on_request)
            .unwrap_or(false)
    }

    /// Get runtime dependencies of this installed package.
    ///
    /// Returns a list of packages this package depends on at runtime. This is useful
    /// for understanding what other packages would be affected if this package is removed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use kombrucha::cellar;
    ///
    /// fn main() -> anyhow::Result<()> {
    ///     let installed = cellar::list_installed()?;
    ///     for pkg in installed {
    ///         let deps = pkg.runtime_dependencies();
    ///         if !deps.is_empty() {
    ///             println!("{} depends on: {:?}", pkg.name, deps);
    ///         }
    ///     }
    ///     Ok(())
    /// }
    /// ```
    #[allow(dead_code)]
    pub fn runtime_dependencies(&self) -> Vec<RuntimeDependency> {
        self.receipt
            .as_ref()
            .map(|r| r.runtime_dependencies.clone())
            .unwrap_or_default()
    }
}

/// Read all installed packages from the Cellar.
///
/// Returns a list of all installed formulae with their versions. Each installed version
/// of each formula is returned as a separate `InstalledPackage` entry.
///
/// # Returns
///
/// - Empty `Vec` if the Cellar doesn't exist yet
/// - `Vec` of `InstalledPackage` with name, version, path, and installation metadata
///
/// # Errors
///
/// Returns an error if the Cellar directory cannot be read (e.g., permission denied).
///
/// # Examples
///
/// ```no_run
/// use kombrucha::cellar;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     // List all installed packages
///     let installed = cellar::list_installed()?;
///     println!("Installed packages: {}", installed.len());
///     for pkg in installed {
///         println!("  {} {}", pkg.name, pkg.version);
///     }
///
///     Ok(())
/// }
/// ```
///
/// # Performance
///
/// O(n) where n is the total number of installed package versions.
/// On a typical system with 200+ packages, this takes 10-50ms.
pub fn list_installed() -> Result<Vec<InstalledPackage>> {
    let cellar = cellar_path();

    if !cellar.exists() {
        return Ok(vec![]);
    }

    let mut packages = Vec::new();

    for entry in fs::read_dir(&cellar)
        .with_context(|| format!("Failed to read Cellar: {}", cellar.display()))?
    {
        let entry = entry?;
        let formula_name = entry.file_name().to_string_lossy().to_string();

        // Skip hidden files
        if formula_name.starts_with('.') {
            continue;
        }

        // Read all versions for this formula
        for version_entry in fs::read_dir(entry.path())? {
            let version_entry = version_entry?;
            let version = version_entry.file_name().to_string_lossy().to_string();

            // Skip hidden files
            if version.starts_with('.') {
                continue;
            }

            let pkg =
                InstalledPackage::from_path(formula_name.clone(), version, version_entry.path());
            packages.push(pkg);
        }
    }

    Ok(packages)
}

/// Get all versions of a specific formula, sorted by version (newest first).
///
/// Returns a list of all installed versions of a formula, with the newest version first.
/// This is useful for checking if a formula is installed and what versions are available locally.
///
/// # Arguments
///
/// * `formula` - The formula name (e.g., `"python"`, `"ripgrep"`)
///
/// # Returns
///
/// - Empty `Vec` if the formula is not installed
/// - Sorted `Vec` with newest version at index 0
///
/// # Errors
///
/// Returns an error if the formula directory cannot be read.
///
/// # Examples
///
/// ```no_run
/// use kombrucha::cellar;
///
/// fn main() -> anyhow::Result<()> {
///     // Get all installed versions of python
///     let versions = cellar::get_installed_versions("python")?;
///     if !versions.is_empty() {
///         println!("Installed versions:");
///         for v in &versions {
///             println!("  {} {}", v.name, v.version);
///         }
///         // Newest version is at [0]
///         println!("Latest: {}", versions[0].version);
///     } else {
///         println!("python is not installed");
///     }
///
///     Ok(())
/// }
/// ```
///
/// # Sorting
///
/// Versions are sorted semantically (e.g., 1.10.0 > 1.9.0). This ensures that accessing
/// the first element always gives you the newest installed version.
pub fn get_installed_versions(formula: &str) -> Result<Vec<InstalledPackage>> {
    let formula_path = cellar_path().join(formula);

    if !formula_path.exists() {
        return Ok(vec![]);
    }

    let mut packages = Vec::new();

    for entry in fs::read_dir(&formula_path)? {
        let entry = entry?;
        let version = entry.file_name().to_string_lossy().to_string();

        if version.starts_with('.') {
            continue;
        }

        let pkg = InstalledPackage::from_path(formula.to_string(), version, entry.path());
        packages.push(pkg);
    }

    // Sort by version - newest first
    // This ensures [0] is always the newest version
    packages.sort_by(|a, b| compare_versions(&a.version, &b.version));
    packages.reverse();

    Ok(packages)
}

/// Compare two version strings semantically
pub fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    // Parse as semantic version numbers
    let a_parts: Vec<u32> = a.split('.').filter_map(|s| s.parse::<u32>().ok()).collect();
    let b_parts: Vec<u32> = b.split('.').filter_map(|s| s.parse::<u32>().ok()).collect();

    // Compare version parts numerically
    for i in 0..a_parts.len().max(b_parts.len()) {
        let a_part = a_parts.get(i).unwrap_or(&0);
        let b_part = b_parts.get(i).unwrap_or(&0);
        match a_part.cmp(b_part) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }

    // Fall back to lexicographic
    a.cmp(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_prefix() {
        let prefix = detect_prefix();
        assert!(
            prefix.to_string_lossy().contains("homebrew")
                || prefix.to_string_lossy().contains("local")
        );
    }

    #[test]
    fn test_cellar_path() {
        let cellar = cellar_path();
        assert!(cellar.ends_with("Cellar"));
    }
}
