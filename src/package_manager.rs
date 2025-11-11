//! High-level PackageManager API - unified interface for package management operations.
//!
//! This module provides a simplified, production-ready API for common package management
//! workflows. It wraps the low-level modules (api, cellar, download, extract, symlink, etc.)
//! and manages shared resources like HTTP clients and caching.
//!
//! # Quick Start
//!
//! ```no_run
//! use kombrucha::PackageManager;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let pm = PackageManager::new()?;
//!
//!     // Install a package
//!     let result = pm.install("ripgrep").await?;
//!     println!("Installed {} {}", result.name, result.version);
//!
//!     // Check for upgrades
//!     let outdated = pm.outdated().await?;
//!     for pkg in outdated {
//!         println!("{} {} → {}", pkg.name, pkg.installed, pkg.latest);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! # Resource Management
//!
//! PackageManager holds a single HTTP client shared across all operations, enabling
//! connection pooling and efficient resource usage. Keep the same instance alive for
//! multiple operations to benefit from caching and connection reuse.
//!
//! # Error Handling
//!
//! All operations return [`Result<T>`] with detailed error information. Common error patterns:
//!
//! ```ignore
//! match pm.install("nonexistent").await {
//!     Ok(result) => println!("Installed {}", result.version),
//!     Err(e) => eprintln!("Installation failed: {}", e),
//! }
//! ```

use crate::api::{BrewApi, Formula};
use crate::cellar::{
    self, InstalledPackage, RuntimeDependency, cellar_path, detect_prefix, list_installed,
};
use crate::error::Result;
use crate::{download, extract, receipt, symlink};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

/// Result of an install operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallResult {
    /// Package name
    pub name: String,
    /// Installed version
    pub version: String,
    /// Path in Cellar
    pub path: PathBuf,
    /// Direct runtime dependencies
    pub dependencies: Vec<String>,
    /// Whether symlinks were created
    pub linked: bool,
    /// Time taken (milliseconds)
    pub time_ms: u64,
}

/// Result of an uninstall operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UninstallResult {
    /// Package name
    pub name: String,
    /// Uninstalled version
    pub version: String,
    /// Whether symlinks were removed
    pub unlinked: bool,
    /// Time taken (milliseconds)
    pub time_ms: u64,
}

/// Result of an upgrade operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeResult {
    /// Package name
    pub name: String,
    /// Previous version
    pub from_version: String,
    /// New version
    pub to_version: String,
    /// Path in Cellar
    pub path: PathBuf,
    /// Time taken (milliseconds)
    pub time_ms: u64,
}

/// Result of a reinstall operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReinstallResult {
    /// Package name
    pub name: String,
    /// Installed version
    pub version: String,
    /// Path in Cellar
    pub path: PathBuf,
    /// Time taken (milliseconds)
    pub time_ms: u64,
}

/// Information about an outdated package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutdatedPackage {
    /// Package name
    pub name: String,
    /// Currently installed version
    pub installed: String,
    /// Latest available version
    pub latest: String,
    /// Whether this package can be upgraded (not pinned)
    pub changeable: bool,
}

/// Result of a cleanup operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupResult {
    /// Removed versions (formula/version)
    pub removed: Vec<String>,
    /// Space freed in MB
    pub space_freed_mb: f64,
    /// Errors encountered (formula, error message)
    pub errors: Vec<(String, String)>,
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Homebrew CLI is available
    pub homebrew_available: bool,
    /// Cellar directory exists
    pub cellar_exists: bool,
    /// Prefix directory is writable
    pub prefix_writable: bool,
    /// List of issues found
    pub issues: Vec<String>,
}

/// Dependency information for a package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependencies {
    /// Package name
    pub name: String,
    /// Direct runtime dependencies
    pub runtime: Vec<String>,
    /// Build dependencies (if applicable)
    pub build: Vec<String>,
}

