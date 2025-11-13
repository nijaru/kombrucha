use anyhow::Result;
use clap::Parser;
use colored::Colorize;

use crate::{
    api::BrewApi,
    cellar,
    commands::{build_runtime_deps, fallback_to_brew, read_pinned, resolve_dependencies},
    extract, receipt, relocate, symlink, tap,
};

/// Reinstall formulae from bottles
#[derive(Parser)]
pub struct ReinstallCommand {
    /// Formula/cask names to reinstall
    #[arg(required = true)]
    names: Vec<String>,

    /// Reinstall cask instead of formula
    #[arg(long)]
    cask: bool,
}

pub async fn reinstall(api: &BrewApi, names: &[String], cask: bool) -> Result<()> {
    if cask {
        return reinstall_cask(api, names).await;
    }

    let formula_names = names;
    if formula_names.is_empty() {
        println!("{} No formulae specified", "✗".red());
        return Ok(());
    }

    println!(
        "Reinstalling {} formulae...",
        formula_names.len().to_string().bold()
    );

    // Check for pinned formulae
    let pinned = read_pinned()?;

    // Resolve dependencies for all formulas to build complete formula map
    // This is critical for generating correct receipts with runtime_dependencies
    let (all_formulae, _) = resolve_dependencies(api, formula_names).await?;

    let mut actually_reinstalled = 0;

    // Create shared HTTP client for all downloads
    let client = reqwest::Client::new();

    for formula_name in formula_names {
        // Skip pinned packages
        if pinned.contains(formula_name) {
            println!(
                "  {} {} is pinned (cannot reinstall pinned formulae)",
                "⚠".yellow(),
                formula_name.bold()
            );
            continue;
        }
        // Check if installed
        let installed_versions = cellar::get_installed_versions(formula_name)?;
        if installed_versions.is_empty() {
            println!("  {} {} not installed", "⚠".yellow(), formula_name.bold());
            continue;
        }

        // Use the linked version as the "old" version (matches Homebrew's linked_keg behavior)
        // This correctly handles interrupted operations where multiple versions may exist
        let old_version = if let Ok(Some(linked_ver)) = symlink::get_linked_version(formula_name) {
            // Find the matching installed version
            let matching = installed_versions.iter().find(|v| v.version == linked_ver);
            if let Some(ver) = matching {
                ver.version.clone()
            } else {
                // Linked version doesn't exist in Cellar (broken state), use newest
                installed_versions[0].version.clone()
            }
        } else {
            // Not linked, use newest version
            installed_versions[0].version.clone()
        };

        let cellar_path = installed_versions
            .iter()
            .find(|v| v.version == old_version)
            .map(|v| &v.path)
            .unwrap_or(&installed_versions[0].path);

        // Check if this is a tap formula BEFORE uninstalling
        // Critical: We must check this before removing the package!
        if let Ok(Some((tap_name, _, _))) = tap::get_package_tap_info(cellar_path) {
            println!(
                "  Reinstalling {} {} (from {})",
                formula_name.cyan(),
                old_version.dimmed(),
                tap_name.dimmed()
            );
            // For tap formulae, delegate to brew to avoid the fetch failure
            let full_name = format!("{}/{}", tap_name, formula_name);
            match fallback_to_brew("reinstall", &full_name) {
                Ok(_) => {
                    actually_reinstalled += 1;
                    println!("  {} Reinstalled {}", "✓".green(), formula_name.bold());
                    continue;
                }
                Err(e) => {
                    println!(
                        "  {} Failed to reinstall {}: {}",
                        "✗".red(),
                        formula_name.bold(),
                        e
                    );
                    continue;
                }
            }
        }

        println!(
            "  Reinstalling {} {}",
            formula_name.cyan(),
            old_version.dimmed()
        );

        // Unlink
        symlink::unlink_formula(formula_name, &old_version)?;

        // Remove from Cellar
        let cellar_path = cellar::cellar_path().join(formula_name).join(old_version);
        if cellar_path.exists() {
            std::fs::remove_dir_all(&cellar_path)?;
        }

        // Fetch formula metadata to get NEW version
        let formula = api.fetch_formula(formula_name).await?;
        let new_version = formula
            .versions
            .stable
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No stable version for {}", formula.name))?;

        // Download bottle
        let bottle_path = match crate::download::download_bottle(&formula, None, &client).await {
            Ok(path) => path,
            Err(_) => {
                // No bottle available - fall back to brew for source build
                match fallback_to_brew("reinstall", formula_name) {
                    Ok(_) => {
                        // Successfully reinstalled via brew, continue to next package
                        actually_reinstalled += 1;
                        continue;
                    }
                    Err(e) => {
                        println!(
                            "  {} Failed to reinstall {}: {}",
                            "✗".red(),
                            formula_name.bold(),
                            e
                        );
                        continue;
                    }
                }
            }
        };

        // Install with NEW version
        let extracted_path = extract::extract_bottle(&bottle_path, formula_name, new_version)?;

        // Get actual installed version (may have bottle revision suffix like 25.1.0_1)
        let actual_new_version = extracted_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| {
                anyhow::anyhow!("Invalid extracted path: {}", extracted_path.display())
            })?;

        // Relocate bottle (fix install names)
        relocate::relocate_bottle(&extracted_path, &cellar::detect_prefix())?;

        // Create symlinks - skip if formula is keg-only (matches Homebrew behavior)
        if !formula.keg_only {
            let linked = symlink::link_formula(formula_name, actual_new_version)?;

            // Create version-agnostic symlinks (opt/ and var/homebrew/linked/)
            symlink::optlink(formula_name, actual_new_version)?;

            println!(
                "    ├ {} Linked {} files",
                "✓".green(),
                linked.len().to_string().dimmed()
            );
        } else {
            println!(
                "    ├ {} {} is keg-only (not linked to prefix)",
                "ℹ".cyan(),
                formula_name
            );
        }

        // Generate receipt
        // Use complete all_formulae map so runtime_dependencies are populated correctly
        let runtime_deps = build_runtime_deps(&formula.dependencies, &all_formulae);
        let receipt_data = receipt::InstallReceipt::new_bottle(&formula, runtime_deps, true);
        receipt_data.write(&extracted_path)?;
        println!(
            "    └ {} Reinstalled {} {}",
            "✓".green(),
            formula_name.bold().green(),
            new_version.dimmed()
        );
        actually_reinstalled += 1;
    }

    if actually_reinstalled > 0 {
        println!(
            "{} Reinstalled {} package{}",
            "✓".green().bold(),
            actually_reinstalled.to_string().bold(),
            if actually_reinstalled == 1 { "" } else { "s" }
        );
    } else {
        println!("No packages were reinstalled");
    }

    Ok(())
}

async fn reinstall_cask(_api: &BrewApi, names: &[String]) -> Result<()> {
    for name in names {
        match fallback_to_brew("reinstall", &format!("--cask {}", name)) {
            Ok(_) => println!("{} Reinstalled cask {}", "✓".green(), name.bold()),
            Err(e) => println!(
                "{} Failed to reinstall cask {}: {}",
                "✗".red(),
                name.bold(),
                e
            ),
        }
    }
    Ok(())
}
