use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use std::collections::HashMap;

use crate::{cellar, symlink};

/// Remove old installed versions of formulae
#[derive(Parser)]
pub struct CleanupCommand {
    /// Formula names to clean up (all if empty)
    formula_names: Vec<String>,

    /// Show what would be removed without actually removing
    #[arg(short = 'n', long)]
    dry_run: bool,

    /// Clean up casks instead of formulae
    #[arg(long)]
    cask: bool,
}

pub fn cleanup(formula_names: &[String], dry_run: bool, cask: bool) -> Result<()> {
    if cask {
        return cleanup_cask(formula_names, dry_run);
    }

    let all_packages = cellar::list_installed()?;

    // Group packages by formula name
    let mut by_formula: HashMap<String, Vec<&cellar::InstalledPackage>> = HashMap::new();
    for pkg in &all_packages {
        by_formula.entry(pkg.name.clone()).or_default().push(pkg);
    }

    // Filter to specified formulae if provided
    let to_clean: Vec<_> = if formula_names.is_empty() {
        by_formula.keys().cloned().collect()
    } else {
        formula_names.to_vec()
    };

    let mut total_removed = 0;
    let mut total_space_freed = 0u64;

    if dry_run {
        println!("Dry run - no files will be removed");
    } else {
        println!("Cleaning up old versions...");
    }

    for formula in &to_clean {
        let versions = match by_formula.get(formula) {
            Some(v) => v,
            None => {
                if !formula_names.is_empty() {
                    println!("  {} {} not installed", "⚠".yellow(), formula.bold());
                }
                continue;
            }
        };

        if versions.len() <= 1 {
            continue;
        }

        // Determine which versions to keep:
        // 1. Always keep the linked version (active installation)
        // 2. Keep the newest version (may be same as linked)
        // This matches Homebrew's behavior of preserving the installed version

        let linked_version = symlink::get_linked_version(formula).ok().flatten();

        // Sort by version to find the newest
        let mut sorted_versions = versions.to_vec();
        sorted_versions.sort_by(|a, b| {
            // Try to parse as semantic version numbers
            let a_parts: Vec<u32> = a
                .version
                .split('.')
                .filter_map(|s| s.parse::<u32>().ok())
                .collect();
            let b_parts: Vec<u32> = b
                .version
                .split('.')
                .filter_map(|s| s.parse::<u32>().ok())
                .collect();

            // Compare version parts numerically
            for i in 0..a_parts.len().max(b_parts.len()) {
                let a_part = a_parts.get(i).unwrap_or(&0);
                let b_part = b_parts.get(i).unwrap_or(&0);
                match a_part.cmp(b_part) {
                    std::cmp::Ordering::Equal => continue,
                    other => return other,
                }
            }

            // If numeric comparison fails, fall back to lexicographic
            a.version.cmp(&b.version)
        });
        sorted_versions.reverse(); // Highest version first

        let newest_version = sorted_versions[0];

        // Collect versions to keep
        let mut versions_to_keep = vec![newest_version];
        if let Some(ref linked_ver) = linked_version
            && let Some(linked_pkg) = sorted_versions.iter().find(|v| &v.version == linked_ver)
            && linked_pkg.version != newest_version.version
        {
            versions_to_keep.push(linked_pkg);
        }

        // Everything else can be removed
        let old_versions: Vec<_> = sorted_versions
            .iter()
            .filter(|v| {
                !versions_to_keep
                    .iter()
                    .any(|keep| keep.version == v.version)
            })
            .copied()
            .collect();

        // Skip if no old versions to remove
        if old_versions.is_empty() {
            continue;
        }

        // Show which versions we're keeping
        if dry_run {
            for keep in &versions_to_keep {
                let marker = if Some(&keep.version) == linked_version.as_ref() {
                    "(linked)"
                } else {
                    "(newest)"
                };
                println!(
                    "  Keeping {} {} {}",
                    keep.name.cyan().bold(),
                    keep.version.green(),
                    marker.dimmed()
                );
            }
        }

        for old in old_versions {
            let version_path = cellar::cellar_path().join(&old.name).join(&old.version);

            // Calculate directory size
            let size = calculate_dir_size(&version_path)?;
            total_space_freed += size;

            if dry_run {
                println!(
                    "  Would remove {} {} ({})",
                    old.name.cyan(),
                    old.version.dimmed(),
                    format_size(size).dimmed()
                );
            } else {
                println!(
                    "  Removing {} {} ({})",
                    old.name.cyan(),
                    old.version.dimmed(),
                    format_size(size).dimmed()
                );

                // Unlink symlinks first
                let unlinked = symlink::unlink_formula(&old.name, &old.version)?;
                if !unlinked.is_empty() {
                    println!(
                        "    {} Unlinked {} symlinks",
                        "✓".green(),
                        unlinked.len().to_string().dimmed()
                    );
                }

                // Remove old version directory
                if version_path.exists() {
                    std::fs::remove_dir_all(&version_path)?;
                }
            }

            total_removed += 1;
        }
    }

    if total_removed == 0 {
        println!("{} No old versions to remove", "✓".green());
    } else if dry_run {
        println!(
            "{} Would remove {} old versions ({})",
            "ℹ".blue(),
            total_removed.to_string().bold(),
            format_size(total_space_freed).bold()
        );
    } else {
        println!(
            "{} Removed {} old versions, freed {}",
            "✓".green().bold(),
            total_removed.to_string().bold(),
            format_size(total_space_freed).bold()
        );
    }

    Ok(())
}

fn cleanup_cask(formula_names: &[String], dry_run: bool) -> Result<()> {
    for name in formula_names {
        match crate::commands::fallback_to_brew("cleanup", &format!("--cask {}", name)) {
            Ok(_) => println!("{} Cleaned up cask {}", "✓".green(), name.bold()),
            Err(e) => println!(
                "{} Failed to clean up cask {}: {}",
                "✗".red(),
                name.bold(),
                e
            ),
        }
    }
    Ok(())
}

fn calculate_dir_size(path: &std::path::Path) -> Result<u64> {
    let mut total = 0u64;

    if !path.exists() {
        return Ok(0);
    }

    for entry in walkdir::WalkDir::new(path).follow_links(false).max_open(64) {
        let entry = entry.map_err(|e| anyhow::anyhow!("Failed to read directory: {}", e))?;
        if entry.file_type().is_file() {
            total += entry
                .metadata()
                .map_err(|e| anyhow::anyhow!("Failed to read metadata: {}", e))?
                .len();
        }
    }

    Ok(total)
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}
