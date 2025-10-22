//! Symlink management for installed formulae

use crate::cellar;
use anyhow::{Context, Result};
use std::fs;
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};

/// Directories to symlink from Cellar to prefix
const LINKABLE_DIRS: &[&str] = &[
    "bin",
    "sbin",
    "lib",
    "include",
    "share",
    "etc",
    "Frameworks",
];

/// Create symlinks for an installed formula
pub fn link_formula(formula_name: &str, version: &str) -> Result<Vec<PathBuf>> {
    let prefix = cellar::detect_prefix();
    let cellar_path = cellar::cellar_path();
    let formula_path = cellar_path.join(formula_name).join(version);

    let mut linked_files = Vec::new();

    // For each linkable directory
    for dir_name in LINKABLE_DIRS {
        let source_dir = formula_path.join(dir_name);

        // Skip if directory doesn't exist in formula
        if !source_dir.exists() || !source_dir.is_dir() {
            continue;
        }

        // Target directory in prefix
        let target_dir = prefix.join(dir_name);

        // Ensure target directory exists
        if !target_dir.exists() {
            fs::create_dir_all(&target_dir)
                .with_context(|| format!("Failed to create directory: {}", target_dir.display()))?;
        }

        // Link all files in this directory
        link_directory(&source_dir, &target_dir, &cellar_path, &mut linked_files)?;
    }

    Ok(linked_files)
}

/// Recursively link files from source to target
fn link_directory(
    source: &Path,
    target: &Path,
    cellar_root: &Path,
    linked_files: &mut Vec<PathBuf>,
) -> Result<()> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let file_name = entry.file_name();
        let target_path = target.join(&file_name);

        if source_path.is_dir() {
            // Create target directory if needed
            if !target_path.exists() {
                fs::create_dir_all(&target_path)?;
            }
            // Recursively link contents
            link_directory(&source_path, &target_path, cellar_root, linked_files)?;
        } else {
            // Create relative symlink
            create_relative_symlink(&source_path, &target_path, cellar_root)?;
            linked_files.push(target_path);
        }
    }

    Ok(())
}

/// Create a relative symlink from source to target
fn create_relative_symlink(source: &Path, target: &Path, cellar_root: &Path) -> Result<()> {
    // If target already exists and points to same source, skip
    if target.symlink_metadata().is_ok() {
        if let Ok(existing) = fs::read_link(target) {
            // Build expected relative path
            let expected_relative = if source.starts_with(cellar_root) {
                let mut path = PathBuf::from("..");
                if let Ok(rel) = source.strip_prefix(cellar_root.parent().unwrap_or(cellar_root)) {
                    path = path.join(rel);
                } else {
                    path = source.to_path_buf();
                }
                path
            } else {
                source.to_path_buf()
            };

            // Compare symlink targets directly without canonicalizing (avoids opening files)
            if existing == expected_relative {
                // Already linked correctly
                return Ok(());
            }
        }

        // Target exists but points elsewhere - skip for safety
        // In future, could add --force flag to overwrite
        return Ok(());
    }

    // Calculate relative path from target to source
    // Both paths should be under the prefix
    let relative_source = if source.starts_with(cellar_root) {
        // Create path like: ../Cellar/formula/version/bin/exe
        let mut path = PathBuf::from("..");

        // Add components from cellar_root to source
        if let Ok(rel) = source.strip_prefix(cellar_root.parent().unwrap_or(cellar_root)) {
            path = path.join(rel);
        } else {
            // Fallback to absolute path
            path = source.to_path_buf();
        }

        path
    } else {
        source.to_path_buf()
    };

    // Create the symlink
    unix_fs::symlink(&relative_source, target).with_context(|| {
        format!(
            "Failed to create symlink: {} -> {}",
            target.display(),
            relative_source.display()
        )
    })?;

    Ok(())
}

/// Unlink all symlinks for a formula
pub fn unlink_formula(formula_name: &str, version: &str) -> Result<Vec<PathBuf>> {
    let prefix = cellar::detect_prefix();
    let cellar_path = cellar::cellar_path();
    let formula_path = cellar_path.join(formula_name).join(version);

    let mut unlinked_files = Vec::new();

    // For each linkable directory
    for dir_name in LINKABLE_DIRS {
        let source_dir = formula_path.join(dir_name);
        let target_dir = prefix.join(dir_name);

        if !source_dir.exists() || !target_dir.exists() {
            continue;
        }

        // Remove symlinks pointing to this formula
        unlink_directory(&source_dir, &target_dir, &formula_path, &mut unlinked_files)?;
    }

    Ok(unlinked_files)
}

/// Recursively remove symlinks
fn unlink_directory(
    source: &Path,
    target: &Path,
    formula_path: &Path,
    unlinked_files: &mut Vec<PathBuf>,
) -> Result<()> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let file_name = entry.file_name();
        let target_path = target.join(&file_name);

        if source_path.is_dir() {
            if target_path.exists() && target_path.is_dir() {
                unlink_directory(&source_path, &target_path, formula_path, unlinked_files)?;
            }
        } else if target_path.symlink_metadata().is_ok() {
            // Check if this symlink points to our formula
            if let Ok(link_target) = fs::read_link(&target_path) {
                // Resolve path without canonicalizing (avoids opening files)
                let resolved = if link_target.is_relative() {
                    target_path.parent().unwrap().join(&link_target)
                } else {
                    link_target.clone()
                };

                // Check if resolved path starts with formula path (don't need to canonicalize)
                if resolved.starts_with(formula_path) {
                    // Remove symlink
                    fs::remove_file(&target_path)?;
                    unlinked_files.push(target_path);
                }
            }
        }
    }

    Ok(())
}
