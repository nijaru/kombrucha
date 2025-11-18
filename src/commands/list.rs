//! List and query operations for installed packages and their states.
//!
//! This module provides commands for listing installed packages, checking for
//! outdated packages, finding leaf packages (not required by others), and
//! detecting missing dependencies.

use crate::api::BrewApi;
use crate::cellar;
use crate::error::Result;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::{HashMap, HashSet};
use std::io::IsTerminal;

/// Format names into column layout for terminal output
fn format_columns(names: &[String]) -> String {
    if names.is_empty() {
        return String::new();
    }

    // Get terminal width (default to 80 if not TTY)
    let term_width = if std::io::stdout().is_terminal() {
        term_size::dimensions().map(|(w, _)| w).unwrap_or(80)
    } else {
        80
    };

    // Find longest name for column width calculation
    let max_len = names.iter().map(|s| s.len()).max().unwrap_or(0);
    let col_width = max_len + 2; // Add 2 for spacing

    // Calculate number of columns that fit in terminal width
    let num_cols = (term_width / col_width).max(1);

    // Pre-allocate string with estimated capacity
    let estimated_lines = names.len().div_ceil(num_cols);
    let mut result = String::with_capacity(estimated_lines * term_width);

    for (i, name) in names.iter().enumerate() {
        result.push_str(name);

        if (i + 1) % num_cols == 0 {
            result.push('\n');
        } else if i < names.len() - 1 {
            // Add spacing to align columns
            let padding = col_width - name.len();
            result.push_str(&" ".repeat(padding));
        }
    }

    // Add final newline if last row is incomplete
    if !names.is_empty() && names.len() % num_cols != 0 {
        result.push('\n');
    }

    result
}

