use crate::api::BrewApi;
use crate::cellar;
use crate::commands::{
    UpgradeCandidate, build_runtime_deps, cleanup_specific_version, fallback_to_brew, read_pinned,
    resolve_dependencies,
};
use crate::download;
use crate::error::Result;
use crate::extract;
use crate::receipt;
use crate::symlink;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::time::Duration;

pub async fn upgrade(
    api: &BrewApi,
    names: &[String],
    cask: bool,
    dry_run: bool,
    force: bool,
) -> Result<()> {
    if cask {
        return upgrade_cask(api, names).await;
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
        spinner.enable_steady_tick(Duration::from_millis(100));

        let all_packages = cellar::list_installed()?;

        // Deduplicate multiple versions - keep only most recent for each formula
        let estimated_capacity = all_packages.len() / 2; // ~50% typical dedup rate
        let mut package_map: HashMap<String, cellar::InstalledPackage> =
            HashMap::with_capacity(estimated_capacity);

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

        let packages: Vec<_> = package_map.into_values().collect();

        // Fetch all formulae in parallel for better performance
        // Handle both homebrew/core (via API) and tap formulae (via file reading)
        let fetch_futures: Vec<_> = packages
            .iter()
            .map(|pkg| async move {
                // Check if this package is from a tap
                if let Ok(Some((tap_name, formula_path, installed_version))) =
                    crate::tap::get_package_tap_info(&pkg.path)
                {
                    // Read version from tap formula file
                    if let Ok(Some(latest_version)) =
                        crate::tap::parse_formula_version(&formula_path)
                    {
                        return Some((pkg.name.clone(), installed_version, latest_version));
                    }
                    // If we can't read formula file, try to get version from tap name
                    if let Ok(Some(latest_version)) =
                        crate::tap::get_tap_formula_version(&tap_name, &pkg.name)
                    {
                        return Some((pkg.name.clone(), installed_version, latest_version));
                    }
                    return None;
                }

                // Fallback to API for homebrew/core packages
                let formula = api.fetch_formula(&pkg.name).await.ok()?;
                let latest = formula.versions.stable.as_ref()?;
                Some((pkg.name.clone(), pkg.version.clone(), latest.clone()))
            })
            .collect();

        let results = futures::future::join_all(fetch_futures).await;

        let mut outdated = Vec::new();
        for (name, pkg_version, latest) in results.into_iter().flatten() {
            // Strip bottle revisions before comparison
            let pkg_version_stripped = strip_bottle_revision(&pkg_version);
            let latest_stripped = strip_bottle_revision(&latest);

            if force || pkg_version_stripped != latest_stripped {
                outdated.push(name);
            }
        }

        spinner.finish_and_clear();

        if outdated.is_empty() {
            println!("{} All packages are up to date", "✓".green());
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
        println!(
            "{} Dry run complete - no packages were upgraded",
            "✓".green()
        );
        return Ok(());
    }

    // Check for pinned formulae
    let pinned = read_pinned()?;

    // Phase 1: Collect all upgrade candidates in parallel
    let fetch_futures: Vec<_> = to_upgrade
        .iter()
        .filter(|name| !pinned.contains(*name))
        .map(|formula_name| async move {
            // Extract actual formula name (strip tap prefix if present)
            let pkg_name = crate::tap::extract_formula_name(formula_name);

            // Check if installed
            let installed_versions = cellar::get_installed_versions(&pkg_name).ok()?;
            if installed_versions.is_empty() {
                return None; // Will install separately
            }

            // Use the linked version as the "old" version (matches Homebrew's linked_keg behavior)
            // This correctly handles interrupted upgrades where multiple versions may exist
            let old_version = if let Ok(Some(linked_ver)) = symlink::get_linked_version(&pkg_name) {
                // Find matching installed version to get its path
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

            // Check if this is a tap formula - if so, we'll upgrade via brew
            if let Ok(Some(_tap_info)) = crate::tap::get_package_tap_info(cellar_path) {
                // For tap formulae, fall back to brew since we don't have bottles
                return None;
            }

            // Fetch latest version from API (homebrew/core only)
            let formula = api.fetch_formula(formula_name).await.ok()?;
            let new_version = formula.versions.stable.as_ref()?.clone();

            // Strip bottle revisions for comparison
            let old_version_stripped = strip_bottle_revision(&old_version);
            let new_version_stripped = strip_bottle_revision(&new_version);

            if old_version_stripped == new_version_stripped {
                return None; // Already at latest version
            }

            Some(UpgradeCandidate {
                name: formula_name.clone(),
                old_version,
                formula,
            })
        })
        .collect();

    let candidates: Vec<_> = futures::future::join_all(fetch_futures)
        .await
        .into_iter()
        .flatten()
        .collect();

    // Separate out tap packages that need to be upgraded via brew
    // Store as (formula_name, tap_name) so we can construct full tap/formula name
    let tap_packages: Vec<(String, String)> = to_upgrade
        .iter()
        .filter_map(|name| {
            // Extract actual formula name (strip tap prefix if present)
            let pkg_name = crate::tap::extract_formula_name(name);

            if let Ok(versions) = cellar::get_installed_versions(&pkg_name)
                && let Some(version) = versions.first()
                && let Ok(Some((tap_name, _, _))) = crate::tap::get_package_tap_info(&version.path)
                && !pinned.contains(&pkg_name)
            {
                return Some((pkg_name, tap_name));
            }
            None
        })
        .collect();

    if candidates.is_empty() && tap_packages.is_empty() {
        println!("{} All packages are up to date", "✓".green());
        return Ok(());
    }

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

    // Phase 2: Resolve dependencies for all candidates to build complete formula map
    // This is critical for generating correct receipts with runtime_dependencies
    let candidate_names: Vec<String> = candidates.iter().map(|c| c.name.clone()).collect();
    let (all_formulae, _) = if !candidate_names.is_empty() {
        resolve_dependencies(api, &candidate_names).await?
    } else {
        (HashMap::new(), vec![])
    };

    // Phase 3: Download all bottles in parallel
    println!("Downloading {} bottles...", candidates.len());
    let formulae: Vec<_> = candidates.iter().map(|c| c.formula.clone()).collect();
    let downloaded = download::download_bottles(api, &formulae).await?;
    let download_map: HashMap<_, _> = downloaded.into_iter().collect();

    // Phase 4: Install sequentially
    for candidate in &candidates {
        let formula_name = &candidate.name;
        let old_version = &candidate.old_version;
        let formula = &candidate.formula;
        let new_version = formula.versions.stable.as_ref().unwrap();

        // Show spinner for this package
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        spinner.set_message(format!("Upgrading {}...", formula_name.cyan()));
        spinner.enable_steady_tick(Duration::from_millis(100));

        let bottle_path = match download_map.get(formula_name) {
            Some(path) => path,
            None => {
                // No bottle available - fall back to brew for source build
                match fallback_to_brew("upgrade", formula_name) {
                    Ok(_) => continue,
                    Err(e) => {
                        println!(
                            "  {} Failed to upgrade {}: {}",
                            "✗".red(),
                            formula_name.bold(),
                            e
                        );
                        continue;
                    }
                }
            }
        };

        // Unlink old version
        symlink::unlink_formula(formula_name, old_version)?;

        // Install new version
        let extracted_path = extract::extract_bottle(bottle_path, formula_name, new_version)?;

        // Get actual installed version (may have bottle revision suffix like 25.1.0_1)
        let actual_new_version = extracted_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| {
                anyhow::anyhow!("Invalid extracted path: {}", extracted_path.display())
            })?;

        // Relocate bottle (fix install names)
        crate::relocate::relocate_bottle(&extracted_path, &crate::cellar::detect_prefix())?;

        // Create symlinks - skip if formula is keg-only (matches Homebrew behavior)
        if !formula.keg_only {
            let linked = symlink::link_formula(formula_name, actual_new_version)?;

            // Create version-agnostic symlinks (opt/ and var/homebrew/linked/)
            symlink::optlink(formula_name, actual_new_version)?;

            spinner.finish_and_clear();

            println!(
                "    ├ {} Linked {} files",
                "✓".green(),
                linked.len().to_string().dimmed()
            );
        } else {
            spinner.finish_and_clear();

            println!(
                "    ├ {} {} is keg-only (not linked to prefix)",
                "ℹ".cyan(),
                formula_name
            );
        }

        // Generate receipt - preserve original installed_on_request status
        // Use complete all_formulae map so runtime_dependencies are populated correctly
        let runtime_deps = build_runtime_deps(&formula.dependencies, &all_formulae);

        // Read old receipt to preserve installed_on_request status
        let old_path = cellar::cellar_path().join(formula_name).join(old_version);
        let installed_on_request = if let Ok(old_receipt) = receipt::InstallReceipt::read(&old_path)
        {
            old_receipt.installed_on_request
        } else {
            // Fallback: assume it was installed on request if we can't read old receipt
            true
        };

        let receipt_data =
            receipt::InstallReceipt::new_bottle(formula, runtime_deps, installed_on_request);
        receipt_data.write(&extracted_path)?;

        // Remove old version
        let old_path = cellar::cellar_path().join(formula_name).join(old_version);
        if old_path.exists() {
            // Unlink symlinks first
            let unlinked = symlink::unlink_formula(formula_name, old_version)?;
            if !unlinked.is_empty() {
                println!(
                    "    ├ {} Unlinked {} symlinks",
                    "✓".green(),
                    unlinked.len().to_string().dimmed()
                );
            }

            // Remove old version directory
            std::fs::remove_dir_all(&old_path)?;
            println!(
                "    ├ {} Removed old version {}",
                "✓".green(),
                old_version.dimmed()
            );
        }

        println!(
            "    └ {} Upgraded {} to {}",
            "✓".green(),
            formula_name.bold().green(),
            new_version.dimmed()
        );
    }

    // Handle tap packages via brew
    if !tap_packages.is_empty() {
        println!(
            "\nUpgrading {} tap packages via brew...",
            tap_packages.len()
        );
        for (formula_name, tap_name) in &tap_packages {
            // Capture old version BEFORE upgrade (matches native upgrade behavior at line 1863)
            let old_version =
                if let Ok(Some(linked_ver)) = symlink::get_linked_version(formula_name) {
                    Some(linked_ver)
                } else if let Ok(versions) = cellar::get_installed_versions(formula_name) {
                    versions.first().map(|v| v.version.clone())
                } else {
                    None
                };

            let full_name = format!("{}/{}", tap_name, formula_name);
            match fallback_to_brew("upgrade", &full_name) {
                Ok(_) => {
                    // Clean up SPECIFIC old version that was replaced
                    if let Some(old_ver) = old_version {
                        if let Err(e) = cleanup_specific_version(formula_name, &old_ver) {
                            println!(
                                "    {} Warning: failed to clean up old version: {}",
                                "⚠".yellow(),
                                e
                            );
                        }
                    }
                    println!("  {} Upgraded {}", "✓".green(), formula_name.bold());
                }
                Err(e) => println!(
                    "  {} Failed to upgrade {}: {}",
                    "✗".red(),
                    formula_name.bold(),
                    e
                ),
            }
        }
    }

    let total_upgraded = candidates.len() + tap_packages.len();

    println!(
        "{} Upgraded {} packages",
        "✓".green().bold(),
        total_upgraded.to_string().bold()
    );

    Ok(())
}

/// Strip bottle revision from version string (e.g., "1.4.0_32" → "1.4.0")
fn strip_bottle_revision(version: &str) -> &str {
    if let Some(pos) = version.rfind('_') {
        // Check if everything after _ is digits (bottle revision)
        if version[pos + 1..].chars().all(|c| c.is_ascii_digit()) {
            return &version[..pos];
        }
    }
    version
}

// Forward declaration for upgrade_cask - this is implemented in commands_old.rs
async fn upgrade_cask(api: &BrewApi, names: &[String]) -> Result<()> {
    // Delegate to the old implementation for now
    crate::commands::upgrade_cask(api, names).await
}
