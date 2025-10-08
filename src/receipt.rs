//! Install receipt generation

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
    #[serde(default)]
    pub changed_files: Vec<String>,
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
            changed_files: vec![],
            time: now,
            source_modified_time: now,
            compiler: Some("clang".to_string()),
            aliases: vec![],
            runtime_dependencies: runtime_deps,
            source: Some(SourceInfo {
                path: None,
                tap: "homebrew/core".to_string(),
                tap_git_head: None,
                spec: "stable".to_string(),
            }),
            arch: Some(std::env::consts::ARCH.to_string()),
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
            cpu_family: std::env::consts::ARCH.to_string(),
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