/// List installed formulae or casks with various output formats
///
/// Supports multiple output modes:
/// - Column layout (default in TTY)
/// - Single column (piped output or --versions)
/// - JSON format
/// - Quiet mode (names only)
pub async fn list(
    _api: &BrewApi,
    show_versions: bool,
    json: bool,
    cask: bool,
    quiet: bool,
    columns: bool,
) -> Result<()> {
    // Detect if stdout is a TTY (for pipe-aware behavior)
    let is_tty = std::io::IsTerminal::is_terminal(&std::io::stdout());

    // Determine output mode to match brew's behavior:
    // - brew list (TTY): columns via ls
    // - brew list (piped): single column names only (auto-quiet)
    // - brew list --versions (piped): single column WITH versions (explicit request)
    // - brew list -1: single column names only (always)

    // Auto-quiet ONLY if piped with no explicit content/format flags
    let has_explicit_flags = show_versions || columns || quiet;
    let use_quiet = quiet || (!is_tty && !json && !has_explicit_flags);

    // Determine if we should use column layout
    // Default: columns in TTY (like brew uses ls), single when piped
    // --versions: forces single column (matching brew)
    // --columns: explicit column override
    let use_columns = if columns {
        // Explicit --columns flag
        true
    } else if show_versions && !columns {
        // --versions without --columns forces single column (brew behavior)
        false
    } else if quiet || use_quiet {
        // --quiet or auto-quiet is always single column
        false
    } else {
        // Default: columns in TTY, single when piped
        is_tty
    };

    if cask {
        // List installed casks
        let casks = crate::cask::list_installed_casks()?;

        if json {
            // Output as JSON
            #[derive(serde::Serialize)]
            struct CaskInfo {
                token: String,
                version: String,
            }

            let cask_list: Vec<CaskInfo> = casks
                .into_iter()
                .map(|(token, version)| CaskInfo { token, version })
                .collect();

            let json_str = serde_json::to_string_pretty(&cask_list)?;
            println!("{}", json_str);
        } else if use_quiet {
            // Quiet mode: just package names, one per line, no headers
            if casks.is_empty() {
                return Ok(());
            }

            for (token, _version) in &casks {
                println!("{}", token);
            }
        } else if use_columns {
            // Column mode (default in TTY or explicit --columns)
            if is_tty {
                println!("Installed casks:");
            }

            if casks.is_empty() {
                if is_tty {
                    println!("No casks installed");
                }
                return Ok(());
            }

            if is_tty {
                println!();
            }

            if show_versions {
                // Columns with versions: "name version" in columns
                let formatted: Vec<String> = casks
                    .iter()
                    .map(|(token, version)| format!("{} {}", token, version))
                    .collect();
                print!("{}", format_columns(&formatted));
            } else {
                // Columns with names only
                let names: Vec<String> = casks.iter().map(|(token, _)| token.clone()).collect();
                print!("{}", format_columns(&names));
            }

            if is_tty {
                println!(
                    "{} {} casks installed",
                    "✓".green(),
                    casks.len().to_string().bold()
                );
            }
        } else {
            // Single column mode (--versions, -1, or piped without explicit --columns)
            if is_tty {
                println!("Installed casks:");
            }

            if casks.is_empty() {
                if is_tty {
                    println!("No casks installed");
                }
                return Ok(());
            }

            if is_tty {
                println!();
            }

            if show_versions {
                // Show versions
                for (token, version) in &casks {
                    println!("{} {}", token.bold().green(), version.dimmed());
                }
            } else {
                // Names only
                for (token, _version) in &casks {
                    println!("{}", token.bold().green());
                }
            }

            if is_tty {
                println!(
                    "{} {} casks installed",
                    "✓".green(),
                    casks.len().to_string().bold()
                );
            }
        }
    } else {
        // List installed formulae
        let packages = cellar::list_installed()?;

        if json {
            // Output as JSON
            #[derive(serde::Serialize)]
            struct PackageInfo {
                name: String,
                versions: Vec<String>,
            }

            // Group by formula name
            let mut by_name: HashMap<String, Vec<String>> = HashMap::new();
            for pkg in packages {
                by_name
                    .entry(pkg.name.clone())
                    .or_default()
                    .push(pkg.version.clone());
            }

            let mut package_list: Vec<PackageInfo> = by_name
                .into_iter()
                .map(|(name, versions)| PackageInfo { name, versions })
                .collect();

            package_list.sort_by(|a, b| a.name.cmp(&b.name));

            let json_str = serde_json::to_string_pretty(&package_list)?;
            println!("{}", json_str);
        } else if use_quiet {
            // Quiet mode: just package names, one per line, no headers
            if packages.is_empty() {
                return Ok(());
            }

            // Group by formula name to get unique names
            let mut names: HashSet<String> = HashSet::with_capacity(packages.len());
            for pkg in packages {
                names.insert(pkg.name.clone());
            }

            let mut sorted_names: Vec<_> = names.into_iter().collect();
            sorted_names.sort();

            for name in sorted_names {
                println!("{}", name);
            }
        } else if use_columns {
            // Column mode (default in TTY or explicit --columns)
            if is_tty {
                println!("Installed packages:");
            }

            if packages.is_empty() {
                if is_tty {
                    println!("No packages installed");
                }
                return Ok(());
            }

            // Group by formula name
            let mut by_name: HashMap<String, Vec<cellar::InstalledPackage>> =
                HashMap::with_capacity(packages.len());
            for pkg in packages {
                by_name.entry(pkg.name.clone()).or_default().push(pkg);
            }

            let mut names: Vec<_> = by_name.keys().cloned().collect();
            names.sort();

            if is_tty {
                println!();
            }

            if show_versions {
                // Columns with versions: "name version" in columns
                let formatted: Vec<String> = names
                    .iter()
                    .map(|name| {
                        let versions = &by_name[name];
                        let pkg = &versions[0]; // Show first version in column mode
                        format!("{} {}", name, pkg.version)
                    })
                    .collect();
                print!("{}", format_columns(&formatted));
            } else {
                // Columns with names only
                print!("{}", format_columns(&names));
            }

            if is_tty {
                println!(
                    "{} {} packages installed",
                    "✓".green(),
                    by_name.len().to_string().bold()
                );
            }
        } else {
            // Single column mode (--versions, -1, or piped without explicit --columns)
            if is_tty {
                println!("Installed packages:");
            }

            if packages.is_empty() {
                if is_tty {
                    println!("No packages installed");
                }
                return Ok(());
            }

            // Group by formula name
            let mut by_name: HashMap<String, Vec<cellar::InstalledPackage>> =
                HashMap::with_capacity(packages.len());
            for pkg in packages {
                by_name.entry(pkg.name.clone()).or_default().push(pkg);
            }

            let mut names: Vec<_> = by_name.keys().cloned().collect();
            names.sort();

            if is_tty {
                println!();
            }

            for name in names {
                let versions = &by_name[&name];

                if show_versions {
                    // Show all versions on one line (brew behavior)
                    let version_str: Vec<String> =
                        versions.iter().map(|pkg| pkg.version.clone()).collect();
                    println!("{} {}", name.bold().green(), version_str.join(" ").dimmed());
                } else {
                    // No versions requested: names only
                    println!("{}", name.bold().green());
                }
            }

            if is_tty {
                println!(
                    "{} {} packages installed",
                    "✓".green(),
                    by_name.len().to_string().bold()
                );
            }
        }
    }

    Ok(())
}

