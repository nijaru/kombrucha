//! Install, upgrade, and uninstall commands for package management
//!
//! This module contains all commands related to installing, upgrading, reinstalling,
//! and uninstalling formulae, including dependency resolution and bottle management.

use crate::api::BrewApi;
use crate::cellar;
use crate::error::Result;
use crate::{download, extract, receipt, symlink};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

use crate::commands::pkg::phases::*;

fn pinned_file_path() -> std::path::PathBuf {
    cellar::detect_prefix().join("var/homebrew/pinned_formulae")
}

pub(crate) fn read_pinned() -> Result<Vec<String>> {
    let path = pinned_file_path();
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&path)?;
    Ok(content.lines().map(String::from).collect())
}

pub async fn fetch(api: &BrewApi, formula_names: &[String]) -> Result<()> {
    let is_tty = std::io::IsTerminal::is_terminal(&std::io::stdout());

    let spinner = if is_tty {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        pb.set_message(format!("Fetching {} formulae...", formula_names.len()));
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        pb
    } else {
        ProgressBar::hidden()
    };

    // Fetch formula metadata in parallel
    let fetch_futures: Vec<_> = formula_names
        .iter()
        .map(|name| async move {
            match api.fetch_formula(name).await {
                Ok(formula) => {
                    // Check if bottle exists
                    if formula.bottle.is_none()
                        || formula
                            .bottle
                            .as_ref()
                            .and_then(|b| b.stable.as_ref())
                            .is_none()
                    {
                        println!("{}: No bottle available", name.bold().yellow());
                        return None;
                    }
                    Some(formula)
                }
                Err(e) => {
                    println!("{}: Failed to fetch formula: {}", name.bold().red(), e);
                    None
                }
            }
        })
        .collect();

    let results = futures::future::join_all(fetch_futures).await;
    let formulae: Vec<_> = results.into_iter().flatten().collect();

    spinner.finish_and_clear();

    if formulae.is_empty() {
        println!("No formulae to download");
        return Ok(());
    }

    // Download bottles in parallel
    match download::download_bottles(&formulae).await {
        Ok(results) => {
            println!(
                "Downloaded {} bottles to {}",
                results.len().to_string().bold().green(),
                download::cache_dir().display().to_string().dimmed()
            );
            for (name, path) in results {
                println!(
                    "  {} {}",
                    name.bold().green(),
                    path.display().to_string().dimmed()
                );
            }
        }
        Err(e) => {
            println!("{}: {}", "Download failed".red().bold(), e);
            return Err(e.into());
        }
    }

    Ok(())
}