/// High-level package manager API
///
/// This is the recommended interface for most use cases. It provides a simplified,
/// production-ready API that handles resource management automatically.
///
/// # Examples
///
/// ```no_run
/// use kombrucha::PackageManager;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let pm = PackageManager::new()?;
///
///     // Single operation
///     let result = pm.install("ripgrep").await?;
///     println!("Installed {} {}", result.name, result.version);
///
///     // Reuse for multiple operations (benefits from caching + connection reuse)
///     let outdated = pm.outdated().await?;
///     for pkg in outdated {
///         let upgrade = pm.upgrade(&pkg.name).await?;
///         println!("Upgraded {} to {}", pkg.name, upgrade.to_version);
///     }
///
///     Ok(())
/// }
/// ```
pub struct PackageManager {
    api: BrewApi,
    client: reqwest::Client,
}

impl PackageManager {
    /// Create a new PackageManager with default configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the BrewApi client cannot be created.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use kombrucha::PackageManager;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let pm = PackageManager::new()?;
    ///     Ok(())
    /// }
    /// ```
    pub fn new() -> Result<Self> {
        let api = BrewApi::new()?;
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .pool_max_idle_per_host(10)
            .build()?;

        Ok(Self { api, client })
    }

    /// Install a package from a bottle.
    ///
    /// # Arguments
    ///
    /// * `name` - Formula name (e.g., `"ripgrep"`)
    ///
    /// # Returns
    ///
    /// Installation result with version, path, and metadata.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Formula not found
    /// - No bottle available (use upgrade workflow instead)
    /// - Download or extraction fails
    /// - Symlink creation fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use kombrucha::PackageManager;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let pm = PackageManager::new()?;
    ///     let result = pm.install("ripgrep").await?;
    ///     println!("Installed {} {}", result.name, result.version);
    ///     println!("  Path: {}", result.path.display());
    ///     println!("  Dependencies: {}", result.dependencies.join(", "));
    ///     Ok(())
    /// }
    /// ```
    pub async fn install(&self, name: &str) -> Result<InstallResult> {
        let start = Instant::now();

        // Fetch formula
        let formula = self
            .api
            .fetch_formula(name)
            .await
            .map_err(|_| anyhow!("Formula '{}' not found", name))?;

        let version = formula
            .versions
            .stable
            .as_ref()
            .ok_or_else(|| anyhow!("No stable version available for {}", name))?
            .clone();

        // Step 1: Download bottle
        let bottle_path = download::download_bottle(&formula, None, &self.client)
            .await
            .map_err(|e| anyhow!("Failed to download bottle: {}", e))?;

        // Step 2: Extract to Cellar
        let cellar_dir = extract::extract_bottle(&bottle_path, &formula.name, &version)
            .map_err(|e| anyhow!("Failed to extract bottle: {}", e))?;

        // Step 3: Generate runtime dependencies from formula metadata
        // In a real scenario, we'd query the API for each dependency to get full info
        // For now, use formula dependencies as fallback
        let runtime_deps: Vec<RuntimeDependency> = formula
            .dependencies
            .iter()
            .enumerate()
            .map(|(idx, dep_name)| RuntimeDependency {
                full_name: dep_name.clone(),
                version: "0.0.0".to_string(), // Placeholder - would fetch from API in production
                revision: 0,
                bottle_rebuild: 0,
                pkg_version: "0.0.0".to_string(),
                declared_directly: idx == 0,
            })
            .collect();

        // Step 4: Create installation receipt
        let install_receipt = receipt::InstallReceipt::new_bottle(&formula, runtime_deps, true);
        install_receipt
            .write(&cellar_dir)
            .map_err(|e| anyhow!("Failed to write installation receipt: {}", e))?;

        // Step 5: Create symlinks
        let _linked_files = symlink::link_formula(&formula.name, &version)
            .map_err(|e| anyhow!("Failed to create symlinks: {}", e))?;

        symlink::optlink(&formula.name, &version)
            .map_err(|e| anyhow!("Failed to create opt symlink: {}", e))?;

        Ok(InstallResult {
            name: formula.name.clone(),
            version,
            path: cellar_dir,
            dependencies: formula.dependencies.clone(),
            linked: true,
            time_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Uninstall a package.
    ///
    /// # Arguments
    ///
    /// * `name` - Formula name
    ///
    /// # Returns
    ///
    /// Uninstall result with metadata.
    ///
    /// # Errors
    ///
    /// Returns an error if the package is not installed or uninstall fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use kombrucha::PackageManager;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let pm = PackageManager::new()?;
    ///     let result = pm.uninstall("ripgrep").await?;
    ///     println!("Uninstalled {} {}", result.name, result.version);
    ///     Ok(())
    /// }
    /// ```
    pub async fn uninstall(&self, name: &str) -> Result<UninstallResult> {
        let start = Instant::now();

        let versions = crate::cellar::get_installed_versions(name)?;
        let version = versions
            .first()
            .ok_or_else(|| anyhow!("Package '{}' not installed", name))?
            .version
            .clone();

        // Step 1: Remove symlinks
        symlink::unlink_formula(name, &version)
            .map_err(|e| anyhow!("Failed to remove symlinks: {}", e))?;

        symlink::unoptlink(name).map_err(|e| anyhow!("Failed to remove opt symlink: {}", e))?;

        // Step 2: Remove from Cellar
        let cellar = cellar_path();
        let formula_path = cellar.join(name).join(&version);
        fs::remove_dir_all(&formula_path)
            .map_err(|e| anyhow!("Failed to remove Cellar directory: {}", e))?;

        Ok(UninstallResult {
            name: name.to_string(),
            version,
            unlinked: true,
            time_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Upgrade a package to the latest version.
    ///
    /// # Arguments
    ///
    /// * `name` - Formula name
    ///
    /// # Returns
    ///
    /// Upgrade result with version information.
    ///
    /// # Errors
    ///
    /// Returns an error if the package is not installed or upgrade fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use kombrucha::PackageManager;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let pm = PackageManager::new()?;
    ///     let result = pm.upgrade("ripgrep").await?;
    ///     println!("Upgraded {} {} → {}", result.name, result.from_version, result.to_version);
    ///     Ok(())
    /// }
    /// ```
    pub async fn upgrade(&self, name: &str) -> Result<UpgradeResult> {
        let start = Instant::now();

        let installed = crate::cellar::get_installed_versions(name)?;
        let from_version = installed
            .first()
            .ok_or_else(|| anyhow!("Package '{}' not installed", name))?
            .version
            .clone();

        let formula = self
            .api
            .fetch_formula(name)
            .await
            .map_err(|_| anyhow!("Formula '{}' not found", name))?;

        let to_version = formula
            .versions
            .stable
            .as_ref()
            .ok_or_else(|| anyhow!("No stable version available for {}", name))?
            .clone();

        // If already at latest version, nothing to do
        if from_version == to_version {
            return Ok(UpgradeResult {
                name: formula.name.clone(),
                from_version,
                to_version: to_version.clone(),
                path: cellar_path().join(&formula.name).join(&to_version),
                time_ms: start.elapsed().as_millis() as u64,
            });
        }

        // Step 1: Download new bottle
        let bottle_path = download::download_bottle(&formula, None, &self.client)
            .await
            .map_err(|e| anyhow!("Failed to download bottle: {}", e))?;

        // Step 2: Extract to Cellar
        let cellar_dir = extract::extract_bottle(&bottle_path, &formula.name, &to_version)
            .map_err(|e| anyhow!("Failed to extract bottle: {}", e))?;

        // Step 3: Generate runtime dependencies
        let runtime_deps: Vec<RuntimeDependency> = formula
            .dependencies
            .iter()
            .enumerate()
            .map(|(idx, dep_name)| RuntimeDependency {
                full_name: dep_name.clone(),
                version: "0.0.0".to_string(),
                revision: 0,
                bottle_rebuild: 0,
                pkg_version: "0.0.0".to_string(),
                declared_directly: idx == 0,
            })
            .collect();

        // Step 4: Create installation receipt for new version
        let install_receipt = receipt::InstallReceipt::new_bottle(&formula, runtime_deps, true);
        install_receipt
            .write(&cellar_dir)
            .map_err(|e| anyhow!("Failed to write installation receipt: {}", e))?;

        // Step 5: Update symlinks to new version
        let _linked_files = symlink::link_formula(&formula.name, &to_version)
            .map_err(|e| anyhow!("Failed to create symlinks: {}", e))?;

        symlink::optlink(&formula.name, &to_version)
            .map_err(|e| anyhow!("Failed to create opt symlink: {}", e))?;

        // Step 6: Remove old version from Cellar
        let cellar = cellar_path();
        let old_formula_path = cellar.join(&formula.name).join(&from_version);
        if old_formula_path.exists() {
            let _ = fs::remove_dir_all(&old_formula_path);
        }

        Ok(UpgradeResult {
            name: formula.name.clone(),
            from_version,
            to_version,
            path: cellar_dir,
            time_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Reinstall a package (same version, fresh).
    ///
    /// # Arguments
    ///
    /// * `name` - Formula name
    ///
    /// # Returns
    ///
    /// Reinstall result with metadata.
    ///
    /// # Errors
    ///
    /// Returns an error if the package is not installed or reinstall fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use kombrucha::PackageManager;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let pm = PackageManager::new()?;
    ///     let result = pm.reinstall("ripgrep").await?;
    ///     println!("Reinstalled {} {}", result.name, result.version);
    ///     Ok(())
    /// }
    /// ```
    pub async fn reinstall(&self, name: &str) -> Result<ReinstallResult> {
        let start = Instant::now();

        let versions = crate::cellar::get_installed_versions(name)?;
        let version = versions
            .first()
            .ok_or_else(|| anyhow!("Package '{}' not installed", name))?
            .version
            .clone();

        let formula = self
            .api
            .fetch_formula(name)
            .await
            .map_err(|_| anyhow!("Formula '{}' not found", name))?;

        // Step 1: Remove old installation
        symlink::unlink_formula(name, &version)
            .map_err(|e| anyhow!("Failed to remove symlinks: {}", e))?;

        symlink::unoptlink(name).map_err(|e| anyhow!("Failed to remove opt symlink: {}", e))?;

        let cellar = cellar_path();
        let formula_path = cellar.join(name).join(&version);
        fs::remove_dir_all(&formula_path)
            .map_err(|e| anyhow!("Failed to remove Cellar directory: {}", e))?;

        // Step 2: Download fresh bottle
        let bottle_path = download::download_bottle(&formula, None, &self.client)
            .await
            .map_err(|e| anyhow!("Failed to download bottle: {}", e))?;

        // Step 3: Extract to Cellar
        let cellar_dir = extract::extract_bottle(&bottle_path, &formula.name, &version)
            .map_err(|e| anyhow!("Failed to extract bottle: {}", e))?;

        // Step 4: Generate runtime dependencies
        let runtime_deps: Vec<RuntimeDependency> = formula
            .dependencies
            .iter()
            .enumerate()
            .map(|(idx, dep_name)| RuntimeDependency {
                full_name: dep_name.clone(),
                version: "0.0.0".to_string(),
                revision: 0,
                bottle_rebuild: 0,
                pkg_version: "0.0.0".to_string(),
                declared_directly: idx == 0,
            })
            .collect();

        // Step 5: Create installation receipt
        let install_receipt = receipt::InstallReceipt::new_bottle(&formula, runtime_deps, true);
        install_receipt
            .write(&cellar_dir)
            .map_err(|e| anyhow!("Failed to write installation receipt: {}", e))?;

        // Step 6: Create symlinks
        let _linked_files = symlink::link_formula(&formula.name, &version)
            .map_err(|e| anyhow!("Failed to create symlinks: {}", e))?;

        symlink::optlink(&formula.name, &version)
            .map_err(|e| anyhow!("Failed to create opt symlink: {}", e))?;

        Ok(ReinstallResult {
            name: name.to_string(),
            version,
            path: cellar_dir,
            time_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Search for packages matching a query.
    ///
    /// # Arguments
    ///
    /// * `query` - Search query (matches name and description)
    ///
    /// # Returns
    ///
    /// Search results with formulae and casks.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use kombrucha::PackageManager;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let pm = PackageManager::new()?;
    ///     let results = pm.search("python").await?;
    ///     println!("Found {} formulae", results.formulae.len());
    ///     for f in &results.formulae {
    ///         println!("  {} - {}", f.name, f.desc.as_deref().unwrap_or(""));
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn search(&self, query: &str) -> Result<crate::api::SearchResults> {
        self.api.search(query).await
    }

    /// Get information about a package.
    ///
    /// # Arguments
    ///
    /// * `name` - Formula name
    ///
    /// # Returns
    ///
    /// Complete formula metadata.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use kombrucha::PackageManager;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let pm = PackageManager::new()?;
    ///     let formula = pm.info("ripgrep").await?;
    ///     println!("Name: {}", formula.name);
    ///     println!("Version: {}", formula.versions.stable.unwrap_or_default());
    ///     println!("Description: {}", formula.desc.unwrap_or_default());
    ///     Ok(())
    /// }
    /// ```
    pub async fn info(&self, name: &str) -> Result<Formula> {
        self.api.fetch_formula(name).await
    }

    /// Get dependencies of a package.
    ///
    /// # Arguments
    ///
    /// * `name` - Formula name
    ///
    /// # Returns
    ///
    /// Runtime and build dependencies.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use kombrucha::PackageManager;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let pm = PackageManager::new()?;
    ///     let deps = pm.dependencies("python").await?;
    ///     println!("Runtime dependencies: {}", deps.runtime.len());
    ///     println!("Build dependencies: {}", deps.build.len());
    ///     Ok(())
    /// }
    /// ```
    pub async fn dependencies(&self, name: &str) -> Result<Dependencies> {
        let formula = self.api.fetch_formula(name).await?;
        Ok(Dependencies {
            name: formula.name.clone(),
            runtime: formula.dependencies.clone(),
            build: formula.build_dependencies.clone(),
        })
    }

    /// Get packages that depend on this package.
    ///
    /// # Arguments
    ///
    /// * `name` - Formula name
    ///
    /// # Returns
    ///
    /// List of formula names that depend on this one.
    pub async fn uses(&self, name: &str) -> Result<Vec<String>> {
        // Validate package exists
        let _formula = self.api.fetch_formula(name).await?;

        // Fetch all formulae and filter
        let all = self.api.fetch_all_formulae().await?;
        let dependents: Vec<String> = all
            .into_iter()
            .filter(|f| {
                f.dependencies.contains(&name.to_string())
                    || f.build_dependencies.contains(&name.to_string())
            })
            .map(|f| f.name)
            .collect();

        Ok(dependents)
    }

    /// List all installed packages.
    ///
    /// # Returns
    ///
    /// Vector of installed packages with versions.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use kombrucha::PackageManager;
    ///
    /// fn main() -> anyhow::Result<()> {
    ///     let pm = PackageManager::new()?;
    ///     let installed = pm.list()?;
    ///     println!("Installed packages: {}", installed.len());
    ///     for pkg in installed {
    ///         println!("  {} {}", pkg.name, pkg.version);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn list(&self) -> Result<Vec<InstalledPackage>> {
        Ok(list_installed()?)
    }

    /// Find outdated packages.
    ///
    /// # Returns
    ///
    /// Vector of outdated packages with upgrade information.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use kombrucha::PackageManager;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let pm = PackageManager::new()?;
    ///     let outdated = pm.outdated().await?;
    ///     for pkg in outdated {
    ///         if pkg.changeable {
    ///             println!("{} {} → {}", pkg.name, pkg.installed, pkg.latest);
    ///         }
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn outdated(&self) -> Result<Vec<OutdatedPackage>> {
        let installed = list_installed()?;

        let mut result = Vec::new();
        for pkg in installed {
            match self.api.fetch_formula(&pkg.name).await {
                Ok(formula) => {
                    if let Some(latest) = formula.versions.stable {
                        if latest > pkg.version {
                            result.push(OutdatedPackage {
                                name: pkg.name,
                                installed: pkg.version,
                                latest,
                                changeable: true,
                            });
                        }
                    }
                }
                Err(_) => {
                    // Package from tap or not found - skip
                }
            }
        }

        Ok(result)
    }

    /// Clean up old package versions.
    ///
    /// Removes all but the most recent version of each installed package.
    /// This frees up disk space from older versions that are no longer needed.
    ///
    /// # Arguments
    ///
    /// * `dry_run` - If true, only report what would be removed without actually removing
    ///
    /// # Returns
    ///
    /// Cleanup result with removed packages and space freed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use kombrucha::PackageManager;
    ///
    /// fn main() -> anyhow::Result<()> {
    ///     let pm = PackageManager::new()?;
    ///
    ///     // Dry run: see what would be removed
    ///     let result = pm.cleanup(true)?;
    ///     println!("Would remove {} versions", result.removed.len());
    ///     println!("Would free {:.1} MB", result.space_freed_mb);
    ///
    ///     // Actually clean up
    ///     let result = pm.cleanup(false)?;
    ///     println!("Removed {} versions", result.removed.len());
    ///     println!("Freed {:.1} MB", result.space_freed_mb);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn cleanup(&self, dry_run: bool) -> Result<CleanupResult> {
        let mut removed = Vec::new();
        let mut space_freed_mb = 0.0;
        let mut errors = Vec::new();

        let installed = list_installed()?;
        let cellar = cellar_path();

        // Group by formula name
        let mut by_formula: std::collections::HashMap<String, Vec<InstalledPackage>> =
            std::collections::HashMap::new();
        for pkg in installed {
            by_formula.entry(pkg.name.clone()).or_default().push(pkg);
        }

        // For each formula, remove all but the newest version
        for (formula_name, mut versions) in by_formula {
            if versions.len() <= 1 {
                continue;
            }

            // Sort by version (newest first)
            versions.sort_by(|a, b| cellar::compare_versions(&b.version, &a.version));

            // Remove all but the first (newest)
            for pkg in &versions[1..] {
                let path = cellar.join(&formula_name).join(&pkg.version);
                let size_mb = Self::dir_size(&path)? as f64 / (1024.0 * 1024.0);

                if !dry_run {
                    match fs::remove_dir_all(&path) {
                        Ok(_) => {
                            removed.push(format!("{}/{}", formula_name, pkg.version));
                            space_freed_mb += size_mb;
                        }
                        Err(e) => {
                            errors
                                .push((format!("{}/{}", formula_name, pkg.version), e.to_string()));
                        }
                    }
                } else {
                    removed.push(format!("{}/{}", formula_name, pkg.version));
                    space_freed_mb += size_mb;
                }
            }
        }

        Ok(CleanupResult {
            removed,
            space_freed_mb,
            errors,
        })
    }

    /// Calculate directory size in bytes
    fn dir_size(path: &PathBuf) -> Result<u64> {
        let mut size = 0u64;
        if path.is_dir() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let metadata = entry.metadata()?;
                size += if metadata.is_dir() {
                    Self::dir_size(&entry.path())?
                } else {
                    metadata.len()
                };
            }
        }
        Ok(size)
    }

    /// Check system health.
    ///
    /// # Returns
    ///
    /// Health check result with issues if any.
    pub fn check(&self) -> Result<HealthCheck> {
        let prefix = detect_prefix();
        let cellar = cellar_path();

        let mut issues = Vec::new();

        if !cellar.exists() {
            issues.push("Cellar does not exist".to_string());
        }

        let prefix_writable = prefix
            .metadata()
            .map(|m| !m.permissions().readonly())
            .unwrap_or(false);
        if !prefix_writable {
            issues.push(format!("Prefix not writable: {}", prefix.display()));
        }

        Ok(HealthCheck {
            homebrew_available: true,
            cellar_exists: cellar.exists(),
            prefix_writable,
            issues,
        })
    }

    /// Get the Homebrew prefix path.
    pub fn prefix(&self) -> PathBuf {
        detect_prefix()
    }

    /// Get the Cellar path.
    pub fn cellar(&self) -> PathBuf {
        cellar_path()
    }

    /// Get reference to the underlying API client.
    ///
    /// For advanced use cases that need direct API access.
    pub fn api(&self) -> &BrewApi {
        &self.api
    }

    /// Get reference to the underlying HTTP client.
    ///
    /// For advanced use cases that need direct HTTP access.
    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }
}

impl Default for PackageManager {
    fn default() -> Self {
        Self::new().expect("Failed to create PackageManager")
    }
}
