use crate::api::{BrewApi, Formula};
use crate::cellar::{self, RuntimeDependency};
use crate::error::Result;
use crate::{download, extract, receipt, symlink};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use futures::StreamExt;

/// Formulae and dependency information for an installation task
pub(crate) struct InstallTargets {
    pub all_formulae: HashMap<String, Formula>,
    pub dep_order: Vec<String>,
    pub to_install: Vec<Formula>,
    pub requested_set: HashSet<String>,
}

/// Package identified for upgrade
pub(crate) struct UpgradeCandidate {
    pub name: String,
    pub old_version: String,
    pub formula: Formula,
}

/// Package from a tap that needs to be upgraded via brew
pub(crate) struct TapUpgrade {
    pub name: String,
    pub tap: String,
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
    let mut depth = 0;
    const MAX_DEP_DEPTH: usize = 50;

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
        depth += 1;
        if depth > MAX_DEP_DEPTH {
            return Err(anyhow::anyhow!(
                "Dependency chain too deep (>{} levels), possible cycle",
                MAX_DEP_DEPTH
            )
            .into());
        }

        // Filter out already processed formulae
        current_level.retain(|name| !processed.contains(name));

        if current_level.is_empty() {
            break;
        }

        // Update spinner message
        spinner.set_message(format!("Fetching {} formulae...", current_level.len()));

