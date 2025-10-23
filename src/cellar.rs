//! Homebrew Cellar management - reading installed packages

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Detect the Homebrew prefix on this system
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

/// Get the Cellar directory path
pub fn cellar_path() -> PathBuf {
    detect_prefix().join("Cellar")
}

/// Runtime dependency from install receipt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeDependency {
    pub full_name: String,
    pub version: String,
    pub revision: u32,
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
    /// Create from a Cellar version directory
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

    /// Check if this was installed on request (vs as dependency)
    #[allow(dead_code)]
    pub fn installed_on_request(&self) -> bool {
        self.receipt
            .as_ref()
            .map(|r| r.installed_on_request)
            .unwrap_or(false)
    }

    /// Get runtime dependencies
    #[allow(dead_code)]
    pub fn runtime_dependencies(&self) -> Vec<RuntimeDependency> {
        self.receipt
            .as_ref()
            .map(|r| r.runtime_dependencies.clone())
            .unwrap_or_default()
    }
}

/// Read all installed packages from the Cellar
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

/// Get all versions of a specific formula, sorted by version (newest first)
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
fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
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
