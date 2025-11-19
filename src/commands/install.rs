//! Install, upgrade, and uninstall commands for package management
//!
//! This module contains all commands related to installing, upgrading, reinstalling,
//! and uninstalling formulae, including dependency resolution and bottle management.

use crate::api::{BrewApi, Formula};
use crate::cellar::{self, RuntimeDependency};
use crate::error::Result;
use crate::{download, extract, receipt, symlink};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

fn pinned_file_path() -> std::path::PathBuf {
    cellar::detect_prefix().join("var/homebrew/pinned_formulae")
}

fn read_pinned() -> Result<Vec<String>> {
    let path = pinned_file_path();
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&path)?;
    Ok(content.lines().map(String::from).collect())
}

/// Resolve all dependencies recursively, parallelizing each level
pub(crate) async fn resolve_dependencies(
    api: &BrewApi,
    root_formulae: &[String],
) -> Result<(HashMap<String, Formula>, Vec<String>)> {
    // Typical dependency depth is 10-20, so estimate total as root_count * 10
    let estimated_capacity = root_formulae.len() * 10;
    let mut all_formulae = HashMap::with_capacity(estimated_capacity);
    let mut current_level = root_formulae.to_vec();
    let mut processed = HashSet::with_capacity(estimated_capacity);

    // Create spinner for dependency resolution (hidden in quiet mode)
    let spinner = if std::env::var("BRU_QUIET").is_ok() {
        ProgressBar::hidden()
    } else {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        pb.enable_steady_tick(Duration::from_millis(80));
        pb
    };

    // Process dependencies level by level in parallel
    while !current_level.is_empty() {
        // Filter out already processed formulae
        current_level.retain(|name| !processed.contains(name));

        if current_level.is_empty() {
            break;
        }

        // Update spinner message
        spinner.set_message(format!("Fetching {} formulae...", current_level.len()));

        // Fetch all formulae at this level in parallel
        let fetch_futures: Vec<_> = current_level
            .iter()
            .map(|name| async move { api.fetch_formula(name).await.ok() })
            .collect();

        let results = futures::future::join_all(fetch_futures).await;

        // Collect next level dependencies
        let mut next_level = Vec::new();
        for (formula, name) in results.into_iter().flatten().zip(current_level.iter()) {
            // Add dependencies to next level
            for dep in &formula.dependencies {
                if !processed.contains(dep) && !all_formulae.contains_key(dep) {
                    next_level.push(dep.clone());
                }
            }

            processed.insert(name.clone());
            all_formulae.insert(formula.name.clone(), formula);
        }

        current_level = next_level;
    }

    spinner.set_message("Building dependency graph...");

    // Build dependency order (topological sort)
    let dep_order = topological_sort(&all_formulae)?;

    spinner.finish_and_clear();

    // Only print summary if not in quiet mode
    if std::env::var("BRU_QUIET").is_err() {
        println!("{} dependencies resolved", all_formulae.len());
    }

    Ok((all_formulae, dep_order))
}

/// Topological sort for dependency order using Kahn's algorithm
fn topological_sort(formulae: &HashMap<String, Formula>) -> anyhow::Result<Vec<String>> {
    let capacity = formulae.len();
    let mut in_degree: HashMap<&str, usize> = HashMap::with_capacity(capacity);
    let mut graph: HashMap<&str, Vec<&str>> = HashMap::with_capacity(capacity);

    // Build dependency graph using borrowed strings
    for (name, formula) in formulae {
        in_degree.entry(name.as_str()).or_insert(0);
        for dep in &formula.dependencies {
            graph.entry(dep.as_str()).or_default().push(name.as_str());
            *in_degree.entry(name.as_str()).or_insert(0) += 1;
        }
    }

    // Kahn's algorithm with VecDeque for efficient queue operations
    let mut queue: VecDeque<&str> = in_degree
        .iter()
        .filter_map(|(&name, &count)| if count == 0 { Some(name) } else { None })
        .collect();
    let mut result = Vec::with_capacity(capacity);

    while let Some(node) = queue.pop_front() {
        result.push(node.to_string());

        if let Some(dependents) = graph.get(node) {
            for &dependent in dependents {
                if let Some(count) = in_degree.get_mut(dependent) {
                    *count -= 1;
                    if *count == 0 {
                        queue.push_back(dependent);
                    }
                }
            }
        }
    }

    if result.len() != formulae.len() {
        anyhow::bail!("Circular dependency detected");
    }

    Ok(result)
}

