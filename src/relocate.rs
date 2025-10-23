//! Bottle relocation - fix install names after extraction
//!
//! Bottles contain placeholders like @@HOMEBREW_PREFIX@@ and @@HOMEBREW_CELLAR@@
//! that need to be replaced with actual paths for the binaries to work.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

/// Relocate a bottle after extraction
///
/// This replaces placeholder paths in binaries and libraries with actual paths.
/// Required for bottles to work correctly.
pub fn relocate_bottle(cellar_path: &Path, prefix: &Path) -> Result<()> {
    let prefix_str = prefix
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid prefix path"))?;
    // cellar_path is /opt/homebrew/Cellar/formula/version
    // We need /opt/homebrew/Cellar (two parents up)
    let cellar_str = cellar_path
        .parent() // /opt/homebrew/Cellar/formula
        .and_then(|p| p.parent()) // /opt/homebrew/Cellar
        .and_then(|p| p.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid cellar path"))?;

    // Find all Mach-O binaries and libraries
    let files = find_mach_o_files(cellar_path)?;

    for file in files {
        relocate_file(&file, prefix_str, cellar_str)?;
    }

    Ok(())
}

/// Find all Mach-O binaries and libraries in a directory
fn find_mach_o_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for entry in WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Skip non-files
        if !path.is_file() {
            continue;
        }

        // Check if it's a Mach-O file
        if is_mach_o(path)? {
            files.push(path.to_path_buf());
        }
    }

    Ok(files)
}

/// Check if a file is a Mach-O binary
fn is_mach_o(path: &Path) -> Result<bool> {
    use std::io::Read;

    // Read only first 4 bytes to check magic number (not entire file)
    let mut file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return Ok(false),
    };

    let mut bytes = [0u8; 4];
    if file.read_exact(&mut bytes).is_err() {
        return Ok(false);
    }

    // Mach-O magic numbers
    let magic = u32::from_ne_bytes(bytes);
    Ok(matches!(
        magic,
        0xfeedface | 0xfeedfacf | 0xcefaedfe | 0xcffaedfe
    ))
}

/// Relocate a single Mach-O file
fn relocate_file(path: &Path, prefix: &str, cellar: &str) -> Result<()> {
    // First, get the current install names
    let output = Command::new("otool")
        .arg("-L")
        .arg(path)
        .output()
        .context("Failed to run otool")?;

    let otool_output = String::from_utf8_lossy(&output.stdout);

    // Find and replace each placeholder
    for line in otool_output.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Parse the line: "path (compatibility version X, current version Y)"
        let parts: Vec<&str> = line.split('(').collect();
        if parts.is_empty() {
            continue;
        }

        let old_path = parts[0].trim();

        // Check if it contains placeholders
        if old_path.contains("@@HOMEBREW_PREFIX@@") || old_path.contains("@@HOMEBREW_CELLAR@@") {
            let new_path = old_path
                .replace("@@HOMEBREW_PREFIX@@", prefix)
                .replace("@@HOMEBREW_CELLAR@@", cellar);

            // Use install_name_tool to change the path
            let status = Command::new("install_name_tool")
                .arg("-change")
                .arg(old_path)
                .arg(&new_path)
                .arg(path)
                .status()
                .context("Failed to run install_name_tool")?;

            if !status.success() {
                tracing::warn!("Failed to relocate {} in {}", old_path, path.display());
            }
        }
    }

    // Also fix the id if it's a library
    if let Some(ext) = path.extension()
        && ext == "dylib"
    {
        fix_library_id(path, prefix, cellar)?;
    }

    Ok(())
}

/// Fix the library id for a dylib
fn fix_library_id(path: &Path, prefix: &str, cellar: &str) -> Result<()> {
    // Get current id
    let output = Command::new("otool")
        .arg("-D")
        .arg(path)
        .output()
        .context("Failed to run otool -D")?;

    let otool_output = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = otool_output.lines().collect();

    if lines.len() < 2 {
        return Ok(());
    }

    let old_id = lines[1].trim();

    // Check if it contains placeholders
    if old_id.contains("@@HOMEBREW_PREFIX@@") || old_id.contains("@@HOMEBREW_CELLAR@@") {
        let new_id = old_id
            .replace("@@HOMEBREW_PREFIX@@", prefix)
            .replace("@@HOMEBREW_CELLAR@@", cellar);

        // Use install_name_tool to change the id
        let status = Command::new("install_name_tool")
            .arg("-id")
            .arg(&new_id)
            .arg(path)
            .status()
            .context("Failed to run install_name_tool -id")?;

        if !status.success() {
            tracing::warn!("Failed to fix id for {}", path.display());
        }
    }

    Ok(())
}
