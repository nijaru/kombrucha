//! Bottle extraction and installation to Cellar.
//!
//! This module handles extracting precompiled Homebrew bottles (tar.gz archives) to the
//! Cellar directory. It:
//! - **Decompresses** GZIP-compressed tar archives
//! - **Extracts** to the correct Cellar location
//! - **Handles bottle revisions** (e.g., `1.0.0_1`, `1.0.0_2`)
//! - **Verifies** extraction succeeded
//!
//! # Architecture
//!
//! Bottles are tar.gz files containing the precompiled formula contents:
//! ```text
//! Input:  formula--1.0.0.arm64_sonoma.bottle.tar.gz
//! Extract to: /opt/homebrew/Cellar/formula/1.0.0/
//!   bin/
//!   lib/
//!   share/
//!   INSTALL_RECEIPT.json
//! ```
//!
//! # Bottle Revisions
//!
//! When Homebrew rebuilds a bottle (without changing the source version), it adds a
//! revision suffix: `1.0.0_1`, `1.0.0_2`, etc. This module handles finding and validating
//! the extracted directory regardless of revision number.
//!
//! # Examples
//!
//! ```no_run
//! use kombrucha::extract;
//!
//! fn main() -> anyhow::Result<()> {
//!     let bottle_path = "/path/to/formula--1.0.0.arm64_sonoma.bottle.tar.gz";
//!     let cellar_path = extract::extract_bottle(
//!         std::path::Path::new(bottle_path),
//!         "formula",
//!         "1.0.0"
//!     )?;
//!
//!     println!("Extracted to: {}", cellar_path.display());
//!     Ok(())
//! }
//! ```

use crate::cellar;
use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use std::fs;
use std::path::{Path, PathBuf};
use tar::Archive;

/// Extract a bottle tar.gz to the Cellar
pub fn extract_bottle(bottle_path: &Path, formula_name: &str, version: &str) -> Result<PathBuf> {
    let cellar = cellar::cellar_path();

    // Ensure Cellar exists
    if !cellar.exists() {
        fs::create_dir_all(&cellar)
            .with_context(|| format!("Failed to create Cellar directory: {}", cellar.display()))?;
    }

    // Open and decompress the bottle
    let file = fs::File::open(bottle_path)
        .with_context(|| format!("Failed to open bottle: {}", bottle_path.display()))?;
    let decompressor = GzDecoder::new(file);
    let mut archive = Archive::new(decompressor);

    // Extract to Cellar
    // Archive contains: {formula}/{version}/* or {formula}/{version}_N/*
    // Should go to: /opt/homebrew/Cellar/{formula}/{version}/*
    archive
        .unpack(&cellar)
        .with_context(|| format!("Failed to extract bottle to: {}", cellar.display()))?;

    // Find the extracted directory - it may have a bottle revision suffix (e.g., 3.13.9_1)
    let formula_dir = cellar.join(formula_name);
    let extracted_path = if formula_dir.join(version).exists() {
        // Exact version match (no bottle revision)
        formula_dir.join(version)
    } else {
        // Look for version with bottle revision suffix (version_N)
        let version_with_revision = fs::read_dir(&formula_dir)
            .with_context(|| {
                format!(
                    "Failed to read formula directory: {}",
                    formula_dir.display()
                )
            })?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name())
            .find(|name| {
                let name_str = name.to_string_lossy();
                name_str.starts_with(version)
                    && (name_str == version || name_str.starts_with(&format!("{}_", version)))
            })
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Extraction failed: no directory found matching version {} in {}",
                    version,
                    formula_dir.display()
                )
            })?;

        formula_dir.join(version_with_revision)
    };

    // Verify extraction
    if !extracted_path.exists() {
        anyhow::bail!(
            "Extraction failed: path does not exist: {}",
            extracted_path.display()
        );
    }

    Ok(extracted_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cellar_path() {
        let path = cellar::cellar_path();
        assert!(path.ends_with("Cellar"));
    }
}
