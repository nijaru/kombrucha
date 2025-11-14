//! Shared utility functions for command implementations

use crate::cellar;
use crate::error::Result;
use crate::symlink;
use colored::Colorize;
use std::process::Command;

/// Check if brew is available for fallback to source builds
pub(super) fn check_brew_available() -> bool {
    Command::new("brew")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Check if a formula name is from a custom tap (format: user/tap/formula)
/// Returns true only for non-core taps (excludes homebrew/core)
pub(super) fn is_tap_formula(name: &str) -> bool {
    // Tap formulas have format: user/tap/formula (at least 2 slashes)
    if name.matches('/').count() < 2 {
        return false;
    }

    // homebrew/core/* are core formulas, not custom taps
    // homebrew/cask/* are casks, handled separately
    if name.starts_with("homebrew/core/") {
        return false;
    }

    // Everything else with tap/formula format is a custom tap
    true
}

/// Fallback to brew for packages that require source builds or custom tap formulas
pub(super) fn fallback_to_brew(command: &str, formula_name: &str) -> Result<()> {
    fallback_to_brew_with_reason(command, formula_name, None)
}

/// Fallback to brew with optional custom reason message
pub(super) fn fallback_to_brew_with_reason(
    command: &str,
    formula_name: &str,
    reason: Option<&str>,
) -> Result<()> {
    if let Some(msg) = reason {
        println!("  {}", msg);
    }

    if !check_brew_available() {
        println!(
            "  {} brew is not installed - cannot install {}",
            "✗".red(),
            formula_name.bold()
        );
        println!("  Install Homebrew to handle this formula");
        return Err(anyhow::anyhow!("brew not available").into());
    }

    println!("  Delegating to {}...", format!("brew {}", command).cyan());

    let status = Command::new("brew")
        .arg(command)
        .arg(formula_name)
        .status()?;

    if status.success() {
        println!(
            "  {} Installed {} via brew",
            "✓".green(),
            formula_name.bold()
        );
        Ok(())
    } else {
        Err(anyhow::anyhow!("brew {} failed for {}", command, formula_name).into())
    }
}

/// Clean up a specific old version of a formula after upgrade
/// This matches the native upgrade behavior
pub(super) fn cleanup_specific_version(formula_name: &str, old_version: &str) -> Result<()> {
    let old_path = cellar::cellar_path().join(formula_name).join(old_version);

    if !old_path.exists() {
        return Ok(()); // Already cleaned up or never existed
    }

    // Unlink symlinks first
    let unlinked = symlink::unlink_formula(formula_name, old_version)?;
    if !unlinked.is_empty() {
        println!(
            "    ├ {} Unlinked {} symlinks",
            "✓".green(),
            unlinked.len().to_string().dimmed()
        );
    }

    // Remove the old version directory
    std::fs::remove_dir_all(&old_path)?;
    println!(
        "    ├ {} Removed old version {}",
        "✓".green(),
        old_version.dimmed()
    );

    Ok(())
}

/// Read the list of pinned formulae from the tracking file
pub(super) fn read_pinned() -> Result<Vec<String>> {
    let path = cellar::detect_prefix().join("var/homebrew/pinned_formulae");
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&path)?;
    // Convert lines to owned strings
    Ok(content.lines().map(String::from).collect())
}