/// Build runtime dependencies list for receipt
fn build_runtime_deps(
    dep_names: &[String],
    all_formulae: &HashMap<String, Formula>,
) -> Vec<RuntimeDependency> {
    dep_names
        .iter()
        .filter_map(|name| {
            all_formulae.get(name).and_then(|f| {
                f.versions.stable.as_ref().map(|v| RuntimeDependency {
                    full_name: f.name.clone(),
                    version: v.clone(),
                    revision: 0,
                    bottle_rebuild: 0,
                    pkg_version: v.clone(),
                    declared_directly: true,
                })
            })
        })
        .collect()
}

struct UpgradeCandidate {
    name: String,
    old_version: String,
    formula: crate::api::Formula,
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
    match download::download_bottles(api, &formulae).await {
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
    formula_names: &[String],
    _only_dependencies: bool,
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

    // Separate tap formulas from core formulas
    let mut tap_formulae = Vec::new();
    let mut core_formulae = Vec::new();

    for name in formula_names {
        if super::utils::is_tap_formula(name) {
            tap_formulae.push(name.clone());
        } else {
            // Strip homebrew/core/ prefix if present
            let clean_name = name
                .strip_prefix("homebrew/core/")
                .unwrap_or(name)
                .to_string();
            core_formulae.push(clean_name);
        }
    }

    // Handle tap formulas by delegating to brew
    if !tap_formulae.is_empty() {
        for tap_formula in &tap_formulae {
            match super::utils::fallback_to_brew_with_reason(
                "install",
                tap_formula,
                Some(&format!("{} (custom tap)", tap_formula.bold())),
            ) {
                Ok(_) => {
                    println!("  {} installed successfully", tap_formula.bold().green());
                }
                Err(e) => {
                    println!("  {}: Failed to install: {}", tap_formula.bold().red(), e);
                }
            }
        }
    }

    // If only tap formulas were requested, we're done
    if core_formulae.is_empty() {
        return Ok(());
    }

    // Step 1: Validate core formulae in parallel, check for casks if formula not found
    println!("Resolving dependencies...");

    let validation_futures: Vec<_> = core_formulae
        .iter()
        .map(|name| async move {
            match api.fetch_formula(name).await {
                Ok(_) => Ok(name.clone()),
                Err(crate::error::BruError::FormulaNotFound(_)) => {
                    // Formula not found, check if it's a cask
                    match api.fetch_cask(name).await {
                        Ok(_) => Err((
                            name.clone(),
                            crate::error::BruError::CaskNotFound(name.clone()),
                        )),
                        Err(crate::error::BruError::CaskNotFound(_)) => {
                            // Neither formula nor cask exists
                            Err((
                                name.clone(),
                                crate::error::BruError::FormulaNotFound(name.clone()),
                            ))
                        }
                        Err(e) => Err((name.clone(), e)),
                    }
                }
                Err(e) => Err((name.clone(), e)),
            }
        })
        .collect();

    let validation_results = futures::future::join_all(validation_futures).await;

    let mut errors = Vec::new();
    let mut casks = Vec::new();
    let mut valid_formulae = Vec::new();

    for result in validation_results {
        match result {
            Ok(name) => valid_formulae.push(name),
            Err((name, crate::error::BruError::CaskNotFound(_))) => {
                // Mark as cask for fallback to brew
                casks.push(name);
            }
            Err((name, e)) => errors.push((name, e)),
        }
    }

    // Handle casks by delegating to brew
    if !casks.is_empty() {
        for cask_name in &casks {
            match super::utils::fallback_to_brew_with_reason(
                "install",
                cask_name,
                Some(&format!("{} (cask)", cask_name.bold())),
            ) {
                Ok(_) => {
                    println!("  {} installed successfully", cask_name.bold().green());
                }
                Err(e) => {
                    println!("  {}: Failed to install: {}", cask_name.bold().red(), e);
                }
            }
        }
    }

    // Report any other errors
    if !errors.is_empty() {
        for (name, err) in &errors {
            println!("{}: {}", name.red().bold(), err);
        }
    }

    // If no valid formulae and no casks handled, fail
    if valid_formulae.is_empty() && casks.is_empty() {
        return Err(crate::error::BruError::Other(anyhow::anyhow!(
            "All formulae failed to install"
        )));
    }

    // If only casks were requested, we're done
    if valid_formulae.is_empty() {
        return Ok(());
    }

    // Report any non-cask errors but continue with valid formulae
    if !errors.is_empty() && !casks.is_empty() {
        println!();
    }

    // Resolve dependencies for valid formulae only
    println!("Resolving dependencies...");
    let (all_formulae, dep_order) = resolve_dependencies(api, &valid_formulae).await?;

    // Filter installed packages (unless --force)
    let installed = cellar::list_installed()?;
    let installed_names: HashSet<_> = installed.iter().map(|p| p.name.as_str()).collect();

    let to_install: Vec<_> = if force {
        // With --force, install all formulae even if already installed
        all_formulae.values().cloned().collect()
    } else {
        // Normal mode: skip already installed
        all_formulae
            .values()
            .filter(|f| !installed_names.contains(f.name.as_str()))
            .cloned()
            .collect()
    };

    if to_install.is_empty() {
        // Show which packages are already installed
        let already_installed: Vec<_> = all_formulae
            .values()
            .filter(|f| installed_names.contains(f.name.as_str()))
            .map(|f| {
                // Try to get the installed version
                if let Ok(versions) = cellar::get_installed_versions(&f.name)
                    && let Some(first) = versions.first()
                {
                    return format!("{} {}", f.name, first.version.dimmed());
                }
                f.name.clone()
            })
            .collect();

        println!("Already installed:");
        for pkg in &already_installed {
            println!("  {}", pkg.cyan());
        }

        if force {
            println!("  Use {} to reinstall", "--force".dimmed());
        }
        return Ok(());
    }

    println!(
        "{} formulae to install: {}",
        to_install.len().to_string().bold(),
        to_install
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

    // Step 2: Download all bottles in parallel
    println!("Downloading bottles...");
    let downloaded = download::download_bottles(api, &to_install).await?;
    let download_map: HashMap<_, _> = downloaded.into_iter().collect();

    // Step 3: Install in dependency order
    let total_to_install = to_install.len();
    let mut installed_count = 0;
    println!("Installing packages...");
    let requested_set: HashSet<_> = formula_names.iter().map(|s| s.as_str()).collect();

    for formula_name in &dep_order {
        let formula = match all_formulae.get(formula_name.as_str()) {
            Some(f) => f,
            None => continue,
        };

        // Skip if already installed
        if installed_names.contains(formula.name.as_str()) {
            continue;
        }

        // Get downloaded bottle path
        let bottle_path = match download_map.get(&formula.name) {
            Some(path) => path,
            None => {
                // No bottle available - fall back to brew for source build
                match super::utils::fallback_to_brew_with_reason(
                    "install",
                    &formula.name,
                    Some(&format!(
                        "{} requires building from source (no bottle available)",
                        formula.name.bold()
                    )),
                ) {
                    Ok(_) => {
                        // Successfully installed via brew, continue to next package
                        continue;
                    }
                    Err(e) => {
                        println!("  {}: Failed to install: {}", formula.name.bold().red(), e);
                        continue;
                    }
                }
            }
        };

        // Determine version
        let version = formula
            .versions
            .stable
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No stable version for {}", formula.name))?;

        installed_count += 1;
        println!(
            "  Installing {} ({}/{})...",
            formula.name.cyan(),
            installed_count,
            total_to_install
        );

        // Extract bottle
        let extracted_path = extract::extract_bottle(bottle_path, &formula.name, version)?;

        // Get actual installed version (may have bottle revision suffix like 25.1.0_1)
        let actual_version = extracted_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| {
                anyhow::anyhow!("Invalid extracted path: {}", extracted_path.display())
            })?;

        // Relocate bottle (fix install names)
        crate::relocate::relocate_bottle(&extracted_path, &crate::cellar::detect_prefix())?;

        // Create symlinks (use actual_version which includes bottle revision if present)
        // Skip linking if formula is keg-only (matches Homebrew behavior)
        if !formula.keg_only {
            let linked = symlink::link_formula(&formula.name, actual_version)?;
            println!("    ├ Linked {} files", linked.len().to_string().dimmed());

            // Create version-agnostic symlinks (opt/ and var/homebrew/linked/)
            symlink::optlink(&formula.name, actual_version)?;
        } else {
            println!(
                "    ├ {} is keg-only (not linked to prefix)",
                formula.name.dimmed()
            );
        }

        // Generate install receipt
        let runtime_deps = build_runtime_deps(&formula.dependencies, &all_formulae);
        let is_requested = requested_set.contains(formula.name.as_str());
        let receipt_data = receipt::InstallReceipt::new_bottle(formula, runtime_deps, is_requested);
        receipt_data.write(&extracted_path)?;

        println!(
            "    └ Installed {} {}",
            formula.name.bold().green(),
            version.dimmed()
        );
    }

    // Summary
    let installed_count = to_install.len();
    println!(
        "Installed {} packages",
        installed_count.to_string().bold().green()
    );

    Ok(())
}

pub async fn upgrade(
    api: &BrewApi,
    names: &[String],
    cask: bool,
    dry_run: bool,
    force: bool,
) -> Result<()> {
    if cask {
        return super::cask::upgrade_cask(api, names).await;
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
        use indicatif::{ProgressBar, ProgressStyle};
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
                // Tap parsing may be incomplete for complex formulas (e.g., bash with patches)
                // but is always up-to-date. API is complete but may lag.

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
            // Bottle revisions indicate rebuilds, not version upgrades
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
                // Find the matching installed version to get its path
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

            // Compare full versions INCLUDING bottle revisions
            // A version change from 1.76.0 to 1.76.0_1 IS an upgrade
            if old_version == new_version {
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
        println!("{}", "All packages are up to date".green());
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

    // Phase 4: Install with parallel extraction/relocation, sequential linking
    // Separate packages that need fallback (no bottles) from those with bottles
    let (with_bottles, without_bottles): (Vec<_>, Vec<_>) = candidates
        .iter()
        .partition(|c| download_map.contains_key(&c.name));

    // Result type for parallel phase (extract + relocate only)
    struct ExtractedPackage {
        name: String,
        old_version: String,
        new_version: String,
        extracted_path: std::path::PathBuf,
        formula: Formula,
    }

    // Progress bar for parallel extraction/relocation (I/O and CPU bound)
    let progress = ProgressBar::new(with_bottles.len() as u64);
    progress.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.cyan} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );
    progress.set_message("Extracting and relocating bottles...");

    let completed = AtomicUsize::new(0);

    // PARALLEL PHASE: Extract and relocate bottles (safe - unique directories)
    let extraction_results: Vec<std::result::Result<ExtractedPackage, String>> = with_bottles
        .par_iter()
        .map(|candidate| {
            let formula_name = &candidate.name;
            let old_version = &candidate.old_version;
            let formula = &candidate.formula;
            let new_version = match formula.versions.stable.as_ref() {
                Some(v) => v.clone(),
                None => return Err(format!("{}: no stable version", formula_name)),
            };

            let bottle_path = download_map.get(formula_name).unwrap();

            // Extract new version (I/O bound - benefits from parallelism)
            let extracted_path =
                match extract::extract_bottle(bottle_path, formula_name, &new_version) {
                    Ok(path) => path,
                    Err(e) => return Err(format!("{}: failed to extract: {}", formula_name, e)),
                };

            // Get actual installed version (may have bottle revision suffix like 25.1.0_1)
            let actual_new_version = match extracted_path.file_name().and_then(|n| n.to_str()) {
                Some(v) => v.to_string(),
                None => {
                    return Err(format!(
                        "{}: invalid extracted path: {}",
                        formula_name,
                        extracted_path.display()
                    ));
                }
            };

            // Relocate bottle (CPU bound - benefits from parallelism)
            if let Err(e) =
                crate::relocate::relocate_bottle(&extracted_path, &crate::cellar::detect_prefix())
            {
                // Clean up extracted bottle to avoid orphans in Cellar
                let _ = std::fs::remove_dir_all(&extracted_path);
                return Err(format!("{}: failed to relocate: {}", formula_name, e));
            }

            // Update progress (Relaxed ordering is sufficient - this is only for UI updates
            // and doesn't require synchronization with other memory operations)
            let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
            progress.set_position(done as u64);

            Ok(ExtractedPackage {
                name: formula_name.clone(),
                old_version: old_version.clone(),
                new_version: actual_new_version,
                extracted_path,
                formula: formula.clone(),
            })
        })
        .collect();

    progress.finish_with_message(format!("Extracted {} bottles", with_bottles.len()));

    // SEQUENTIAL PHASE: Link and cleanup (touches shared directories - no race conditions)
    println!("Linking packages...");
    let mut successful_upgrades = 0;

    for result in extraction_results {
        match result {
            Ok(pkg) => {
                // Unlink old version (sequential - touches shared /opt/homebrew/bin/)
                if let Err(e) = symlink::unlink_formula(&pkg.name, &pkg.old_version) {
                    println!(
                        "  {}: failed to unlink old version symlink: {}",
                        pkg.name.bold().red(),
                        e
                    );
                }

                // Create symlinks (sequential - touches shared directories)
                let mut linking_failed = false;
                let linked_count = if !pkg.formula.keg_only {
                    let linked = match symlink::link_formula(&pkg.name, &pkg.new_version) {
                        Ok(l) => l,
                        Err(e) => {
                            println!("  {}: failed to link: {}", pkg.name.bold().red(), e);
                            continue;
                        }
                    };

                    // Create version-agnostic symlinks (opt/ and var/homebrew/linked/)
                    if let Err(e) = symlink::optlink(&pkg.name, &pkg.new_version) {
                        println!(
                            "  {}: failed to create opt link: {}",
                            pkg.name.bold().red(),
                            e
                        );
                        linking_failed = true;
                    }

                    linked.len()
                } else {
                    0
                };

                // Generate receipt
                let runtime_deps = build_runtime_deps(&pkg.formula.dependencies, &all_formulae);

                // Read old receipt to preserve installed_on_request status
                let old_path = cellar::cellar_path().join(&pkg.name).join(&pkg.old_version);
                let installed_on_request =
                    if let Ok(old_receipt) = receipt::InstallReceipt::read(&old_path) {
                        old_receipt.installed_on_request
                    } else {
                        true
                    };

                let receipt_data = receipt::InstallReceipt::new_bottle(
                    &pkg.formula,
                    runtime_deps,
                    installed_on_request,
                );
                let mut receipt_failed = false;
                if let Err(e) = receipt_data.write(&pkg.extracted_path) {
                    println!(
                        "  {}: failed to write receipt: {}",
                        pkg.name.bold().red(),
                        e
                    );
                    receipt_failed = true;
                }

                // Remove old version directory (always cleanup to avoid inconsistent state)
                let old_removed = if old_path.exists() {
                    match std::fs::remove_dir_all(&old_path) {
                        Ok(_) => true,
                        Err(e) => {
                            println!(
                                "  {}: failed to remove old version: {}",
                                pkg.name.bold().red(),
                                e
                            );
                            // Continue anyway - new version is installed
                            false
                        }
                    }
                } else {
                    false
                };

                // Report success
                if linked_count > 0 {
                    println!("    ├ Linked {} files", linked_count.to_string().dimmed());
                }
                if pkg.formula.keg_only {
                    println!(
                        "    ├ {} is keg-only (not linked to prefix)",
                        pkg.name.dimmed()
                    );
                }
                if old_removed {
                    println!("    ├ Removed old version {}", pkg.old_version.dimmed());
                }

                if linking_failed || receipt_failed {
                    println!(
                        "    └ Upgraded {} to {} (with warnings)",
                        pkg.name.bold().yellow(),
                        pkg.new_version.dimmed()
                    );
                } else {
                    println!(
                        "    └ Upgraded {} to {}",
                        pkg.name.bold().green(),
                        pkg.new_version.dimmed()
                    );
                    successful_upgrades += 1;
                }
            }
            Err(err) => {
                println!("  {}", err.to_string().red());
            }
        }
    }

    // Handle packages without bottles sequentially (fallback to brew)
    for candidate in &without_bottles {
        let formula_name = &candidate.name;
        let new_version = candidate.formula.versions.stable.as_ref().unwrap();

        match super::utils::fallback_to_brew("upgrade", formula_name) {
            Ok(_) => {
                println!(
                    "    └ Upgraded {} to {} (via brew)",
                    formula_name.bold().green(),
                    new_version.dimmed()
                );
                successful_upgrades += 1;
            }
            Err(e) => {
                println!("  {}: Failed to upgrade: {}", formula_name.bold().red(), e);
            }
        }
    }

    // Handle tap packages via brew
    let mut tap_upgrades = 0;
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
            match super::utils::fallback_to_brew("upgrade", &full_name) {
                Ok(_) => {
                    // Clean up the SPECIFIC old version that was replaced
                    if let Some(old_ver) = old_version
                        && let Err(e) =
                            super::utils::cleanup_specific_version(formula_name, &old_ver)
                    {
                        println!(
                            "    Warning: failed to clean up old version: {}",
                            e.to_string().yellow()
                        );
                    }
                    println!("  Upgraded {}", formula_name.bold().green());
                    tap_upgrades += 1;
                }
                Err(e) => println!("  {}: Failed to upgrade: {}", formula_name.bold().red(), e),
            }
        }
    }

    let total_upgraded = successful_upgrades + tap_upgrades;

    println!(
        "Upgraded {} packages",
        total_upgraded.to_string().bold().green()
    );

    Ok(())
}

