//! Formula linking and pinning operations
//!
//! This module handles symlink management and version pinning for installed formulae:
//! - `link`: Create symlinks from Cellar to system directories (bin, lib, etc.)
//! - `unlink`: Remove symlinks for a formula
//! - `pin`: Prevent a formula from being upgraded
//! - `unpin`: Allow a formula to be upgraded again
//! - `postinstall`: Run post-install hooks (stub for Phase 5 Ruby interop)

use crate::api::BrewApi;
use crate::cellar;
use crate::error::Result;
use crate::symlink;
use colored::Colorize;
use std::path::PathBuf;

/// Returns the path to the pinned formulae tracking file
fn pinned_file_path() -> PathBuf {
    cellar::detect_prefix().join("var/homebrew/pinned_formulae")
}

/// Read the list of pinned formulae from the tracking file
fn read_pinned() -> Result<Vec<String>> {
    let path = pinned_file_path();
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&path)?;
    // Convert lines to owned strings
    Ok(content.lines().map(String::from).collect())
}

/// Write the list of pinned formulae to the tracking file
fn write_pinned(pinned: &[String]) -> Result<()> {
    let path = pinned_file_path();

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(&path, pinned.join("\n"))?;
    Ok(())
}

/// Pin formulae to prevent them from being upgraded
///
/// Pinned formulae are excluded from `bru upgrade` operations. This is useful
/// when you need to keep a specific version installed (e.g., for compatibility).
pub fn pin(formula_names: &[String]) -> Result<()> {
    if formula_names.is_empty() {
        println!("{} No formulae specified", "✗".red());
        return Ok(());
    }

    println!("Pinning formulae...");

    let mut pinned = read_pinned()?;

    for formula in formula_names {
        // Verify formula is installed before pinning
        let versions = cellar::get_installed_versions(formula)?;
        if versions.is_empty() {
            println!("  {} {} is not installed", "⚠".yellow(), formula.bold());
            continue;
        }

        if pinned.contains(formula) {
            println!("  {} is already pinned", formula.bold());
        } else {
            pinned.push(formula.clone());
            println!("  {} Pinned {}", "✓".green(), formula.bold().green());
        }
    }

    write_pinned(&pinned)?;

    Ok(())
}

/// Unpin formulae to allow them to be upgraded
///
/// Removes the pin from formulae, allowing them to be upgraded by `bru upgrade`.
pub fn unpin(formula_names: &[String]) -> Result<()> {
    if formula_names.is_empty() {
        println!("{} No formulae specified", "✗".red());
        return Ok(());
    }

    println!("Unpinning formulae...");

    let mut pinned = read_pinned()?;

    for formula in formula_names {
        // Find and remove formula from pinned list
        if let Some(pos) = pinned.iter().position(|x| x == formula) {
            pinned.remove(pos);
            println!("  {} Unpinned {}", "✓".green(), formula.bold().green());
        } else {
            println!("  {} is not pinned", formula.bold());
        }
    }

    write_pinned(&pinned)?;

    Ok(())
}

/// Link installed formulae into system directories
///
/// Creates symlinks from the Cellar installation directory to standard system
/// locations like `/usr/local/bin`. This makes formula executables and libraries
/// available in your PATH.
///
/// Keg-only formulae cannot be linked as they are designed to be isolated.
pub async fn link(api: &BrewApi, formula_names: &[String]) -> Result<()> {
    if formula_names.is_empty() {
        println!("{} No formulae specified", "✗".red());
        return Ok(());
    }

    println!("Linking formulae...");

    for formula_name in formula_names {
        // Check if formula is installed
        let versions = cellar::get_installed_versions(formula_name)?;
        if versions.is_empty() {
            println!(
                "  {} {} is not installed",
                "⚠".yellow(),
                formula_name.bold()
            );
            continue;
        }

        // Fetch formula metadata to check for keg-only status
        let formula = match api.fetch_formula(formula_name).await {
            Ok(f) => f,
            Err(_) => {
                println!(
                    "  {} Failed to fetch metadata for {}",
                    "✗".red(),
                    formula_name.bold()
                );
                continue;
            }
        };

        // Keg-only formulae are not linkable by design
        // They are typically system-conflicting packages (e.g., openssl, curl)
        if formula.keg_only {
            println!(
                "  {} {} is keg-only and cannot be linked",
                "⚠".yellow(),
                formula_name.bold()
            );
            if let Some(reason) = &formula.keg_only_reason {
                println!("    {} {}", "ℹ".cyan(), reason.explanation);
            }
            continue;
        }

        // Link the most recent installed version
        let version = &versions[0].version;
        println!("  Linking {} {}", formula_name.cyan(), version.dimmed());

        let linked = symlink::link_formula(formula_name, version)?;

        // Create version-agnostic symlinks (opt/ and var/homebrew/linked/)
        // These allow formulae to reference each other without version dependencies
        symlink::optlink(formula_name, version)?;

        println!(
            "    {} Linked {} files",
            "✓".green(),
            linked.len().to_string().dimmed()
        );
    }

    Ok(())
}

/// Unlink formulae from system directories
///
/// Removes symlinks created by `link`, making the formula's executables and
/// libraries no longer available in standard locations. The formula remains
/// installed in the Cellar.
pub fn unlink(formula_names: &[String]) -> Result<()> {
    if formula_names.is_empty() {
        println!("{} No formulae specified", "✗".red());
        return Ok(());
    }

    println!("Unlinking formulae...");

    for formula_name in formula_names {
        // Verify formula is installed
        let versions = cellar::get_installed_versions(formula_name)?;
        if versions.is_empty() {
            println!(
                "  {} {} is not installed",
                "⚠".yellow(),
                formula_name.bold()
            );
            continue;
        }

        // Determine which version is currently linked
        // This is important when multiple versions are installed
        let version = if let Ok(Some(linked_ver)) = symlink::get_linked_version(formula_name) {
            linked_ver
        } else {
            println!("  {} {} is not linked", "⚠".yellow(), formula_name.bold());
            continue;
        };

        println!("  Unlinking {} {}", formula_name.cyan(), version.dimmed());

        let unlinked = symlink::unlink_formula(formula_name, &version)?;

        // Remove version-agnostic symlinks (opt/ and var/homebrew/linked/)
        symlink::unoptlink(formula_name)?;

        println!(
            "    {} Unlinked {} files",
            "✓".green(),
            unlinked.len().to_string().dimmed()
        );
    }

    Ok(())
}

/// Run post-install hooks for formulae
///
/// This command would execute the `postinstall` block from a formula's Ruby DSL.
/// Examples include setting up databases, creating config files, or registering
/// services.
///
/// **Note**: Currently a stub. Full implementation requires Phase 5 (Ruby interop
/// via `magnus` crate) to execute formula Ruby code.
pub fn postinstall(formula_names: &[String]) -> anyhow::Result<()> {
    if formula_names.is_empty() {
        println!("{} No formulae specified", "✗".red());
        return Ok(());
    }

    println!(
        "Running post-install for {} formulae...",
        formula_names.len().to_string().bold()
    );
    println!();

    for formula_name in formula_names {
        println!("{}", formula_name.cyan());

        // Verify formula is installed
        let versions = cellar::get_installed_versions(formula_name)?;
        if versions.is_empty() {
            println!("  {} Not installed", "⚠".yellow());
            continue;
        }

        // Post-install blocks are defined in formula .rb files
        // Requires Ruby interpreter to execute the DSL
        println!("  Post-install not yet implemented");
        println!("  Requires Phase 5 (Ruby interop) to execute formula post-install blocks");
    }

    Ok(())
}
