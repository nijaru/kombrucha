//! Bottle extraction to Cellar

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
