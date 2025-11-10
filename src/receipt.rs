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
    /// Read an existing INSTALL_RECEIPT.json file
    pub fn read(cellar_path: &Path) -> Result<Self> {
        let receipt_path = cellar_path.join("INSTALL_RECEIPT.json");
        let contents = fs::read_to_string(&receipt_path)
            .with_context(|| format!("Failed to read receipt: {}", receipt_path.display()))?;

        let receipt: Self =
            serde_json::from_str(&contents).context("Failed to parse INSTALL_RECEIPT.json")?;

        Ok(receipt)
    }

    /// Create a new receipt for a bottle installation
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

    /// Write receipt to INSTALL_RECEIPT.json
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