pub async fn reinstall(api: &BrewApi, names: &[String], cask: bool) -> Result<()> {
    if cask {
        return super::cask::reinstall_cask(api, names).await;
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
    // This is critical for generating correct receipts with runtime_dependencies
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
        if let Ok(Some((tap_name, _, _))) = crate::tap::get_package_tap_info(cellar_path) {
            println!(
                "  Reinstalling {} {} (from {})",
                formula_name.cyan(),
                old_version.dimmed(),
                tap_name.dimmed()
            );
            // For tap formulae, delegate to brew to avoid the fetch failure
            let full_name = format!("{}/{}", tap_name, formula_name);
            match super::utils::fallback_to_brew("reinstall", &full_name) {
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
                // No bottle available - fall back to brew for source build
                match super::utils::fallback_to_brew("reinstall", formula_name) {
                    Ok(_) => {
                        // Successfully reinstalled via brew, continue to next package
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

            println!("    ├ Linked {} files", linked.len().to_string().dimmed());
        } else {
            println!(
                "    ├ {} is keg-only (not linked to prefix)",
                formula_name.dimmed()
            );
        }

        // Generate receipt
        // Use complete all_formulae map so runtime_dependencies are populated correctly
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
            println!("  {}: not installed", formula_name.bold().yellow());
            continue;
        }

        // Use the linked version as the version to uninstall (matches Homebrew's behavior)
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

        // Unlink symlinks
        let unlinked = symlink::unlink_formula(formula_name, &version)?;
        if !unlinked.is_empty() {
            println!(
                "    ├ Unlinked {} files",
                unlinked.len().to_string().dimmed()
            );
        }

        // Remove version-agnostic symlinks (opt/ and var/homebrew/linked/)
        symlink::unoptlink(formula_name)?;

        // Remove from Cellar with progress indication for large packages
        let cellar_path = cellar::cellar_path().join(formula_name).join(&version);
        if cellar_path.exists() {
            use indicatif::{ProgressBar, ProgressStyle};

            // Calculate size and show spinner for large deletions (> 10 MB)
            let size = calculate_dir_size(&cellar_path).unwrap_or(0);
            let show_spinner = size > 10 * 1024 * 1024;
            let spinner = if show_spinner {
                let pb = ProgressBar::new_spinner();
                pb.set_style(
                    ProgressStyle::default_spinner()
                        .template("    {spinner:.cyan} Removing files...")
                        .unwrap(),
                );
                pb.enable_steady_tick(std::time::Duration::from_millis(100));
                pb
            } else {
                ProgressBar::hidden()
            };

            std::fs::remove_dir_all(&cellar_path)?;

            if show_spinner {
                spinner.finish_and_clear();
            }
        }

        // Remove formula directory if empty (or if it's a symlink)
        let formula_dir = cellar::cellar_path().join(formula_name);
        if formula_dir.exists() {
            // Check if it's a symlink first
            let metadata = std::fs::symlink_metadata(&formula_dir)?;
            if metadata.is_symlink() {
                // Remove the symlink
                std::fs::remove_file(&formula_dir)?;
            } else if metadata.is_dir() && formula_dir.read_dir()?.next().is_none() {
                // Remove empty directory
                std::fs::remove_dir(&formula_dir)?;
            }
        }

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

/// Calculate the total size of a directory recursively
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
