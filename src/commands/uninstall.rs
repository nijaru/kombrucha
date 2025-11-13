use crate::api::BrewApi;
use crate::cellar;
use crate::error::Result;
use crate::symlink;
use colored::Colorize;

pub async fn uninstall(_api: &BrewApi, formula_names: &[String], force: bool) -> Result<()> {
    println!(
        "Uninstalling {} formulae...",
        formula_names.len().to_string().bold()
    );

    // Get all installed packages to check dependencies
    let all_installed = cellar::list_installed()?;
    let mut actually_uninstalled = 0;

    for formula_name in formula_names {
        // Check if installed
        let installed_versions = cellar::get_installed_versions(formula_name)?;
        if installed_versions.is_empty() {
            println!("  {} {} not installed", "⚠".yellow(), formula_name.bold());
            continue;
        }

        // Use linked version as version to uninstall (matches Homebrew's behavior)
        // This correctly handles cases where multiple versions exist
        let version = if let Ok(Some(linked_ver)) = symlink::get_linked_version(formula_name) {
            linked_ver
        } else {
            // Not linked, use newest version
            installed_versions[0].version.clone()
        };

        // Check if other packages depend on this one (unless --force)
        if !force {
            let dependents: Vec<_> = all_installed
                .iter()
                .filter(|pkg| {
                    pkg.name != *formula_name
                        && pkg
                            .runtime_dependencies()
                            .iter()
                            .any(|dep| dep.full_name == *formula_name)
                })
                .map(|pkg| pkg.name.as_str())
                .collect();

            if !dependents.is_empty() {
                println!(
                    "  {} Cannot uninstall {} - required by: {}",
                    "⚠".yellow(),
                    formula_name.bold(),
                    dependents.join(", ").cyan()
                );
                println!("    Use {} to force uninstall", "--force".dimmed());
                continue;
            }
        }

        println!(
            "  Uninstalling {} {}",
            formula_name.cyan(),
            version.dimmed()
        );

        // Unlink symlinks
        let unlinked = symlink::unlink_formula(formula_name, &version)?;
        if !unlinked.is_empty() {
            println!(
                "    ├ {} Unlinked {} files",
                "✓".green(),
                unlinked.len().to_string().dimmed()
            );
        }

        // Remove version-agnostic symlinks (opt/ and var/homebrew/linked/)
        symlink::unoptlink(formula_name)?;

        // Remove from Cellar
        let cellar_path = cellar::cellar_path().join(formula_name).join(&version);
        if cellar_path.exists() {
            std::fs::remove_dir_all(&cellar_path)?;
        }

        // Remove formula directory if empty (or if it's a symlink)
        let formula_dir = cellar::cellar_path().join(formula_name);
        if formula_dir.exists() {
            // Check if it's a symlink first
            let metadata = std::fs::symlink_metadata(&formula_dir)?;
            if metadata.is_symlink() {
                // Remove symlink
                std::fs::remove_file(&formula_dir)?;
            } else if metadata.is_dir() && formula_dir.read_dir()?.next().is_none() {
                // Remove empty directory
                std::fs::remove_dir(&formula_dir)?;
            }
        }

        println!(
            "    └ {} Uninstalled {} {}",
            "✓".green(),
            formula_name.bold().green(),
            version.dimmed()
        );
        actually_uninstalled += 1;
    }

    if actually_uninstalled > 0 {
        println!(
            "{} Uninstalled {} package{}",
            "✓".green().bold(),
            actually_uninstalled.to_string().bold(),
            if actually_uninstalled == 1 { "" } else { "s" }
        );
    } else {
        println!("No packages were uninstalled");
    }

    Ok(())
}
