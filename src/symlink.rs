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
            // Build expected relative path using same calculation as below
            let prefix = cellar_root.parent().unwrap_or(cellar_root);
            let expected_relative = if source.starts_with(cellar_root) && target.starts_with(prefix)
            {
                let target_dir = target.parent().unwrap_or(target);
                let depth = if let Ok(rel_target) = target_dir.strip_prefix(prefix) {
                    rel_target.components().count()
                } else {
                    1
                };

                let mut path = PathBuf::new();
                for _ in 0..depth {
                    path.push("..");
                }

                if let Ok(rel_source) = source.strip_prefix(prefix) {
                    path.join(rel_source)
                } else {
                    source.to_path_buf()
                }
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
    // Count how many directories we need to go up from target to reach prefix
    let prefix = cellar_root.parent().unwrap_or(cellar_root);
    let relative_source = if source.starts_with(cellar_root) && target.starts_with(prefix) {
        // Calculate depth: how many levels down from prefix is the target?
        let target_dir = target.parent().unwrap_or(target);
        let depth = if let Ok(rel_target) = target_dir.strip_prefix(prefix) {
            rel_target.components().count()
        } else {
            1 // Fallback: assume 1 level
        };

        // Build relative path: ../../../Cellar/formula/version/...
        let mut path = PathBuf::new();
        for _ in 0..depth {
            path.push("..");
        }

        // Add path from prefix to source
        if let Ok(rel_source) = source.strip_prefix(prefix) {
            path = path.join(rel_source);
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

    // For each linkable directory, scan the target directory for symlinks pointing to this formula
    for dir_name in LINKABLE_DIRS {
        let target_dir = prefix.join(dir_name);

        if !target_dir.exists() {
            continue;
        }

        // Remove all symlinks in target directory that point to this formula
        unlink_symlinks_in_directory(&target_dir, &formula_path, &mut unlinked_files)?;
    }

    Ok(unlinked_files)
}

/// Recursively scan target directory and remove symlinks pointing to formula_path
fn unlink_symlinks_in_directory(
    target: &Path,
    formula_path: &Path,
    unlinked_files: &mut Vec<PathBuf>,
) -> Result<()> {
    // Scan all entries in target directory
    for entry in fs::read_dir(target)? {
        let entry = entry?;
        let target_path = entry.path();

        // Get metadata without following symlinks
        if let Ok(metadata) = fs::symlink_metadata(&target_path) {
            if metadata.is_symlink() {
                // Check if this symlink points to our formula
                if let Ok(link_target) = fs::read_link(&target_path) {
                    // Resolve path without canonicalizing (avoids opening files)
                    let resolved = if link_target.is_relative() {
                        target_path.parent().unwrap_or(target).join(&link_target)
                    } else {
                        link_target.clone()
                    };

                    // Normalize the path by removing .. and . components
                    let normalized = normalize_path(&resolved);

                    // Check if normalized path starts with formula path
                    if normalized.starts_with(formula_path) {
                        // Remove symlink
                        if let Err(e) = fs::remove_file(&target_path) {
                            eprintln!("Warning: Failed to remove symlink {:?}: {}", target_path, e);
                        } else {
                            unlinked_files.push(target_path);
                        }
                    }
                }
            } else if metadata.is_dir() {
                // Recurse into subdirectories
                unlink_symlinks_in_directory(&target_path, formula_path, unlinked_files)?;
            }
        }
    }

    Ok(())
}

/// Normalize a path by resolving . and .. components
pub fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::CurDir => {}
            c => components.push(c),
        }
    }
    components.iter().collect()
}
