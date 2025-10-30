//! Bottle relocation - fix install names after extraction
//!
//! Bottles contain placeholders like @@HOMEBREW_PREFIX@@ and @@HOMEBREW_CELLAR@@
//! that need to be replaced with actual paths for the binaries to work.

use anyhow::{Context, Result};
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

/// Relocate a bottle after extraction
///
/// This replaces placeholder paths in binaries, libraries, and scripts with actual paths.
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
    let mach_o_files = find_mach_o_files(cellar_path)?;

    // Process Mach-O files in parallel
    let mach_o_results: Vec<Result<()>> = mach_o_files
        .par_iter()
        .map(|file| relocate_file(file, prefix_str, cellar_str))
        .collect();

    for result in mach_o_results {
        result?;
    }

    // Find and process scripts with unreplaced shebangs (only in bin/ directories)
    let script_files = find_scripts_with_placeholders(cellar_path)?;

    // Process scripts in parallel
    let script_results: Vec<Result<()>> = script_files
        .par_iter()
        .map(|file| relocate_script_shebang(file, prefix_str, cellar_str))
        .collect();

    for result in script_results {
        result?;
    }

    Ok(())
}

/// Find all Mach-O binaries and libraries in a directory
fn find_mach_o_files(dir: &Path) -> Result<Vec<PathBuf>> {
    // Collect all file paths first without checking if they're Mach-O
    // WalkDir keeps directory handles open, so we process in batches
    let mut all_files = Vec::new();

    for entry in WalkDir::new(dir)
        .follow_links(false)
        .max_open(64) // Limit concurrent open directory handles
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() {
            all_files.push(path.to_path_buf());
        }
    }

    // Now check which ones are Mach-O files in parallel (file handles closed between checks)
    let mach_o_files: Vec<PathBuf> = all_files
        .into_par_iter()
        .filter(|path| is_mach_o(path).unwrap_or(false))
        .collect();

    Ok(mach_o_files)
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
    let result = if file.read_exact(&mut bytes).is_err() {
        false
    } else {
        // Mach-O magic numbers
        let magic = u32::from_ne_bytes(bytes);
        matches!(magic, 0xfeedface | 0xfeedfacf | 0xcefaedfe | 0xcffaedfe)
    };

    // Explicitly drop file handle before returning
    drop(file);
    Ok(result)
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

            // Use install_name_tool to change the path (suppress stderr warnings)
            let output = Command::new("install_name_tool")
                .arg("-change")
                .arg(old_path)
                .arg(&new_path)
                .arg(path)
                .output()
                .context("Failed to run install_name_tool")?;

            if !output.status.success() {
                // Only log actual errors (not warnings about code signatures)
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !stderr.contains("warning:") {
                    tracing::warn!(
                        "Failed to relocate {} in {}: {}",
                        old_path,
                        path.display(),
                        stderr
                    );
                }
            }

            // Remove code signature after modification (install_name_tool invalidates signatures)
            let _ = Command::new("codesign")
                .arg("--remove-signature")
                .arg(path)
                .output();  // Ignore errors - not all files are signed
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

        // Use install_name_tool to change the id (suppress stderr warnings)
        let output = Command::new("install_name_tool")
            .arg("-id")
            .arg(&new_id)
            .arg(path)
            .output()
            .context("Failed to run install_name_tool -id")?;

        if !output.status.success() {
            // Only log actual errors (not warnings about code signatures)
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.contains("warning:") {
                tracing::warn!("Failed to fix id for {}: {}", path.display(), stderr);
            }
        }

        // Remove code signature after modification
        let _ = Command::new("codesign")
            .arg("--remove-signature")
            .arg(path)
            .output();  // Ignore errors - not all files are signed
    }

    Ok(())
}

/// Find executable scripts in bin/ directories with unreplaced shebang placeholders
fn find_scripts_with_placeholders(dir: &Path) -> Result<Vec<PathBuf>> {
    use std::io::Read;
    use std::os::unix::fs::PermissionsExt;

    let mut candidates = Vec::new();

    // Only look in bin/ subdirectories
    for entry in WalkDir::new(dir)
        .follow_links(false)
        .max_depth(3) // Don't go too deep
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Only process files in directories named "bin"
        if let Some(parent) = path.parent() {
            if let Some(dir_name) = parent.file_name() {
                if dir_name != "bin" {
                    continue;
                }
            } else {
                continue;
            }
        } else {
            continue;
        }

        if !path.is_file() {
            continue;
        }

        // Check if file is executable
        if let Ok(metadata) = fs::metadata(path) {
            if metadata.permissions().mode() & 0o111 == 0 {
                continue; // Not executable
            }
        } else {
            continue;
        }

        candidates.push(path.to_path_buf());
    }

    // Check candidates in parallel for placeholder shebangs
    let scripts: Vec<PathBuf> = candidates
        .into_par_iter()
        .filter(|path| {
            // Skip Mach-O binaries (already handled)
            if is_mach_o(path).unwrap_or(false) {
                return false;
            }

            // Read first line to check shebang
            let mut file = match fs::File::open(path) {
                Ok(f) => f,
                Err(_) => return false,
            };

            let mut buffer = [0u8; 256];
            let bytes_read = match file.read(&mut buffer) {
                Ok(n) => n,
                Err(_) => return false,
            };

            // Check if starts with shebang and contains placeholders
            if let Ok(content) = std::str::from_utf8(&buffer[..bytes_read]) {
                if let Some(first_line) = content.lines().next() {
                    return first_line.starts_with("#!")
                        && (first_line.contains("@@HOMEBREW_PREFIX@@")
                            || first_line.contains("@@HOMEBREW_CELLAR@@"));
                }
            }

            false
        })
        .collect();

    Ok(scripts)
}

/// Replace placeholders in a script's shebang line
fn relocate_script_shebang(path: &Path, prefix: &str, cellar: &str) -> Result<()> {
    let content = fs::read_to_string(path).context("Failed to read script")?;

    let mut lines: Vec<String> = content.lines().map(String::from).collect();
    if lines.is_empty() {
        return Ok(());
    }

    // Only modify shebang line
    let shebang = &lines[0];
    if shebang.starts_with("#!")
        && (shebang.contains("@@HOMEBREW_PREFIX@@") || shebang.contains("@@HOMEBREW_CELLAR@@"))
    {
        lines[0] = shebang
            .replace("@@HOMEBREW_PREFIX@@", prefix)
            .replace("@@HOMEBREW_CELLAR@@", cellar);

        // Reconstruct with proper line endings
        let new_content = lines.join("\n");
        let final_content = if content.ends_with('\n') {
            format!("{}\n", new_content)
        } else {
            new_content
        };

        fs::write(path, final_content).context("Failed to write script")?;
    }

    Ok(())
}
