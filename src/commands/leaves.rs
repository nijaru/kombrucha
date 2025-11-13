use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use std::io::IsTerminal;

use crate::cellar;

/// List packages not required by others
#[derive(Parser)]
pub struct LeavesCommand {}

pub fn leaves() -> Result<()> {
    // Detect if stdout is a TTY (for brew-compatible behavior)
    let is_tty = IsTerminal::is_terminal(&std::io::stdout());

    if is_tty {
        println!("{}", "==> Leaf Packages".bold().green());
        println!("(Packages not required by other packages)");
        println!();
    }

    let all_packages = cellar::list_installed()?;

    // Deduplicate by package name - keep only most recent version of each
    let mut package_map: std::collections::HashMap<String, cellar::InstalledPackage> =
        std::collections::HashMap::new();

    for pkg in all_packages {
        package_map
            .entry(pkg.name.clone())
            .and_modify(|existing| {
                // Compare modification times - keep more recent one
                if let (Ok(existing_meta), Ok(pkg_meta)) = (
                    std::fs::metadata(&existing.path),
                    std::fs::metadata(&pkg.path),
                ) && let (Ok(existing_time), Ok(pkg_time)) =
                    (existing_meta.modified(), pkg_meta.modified())
                    && pkg_time > existing_time
                {
                    *existing = pkg.clone();
                }
            })
            .or_insert(pkg);
    }

    let unique_packages: Vec<_> = package_map.into_values().collect();

    // Build a set of all packages that are dependencies of others
    let mut required_by_others = std::collections::HashSet::new();
    for pkg in &unique_packages {
        for dep in pkg.runtime_dependencies() {
            required_by_others.insert(dep.full_name.clone());
        }
    }

    // Filter to packages that are NOT in required set
    let mut leaves: Vec<_> = unique_packages
        .iter()
        .filter(|pkg| !required_by_others.contains(&pkg.name))
        .collect();

    leaves.sort_by(|a, b| a.name.cmp(&b.name));

    if leaves.is_empty() {
        if is_tty {
            println!("No leaf packages found");
        }
    } else {
        for pkg in &leaves {
            if is_tty {
                println!("{}", pkg.name.cyan());
            } else {
                // Piped: just names, no colors (brew behavior)
                println!("{}", pkg.name);
            }
        }

        if is_tty {
            println!();
            println!(
                "{} {} leaf packages",
                "ℹ".blue(),
                leaves.len().to_string().bold()
            );
        }
    }

    Ok(())
}