pub async fn install(
    api: &BrewApi,
    _ruby_handle: &magnus::Ruby,
    formula_names: &[String],
    dry_run: bool,
    force: bool,
) -> Result<()> {
    if dry_run {
        println!("Dry run mode - no packages will be installed");
    }

    println!(
        "Installing {} formulae...",
        formula_names.len().to_string().bold()
    );

    let targets = match validate_install_targets(api, formula_names, force).await? {
        Some(t) => t,
        None => return Ok(()),
    };

    println!(
        "{} formulae to install: {}",
        targets.to_install.len().to_string().bold(),
        targets
            .to_install
            .iter()
            .map(|f| f.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
            .cyan()
    );

    // If dry-run, stop here
    if dry_run {
        println!(
            "{}",
            "Dry run complete - no packages were installed".green()
        );
        return Ok(());
    }

    // Step 2: Download and extract all bottles
    let extracted_paths = download_and_extract_bottles(&targets.to_install).await?;

    // Step 3: Finalize installation (relocation, symlinks, receipts)
    let installed_count = finalize_installation(
        &targets.all_formulae,
        &targets.dep_order,
        &extracted_paths,
        &targets.requested_set,
        _ruby_handle,
    )?;

    // Summary
    println!(
        "Installed {} packages",
        installed_count.to_string().bold().green()
    );

    Ok(())
}

pub async fn upgrade(
    api: &BrewApi,
    ruby_handle: &magnus::Ruby,
    names: &[String],
    cask: bool,
    dry_run: bool,
    force: bool,
) -> Result<()> {
    if cask {
        return crate::commands::cask::upgrade_cask(api, names).await;
    }

    if dry_run {
        println!(
            "{} Dry run mode - no packages will be upgraded",
            " ℹ".blue()
        );
    }

    let formula_names = names;

    // Determine which formulae to upgrade
    let to_upgrade = if formula_names.is_empty() {
        // Upgrade all outdated
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        spinner.set_message("Checking for outdated packages...");
        spinner.enable_steady_tick(std::time::Duration::from_millis(100));

        let all_packages = cellar::list_installed()?;

        // Deduplicate multiple versions - keep only the most recent for each formula
        let estimated_capacity = all_packages.len() / 2; // ~50% typical dedup rate
        let mut package_map: std::collections::HashMap<String, cellar::InstalledPackage> =
            std::collections::HashMap::with_capacity(estimated_capacity);

        for pkg in all_packages {
            package_map
                .entry(pkg.name.clone())
                .and_modify(|existing| {
                    // Compare modification times - keep the more recent one
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

        let packages: Vec<_> = package_map.into_values().collect();

        // Fetch all formulae in parallel for better performance
        // Handle both homebrew/core (via API) and tap formulae (via file reading)
        let fetch_futures: Vec<_> = packages
            .iter()
            .map(|pkg| async move {
                // Hybrid approach: use API for accuracy, tap for fallback
                // Try API first (complete and accurate)
                if let Ok(formula) = api.fetch_formula(&pkg.name).await {
                    if let Some(latest) = formula.versions.stable.as_ref() {
                        return Some((pkg.name.clone(), pkg.version.clone(), latest.clone()));
                    }
                }

                // API unavailable - check if this package is from a tap
                if let Ok(Some((tap_name, formula_path, installed_version))) =
                    crate::tap::get_package_tap_info(&pkg.path)
                {
                    // Read version from tap formula file
                    if let Ok(Some(latest_version)) =
                        crate::tap::parse_formula_version(&formula_path)
                    {
                        return Some((pkg.name.clone(), installed_version, latest_version));
                    }
                    // If we can't read the formula file, try to get version from tap name
                    if let Ok(Some(latest_version)) =
                        crate::tap::get_tap_formula_version(&tap_name, &pkg.name)
                    {
                        return Some((pkg.name.clone(), installed_version, latest_version));
                    }
                }

                // Fallback to local homebrew/core tap
                if let Ok(Some(latest_version)) = crate::tap::get_core_formula_version(&pkg.name) {
                    return Some((pkg.name.clone(), pkg.version.clone(), latest_version));
                }

                None
            })
            .collect();

        let results = futures::future::join_all(fetch_futures).await;

        let mut outdated = Vec::new();
        for (name, pkg_version, latest) in results.into_iter().flatten() {
            // Strip bottle revisions for comparison (e.g., "6.9.3_1" -> "6.9.3")
            let installed_base = pkg_version.split('_').next().unwrap_or(&pkg_version);
            let latest_base = latest.split('_').next().unwrap_or(&latest);

            if force || installed_base != latest_base {
                outdated.push(name);
            }
        }

        spinner.finish_and_clear();

        if outdated.is_empty() {
            println!("{}", "All packages are up to date".green());
            return Ok(());
        }

        println!(
            "Found {} outdated packages: {}",
            outdated.len().to_string().bold(),
            outdated.join(", ").cyan()
        );
        outdated
    } else {
        formula_names.to_vec()
    };

    // If dry-run, stop after showing what would be upgraded
    if dry_run {
        println!("{}", "Dry run complete - no packages were upgraded".green());
        return Ok(());
    }

    // Check for pinned formulae
    let pinned = read_pinned()?;

    // Collect upgrade candidates
    let (candidates, tap_packages) = collect_upgrade_candidates(api, &to_upgrade, &pinned).await?;

    let have_updates = !candidates.is_empty() || !tap_packages.is_empty();

    if !have_updates && !names.is_empty() {
        println!("{}", "All packages are up to date".green());
        return Ok(());
    }

    if have_updates {
        // Show what will be upgraded
        for candidate in &candidates {
            if let Some(new_version) = candidate.formula.versions.stable.as_ref() {
                println!(
                    "Upgrading {} {} -> {}",
                    candidate.name.cyan(),
                    candidate.old_version.dimmed(),
                    new_version.cyan()
                );
            } else {
                println!(
                    "Skipping {} {} (no stable version)",
                    candidate.name.cyan(),
                    candidate.old_version.dimmed()
                );
            }
        }

        // Perform core upgrades
        let successful_upgrades = perform_upgrades(api, ruby_handle, &candidates).await?;

        // Handle tap upgrades
        let tap_upgrades = handle_tap_upgrades(&tap_packages).await?;

        let total_upgraded = successful_upgrades + tap_upgrades;

        println!(
            "Upgraded {} packages",
            total_upgraded.to_string().bold().green()
        );
    }

    // If upgrading all packages, also upgrade casks
    if names.is_empty() {
        println!();
        if let Err(e) = crate::commands::cask::upgrade_cask(api, &[]).await {
            println!("Failed to upgrade casks: {}", e);
        }
    }

    Ok(())
}

pub async fn reinstall(
    api: &BrewApi,
    _ruby_handle: &magnus::Ruby,
    names: &[String],
    cask: bool,
) -> Result<()> {
    if cask {
        return crate::commands::cask::reinstall_cask(api, names).await;
    }

    let formula_names = names;
    if formula_names.is_empty() {
        println!("{}", "No formulae specified".red());
        return Ok(());
    }

    println!(
        "Reinstalling {} formulae...",
        formula_names.len().to_string().bold()
    );

    // Check for pinned formulae
    let pinned = read_pinned()?;

    // Resolve dependencies for all formulas to build complete formula map
    let (all_formulae, _) = resolve_dependencies(api, formula_names).await?;

    let mut actually_reinstalled = 0;

    // Create shared HTTP client for all downloads
    let client = reqwest::Client::new();

    for formula_name in formula_names {
        // Skip pinned packages
        if pinned.contains(formula_name) {
            println!(
                "  {}: pinned (cannot reinstall pinned formulae)",
                formula_name.bold().yellow()
            );
            continue;
        }
        // Check if installed
        let installed_versions = cellar::get_installed_versions(formula_name)?;
        if installed_versions.is_empty() {
            println!("  {}: not installed", formula_name.bold().yellow());
            continue;
        }

        // Use the linked version as the "old" version
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
        if let Ok(Some((tap_name, _, _))) = crate::tap::get_package_tap_info(cellar_path) {
            println!(
                "  Reinstalling {} {} (from {})",
                formula_name.cyan(),
                old_version.dimmed(),
                tap_name.dimmed()
            );
            // For tap formulae, delegate to brew
            let full_name = format!("{}/{}", tap_name, formula_name);
            match crate::commands::utils::fallback_to_brew("reinstall", &full_name) {
                Ok(_) => {
                    actually_reinstalled += 1;
                    println!("  Reinstalled {}", formula_name.bold().green());
                    continue;
                }
                Err(e) => {
                    println!(
                        "  {}: Failed to reinstall: {}",
                        formula_name.bold().red(),
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
        let bottle_path = match download::download_bottle(&formula, None, &client).await {
            Ok(path) => path,
            Err(_) => {
                // No bottle available - fall back to brew
                match crate::commands::utils::fallback_to_brew("reinstall", formula_name) {
                    Ok(_) => {
                        actually_reinstalled += 1;
                        continue;
                    }
                    Err(e) => {
                        println!(
                            "  {}: Failed to reinstall: {}",
                            formula_name.bold().red(),
                            e
                        );
                        continue;
                    }
                }
            }
        };

        // Install with NEW version
        let extracted_path = extract::extract_bottle(&bottle_path, formula_name, new_version)?;

        // Get actual installed version
        let actual_new_version = extracted_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| {
                anyhow::anyhow!("Invalid extracted path: {}", extracted_path.display())
            })?;

        // Relocate bottle
        crate::relocate::relocate_bottle(&extracted_path, &crate::cellar::detect_prefix())?;

        // Create symlinks
        if !formula.keg_only {
            let linked = symlink::link_formula(formula_name, actual_new_version)?;
            symlink::optlink(formula_name, actual_new_version)?;
            println!("    ├ Linked {} files", linked.len().to_string().dimmed());
        } else {
            println!(
                "    ├ {} is keg-only (not linked to prefix)",
                formula_name.dimmed()
            );
        }

        // Generate receipt
        let runtime_deps = build_runtime_deps(&formula.dependencies, &all_formulae);
        let receipt_data = receipt::InstallReceipt::new_bottle(&formula, runtime_deps, true);
        receipt_data.write(&extracted_path)?;
        println!(
            "    └ Reinstalled {} {}",
            formula_name.bold().green(),
            new_version.dimmed()
        );
        actually_reinstalled += 1;
    }

    if actually_reinstalled > 0 {
        println!(
            "Reinstalled {} package{}",
            actually_reinstalled.to_string().bold().green(),
            if actually_reinstalled == 1 { "" } else { "s" }
        );
    } else {
        println!("No packages were reinstalled");
    }

    Ok(())
}

pub async fn uninstall(
    _api: &BrewApi,
    _ruby_handle: &magnus::Ruby,
    formula_names: &[String],
    force: bool,
) -> Result<()> {
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
            // Check if it's a cask
            if crate::cask::is_cask_installed(formula_name) {
                // Delegate to cask uninstall
                if let Err(e) = crate::commands::cask::uninstall_cask(&[formula_name.to_string()]) {
                    println!(
                        "  {}: Failed to uninstall cask: {}",
                        formula_name.bold().red(),
                        e
                    );
                } else {
                    actually_uninstalled += 1;
                }
                continue;
            }

            println!("  {}: not installed", formula_name.bold().yellow());
            continue;
        }

        // Use the linked version as the version to uninstall
        let version = if let Ok(Some(linked_ver)) = symlink::get_linked_version(formula_name) {
            linked_ver
        } else {
            // Not linked, use newest version
            installed_versions[0].version.clone()
        };

        // Check if other packages depend on this one (unless --force)
        if !force {
            let dependents = check_dependents(formula_name, &all_installed);

            if !dependents.is_empty() {
                println!(
                    "  {}: Cannot uninstall - required by: {}",
                    formula_name.bold().yellow(),
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

        // Perform the uninstallation
        perform_uninstall(formula_name, &version)?;

        println!(
            "    └ Uninstalled {} {}",
            formula_name.bold().green(),
            version.dimmed()
        );
        actually_uninstalled += 1;
    }

    if actually_uninstalled > 0 {
        println!(
            "Uninstalled {} package{}",
            actually_uninstalled.to_string().bold().green(),
            if actually_uninstalled == 1 { "" } else { "s" }
        );
    } else {
        println!("No packages were uninstalled");
    }

    Ok(())
}