        // Fetch all formulae at this level in parallel with a concurrency limit
        const FETCH_CONCURRENCY: usize = 20;
        let results = futures::stream::iter(current_level.iter())
            .map(|name| async move {
                match api.fetch_formula(name).await {
                    Ok(formula) => Some(formula),
                    Err(e) => {
                        tracing::warn!("Failed to fetch dependency {}: {}", name, e);
                        None
                    }
                }
            })
            .buffer_unordered(FETCH_CONCURRENCY)
            .collect::<Vec<_>>()
            .await;

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
pub(crate) fn topological_sort(formulae: &HashMap<String, Formula>) -> anyhow::Result<Vec<String>> {
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
pub(crate) fn build_runtime_deps(
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

/// Validate installation targets, handling taps and casks by delegating to brew
pub(crate) async fn validate_install_targets(
    api: &BrewApi,
    formula_names: &[String],
    force: bool,
) -> Result<Option<InstallTargets>> {
    // Separate tap formulas from core formulas
    let mut tap_formulae = Vec::new();
    let mut core_formulae = Vec::new();

    for name in formula_names {
        if crate::commands::utils::is_tap_formula(name) {
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
            match crate::commands::utils::fallback_to_brew_with_reason(
                "install",
                tap_formula,
                Some(&format!("{} (custom tap)", tap_formula.bold()))
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
        return Ok(None);
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
                        Ok(_) => Err((name.clone(), crate::error::BruError::CaskNotFound(name.clone()))),
                        Err(crate::error::BruError::CaskNotFound(_)) => {
                            // Neither formula nor cask exists
                            Err((name.clone(), crate::error::BruError::FormulaNotFound(name.clone())))
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
            match crate::commands::utils::fallback_to_brew_with_reason(
                "install",
                cask_name,
                Some(&format!("{} (cask)", cask_name.bold()))
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
        return Ok(None);
    }

    // Report any non-cask errors but continue with valid formulae
    if !errors.is_empty() && !casks.is_empty() {
        println!();
    }

    // Resolve dependencies for valid formulae only
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
        return Ok(None);
    }

    let requested_set = formula_names.iter().cloned().collect();

    Ok(Some(InstallTargets {
        all_formulae,
        dep_order,
        to_install,
        requested_set,
    }))
}

/// Download and extract bottles for all target formulae
pub(crate) async fn download_and_extract_bottles(
    to_install: &[Formula],
) -> Result<HashMap<String, PathBuf>> {
    println!("Downloading bottles...");
    let downloaded = download::download_bottles(to_install).await?;
    let download_map: HashMap<_, _> = downloaded.into_iter().collect();

    // Separate packages that need fallback (no bottles) from those with bottles
    let (with_bottles, without_bottles): (Vec<_>, Vec<_>) = to_install
        .iter()
        .partition(|f| download_map.contains_key(&f.name));

    println!("Extracting bottles...");
    let mut extracted_paths = HashMap::new();

    // Progress bar for parallel extraction/relocation
    let progress = ProgressBar::new(with_bottles.len() as u64);
    progress.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.cyan} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );
    progress.set_message("Extracting and relocating bottles...");

    let completed = AtomicUsize::new(0);

    // PARALLEL PHASE: Extract and relocate bottles
    let extraction_results: Vec<std::result::Result<(String, PathBuf), String>> = with_bottles
        .par_iter()
        .map(|formula| {
            let formula_name = &formula.name;
            let new_version = match formula.versions.stable.as_ref() {
                Some(v) => v,
                None => return Err(format!("{}: no stable version", formula_name)),
            };

            let bottle_path = download_map.get(formula_name).unwrap();

            // Extract bottle
            let extracted_path = match extract::extract_bottle(bottle_path, formula_name, new_version) {
                Ok(path) => path,
                Err(e) => {
                    return Err(format!("{}: failed to extract: {}", formula_name, e));
                }
            };

            // Relocate bottle
            if let Err(e) = 
                crate::relocate::relocate_bottle(&extracted_path, &crate::cellar::detect_prefix())
            {
                let _ = std::fs::remove_dir_all(&extracted_path);
                return Err(format!("{}: failed to relocate: {}", formula_name, e));
            }

            let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
            progress.set_position(done as u64);

            Ok((formula_name.clone(), extracted_path))
        })
        .collect();

    progress.finish_with_message(format!("Extracted {} bottles", with_bottles.len()));

    // Collect parallel results
    for result in extraction_results {
        match result {
            Ok((name, path)) => {
                extracted_paths.insert(name, path);
            }
            Err(e) => {
                println!("  {}", e.red());
            }
        }
    }

    // Handle packages without bottles sequentially (fallback to brew)
    if !without_bottles.is_empty() {
        println!("Installing packages without bottles via brew...");
        for formula in without_bottles {
            match crate::commands::utils::fallback_to_brew_with_reason(
                "install",
                &formula.name,
                Some(&format!(
                    "{} requires building from source (no bottle available)",
                    formula.name.bold()
                )))
            {
                Ok(_) => {
                    // Formula was installed via brew
                }
                Err(e) => {
                    println!("  {}: Failed to install: {}", formula.name.bold().red(), e);
                }
            }
        }
    }

    Ok(extracted_paths)
}

/// Perform post-installation tasks: relocation, symlinking, and generating receipts
pub(crate) fn finalize_installation(
    all_formulae: &HashMap<String, Formula>,
    dep_order: &[String],
    extracted_paths: &HashMap<String, PathBuf>,
    requested_set: &HashSet<String>,
    _ruby_handle: &magnus::Ruby,
) -> Result<usize> {
    println!("Installing packages...");
    let mut installed_count = 0;
    
    // We can't rely on extracted_paths.len() anymore because some are source builds
    let total_to_install = dep_order.iter()
        .filter(|name| {
            // Count if it's in extracted paths OR if it's a requested formula that needs source build
            extracted_paths.contains_key(*name) || (requested_set.contains(*name) && !cellar::get_installed_versions(name).map(|v| !v.is_empty()).unwrap_or(false))
        })
        .count();

    // Get list of already installed packages for skipping
    let installed = cellar::list_installed()?;
    let installed_names: HashSet<_> = installed.iter().map(|p| p.name.as_str()).collect();

    for formula_name in dep_order {
        let formula = match all_formulae.get(formula_name) {
            Some(f) => f,
            None => continue,
        };

        // If it was already installed (and not forced), skip it
        if installed_names.contains(formula.name.as_str()) {
            continue;
        }

        // Case 1: Bottle was downloaded and extracted
        if let Some(extracted_path) = extracted_paths.get(&formula.name) {
            // Determine version
            let version = formula
                .versions
                .stable
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("No stable version for {}", formula.name))?;

            installed_count += 1;
            println!(
                "  Installing {} ({} / {})",
                formula.name.cyan(),
                installed_count,
                total_to_install
            );

            // Get actual installed version (may have bottle revision suffix like 25.1.0_1)
            let actual_version = extracted_path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| {
                    anyhow::anyhow!("Invalid extracted path: {}", extracted_path.display())
                })?;

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
            let runtime_deps = build_runtime_deps(&formula.dependencies, all_formulae);
            let is_requested = requested_set.contains(formula.name.as_str());
            let receipt_data = receipt::InstallReceipt::new_bottle(formula, runtime_deps, is_requested);
            receipt_data.write(extracted_path)?;

            println!(
                "    └ Installed {} {}",
                formula.name.bold().green(),
                version.dimmed()
            );
        } else if requested_set.contains(formula_name) {
            // Case 2: No bottle, requested directly -> Native Ruby Source Build
            // For MVP, we've already tried 'brew' fallback in download_and_extract_bottles.
            // Future: Call self::ruby::load_formula and loader.execute_install(class)
        }
    }

    Ok(installed_count)
}

/// Identify which packages have updates available
pub(crate) async fn collect_upgrade_candidates(
    api: &BrewApi,
    to_upgrade: &[String],
    pinned: &[String],
) -> Result<(Vec<UpgradeCandidate>, Vec<TapUpgrade>)> {
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
    let tap_packages: Vec<TapUpgrade> = to_upgrade
        .iter()
        .filter_map(|name| {
            // Extract actual formula name (strip tap prefix if present)
            let pkg_name = crate::tap::extract_formula_name(name);

            if let Ok(versions) = cellar::get_installed_versions(&pkg_name)
                && let Some(version) = versions.first()
                && let Ok(Some((tap_name, _, _))) = crate::tap::get_package_tap_info(&version.path)
                && !pinned.contains(&pkg_name)
            {
                return Some(TapUpgrade {
                    name: pkg_name,
                    tap: tap_name,
                });
            }
            None
        })
        .collect();

    Ok((candidates, tap_packages))
}

/// Result of a parallel bottle extraction task
struct ExtractedPackage {
    name: String,
    old_version: String,
    new_version: String,
    extracted_path: std::path::PathBuf,
    formula: Formula,
}

/// Download and install updates for core packages
pub(crate) async fn perform_upgrades(
    api: &BrewApi,
    _ruby_handle: &magnus::Ruby,
    candidates: &[UpgradeCandidate],
) -> Result<usize> {
    if candidates.is_empty() {
        return Ok(0);
    }

    // Phase 2: Resolve dependencies for all candidates to build complete formula map
    let candidate_names: Vec<String> = candidates.iter().map(|c| c.name.clone()).collect();
    let (all_formulae, _) = resolve_dependencies(api, &candidate_names).await?;

    // Phase 3: Download all bottles in parallel
    println!("Downloading {} bottles...", candidates.len());
    let formulae: Vec<_> = candidates.iter().map(|c| c.formula.clone()).collect();
    let downloaded = download::download_bottles(&formulae).await?;
    let download_map: HashMap<_, _> = downloaded.into_iter().collect();

    // Phase 4: Install with parallel extraction/relocation, sequential linking
    let (with_bottles, without_bottles): (Vec<_>, Vec<_>) = candidates
        .iter()
        .partition(|c| download_map.contains_key(&c.name));

    // Progress bar for parallel extraction/relocation
    let progress = ProgressBar::new(with_bottles.len() as u64);
    progress.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.cyan} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );
    progress.set_message("Extracting and relocating bottles...");