/// Check for outdated formulae or casks
///
/// Compares installed versions against latest available versions from the API.
/// Shows version differences in TTY mode, names only when piped or with --quiet.
pub async fn outdated(api: &BrewApi, cask: bool, quiet: bool) -> Result<()> {
    // Detect if stdout is a TTY (for brew-compatible behavior)
    let is_tty = std::io::IsTerminal::is_terminal(&std::io::stdout());

    // Show version info in TTY, suppress when piped (brew behavior)
    // --quiet forces names-only even in TTY
    let show_versions = is_tty && !quiet;

    if cask {
        // Check outdated casks
        let installed_casks = crate::cask::list_installed_casks()?;

        if installed_casks.is_empty() {
            return Ok(());
        }

        // Show spinner in TTY mode
        let spinner = if is_tty {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.cyan} {msg}")
                    .unwrap(),
            );
            pb.set_message("Checking for outdated casks...");
            pb.enable_steady_tick(std::time::Duration::from_millis(100));
            pb
        } else {
            ProgressBar::hidden()
        };

        // Fetch all cask versions in parallel
        let fetch_futures: Vec<_> = installed_casks
            .iter()
            .map(|(token, installed_version)| {
                let token = token.clone();
                let installed_version = installed_version.clone();
                async move {
                    // Check if cask has a newer version available
                    if let Ok(cask) = api.fetch_cask(&token).await
                        && let Some(latest) = &cask.version
                        && latest != &installed_version
                    {
                        return Some((token, installed_version, latest.clone()));
                    }
                    None
                }
            })
            .collect();

        let results = futures::future::join_all(fetch_futures).await;
        let outdated_casks: Vec<_> = results.into_iter().flatten().collect();

        spinner.finish_and_clear();

        if outdated_casks.is_empty() {
            return Ok(());
        }

        for (token, installed, latest) in &outdated_casks {
            if show_versions {
                // TTY mode: show versions in brew format
                println!(
                    "{} ({}) < {}",
                    token.bold().green(),
                    installed.dimmed(),
                    latest.cyan()
                );
            } else {
                // Piped/quiet mode: just names (brew behavior)
                println!("{}", token);
            }
        }

        // Show summary in TTY mode
        if show_versions {
            let count = outdated_casks.len();
            println!(
                "{} outdated {} found",
                count.to_string().bold(),
                if count == 1 { "cask" } else { "casks" }
            );
        }
    } else {
        // Check outdated formulae
        let all_packages = cellar::list_installed()?;

        if all_packages.is_empty() {
            return Ok(());
        }

        // Deduplicate multiple versions - keep only the most recent for each formula
        let mut package_map: HashMap<String, cellar::InstalledPackage> =
            HashMap::with_capacity(all_packages.len());

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

        // Show spinner in TTY mode
        let spinner = if is_tty {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.cyan} {msg}")
                    .unwrap(),
            );
            pb.set_message("Checking for outdated packages...");
            pb.enable_steady_tick(std::time::Duration::from_millis(100));
            pb
        } else {
            ProgressBar::hidden()
        };

        // Fetch all formula versions in parallel
        let fetch_futures: Vec<_> = packages
            .iter()
            .map(|pkg| async move {
                // Hybrid approach: check tap for freshness, use API for accuracy
                // Tap parsing may be incomplete for complex formulas (e.g., bash with patches)
                // but is always up-to-date. API is complete but may lag.

                // Try API first (complete and accurate)
                if let Ok(formula) = api.fetch_formula(&pkg.name).await
                    && let Some(api_version) = &formula.versions.stable
                {
                    // Strip bottle revisions for comparison (e.g., "6.9.3_1" -> "6.9.3")
                    // Bottle revisions indicate rebuilds, not version upgrades
                    let installed_base = pkg.version.split('_').next().unwrap_or(&pkg.version);
                    let api_base = api_version.split('_').next().unwrap_or(api_version);

                    // Only flag as outdated if the base version changed
                    if installed_base != api_base {
                        return Some((pkg.clone(), api_version.clone()));
                    }
                    return None;
                }

                // API unavailable - fall back to tap parsing
                // This ensures we still work when offline or if API is down
                if let Ok(Some(tap_ver)) = crate::tap::get_core_formula_version(&pkg.name) {
                    if pkg.version != tap_ver {
                        return Some((pkg.clone(), tap_ver));
                    }
                }

                None
            })
            .collect();

        let results = futures::future::join_all(fetch_futures).await;
        let outdated_packages: Vec<_> = results.into_iter().flatten().collect();

        spinner.finish_and_clear();

        if outdated_packages.is_empty() {
            return Ok(());
        }

        for (pkg, latest) in &outdated_packages {
            if show_versions {
                // TTY mode: show versions in brew format
                println!(
                    "{} ({}) < {}",
                    pkg.name.bold().green(),
                    pkg.version.dimmed(),
                    latest.cyan()
                );
            } else {
                // Piped/quiet mode: just names (brew behavior)
                println!("{}", pkg.name);
            }
        }

        // Show summary in TTY mode
        if show_versions {
            let count = outdated_packages.len();
            println!(
                "{} outdated {} found",
                count.to_string().bold(),
                if count == 1 { "package" } else { "packages" }
            );
        }
    }

    Ok(())
}

