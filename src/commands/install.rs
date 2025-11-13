use crate::api::BrewApi;
use crate::cellar;
use crate::commands::{
    build_runtime_deps, fallback_to_brew_with_reason, is_tap_formula, resolve_dependencies,
};
use crate::download;
use crate::error::Result;
use crate::extract;
use crate::receipt;
use crate::symlink;
use colored::Colorize;
use std::collections::{HashMap, HashSet};

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
        if is_tap_formula(name) {
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
    let mut tap_failures = Vec::new();
    if !tap_formulae.is_empty() {
        for tap_formula in &tap_formulae {
            match fallback_to_brew_with_reason(
                "install",
                tap_formula,
                Some(&format!("{} (custom tap)", tap_formula.bold())),
            ) {
                Ok(_) => {
                    println!(
                        "  {} {} installed successfully",
                        "✓".green(),
                        tap_formula.bold()
                    );
                }
                Err(e) => {
                    println!(
                        "  {} Failed to install {}: {}",
                        "✗".red(),
                        tap_formula.bold(),
                        e
                    );
                    tap_failures.push(tap_formula.clone());
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
    let mut cask_failures = Vec::new();
    if !casks.is_empty() {
        for cask_name in &casks {
            match fallback_to_brew_with_reason(
                "install",
                cask_name,
                Some(&format!("{} (cask)", cask_name.bold())),
            ) {
                Ok(_) => {
                    println!(
                        "  {} {} installed successfully",
                        "✓".green(),
                        cask_name.bold()
                    );
                }
                Err(e) => {
                    println!(
                        "  {} Failed to install {}: {}",
                        "✗".red(),
                        cask_name.bold(),
                        e
                    );
                    cask_failures.push(cask_name.clone());
                }
            }
        }
    }

    // Report any other errors
    if !errors.is_empty() {
        for (name, err) in &errors {
            println!("{} {}: {}", "✗".red(), name, err);
        }
    }

    // If no valid formulae and no casks handled, fail
    if valid_formulae.is_empty() && casks.is_empty() {
        return Err(crate::error::BruError::Other(anyhow::anyhow!(
            "All formulae failed to install"
        )));
    }

    // If only casks were requested, check for failures
    if valid_formulae.is_empty() {
        if !cask_failures.is_empty() || !tap_failures.is_empty() {
            return Err(crate::error::BruError::Other(anyhow::anyhow!(
                "Some packages failed to install"
            )));
        }
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
            "{} Dry run complete - no packages were installed",
            "✓".green()
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
                match fallback_to_brew_with_reason(
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
                        println!(
                            "  {} Failed to install {}: {}",
                            "✗".red(),
                            formula.name.bold(),
                            e
                        );
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
            println!(
                "    ├ {} Linked {} files",
                "✓".green(),
                linked.len().to_string().dimmed()
            );

            // Create version-agnostic symlinks (opt/ and var/homebrew/linked/)
            symlink::optlink(&formula.name, actual_version)?;
        } else {
            println!(
                "    ├ {} {} is keg-only (not linked to prefix)",
                "ℹ".cyan(),
                formula.name
            );
        }

        // Generate install receipt
        let runtime_deps = build_runtime_deps(&formula.dependencies, &all_formulae);
        let is_requested = requested_set.contains(formula.name.as_str());
        let receipt_data = receipt::InstallReceipt::new_bottle(formula, runtime_deps, is_requested);
        receipt_data.write(&extracted_path)?;

        println!(
            "    └ {} Installed {} {}",
            "✓".green(),
            formula.name.bold().green(),
            version.dimmed()
        );
    }

    // Summary
    let installed_count = to_install.len();
    println!(
        "{} Installed {} packages",
        "✓".green().bold(),
        installed_count.to_string().bold()
    );

    Ok(())
}