    let completed = AtomicUsize::new(0);

    // PARALLEL PHASE: Extract and relocate bottles
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

            // Extract new version
            let extracted_path =
                match extract::extract_bottle(bottle_path, formula_name, &new_version) {
                    Ok(path) => path,
                    Err(e) => {
                        return Err(format!("{}: failed to extract: {}", formula_name, e));
                    }
                };

            // Get actual installed version
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

            // Relocate bottle
            if let Err(e) = 
                crate::relocate::relocate_bottle(&extracted_path, &crate::cellar::detect_prefix())
            {
                let _ = std::fs::remove_dir_all(&extracted_path);
                return Err(format!("{}: failed to relocate: {}", formula_name, e));
            }

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

    // SEQUENTIAL PHASE: Link and cleanup
    println!("Linking packages...");
    let mut successful_upgrades = 0;

    for result in extraction_results {
        match result {
            Ok(pkg) => {
                // Unlink old version
                if let Err(e) = symlink::unlink_formula(&pkg.name, &pkg.old_version) {
                    println!(
                        "  {}: failed to unlink old version symlink: {}",
                        pkg.name.bold().red(),
                        e
                    );
                }

                // Create symlinks
                let mut linking_failed = false;
                let linked_count = if !pkg.formula.keg_only {
                    let linked = match symlink::link_formula(&pkg.name, &pkg.new_version) {
                        Ok(l) => l,
                        Err(e) => {
                            println!("  {}: failed to link: {}", pkg.name.bold().red(), e);
                            continue;
                        }
                    };

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

                // Remove old version directory
                let old_removed = if old_path.exists() {
                    match std::fs::remove_dir_all(&old_path) {
                        Ok(_) => true,
                        Err(e) => {
                            println!(
                                "  {}: failed to remove old version: {}",
                                pkg.name.bold().red(),
                                e
                            );
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

    // Handle packages without bottles sequentially
    for candidate in &without_bottles {
        let formula_name = &candidate.name;
        let new_version = candidate.formula.versions.stable.as_ref().unwrap();

        match crate::commands::utils::fallback_to_brew("upgrade", formula_name) {
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

    Ok(successful_upgrades)
}

/// Upgrade tap packages by delegating to brew
pub(crate) async fn handle_tap_upgrades(tap_packages: &[TapUpgrade]) -> Result<usize> {
    if tap_packages.is_empty() {
        return Ok(0);
    }

    println!(
        "\nUpgrading {} tap packages via brew...",
        tap_packages.len()
    );
    let mut tap_upgrades = 0;

    for pkg in tap_packages {
        let formula_name = &pkg.name;
        let tap_name = &pkg.tap;

        // Capture old version BEFORE upgrade
        let old_version = if let Ok(Some(linked_ver)) = symlink::get_linked_version(formula_name) {
            Some(linked_ver)
        } else if let Ok(versions) = cellar::get_installed_versions(formula_name) {
            versions.first().map(|v| v.version.clone())
        } else {
            None
        };

        let full_name = format!("{}/{}", tap_name, formula_name);
        match crate::commands::utils::fallback_to_brew("upgrade", &full_name) {
            Ok(_) => {
                // Clean up the SPECIFIC old version that was replaced
                if let Some(old_ver) = old_version
                    && let Err(e) = crate::commands::utils::cleanup_specific_version(formula_name, &old_ver)
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

    Ok(tap_upgrades)
}

/// Check if other packages depend on the given formula
pub(crate) fn check_dependents(
    formula_name: &str,
    all_installed: &[cellar::InstalledPackage],
) -> Vec<String> {
    all_installed
        .iter()
        .filter(|pkg| {
            pkg.name != formula_name
                && pkg
                    .runtime_dependencies()
                    .iter()
                    .any(|dep| dep.full_name == formula_name)
        })
        .map(|pkg| pkg.name.clone())
        .collect()
}

/// Perform the actual uninstallation: unlinking and directory removal
pub(crate) fn perform_uninstall(formula_name: &str, version: &str) -> Result<()> {
    // Unlink symlinks
    let unlinked = symlink::unlink_formula(formula_name, version)?;
    if !unlinked.is_empty() {
        println!(
            "    ├ Unlinked {} files",
            unlinked.len().to_string().dimmed()
        );
    }

    // Remove version-agnostic symlinks (opt/ and var/homebrew/linked/)
    symlink::unoptlink(formula_name)?;

    // Remove from Cellar with progress indication for large packages
    let cellar_path = cellar::cellar_path().join(formula_name).join(version);
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

    Ok(())
}

/// Calculate the total size of a directory recursively
pub(crate) fn calculate_dir_size(path: &std::path::Path) -> Result<u64> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{BrewApi, Formula, Versions};

    #[tokio::test]
    async fn test_dependency_depth_limit() {
        let api = BrewApi::new().unwrap();

        // Create a chain of 51 dependencies (exceeding limit of 50)
        for i in 0..=50 {
            let name = format!("pkg_{}", i);
            let dep = if i < 50 {
                vec![format!("pkg_{}", i + 1)]
            } else {
                vec![]
            };

            let formula = Formula {
                name: name.clone(),
                full_name: name.clone(),
                desc: None,
                homepage: None,
                versions: Versions::default(),
                dependencies: dep,
                build_dependencies: vec![],
                bottle: None,
                keg_only: false,
                keg_only_reason: None,
            };
            api.inject_formula(formula).await;
        }

        let result = resolve_dependencies(&api, &["pkg_0".to_string()]).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Dependency chain too deep"));
    }
}