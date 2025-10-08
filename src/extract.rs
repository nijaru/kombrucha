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
    // Archive contains: {formula}/{version}/*
    // Should go to: /opt/homebrew/Cellar/{formula}/{version}/*
    archive
        .unpack(&cellar)
        .with_context(|| format!("Failed to extract bottle to: {}", cellar.display()))?;

    let extracted_path = cellar.join(formula_name).join(version);

    // Verify extraction
    if !extracted_path.exists() {
        anyhow::bail!(
            "Extraction failed: expected path does not exist: {}",
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