/// Find leaf packages (packages not required by any other packages)
///
/// Useful for identifying packages that can be safely removed without
/// breaking dependencies.
pub fn leaves() -> Result<()> {
    // Detect if stdout is a TTY (for brew-compatible behavior)
    let is_tty = std::io::IsTerminal::is_terminal(&std::io::stdout());

    if is_tty {
        println!("{}", "==> Leaf Packages".bold().green());
        println!("(Packages not required by other packages)");
        println!();
    }

    let all_packages = cellar::list_installed()?;

    // Deduplicate by package name - keep only the most recent version of each
    let mut package_map: HashMap<String, cellar::InstalledPackage> =
        HashMap::with_capacity(all_packages.len());

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

    let unique_packages: Vec<_> = package_map.into_values().collect();

    // Build a set of all packages that are dependencies of others
    let mut required_by_others = HashSet::with_capacity(unique_packages.len());
    for pkg in &unique_packages {
        for dep in pkg.runtime_dependencies() {
            required_by_others.insert(dep.full_name.clone());
        }
    }

    // Filter to packages that are NOT in the required set
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

/// Check for missing runtime dependencies in installed packages
///
/// Verifies that all runtime dependencies declared by installed packages
/// are themselves installed. Useful for detecting broken installations.
pub fn missing(formula_names: &[String]) -> Result<()> {
    let to_check = if formula_names.is_empty() {
        // Check all installed packages
        cellar::list_installed()?
            .into_iter()
            .map(|p| p.name)
            .collect()
    } else {
        formula_names.to_vec()
    };

    if to_check.is_empty() {
        println!("No packages installed");
        return Ok(());
    }

    println!("Checking for missing dependencies...");
    println!();

    let all_installed = cellar::list_installed()?;
    let installed_set: HashSet<_> = all_installed.iter().map(|p| p.name.as_str()).collect();

    let mut has_missing = false;

    for formula_name in &to_check {
        // Check if formula is installed
        let pkg = match all_installed.iter().find(|p| &p.name == formula_name) {
            Some(p) => p,
            None => {
                if !formula_names.is_empty() {
                    println!("{} {} is not installed", "⚠".yellow(), formula_name.bold());
                }
                continue;
            }
        };

        // Check each runtime dependency
        let runtime_deps = pkg.runtime_dependencies();
        let missing_deps: Vec<_> = runtime_deps
            .iter()
            .filter(|dep| !installed_set.contains(dep.full_name.as_str()))
            .collect();

        if !missing_deps.is_empty() {
            has_missing = true;
            println!(
                "{} {} is missing dependencies:",
                "✗".red(),
                formula_name.bold()
            );
            for dep in missing_deps {
                println!("  {} {}", dep.full_name.cyan(), dep.version.dimmed());
            }
            println!();
        }
    }

    if !has_missing {
        println!("{} No missing dependencies found", "✓".green());
    }

    Ok(())
}
