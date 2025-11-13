use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use std::collections::HashSet;

use crate::{cellar, symlink};

/// Remove unused dependencies
#[derive(Parser)]
pub struct AutoremoveCommand {
    /// Show what would be removed without actually removing
    #[arg(short = 'n', long)]
    dry_run: bool,
}

pub fn autoremove(dry_run: bool) -> Result<()> {
    if dry_run {
        println!("Dry run - no packages will be removed");
    } else {
        println!("Removing unused dependencies...");
    }

    let all_packages = cellar::list_installed()?;

    // Build a set of all packages installed on request
    let on_request: HashSet<String> = all_packages
        .iter()
        .filter(|p| p.installed_on_request())
        .map(|p| p.name.clone())
        .collect();

    // Build a set of all dependencies required by packages installed on request
    // This uses a breadth-first traversal of the dependency graph from receipts
    let mut required = HashSet::new();
    let mut to_check: std::collections::VecDeque<String> = on_request.iter().cloned().collect();
    let mut checked = HashSet::new();

    // Traverse dependency graph using receipts only (matches Homebrew behavior)
    // NO network calls - instant operation
    while let Some(name) = to_check.pop_front() {
        if !checked.insert(name.clone()) {
            continue; // Already processed
        }

        // Find package and add its runtime dependencies from receipt
        if let Some(pkg) = all_packages.iter().find(|p| p.name == name) {
            for dep in pkg.runtime_dependencies() {
                required.insert(dep.full_name.clone());
                to_check.push_back(dep.full_name.clone());
            }
        }
    }

    // Find packages that are:
    // 1. Installed as dependency (not on request)
    // 2. Not required by any package installed on request
    let mut to_remove: Vec<_> = all_packages
        .iter()
        .filter(|pkg| !pkg.installed_on_request() && !required.contains(&pkg.name))
        .collect();

    if to_remove.is_empty() {
        println!("{} No unused dependencies to remove", "✓".green());
        return Ok(());
    }

    to_remove.sort_by(|a, b| a.name.cmp(&b.name));

    println!(
        "Found {} unused dependencies:",
        to_remove.len().to_string().bold()
    );

    for pkg in &to_remove {
        println!("  {} {}", pkg.name.cyan(), pkg.version.dimmed());
    }

    if dry_run {
        println!(
            "{} Would remove {} packages",
            "ℹ".blue(),
            to_remove.len().to_string().bold()
        );
        println!("Run without {} to remove them", "--dry-run".dimmed());
        return Ok(());
    }

    // Remove packages
    for pkg in &to_remove {
        println!(
            "  Uninstalling {} {}",
            pkg.name.cyan(),
            pkg.version.dimmed()
        );

        // Unlink symlinks
        let unlinked = symlink::unlink_formula(&pkg.name, &pkg.version)?;
        if !unlinked.is_empty() {
            println!(
                "    {} Unlinked {} files",
                "✓".green(),
                unlinked.len().to_string().dimmed()
            );
        }

        // Remove version-agnostic symlinks (opt/ and var/homebrew/linked/)
        symlink::unoptlink(&pkg.name)?;

        // Remove from Cellar
        let cellar_path = cellar::cellar_path().join(&pkg.name).join(&pkg.version);
        if cellar_path.exists() {
            std::fs::remove_dir_all(&cellar_path)?;
        }

        // Remove formula directory if empty
        let formula_dir = cellar::cellar_path().join(&pkg.name);
        if formula_dir.exists() && formula_dir.read_dir()?.next().is_none() {
            std::fs::remove_dir(&formula_dir)?;
        }

        println!("    {} Removed {}", "✓".green(), pkg.name.bold().green());
    }

    println!(
        "{} Removed {} unused packages",
        "✓".green().bold(),
        to_remove.len().to_string().bold()
    );

    Ok(())
}
