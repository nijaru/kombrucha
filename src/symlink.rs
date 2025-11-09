//! Symlink management for installed formulae

use crate::cellar;
use anyhow::{Context, Result};
use rayon::prelude::*;
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
    // Collect all files and directories that need linking
    let mut operations = Vec::new();
    collect_link_operations(source, target, cellar_root, &mut operations)?;

    // Create all target directories first (can be done in parallel)
    let dir_results: Vec<Result<()>> = operations
        .iter()
        .filter_map(|op| {
            if let LinkOperation::CreateDirectory { target_dir } = op {
                Some(create_directory_if_needed(target_dir))
            } else {
                None
            }
        })
        .collect();

    for result in dir_results {
        result?;
    }

    // Create symlinks in parallel
    let symlink_results: Vec<Result<PathBuf>> = operations
        .into_par_iter()
        .filter_map(|op| {
            if let LinkOperation::CreateSymlink {
                source_path,
                target_path,
            } = op
            {
                Some(create_symlink_operation(
                    source_path,
                    target_path,
                    cellar_root,
                ))
            } else {
                None
            }
        })
        .collect();

    // Collect results
    for result in symlink_results {
        linked_files.push(result?);
    }

    Ok(())
}

/// Types of linking operations needed
enum LinkOperation {
    CreateDirectory {
        target_dir: PathBuf,
    },
    CreateSymlink {
        source_path: PathBuf,
        target_path: PathBuf,
    },
}

/// Collect all linking operations needed (files and directories)
fn collect_link_operations(
    source: &Path,
    target: &Path,
    _cellar_root: &Path,
    operations: &mut Vec<LinkOperation>,
) -> Result<()> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let file_name = entry.file_name();
        let target_path = target.join(&file_name);

        if source_path.is_dir() {
            // Need to create target directory
            operations.push(LinkOperation::CreateDirectory {
                target_dir: target_path.clone(),
            });
            // Recursively collect operations for contents
            collect_link_operations(&source_path, &target_path, _cellar_root, operations)?;
        } else {
            // Need to create symlink
            operations.push(LinkOperation::CreateSymlink {
                source_path,
                target_path,
            });
        }
    }

    Ok(())
}

/// Create a directory if it doesn't exist
fn create_directory_if_needed(target_dir: &Path) -> Result<()> {
    if !target_dir.exists() {
        fs::create_dir_all(target_dir)
            .with_context(|| format!("Failed to create directory: {}", target_dir.display()))?;
    }
    Ok(())
}

/// Create a symlink and return the target path
fn create_symlink_operation(
    source_path: PathBuf,
    target_path: PathBuf,
    cellar_root: &Path,
) -> Result<PathBuf> {
    create_relative_symlink(&source_path, &target_path, cellar_root)?;
    Ok(target_path)
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

        // Target exists but points elsewhere
        // Match Homebrew's behavior: overwrite existing symlinks (like `brew link --overwrite`)
        // Check if it's a symlink or a regular file
        if let Ok(metadata) = target.symlink_metadata() {
            if metadata.is_symlink() {
                // It's a symlink - safe to remove and replace (likely old version)
                fs::remove_file(target).with_context(|| {
                    format!("Failed to remove existing symlink: {}", target.display())
                })?;
                // Continue to create new symlink below
            } else {
                // It's a real file - skip for safety
                eprintln!(
                    "Warning: {} exists as a file (not symlink), skipping link",
                    target.display()
                );
                return Ok(());
            }
        } else {
            // Metadata failed but target exists according to line 86 check
            // Might be a broken symlink - try to remove it
            let _ = fs::remove_file(target);
            // Continue to create new symlink below
        }
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

/// Create version-agnostic symlinks for a formula in opt/ and var/homebrew/linked/
///
/// This matches Homebrew's `optlink` behavior by creating:
/// - /opt/homebrew/opt/<formula> -> ../Cellar/<formula>/<version>
/// - /opt/homebrew/var/homebrew/linked/<formula> -> ../../../Cellar/<formula>/<version>
pub fn optlink(formula_name: &str, version: &str) -> Result<()> {
    let prefix = cellar::detect_prefix();

    // Create opt/ symlink: /opt/homebrew/opt/<formula> -> ../Cellar/<formula>/<version>
    let opt_record = prefix.join("opt").join(formula_name);

    // Ensure opt directory exists
    if let Some(opt_dir) = opt_record.parent() {
        fs::create_dir_all(opt_dir)
            .with_context(|| format!("Failed to create opt directory: {}", opt_dir.display()))?;
    }

    // Remove existing symlink if present
    if opt_record.symlink_metadata().is_ok() {
        fs::remove_file(&opt_record).with_context(|| {
            format!(
                "Failed to remove existing opt symlink: {}",
                opt_record.display()
            )
        })?;
    }

    // Calculate relative path from opt_record to formula_path
    // opt/<formula> is 1 level deep, so need 1 ".."
    let relative_path = PathBuf::from("../Cellar").join(formula_name).join(version);

    // Create the symlink
    unix_fs::symlink(&relative_path, &opt_record).with_context(|| {
        format!(
            "Failed to create opt symlink: {} -> {}",
            opt_record.display(),
            relative_path.display()
        )
    })?;

    // Create linked/ symlink: /opt/homebrew/var/homebrew/linked/<formula> -> ../../../Cellar/<formula>/<version>
    let linked_record = prefix
        .join("var")
        .join("homebrew")
        .join("linked")
        .join(formula_name);

    // Ensure linked directory exists
    if let Some(linked_dir) = linked_record.parent() {
        fs::create_dir_all(linked_dir).with_context(|| {
            format!(
                "Failed to create linked directory: {}",
                linked_dir.display()
            )
        })?;
    }

    // Remove existing symlink if present
    if linked_record.symlink_metadata().is_ok() {
        fs::remove_file(&linked_record).with_context(|| {
            format!(
                "Failed to remove existing linked symlink: {}",
                linked_record.display()
            )
        })?;
    }

    // Calculate relative path from linked_record to formula_path
    // var/homebrew/linked/<formula> is 3 levels deep, so need 3 ".."
    let relative_path = PathBuf::from("../../../Cellar")
        .join(formula_name)
        .join(version);

    // Create the symlink
    unix_fs::symlink(&relative_path, &linked_record).with_context(|| {
        format!(
            "Failed to create linked symlink: {} -> {}",
            linked_record.display(),
            relative_path.display()
        )
    })?;

    Ok(())
}

/// Remove version-agnostic symlinks for a formula
///
/// This removes:
/// - /opt/homebrew/opt/<formula>
/// - /opt/homebrew/var/homebrew/linked/<formula>
pub fn unoptlink(formula_name: &str) -> Result<()> {
    let prefix = cellar::detect_prefix();

    // Remove opt/ symlink
    let opt_record = prefix.join("opt").join(formula_name);
    if opt_record.symlink_metadata().is_ok() {
        fs::remove_file(&opt_record)
            .with_context(|| format!("Failed to remove opt symlink: {}", opt_record.display()))?;
    }

    // Remove linked/ symlink
    let linked_record = prefix
        .join("var")
        .join("homebrew")
        .join("linked")
        .join(formula_name);
    if linked_record.symlink_metadata().is_ok() {
        fs::remove_file(&linked_record).with_context(|| {
            format!(
                "Failed to remove linked symlink: {}",
                linked_record.display()
            )
        })?;
    }

    Ok(())
}

/// Get the currently linked version of a formula
///
/// Returns the version that is currently linked via /opt/homebrew/opt/<formula>
/// This matches Homebrew's linked_keg behavior and is critical for handling
/// interrupted upgrades correctly.
///
/// Returns None if the formula is not currently linked.
pub fn get_linked_version(formula_name: &str) -> Result<Option<String>> {
    let prefix = cellar::detect_prefix();
    let opt_link = prefix.join("opt").join(formula_name);

    // Check if opt symlink exists
    if opt_link.symlink_metadata().is_err() {
        return Ok(None);
    }

    // Read the symlink target
    let link_target = fs::read_link(&opt_link)
        .with_context(|| format!("Failed to read opt symlink: {}", opt_link.display()))?;

    // The symlink points to ../Cellar/<formula>/<version>
    // Extract the version from the last component of the path
    if let Some(version) = link_target.file_name()
        && let Some(version_str) = version.to_str()
    {
        return Ok(Some(version_str.to_string()));
    }

    Ok(None)
}
