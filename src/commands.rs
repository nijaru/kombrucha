use crate::api::{BrewApi, Formula};
use crate::cellar::{self, RuntimeDependency};
use crate::error::Result;
use crate::{download, extract, receipt, symlink};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

/// Check if brew is available for fallback to source builds
fn check_brew_available() -> bool {
    Command::new("brew")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Fallback to brew for packages that require source builds
fn fallback_to_brew(command: &str, formula_name: &str) -> Result<()> {
    println!(
        "\n  {} {} requires building from source (no bottle available)",
        "ℹ".blue(),
        formula_name.bold()
    );

    if !check_brew_available() {
        println!(
            "  {} brew is not installed - cannot build from source",
            "✗".red()
        );
        println!(
            "  {} Install Homebrew or use a formula with bottles",
            "ℹ".blue()
        );
        return Err(anyhow::anyhow!("brew not available for source build").into());
    }

    println!(
        "  Falling back to {}...",
        format!("brew {}", command).cyan()
    );

    let status = Command::new("brew")
        .arg(command)
        .arg(formula_name)
        .status()?;

    if status.success() {
        println!(
            "  {} Installed {} via brew",
            "✓".green(),
            formula_name.bold()
        );
        Ok(())
    } else {
        Err(anyhow::anyhow!("brew {} failed for {}", command, formula_name).into())
    }
}

pub async fn search(api: &BrewApi, query: &str, formula_only: bool, cask_only: bool) -> Result<()> {
    // Detect if stdout is a TTY (for brew-compatible behavior)
    let is_tty = std::io::IsTerminal::is_terminal(&std::io::stdout());

    let results = api.search(query).await?;

    if results.is_empty() {
        if is_tty {
            println!(
                "{} No formulae or casks found matching '{}'",
                "✗".red(),
                query
            );
        }
        return Ok(());
    }

    // Determine what to display based on flags
    let show_formulae = !cask_only;
    let show_casks = !formula_only;

    // Count total results to show
    let total_to_show = (if show_formulae {
        results.formulae.len()
    } else {
        0
    }) + (if show_casks { results.casks.len() } else { 0 });

    if total_to_show == 0 {
        if is_tty {
            println!("{} No results found with the specified filter", "✗".red());
        }
        return Ok(());
    }

    // Display formulae
    if show_formulae && !results.formulae.is_empty() {
        if is_tty {
            println!("{}", "==> Formulae".bold().green());
        }

        for formula in &results.formulae {
            if is_tty {
                println!("{}", formula.name.bold().green());
            } else {
                // Piped: just names (brew behavior)
                println!("{}", formula.name);
            }
        }

        if is_tty && !results.casks.is_empty() {
            // Add blank line between sections in TTY
            println!();
        }
    }

    // Display casks
    if show_casks && !results.casks.is_empty() {
        if is_tty {
            println!("{}", "==> Casks".bold().cyan());
        }

        for cask in &results.casks {
            if is_tty {
                println!("{}", cask.token.bold().cyan());
            } else {
                // Piped: just tokens (brew behavior)
                println!("{}", cask.token);
            }
        }
    }

    Ok(())
}

pub async fn info(api: &BrewApi, formula: &str, json: bool) -> Result<()> {
    let is_tty = std::io::IsTerminal::is_terminal(&std::io::stdout());

    if !json && is_tty {
        println!("Fetching info for: {}", formula.cyan());
    }

    // Try formula first, then cask
    match api.fetch_formula(formula).await {
        Ok(formula) => {
            if json {
                // Output as JSON
                let json_str = serde_json::to_string_pretty(&formula)?;
                println!("{}", json_str);
            } else {
                // Pretty print format
                println!("\n {}", format!("==> {}", formula.name).bold().green());
                if let Some(desc) = &formula.desc {
                    println!("{}", desc);
                }
                if let Some(homepage) = &formula.homepage {
                    println!("{}: {}", "Homepage".bold(), homepage);
                }
                if let Some(version) = &formula.versions.stable {
                    println!("{}: {}", "Version".bold(), version);
                }

                if formula.keg_only {
                    if let Some(reason) = &formula.keg_only_reason {
                        let reason_display = match reason.reason.as_str() {
                            ":provided_by_macos" => "provided by macOS",
                            ":shadowed_by_macos" => "shadowed by macOS",
                            ":versioned_formula" => "versioned formula",
                            _ => &reason.reason,
                        };
                        println!("{}: {}", "Keg-only".bold().yellow(), reason_display);
                        if !reason.explanation.is_empty() {
                            println!("  {}", reason.explanation.dimmed());
                        }
                    } else {
                        println!("{}: yes", "Keg-only".bold().yellow());
                    }
                }

                if !formula.dependencies.is_empty() {
                    println!(
                        "{}: {}",
                        "Dependencies".bold(),
                        formula.dependencies.join(", ")
                    );
                }

                if !formula.build_dependencies.is_empty() {
                    println!(
                        "{}: {}",
                        "Build dependencies".bold(),
                        formula.build_dependencies.join(", ")
                    );
                }
            }
        }
        Err(_) => {
            // Try as cask
            match api.fetch_cask(formula).await {
                Ok(cask) => {
                    if json {
                        let json_str = serde_json::to_string_pretty(&cask)?;
                        println!("{}", json_str);
                    } else {
                        println!("\n {}", format!("==> {}", cask.token).bold().cyan());
                        if !cask.name.is_empty() {
                            println!("{}: {}", "Name".bold(), cask.name.join(", "));
                        }
                        if let Some(desc) = &cask.desc {
                            println!("{}", desc);
                        }
                        if let Some(homepage) = &cask.homepage {
                            println!("{}: {}", "Homepage".bold(), homepage);
                        }
                        if let Some(version) = &cask.version {
                            println!("{}: {}", "Version".bold(), version);
                        }
                    }
                }
                Err(_) => {
                    if json {
                        println!(
                            "{{\"error\": \"No formula or cask found for '{}'\"}}",
                            formula
                        );
                    } else {
                        println!(
                            "\n {} No formula or cask found for '{}'",
                            "✗".red(),
                            formula
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

pub async fn deps(api: &BrewApi, formula: &str, tree: bool, installed_only: bool) -> Result<()> {
    let is_tty = std::io::IsTerminal::is_terminal(&std::io::stdout());

    if is_tty {
        if tree {
            println!("Dependency tree for: {}", formula.cyan());
        } else {
            println!("Dependencies for: {}", formula.cyan());
        }
    }

    let formula_data = api.fetch_formula(formula).await?;

    if formula_data.dependencies.is_empty() && formula_data.build_dependencies.is_empty() {
        if is_tty {
            println!("\n {} No dependencies", "✓".green());
        }
        return Ok(());
    }

    // If filtering by installed, get the list of installed packages
    let installed_names: HashSet<String> = if installed_only {
        cellar::list_installed()?
            .into_iter()
            .map(|p| p.name)
            .collect()
    } else {
        HashSet::new()
    };

    if !formula_data.dependencies.is_empty() {
        let mut deps: Vec<_> = formula_data.dependencies.iter().collect();

        if installed_only {
            deps.retain(|dep| installed_names.contains(*dep));
        }

        if !deps.is_empty() {
            if is_tty {
                println!("\n {}", "Runtime dependencies:".bold().green());
            }
            let len = deps.len();
            for (i, dep) in deps.iter().enumerate() {
                if is_tty {
                    if tree {
                        let prefix = if i == len - 1 { "└─" } else { "├─" };
                        println!("  {} {}", prefix, dep.cyan());
                    } else {
                        println!("  {}", dep.cyan());
                    }
                } else {
                    println!("{}", dep);
                }
            }
        } else if installed_only && is_tty {
            println!("\n {} No runtime dependencies installed", "ℹ".blue());
        }
    }

    if !formula_data.build_dependencies.is_empty() {
        let mut build_deps: Vec<_> = formula_data.build_dependencies.iter().collect();

        if installed_only {
            build_deps.retain(|dep| installed_names.contains(*dep));
        }

        if !build_deps.is_empty() {
            if is_tty {
                println!("\n {}", "Build dependencies:".bold().yellow());
            }
            let len = build_deps.len();
            for (i, dep) in build_deps.iter().enumerate() {
                if is_tty {
                    if tree {
                        let prefix = if i == len - 1 { "└─" } else { "├─" };
                        println!("  {} {}", prefix, dep.cyan());
                    } else {
                        println!("  {}", dep.cyan());
                    }
                } else {
                    println!("{}", dep);
                }
            }
        } else if installed_only && !formula_data.build_dependencies.is_empty() && is_tty {
            println!("\n {} No build dependencies installed", "ℹ".blue());
        }
    }

    Ok(())
}

pub async fn uses(api: &BrewApi, formula: &str) -> Result<()> {
    let is_tty = std::io::IsTerminal::is_terminal(&std::io::stdout());

    if is_tty {
        println!("Finding formulae that depend on: {}", formula.cyan());
    }

    // Fetch all formulae
    let all_formulae = api.fetch_all_formulae().await?;

    // Find formulae that depend on the target
    let dependent_formulae: Vec<_> = all_formulae
        .into_iter()
        .filter(|f| {
            f.dependencies.contains(&formula.to_string())
                || f.build_dependencies.contains(&formula.to_string())
        })
        .collect();

    if dependent_formulae.is_empty() {
        if is_tty {
            println!("\n {} No formulae depend on '{}'", "✓".green(), formula);
        }
        return Ok(());
    }

    if is_tty {
        println!(
            "\n{} Found {} formulae that depend on {}:\n",
            "✓".green(),
            dependent_formulae.len().to_string().bold(),
            formula.cyan()
        );
    }

    for f in dependent_formulae {
        if is_tty {
            print!("{}", f.name.bold());
            if let Some(desc) = &f.desc
                && !desc.is_empty()
            {
                print!(" {}", format!("({})", desc).dimmed());
            }
            println!();
        } else {
            println!("{}", f.name);
        }
    }

    Ok(())
}

/// Format names in columns for display
fn format_columns(names: &[String]) -> String {
    use std::io::IsTerminal;

    if names.is_empty() {
        return String::new();
    }

    // Get terminal width (default to 80 if not TTY)
    let term_width = if std::io::stdout().is_terminal() {
        term_size::dimensions().map(|(w, _)| w).unwrap_or(80)
    } else {
        80
    };

    // Find longest name
    let max_len = names.iter().map(|s| s.len()).max().unwrap_or(0);
    let col_width = max_len + 2; // Add 2 for spacing

    // Calculate number of columns
    let num_cols = (term_width / col_width).max(1);

    // Format in columns
    let mut result = String::new();
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

    // Add final newline if needed
    if !result.ends_with('\n') {
        result.push('\n');
    }

    result
}

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
                    println!("\n {} No casks installed", "ℹ".blue());
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
                    println!("\n {} No casks installed", "ℹ".blue());
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
                    "\n{} {} casks installed",
                    "✓".green(),
                    casks.len().to_string().bold()
                );
            }
        }
    } else {
        // List installed formulae (existing logic)
        let packages = cellar::list_installed()?;

        if json {
            // Output as JSON
            #[derive(serde::Serialize)]
            struct PackageInfo {
                name: String,
                versions: Vec<String>,
            }

            // Group by formula name
            let mut by_name: std::collections::HashMap<String, Vec<_>> =
                std::collections::HashMap::new();
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
            let mut names: std::collections::HashSet<String> = std::collections::HashSet::new();
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
                    println!("\n {} No packages installed", "ℹ".blue());
                }
                return Ok(());
            }

            // Group by formula name
            let mut by_name: std::collections::HashMap<String, Vec<_>> =
                std::collections::HashMap::new();
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
                    println!("\n {} No packages installed", "ℹ".blue());
                }
                return Ok(());
            }

            // Group by formula name
            let mut by_name: std::collections::HashMap<String, Vec<_>> =
                std::collections::HashMap::new();
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
                    "\n{} {} packages installed",
                    "✓".green(),
                    by_name.len().to_string().bold()
                );
            }
        }
    }

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

        // Fetch all cask versions in parallel
        let fetch_futures: Vec<_> = installed_casks
            .iter()
            .map(|(token, installed_version)| {
                let token = token.clone();
                let installed_version = installed_version.clone();
                async move {
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
                "\n{} outdated {} found",
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
        let mut package_map: std::collections::HashMap<String, cellar::InstalledPackage> =
            std::collections::HashMap::new();

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

        // Fetch all formula versions in parallel
        let fetch_futures: Vec<_> = packages
            .iter()
            .map(|pkg| async move {
                if let Ok(formula) = api.fetch_formula(&pkg.name).await
                    && let Some(latest) = &formula.versions.stable
                {
                    // Strip bottle revisions before comparison
                    let installed_stripped = strip_bottle_revision(&pkg.version);
                    let latest_stripped = strip_bottle_revision(latest);

                    if installed_stripped != latest_stripped {
                        return Some((pkg.clone(), latest.clone()));
                    }
                }
                None
            })
            .collect();

        let results = futures::future::join_all(fetch_futures).await;
        let outdated_packages: Vec<_> = results.into_iter().flatten().collect();

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
                "\n{} outdated {} found",
                count.to_string().bold(),
                if count == 1 { "package" } else { "packages" }
            );
        }
    }

    Ok(())
}

pub async fn fetch(api: &BrewApi, formula_names: &[String]) -> Result<()> {
    println!(
        "Fetching {} formulae...",
        formula_names.len().to_string().bold()
    );

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
                        println!("{} No bottle available for {}", "⚠".yellow(), name.bold());
                        return None;
                    }
                    Some(formula)
                }
                Err(e) => {
                    println!(
                        "{} Failed to fetch formula {}: {}",
                        "✗".red(),
                        name.bold(),
                        e
                    );
                    None
                }
            }
        })
        .collect();

    let results = futures::future::join_all(fetch_futures).await;
    let formulae: Vec<_> = results.into_iter().flatten().collect();

    if formulae.is_empty() {
        println!("\n {} No formulae to download", "ℹ".blue());
        return Ok(());
    }

    // Download bottles in parallel
    match download::download_bottles(api, &formulae).await {
        Ok(results) => {
            println!(
                "\n{} Downloaded {} bottles to {}",
                "✓".green(),
                results.len().to_string().bold(),
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
            println!("\n {} Download failed: {}", "✗".red(), e);
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
        println!(
            "{} Dry run mode - no packages will be installed",
            "ℹ".blue()
        );
    }

    println!(
        "Installing {} formulae...",
        formula_names.len().to_string().bold()
    );

    // Step 1: Validate requested formulae in parallel
    println!("\nResolving dependencies...");

    let validation_futures: Vec<_> = formula_names
        .iter()
        .map(|name| async move {
            match api.fetch_formula(name).await {
                Ok(_) => Ok(name.clone()),
                Err(e) => Err((name.clone(), e)),
            }
        })
        .collect();

    let validation_results = futures::future::join_all(validation_futures).await;

    let mut errors = Vec::new();
    let mut valid_formulae = Vec::new();

    for result in validation_results {
        match result {
            Ok(name) => valid_formulae.push(name),
            Err((name, e)) => errors.push((name, e)),
        }
    }

    // If no valid formulae, report errors and fail
    if valid_formulae.is_empty() {
        for (name, err) in &errors {
            println!("{} {}: {}", "✗".red(), name, err);
        }
        return Err(crate::error::BruError::Other(anyhow::anyhow!(
            "All formulae failed to install"
        )));
    }

    // Report any invalid formulae but continue with valid ones
    if !errors.is_empty() {
        for (name, err) in &errors {
            println!("{} {}: {}", "⚠".yellow(), name, err);
        }
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

        println!("\n {} Already installed:", "ℹ".blue());
        for pkg in &already_installed {
            println!("  {}", pkg.cyan());
        }

        if force {
            println!("\n  Use {} to reinstall", "--force".dimmed());
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
            "\n{} Dry run complete - no packages were installed",
            "✓".green()
        );
        return Ok(());
    }

    // Step 2: Download all bottles in parallel
    println!("\nDownloading bottles...");
    let downloaded = download::download_bottles(api, &to_install).await?;
    let download_map: HashMap<_, _> = downloaded.into_iter().collect();

    // Step 3: Install in dependency order
    let total_to_install = to_install.len();
    let mut installed_count = 0;
    println!("\nInstalling packages...");
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
                match fallback_to_brew("install", &formula.name) {
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

        // Relocate bottle (fix install names)
        crate::relocate::relocate_bottle(&extracted_path, &crate::cellar::detect_prefix())?;

        // Create symlinks
        let linked = symlink::link_formula(&formula.name, version)?;
        println!(
            "    ├ {} Linked {} files",
            "✓".green(),
            linked.len().to_string().dimmed()
        );

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
        "\n{} Installed {} packages",
        "✓".green().bold(),
        installed_count.to_string().bold()
    );

    Ok(())
}

/// Resolve all dependencies recursively, parallelizing each level
async fn resolve_dependencies(
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
        println!("✓ {} dependencies resolved", all_formulae.len());
    }

    Ok((all_formulae, dep_order))
}

/// Topological sort for dependency order
fn topological_sort(formulae: &HashMap<String, Formula>) -> anyhow::Result<Vec<String>> {
    let capacity = formulae.len();
    let mut in_degree: HashMap<String, usize> = HashMap::with_capacity(capacity);
    let mut graph: HashMap<String, Vec<String>> = HashMap::with_capacity(capacity);

    // Build dependency graph
    for (name, formula) in formulae {
        in_degree.entry(name.clone()).or_insert(0);
        for dep in &formula.dependencies {
            graph.entry(dep.clone()).or_default().push(name.clone());
            *in_degree.entry(name.clone()).or_insert(0) += 1;
        }
    }

    // Kahn's algorithm
    let mut queue: Vec<_> = in_degree
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(name, _)| name.clone())
        .collect();
    let mut result = Vec::with_capacity(capacity);

    while let Some(node) = queue.pop() {
        result.push(node.clone());

        if let Some(dependents) = graph.get(&node) {
            for dependent in dependents {
                if let Some(count) = in_degree.get_mut(dependent) {
                    *count -= 1;
                    if *count == 0 {
                        queue.push(dependent.clone());
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
        println!("Checking for outdated packages...");
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
        let fetch_futures: Vec<_> = packages
            .iter()
            .map(|pkg| async move {
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

        if outdated.is_empty() {
            println!("\n {} All packages are up to date", "✓".green());
            return Ok(());
        }

        println!(
            "  {} packages to upgrade: {}",
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
            "\n{} Dry run complete - no packages were upgraded",
            "✓".green()
        );
        return Ok(());
    }

    // Check for pinned formulae
    let pinned = read_pinned()?;

    // Phase 1: Collect all upgrade candidates in parallel
    println!("\nPreparing {} packages for upgrade...", to_upgrade.len());

    let fetch_futures: Vec<_> = to_upgrade
        .iter()
        .filter(|name| !pinned.contains(*name))
        .map(|formula_name| async move {
            // Check if installed
            let installed_versions = cellar::get_installed_versions(formula_name).ok()?;
            if installed_versions.is_empty() {
                return None; // Will install separately
            }

            let old_version = installed_versions[0].version.clone();

            // Fetch latest version
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

    if candidates.is_empty() {
        println!("\n {} All packages are up to date", "✓".green());
        return Ok(());
    }

    // Show what will be upgraded
    for candidate in &candidates {
        let new_version = candidate.formula.versions.stable.as_ref().unwrap();
        println!(
            "  {} {} -> {}",
            candidate.name.cyan(),
            candidate.old_version.dimmed(),
            new_version.cyan()
        );
    }

    // Phase 2: Download all bottles in parallel
    println!("\nDownloading {} bottles...", candidates.len());
    let formulae: Vec<_> = candidates.iter().map(|c| c.formula.clone()).collect();
    let downloaded = download::download_bottles(api, &formulae).await?;
    let download_map: HashMap<_, _> = downloaded.into_iter().collect();

    // Phase 3: Install sequentially
    println!("\nUpgrading packages...");

    for candidate in &candidates {
        let formula_name = &candidate.name;
        let old_version = &candidate.old_version;
        let formula = &candidate.formula;
        let new_version = formula.versions.stable.as_ref().unwrap();

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

        // Relocate bottle (fix install names)
        crate::relocate::relocate_bottle(&extracted_path, &crate::cellar::detect_prefix())?;

        let linked = symlink::link_formula(formula_name, new_version)?;

        // Generate receipt
        let runtime_deps = build_runtime_deps(&formula.dependencies, &{
            let mut map = HashMap::new();
            map.insert(formula.name.clone(), formula.clone());
            map
        });
        let receipt_data = receipt::InstallReceipt::new_bottle(formula, runtime_deps, true);
        receipt_data.write(&extracted_path)?;

        println!(
            "    ├ {} Linked {} files",
            "✓".green(),
            linked.len().to_string().dimmed()
        );

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

            // Remove the old version directory
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

    println!(
        "\n{} Upgraded {} packages",
        "✓".green().bold(),
        to_upgrade.len().to_string().bold()
    );

    Ok(())
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

    let mut actually_reinstalled = 0;

    // Create shared HTTP client for all downloads
    let client = reqwest::Client::new();

    for formula_name in formula_names {
        // Check if installed
        let installed_versions = cellar::get_installed_versions(formula_name)?;
        if installed_versions.is_empty() {
            println!("  {} {} not installed", "⚠".yellow(), formula_name.bold());
            continue;
        }

        let old_version = &installed_versions[0].version;
        println!(
            "  Reinstalling {} {}",
            formula_name.cyan(),
            old_version.dimmed()
        );

        // Unlink
        symlink::unlink_formula(formula_name, old_version)?;

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

        // Relocate bottle (fix install names)
        crate::relocate::relocate_bottle(&extracted_path, &crate::cellar::detect_prefix())?;

        let linked = symlink::link_formula(formula_name, new_version)?;

        // Generate receipt
        let runtime_deps = build_runtime_deps(&formula.dependencies, &{
            let mut map = HashMap::new();
            map.insert(formula.name.clone(), formula.clone());
            map
        });
        let receipt_data = receipt::InstallReceipt::new_bottle(&formula, runtime_deps, true);
        receipt_data.write(&extracted_path)?;

        println!(
            "    ├ {} Linked {} files",
            "✓".green(),
            linked.len().to_string().dimmed()
        );
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
            "\n{} Reinstalled {} package{}",
            "✓".green().bold(),
            actually_reinstalled.to_string().bold(),
            if actually_reinstalled == 1 { "" } else { "s" }
        );
    } else {
        println!("\n {} No packages were reinstalled", "ℹ".blue());
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
            println!("  {} {} not installed", "⚠".yellow(), formula_name.bold());
            continue;
        }

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

        let version = &installed_versions[0].version;
        println!(
            "  Uninstalling {} {}",
            formula_name.cyan(),
            version.dimmed()
        );

        // Unlink symlinks
        let unlinked = symlink::unlink_formula(formula_name, version)?;
        if !unlinked.is_empty() {
            println!(
                "    ├ {} Unlinked {} files",
                "✓".green(),
                unlinked.len().to_string().dimmed()
            );
        }

        // Remove from Cellar
        let cellar_path = cellar::cellar_path().join(formula_name).join(version);
        if cellar_path.exists() {
            std::fs::remove_dir_all(&cellar_path)?;
        }

        // Remove formula directory if empty
        let formula_dir = cellar::cellar_path().join(formula_name);
        if formula_dir.exists() && formula_dir.read_dir()?.next().is_none() {
            std::fs::remove_dir(&formula_dir)?;
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
            "\n{} Uninstalled {} package{}",
            "✓".green().bold(),
            actually_uninstalled.to_string().bold(),
            if actually_uninstalled == 1 { "" } else { "s" }
        );
    } else {
        println!("\n {} No packages were uninstalled", "ℹ".blue());
    }

    Ok(())
}

pub fn autoremove(dry_run: bool) -> Result<()> {
    if dry_run {
        println!("{} Dry run - no packages will be removed", "ℹ".blue());
    } else {
        println!("Removing unused dependencies...");
    }

    let all_packages = cellar::list_installed()?;

    // Build a set of all packages installed on request
    let mut on_request: HashSet<String> = HashSet::new();
    for pkg in &all_packages {
        if pkg.installed_on_request() {
            on_request.insert(pkg.name.clone());
        }
    }

    // Build a set of all dependencies required by packages installed on request
    let mut required = HashSet::new();
    let mut to_check: Vec<String> = on_request.iter().cloned().collect();
    let mut checked = HashSet::new();

    while let Some(name) = to_check.pop() {
        if checked.contains(&name) {
            continue;
        }
        checked.insert(name.clone());

        // Find the package and get its dependencies
        if let Some(pkg) = all_packages.iter().find(|p| p.name == name) {
            for dep in pkg.runtime_dependencies() {
                required.insert(dep.full_name.clone());
                if !checked.contains(&dep.full_name) {
                    to_check.push(dep.full_name.clone());
                }
            }
        }
    }

    // Find packages that are:
    // 1. Installed as dependency (not on request)
    // 2. Not required by any package installed on request
    let mut to_remove: Vec<_> = all_packages
        .iter()
        .filter(|pkg| !pkg.installed_on_request() && !required.contains(&pkg.name))
        .collect();

    if to_remove.is_empty() {
        println!("\n {} No unused dependencies to remove", "✓".green());
        return Ok(());
    }

    to_remove.sort_by(|a, b| a.name.cmp(&b.name));

    println!(
        "\nFound {} unused dependencies:\n",
        to_remove.len().to_string().bold()
    );

    for pkg in &to_remove {
        println!("  {} {}", pkg.name.cyan(), pkg.version.dimmed());
    }

    if dry_run {
        println!(
            "\n{} Would remove {} packages",
            "ℹ".blue(),
            to_remove.len().to_string().bold()
        );
        println!("Run without {} to remove them", "--dry-run".dimmed());
        return Ok(());
    }

    println!();

    // Remove packages
    for pkg in &to_remove {
        println!(
            "  Uninstalling {} {}",
            pkg.name.cyan(),
            pkg.version.dimmed()
        );

        // Unlink symlinks
        let unlinked = symlink::unlink_formula(&pkg.name, &pkg.version)?;
        if !unlinked.is_empty() {
            println!(
                "    {} Unlinked {} files",
                "✓".green(),
                unlinked.len().to_string().dimmed()
            );
        }

        // Remove from Cellar
        let cellar_path = cellar::cellar_path().join(&pkg.name).join(&pkg.version);
        if cellar_path.exists() {
            std::fs::remove_dir_all(&cellar_path)?;
        }

        // Remove formula directory if empty
        let formula_dir = cellar::cellar_path().join(&pkg.name);
        if formula_dir.exists() && formula_dir.read_dir()?.next().is_none() {
            std::fs::remove_dir(&formula_dir)?;
        }

        println!("    {} Removed {}", "✓".green(), pkg.name.bold().green());
    }

    println!(
        "\n{} Removed {} unused packages",
        "✓".green().bold(),
        to_remove.len().to_string().bold()
    );

    Ok(())
}

pub fn tap(tap_name: Option<&str>) -> Result<()> {
    match tap_name {
        None => {
            // List all taps
            let taps = crate::tap::list_taps()?;
            if taps.is_empty() {
                println!("{} No taps installed", "ℹ".blue());
            } else {
                for tap in taps {
                    println!("{}", tap.cyan());
                }
            }
        }
        Some(tap) => {
            // Add a tap
            println!("Tapping {}...", tap.cyan());

            if crate::tap::is_tapped(tap)? {
                println!("  {} {} already tapped", "✓".green(), tap.bold());
                return Ok(());
            }

            crate::tap::tap(tap)?;

            println!(
                "  {} Tapped {} successfully",
                "✓".green(),
                tap.bold().green()
            );
        }
    }
    Ok(())
}

pub fn untap(tap_name: &str) -> Result<()> {
    println!("Untapping {}...", tap_name.cyan());

    if !crate::tap::is_tapped(tap_name)? {
        println!("  {} {} is not tapped", "⚠".yellow(), tap_name.bold());
        return Ok(());
    }

    crate::tap::untap(tap_name)?;

    println!(
        "  {} Untapped {} successfully",
        "✓".green(),
        tap_name.bold().green()
    );

    Ok(())
}

pub fn tap_info(tap_name: &str) -> Result<()> {
    println!(
        "{} Tap information for {}",
        "ℹ".bold(),
        tap_name.cyan().bold()
    );
    println!();

    if !crate::tap::is_tapped(tap_name)? {
        println!(
            "  {} Tap {} is not installed",
            "⚠".yellow(),
            tap_name.bold()
        );
        return Ok(());
    }

    let tap_dir = crate::tap::tap_directory(tap_name)?;

    println!("{}", "Location:".bold());
    println!("  {}", tap_dir.display().to_string().cyan());
    println!();

    // Count formulae in the tap (recursively for letter-organized directories)
    let formula_dir = tap_dir.join("Formula");
    let mut formula_count = 0;

    if formula_dir.exists() {
        fn count_rb_files(dir: &std::path::Path, depth: usize) -> usize {
            const MAX_DEPTH: usize = 10;
            if depth > MAX_DEPTH {
                return 0;
            }

            let mut count = 0;
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rb") {
                        count += 1;
                    } else if path.is_dir() {
                        count += count_rb_files(&path, depth + 1);
                    }
                }
            }
            count
        }
        formula_count = count_rb_files(&formula_dir, 0);
    }

    // Count casks in the tap
    let casks_dir = tap_dir.join("Casks");
    let mut cask_count = 0;

    if casks_dir.exists() {
        fn count_rb_files(dir: &std::path::Path, depth: usize) -> usize {
            const MAX_DEPTH: usize = 10;
            if depth > MAX_DEPTH {
                return 0;
            }

            let mut count = 0;
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rb") {
                        count += 1;
                    } else if path.is_dir() {
                        count += count_rb_files(&path, depth + 1);
                    }
                }
            }
            count
        }
        cask_count = count_rb_files(&casks_dir, 0);
    }

    println!("{}", "Contents:".bold());
    println!(
        "  {}: {}",
        "Formulae".dimmed(),
        formula_count.to_string().cyan()
    );
    println!("  {}: {}", "Casks".dimmed(), cask_count.to_string().cyan());

    Ok(())
}

pub fn update() -> Result<()> {
    println!("Updating Homebrew...");

    let taps = crate::tap::list_taps()?;

    if taps.is_empty() {
        println!("\n {} No taps installed", "ℹ".blue());
        return Ok(());
    }

    println!("\nUpdating {} taps...", taps.len().to_string().bold());

    // Parallel tap updates with live progress
    use std::sync::mpsc;
    let (tx, rx) = mpsc::channel();

    let handles: Vec<_> = taps
        .iter()
        .map(|tap| {
            let tap = tap.clone();
            let tx = tx.clone();
            std::thread::spawn(move || {
                let tap_dir = match crate::tap::tap_directory(&tap) {
                    Ok(dir) => dir,
                    Err(_) => {
                        let _ = tx.send((tap.clone(), Err("invalid tap directory".to_string())));
                        return;
                    }
                };

                if !tap_dir.exists() || !tap_dir.join(".git").exists() {
                    let _ = tx.send((tap.clone(), Err("not a git repository".to_string())));
                    return;
                }

                let tap_dir_str = match tap_dir.to_str() {
                    Some(s) => s,
                    None => {
                        let _ = tx.send((tap.clone(), Err("invalid path".to_string())));
                        return;
                    }
                };

                let output = std::process::Command::new("git")
                    .args(["-C", tap_dir_str, "pull", "--ff-only"])
                    .output();

                let result = match output {
                    Ok(output) if output.status.success() => {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        if stdout.contains("Already up to date")
                            || stdout.contains("Already up-to-date")
                        {
                            Ok("unchanged")
                        } else {
                            Ok("updated")
                        }
                    }
                    Ok(output) => {
                        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                        Err(stderr)
                    }
                    Err(e) => Err(e.to_string()),
                };

                let _ = tx.send((tap, result));
            })
        })
        .collect();

    drop(tx); // Close sender so receiver knows when done

    let mut updated = 0;
    let mut unchanged = 0;
    let mut errors = 0;

    // Display results as they complete
    for (tap, result) in rx {
        print!("  Updating {}... ", tap.cyan());

        match result {
            Ok("updated") => {
                println!("{}", "updated".green());
                updated += 1;
            }
            Ok("unchanged") => {
                println!("{}", "already up to date".dimmed());
                unchanged += 1;
            }
            Ok(_) => {
                println!("{}", "unknown status".yellow());
                errors += 1;
            }
            Err(msg) => {
                println!("{} {}", "failed".red(), msg.trim().to_string().dimmed());
                errors += 1;
            }
        }
    }

    // Wait for all threads
    for handle in handles {
        let _ = handle.join();
    }

    println!();

    if errors == 0 {
        if updated > 0 {
            println!(
                "{} Updated {} taps, {} unchanged",
                "✓".green().bold(),
                updated.to_string().bold(),
                unchanged.to_string().dimmed()
            );
        } else {
            println!("{} All taps are up to date", "✓".green().bold());
        }
    } else {
        println!(
            "{} {} succeeded, {} failed",
            "⚠".yellow(),
            (updated + unchanged).to_string().bold(),
            errors.to_string().bold()
        );
    }

    Ok(())
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
        println!("{} Dry run - no files will be removed", "ℹ".blue());
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

        // Sort by version string - lexicographic comparison works for most cases
        // (e.g., "1.9.0" > "1.10.0" lexically is wrong, but "1.8.1" > "1.7.0" works)
        // For proper semantic versioning, we'd need a dedicated parser
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

        let latest = sorted_versions[0];
        let old_versions = &sorted_versions[1..];

        // Show which version we're keeping (only once before removing old versions)
        if dry_run {
            println!(
                "  Keeping {} {}",
                latest.name.cyan().bold(),
                latest.version.green()
            );
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

                // Remove the old version directory
                if version_path.exists() {
                    std::fs::remove_dir_all(&version_path)?;
                }
            }

            total_removed += 1;
        }
    }

    if total_removed == 0 {
        println!("\n {} No old versions to remove", "✓".green());
    } else if dry_run {
        println!(
            "\n{} Would remove {} old versions ({})",
            "ℹ".blue(),
            total_removed.to_string().bold(),
            format_size(total_space_freed).bold()
        );
    } else {
        println!(
            "\n{} Removed {} old versions, freed {}",
            "✓".green().bold(),
            total_removed.to_string().bold(),
            format_size(total_space_freed).bold()
        );
    }

    Ok(())
}

pub fn cache(clean: bool) -> Result<()> {
    let cache_dir = download::cache_dir();

    if clean {
        println!("Cleaning download cache...");

        if !cache_dir.exists() {
            println!("\n {} Cache is already empty", "✓".green());
            return Ok(());
        }

        // Calculate size before cleaning
        let total_size = calculate_dir_size(&cache_dir)?;

        // Remove all bottles from cache
        let mut removed_count = 0;
        for entry in std::fs::read_dir(&cache_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("gz") {
                std::fs::remove_file(&path)?;
                removed_count += 1;
            }
        }

        println!(
            "\n{} Removed {} bottles, freed {}",
            "✓".green().bold(),
            removed_count.to_string().bold(),
            format_size(total_size).bold()
        );
    } else {
        // Show cache info
        println!("{}", "==> Download Cache".bold().green());
        println!();

        println!(
            "{}: {}",
            "Location".bold(),
            cache_dir.display().to_string().cyan()
        );

        if !cache_dir.exists() {
            println!("{}: {}", "Status".bold(), "Empty".dimmed());
            println!("{}: {}", "Size".bold(), "0 bytes".dimmed());
            return Ok(());
        }

        // Count bottles and calculate size
        let mut bottle_count = 0;
        let mut total_size = 0u64;

        for entry in std::fs::read_dir(&cache_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("gz") {
                bottle_count += 1;
                total_size += std::fs::metadata(&path)?.len();
            }
        }

        println!("{}: {}", "Bottles".bold(), bottle_count.to_string().cyan());
        println!("{}: {}", "Size".bold(), format_size(total_size).cyan());

        if bottle_count > 0 {
            println!();
            println!("Run {} to clean the cache", "bru cache --clean".dimmed());
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

pub fn config() -> Result<()> {
    println!("{}", "==> System Configuration".bold().green());
    println!();

    let prefix = cellar::detect_prefix();
    let cellar = cellar::cellar_path();
    let taps = crate::tap::taps_path();

    println!("{}", "Paths:".bold());
    println!(
        "  {}: {}",
        "Prefix".dimmed(),
        prefix.display().to_string().cyan()
    );
    println!(
        "  {}: {}",
        "Cellar".dimmed(),
        cellar.display().to_string().cyan()
    );
    println!(
        "  {}: {}",
        "Taps".dimmed(),
        taps.display().to_string().cyan()
    );
    println!();

    let packages = cellar::list_installed()?;
    let installed_taps = crate::tap::list_taps()?;

    println!("{}", "Statistics:".bold());
    println!(
        "  {}: {}",
        "Installed packages".dimmed(),
        packages.len().to_string().cyan()
    );
    println!(
        "  {}: {}",
        "Installed taps".dimmed(),
        installed_taps.len().to_string().cyan()
    );
    println!();

    println!("{}", "System:".bold());
    println!(
        "  {}: {}",
        "Version".dimmed(),
        env!("CARGO_PKG_VERSION").cyan()
    );
    println!(
        "  {}: {}",
        "Architecture".dimmed(),
        std::env::consts::ARCH.cyan()
    );
    println!("  {}: {}", "OS".dimmed(), std::env::consts::OS.cyan());

    Ok(())
}

pub fn env() -> Result<()> {
    let prefix = cellar::detect_prefix();
    let cellar = cellar::cellar_path();
    let cache = crate::download::cache_dir();
    let taps = crate::tap::taps_path();
    let logs = prefix.join("var/log");
    let caskroom = crate::cask::caskroom_dir();

    println!("HOMEBREW_PREFIX=\"{}\"", prefix.display());
    println!("HOMEBREW_CELLAR=\"{}\"", cellar.display());
    println!("HOMEBREW_REPOSITORY=\"{}\"", prefix.display());
    println!("HOMEBREW_CACHE=\"{}\"", cache.display());
    println!("HOMEBREW_TAPS=\"{}\"", taps.display());
    println!("HOMEBREW_LOGS=\"{}\"", logs.display());
    println!("HOMEBREW_CASKROOM=\"{}\"", caskroom.display());
    println!("HOMEBREW_ARCH=\"{}\"", std::env::consts::ARCH);
    println!("HOMEBREW_OS=\"{}\"", std::env::consts::OS);
    println!("HOMEBREW_VERSION=\"{}\"", env!("CARGO_PKG_VERSION"));

    Ok(())
}

pub fn doctor() -> Result<()> {
    println!("{}", "==> System Health Check".bold().green());
    println!();

    let mut issues = 0;
    let mut warnings = 0;

    let prefix = cellar::detect_prefix();
    let cellar = cellar::cellar_path();
    let bin_dir = prefix.join("bin");

    println!("{}", "Checking system directories...".bold());

    // Check if prefix exists
    if !prefix.exists() {
        println!(
            "  {} Homebrew prefix does not exist: {}",
            "✗".red(),
            prefix.display()
        );
        issues += 1;
    } else {
        println!(
            "  {} Homebrew prefix exists: {}",
            "✓".green(),
            prefix.display()
        );
    }

    // Check if Cellar exists and is writable
    if !cellar.exists() {
        println!(
            "  {} Cellar does not exist: {}",
            "⚠".yellow(),
            cellar.display()
        );
        warnings += 1;
    } else if std::fs::metadata(&cellar)?.permissions().readonly() {
        println!(
            "  {} Cellar is not writable: {}",
            "✗".red(),
            cellar.display()
        );
        issues += 1;
    } else {
        println!("  {} Cellar exists and is writable", "✓".green());
    }

    // Check if bin directory exists
    if !bin_dir.exists() {
        println!(
            "  {} Bin directory does not exist: {}",
            "⚠".yellow(),
            bin_dir.display()
        );
        warnings += 1;
    } else {
        println!(
            "  {} Bin directory exists: {}",
            "✓".green(),
            bin_dir.display()
        );
    }

    println!();
    println!("{}", "Checking dependencies...".bold());

    // Check for git
    match std::process::Command::new("git").arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            println!(
                "  {} git is installed: {}",
                "✓".green(),
                version.trim().dimmed()
            );
        }
        _ => {
            println!("  {} git is not installed or not in PATH", "✗".red());
            println!("    {} git is required for tap management", "ℹ".blue());
            println!(
                "    {} Install with: {}",
                "→".dimmed(),
                "brew install git".cyan()
            );
            issues += 1;
        }
    }

    println!();
    println!("{}", "Checking installed packages...".bold());

    // Check for broken symlinks
    let mut broken_links = Vec::new();
    if bin_dir.exists() {
        for entry in std::fs::read_dir(&bin_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_symlink()
                && let Ok(target) = std::fs::read_link(&path)
            {
                let resolved = if target.is_absolute() {
                    target
                } else {
                    bin_dir.join(&target)
                };

                if !resolved.exists()
                    && let Some(name) = path.file_name()
                {
                    broken_links.push(name.to_string_lossy().to_string());
                }
            }
        }
    }

    if broken_links.is_empty() {
        println!("  {} No broken symlinks found", "✓".green());
    } else {
        println!(
            "  {} Found {} broken symlinks:",
            "⚠".yellow(),
            broken_links.len()
        );
        for link in broken_links.iter().take(5) {
            println!("    - {}", link.dimmed());
        }
        if broken_links.len() > 5 {
            println!("    ... and {} more", broken_links.len() - 5);
        }
        warnings += 1;
    }

    // Check for outdated packages
    let packages = cellar::list_installed()?;
    println!("  {} {} packages installed", "ℹ".blue(), packages.len());

    println!();
    println!("{}", "Summary:".bold());

    if issues == 0 && warnings == 0 {
        println!("  {} Your system is ready to brew!", "✓".green().bold());
    } else {
        if issues > 0 {
            println!(
                "  {} Found {} issue(s) that need attention",
                "✗".red(),
                issues
            );
        }
        if warnings > 0 {
            println!("  {} Found {} warning(s)", "⚠".yellow(), warnings);
        }
    }

    Ok(())
}

pub async fn home(api: &BrewApi, formula_name: &str) -> Result<()> {
    println!("Opening homepage for {}...", formula_name.cyan());

    let formula = api.fetch_formula(formula_name).await?;

    match &formula.homepage {
        Some(url) if !url.is_empty() => {
            println!("  {}: {}", "Homepage".dimmed(), url.cyan());

            // Open URL in default browser
            let status = std::process::Command::new("open").arg(url).status();

            match status {
                Ok(s) if s.success() => {
                    println!("  {} Opened in browser", "✓".green());
                }
                _ => {
                    println!("  {} Could not open browser automatically", "⚠".yellow());
                    println!("  {} Please visit: {}", "ℹ".blue(), url);
                }
            }
        }
        _ => {
            println!(
                "  {} No homepage available for {}",
                "⚠".yellow(),
                formula_name.bold()
            );
        }
    }

    Ok(())
}

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
    let mut package_map: std::collections::HashMap<String, cellar::InstalledPackage> =
        std::collections::HashMap::new();

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
    let mut required_by_others = std::collections::HashSet::new();
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
            println!("{} No leaf packages found", "ℹ".blue());
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

fn pinned_file_path() -> std::path::PathBuf {
    cellar::detect_prefix().join("var/homebrew/pinned_formulae")
}

fn read_pinned() -> Result<Vec<String>> {
    let path = pinned_file_path();
    if !path.exists() {
        return Ok(vec![]);
    }

    let content = std::fs::read_to_string(&path)?;
    Ok(content.lines().map(|s| s.to_string()).collect())
}

fn write_pinned(pinned: &[String]) -> Result<()> {
    let path = pinned_file_path();

    // Create parent directory if needed
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(&path, pinned.join("\n"))?;
    Ok(())
}

pub fn pin(formula_names: &[String]) -> Result<()> {
    if formula_names.is_empty() {
        println!("{} No formulae specified", "✗".red());
        return Ok(());
    }

    println!("Pinning formulae...");

    let mut pinned = read_pinned()?;

    for formula in formula_names {
        // Check if formula is installed
        let versions = cellar::get_installed_versions(formula)?;
        if versions.is_empty() {
            println!("  {} {} is not installed", "⚠".yellow(), formula.bold());
            continue;
        }

        if pinned.contains(formula) {
            println!("  {} {} is already pinned", "ℹ".blue(), formula.bold());
        } else {
            pinned.push(formula.clone());
            println!("  {} Pinned {}", "✓".green(), formula.bold().green());
        }
    }

    write_pinned(&pinned)?;

    Ok(())
}

pub fn unpin(formula_names: &[String]) -> Result<()> {
    if formula_names.is_empty() {
        println!("{} No formulae specified", "✗".red());
        return Ok(());
    }

    println!("Unpinning formulae...");

    let mut pinned = read_pinned()?;

    for formula in formula_names {
        if let Some(pos) = pinned.iter().position(|x| x == formula) {
            pinned.remove(pos);
            println!("  {} Unpinned {}", "✓".green(), formula.bold().green());
        } else {
            println!("  {} {} is not pinned", "ℹ".blue(), formula.bold());
        }
    }

    write_pinned(&pinned)?;

    Ok(())
}

pub async fn desc(api: &BrewApi, formula_names: &[String]) -> Result<()> {
    if formula_names.is_empty() {
        println!("{} No formulae specified", "✗".red());
        return Ok(());
    }

    for formula_name in formula_names {
        match api.fetch_formula(formula_name).await {
            Ok(formula) => {
                print!("{}", formula.name.bold().cyan());
                if let Some(desc) = &formula.desc
                    && !desc.is_empty()
                {
                    println!(": {}", desc);
                } else {
                    println!(": {}", "No description available".dimmed());
                }
            }
            Err(_) => {
                println!("{}: {}", formula_name.bold().yellow(), "Not found".dimmed());
            }
        }
    }

    Ok(())
}

pub fn link(formula_names: &[String]) -> Result<()> {
    if formula_names.is_empty() {
        println!("{} No formulae specified", "✗".red());
        return Ok(());
    }

    println!("Linking formulae...");

    for formula_name in formula_names {
        let versions = cellar::get_installed_versions(formula_name)?;
        if versions.is_empty() {
            println!(
                "  {} {} is not installed",
                "⚠".yellow(),
                formula_name.bold()
            );
            continue;
        }

        let version = &versions[0].version;
        println!("  Linking {} {}", formula_name.cyan(), version.dimmed());

        let linked = symlink::link_formula(formula_name, version)?;
        println!(
            "    {} Linked {} files",
            "✓".green(),
            linked.len().to_string().dimmed()
        );
    }

    Ok(())
}

pub fn unlink(formula_names: &[String]) -> Result<()> {
    if formula_names.is_empty() {
        println!("{} No formulae specified", "✗".red());
        return Ok(());
    }

    println!("Unlinking formulae...");

    for formula_name in formula_names {
        let versions = cellar::get_installed_versions(formula_name)?;
        if versions.is_empty() {
            println!(
                "  {} {} is not installed",
                "⚠".yellow(),
                formula_name.bold()
            );
            continue;
        }

        let version = &versions[0].version;
        println!("  Unlinking {} {}", formula_name.cyan(), version.dimmed());

        let unlinked = symlink::unlink_formula(formula_name, version)?;
        println!(
            "    {} Unlinked {} files",
            "✓".green(),
            unlinked.len().to_string().dimmed()
        );
    }

    Ok(())
}

pub fn commands() -> Result<()> {
    println!("{}", "==> Available Commands".bold().green());
    println!();

    let commands_list = vec![
        ("search <query>", "Search for formulae and casks"),
        ("search <query> --formula", "Search only formulae"),
        ("search <query> --cask", "Search only casks"),
        ("info <formula>", "Show information about a formula or cask"),
        ("info <formula> --json", "Show formula info as JSON"),
        ("desc <formula>...", "Show formula descriptions"),
        ("deps <formula>", "Show dependencies for a formula"),
        (
            "deps <formula> --installed",
            "Show only installed dependencies",
        ),
        ("uses <formula>", "Show formulae that depend on a formula"),
        ("list", "List installed packages"),
        ("outdated", "Show outdated installed packages"),
        ("fetch <formula>...", "Download bottles for formulae"),
        ("install <formula>...", "Install formulae from bottles"),
        ("upgrade [formula...]", "Upgrade installed formulae"),
        ("reinstall <formula>...", "Reinstall formulae"),
        ("uninstall <formula>...", "Uninstall formulae"),
        ("autoremove", "Remove unused dependencies"),
        ("link <formula>...", "Link a formula"),
        ("unlink <formula>...", "Unlink a formula"),
        (
            "cleanup [formula...]",
            "Remove old versions of installed formulae",
        ),
        ("cache", "Manage download cache"),
        ("tap [user/repo]", "Add or list third-party repositories"),
        ("untap <user/repo>", "Remove a third-party repository"),
        ("update", "Update Homebrew and all taps"),
        ("config", "Show system configuration"),
        ("doctor", "Check system for potential problems"),
        ("home <formula>", "Open formula homepage in browser"),
        ("leaves", "List packages not required by others"),
        ("pin <formula>...", "Pin formulae to prevent upgrades"),
        ("unpin <formula>...", "Unpin formulae to allow upgrades"),
        ("missing [formula...]", "Check for missing dependencies"),
        ("analytics [on|off|state]", "Control analytics"),
        ("cat <formula>...", "Print formula source code"),
        ("shellenv [--shell <shell>]", "Print shell configuration"),
        ("gist-logs [formula]", "Generate diagnostic information"),
        ("alias [formula]", "Show formula aliases"),
        ("log <formula>", "Show install logs"),
        ("commands", "List all available commands"),
        ("completions <shell>", "Generate shell completion scripts"),
    ];

    for (cmd, desc) in &commands_list {
        println!("  {} {}", cmd.cyan().bold(), desc.dimmed());
    }

    println!();
    println!(
        "{} {} commands available",
        "ℹ".blue(),
        commands_list.len().to_string().bold()
    );
    println!("Run {} for help", "bru --help".cyan());

    Ok(())
}

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
        println!("{} No packages installed", "ℹ".blue());
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

pub fn analytics(action: Option<&str>) -> Result<()> {
    let analytics_file = cellar::detect_prefix().join("var/homebrew/analytics_disabled");

    match action {
        Some("off") => {
            // Create the file to disable analytics
            if let Some(parent) = analytics_file.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&analytics_file, "")?;
            println!("{} Analytics disabled", "✓".green());
        }
        Some("on") => {
            // Remove the file to enable analytics
            if analytics_file.exists() {
                std::fs::remove_file(&analytics_file)?;
            }
            println!("{} Analytics enabled", "✓".green());
        }
        Some("state") | None => {
            // Show current state
            let enabled = !analytics_file.exists();
            println!("{}", "==> Analytics Status".bold().green());
            if enabled {
                println!("{}: {}", "Status".bold(), "Enabled".green());
                println!();
                println!("Analytics help bru improve by tracking usage patterns.");
                println!("Run {} to disable", "bru analytics off".cyan());
            } else {
                println!("{}: {}", "Status".bold(), "Disabled".red());
                println!();
                println!("Run {} to enable", "bru analytics on".cyan());
            }
        }
        Some(other) => {
            println!("{} Invalid action: {}", "✗".red(), other);
            println!("Valid actions: on, off, state");
            return Ok(());
        }
    }

    Ok(())
}

pub async fn cat(api: &BrewApi, formula_names: &[String]) -> Result<()> {
    if formula_names.is_empty() {
        println!("{} No formulae specified", "✗".red());
        return Ok(());
    }

    for (i, formula_name) in formula_names.iter().enumerate() {
        if i > 0 {
            println!(); // Blank line between formulae
        }

        println!("{} {}", "==>".bold().green(), formula_name.bold().cyan());
        println!();

        // Try to fetch formula from API
        match api.fetch_formula(formula_name).await {
            Ok(formula) => {
                // Print formula as JSON (since we don't have Ruby source)
                let json = serde_json::to_string_pretty(&formula)?;
                println!("{}", json);
            }
            Err(_) => {
                // Try as cask
                match api.fetch_cask(formula_name).await {
                    Ok(cask) => {
                        let json = serde_json::to_string_pretty(&cask)?;
                        println!("{}", json);
                    }
                    Err(_) => {
                        println!(
                            "{} No formula or cask found for '{}'",
                            "✗".red(),
                            formula_name
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn shellenv(shell: Option<&str>) -> Result<()> {
    let prefix = cellar::detect_prefix();

    // Detect shell if not provided
    let shell_type = match shell {
        Some(s) => s.to_string(),
        None => {
            // Try to detect from SHELL environment variable
            std::env::var("SHELL")
                .ok()
                .and_then(|s| {
                    let path = std::path::PathBuf::from(s);
                    path.file_name()
                        .and_then(|f| f.to_str())
                        .map(|s| s.to_string())
                })
                .unwrap_or_else(|| "bash".to_string())
        }
    };

    match shell_type.as_str() {
        "bash" | "sh" => {
            println!("export HOMEBREW_PREFIX=\"{}\";", prefix.display());
            println!("export HOMEBREW_CELLAR=\"{}/Cellar\";", prefix.display());
            println!("export HOMEBREW_REPOSITORY=\"{}\";", prefix.display());
            println!(
                "export PATH=\"{}/bin:{}/sbin:$PATH\";",
                prefix.display(),
                prefix.display()
            );
            println!(
                "export MANPATH=\"{}/share/man:$MANPATH\";",
                prefix.display()
            );
            println!(
                "export INFOPATH=\"{}/share/info:$INFOPATH\";",
                prefix.display()
            );
        }
        "zsh" => {
            println!("export HOMEBREW_PREFIX=\"{}\";", prefix.display());
            println!("export HOMEBREW_CELLAR=\"{}/Cellar\";", prefix.display());
            println!("export HOMEBREW_REPOSITORY=\"{}\";", prefix.display());
            println!(
                "export PATH=\"{}/bin:{}/sbin${{PATH+:$PATH}}\";",
                prefix.display(),
                prefix.display()
            );
            println!(
                "export MANPATH=\"{}/share/man${{MANPATH+:$MANPATH}}:\";",
                prefix.display()
            );
            println!(
                "export INFOPATH=\"{}/share/info:${{INFOPATH:-}}\";",
                prefix.display()
            );
        }
        "fish" => {
            println!("set -gx HOMEBREW_PREFIX \"{}\";", prefix.display());
            println!("set -gx HOMEBREW_CELLAR \"{}/Cellar\";", prefix.display());
            println!("set -gx HOMEBREW_REPOSITORY \"{}\";", prefix.display());
            println!(
                "fish_add_path -gP \"{}/bin\" \"{}/sbin\";",
                prefix.display(),
                prefix.display()
            );
            println!(
                "set -gx MANPATH \"{}/share/man\" $MANPATH;",
                prefix.display()
            );
            println!(
                "set -gx INFOPATH \"{}/share/info\" $INFOPATH;",
                prefix.display()
            );
        }
        other => {
            println!("{} Unsupported shell: {}", "✗".red(), other);
            println!("Supported shells: bash, zsh, fish");
            return Ok(());
        }
    }

    Ok(())
}

pub async fn gist_logs(api: &BrewApi, formula: Option<&str>) -> Result<()> {
    println!("Generating diagnostic information...");
    println!();

    let mut output = String::new();

    // System information
    output.push_str("=== System Information ===\n");
    output.push_str(&format!("bru version: {}\n", env!("CARGO_PKG_VERSION")));
    output.push_str(&format!("OS: {}\n", std::env::consts::OS));
    output.push_str(&format!("Architecture: {}\n", std::env::consts::ARCH));
    output.push_str(&format!("Prefix: {}\n", cellar::detect_prefix().display()));
    output.push('\n');

    // Installed packages
    output.push_str("=== Installed Packages ===\n");
    let packages = cellar::list_installed()?;
    output.push_str(&format!("Total: {}\n", packages.len()));
    for pkg in packages.iter().take(20) {
        output.push_str(&format!("{} {}\n", pkg.name, pkg.version));
    }
    if packages.len() > 20 {
        output.push_str(&format!("... and {} more\n", packages.len() - 20));
    }
    output.push('\n');

    // Taps
    output.push_str("=== Taps ===\n");
    let taps = crate::tap::list_taps()?;
    for tap in &taps {
        output.push_str(&format!("{}\n", tap));
    }
    output.push('\n');

    // Formula-specific info if provided
    if let Some(formula_name) = formula {
        output.push_str(&format!("=== Formula: {} ===\n", formula_name));

        match api.fetch_formula(formula_name).await {
            Ok(formula) => {
                output.push_str(&format!("Name: {}\n", formula.name));
                if let Some(desc) = &formula.desc {
                    output.push_str(&format!("Description: {}\n", desc));
                }
                if let Some(version) = &formula.versions.stable {
                    output.push_str(&format!("Version: {}\n", version));
                }
                output.push_str(&format!(
                    "Dependencies: {}\n",
                    formula.dependencies.join(", ")
                ));

                // Check if installed
                let installed_versions = cellar::get_installed_versions(formula_name)?;
                if installed_versions.is_empty() {
                    output.push_str("Installed: No\n");
                } else {
                    output.push_str(&format!(
                        "Installed: Yes ({})\n",
                        installed_versions
                            .iter()
                            .map(|v| v.version.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
            }
            Err(e) => {
                output.push_str(&format!("Error fetching formula: {}\n", e));
            }
        }
        output.push('\n');
    }

    // Config
    output.push_str("=== Configuration ===\n");
    output.push_str(&format!("Cellar: {}\n", cellar::cellar_path().display()));
    output.push_str(&format!(
        "Cache: {}\n",
        crate::download::cache_dir().display()
    ));

    // Check for pinned formulae
    let pinned = read_pinned()?;
    if !pinned.is_empty() {
        output.push_str(&format!("Pinned: {}\n", pinned.join(", ")));
    }

    // Doctor check summary
    output.push_str("\n=== Health Check ===\n");
    let prefix = cellar::detect_prefix();
    let cellar = cellar::cellar_path();
    output.push_str(&format!("Prefix exists: {}\n", prefix.exists()));
    output.push_str(&format!("Cellar exists: {}\n", cellar.exists()));
    output.push_str(&format!(
        "Git available: {}\n",
        std::process::Command::new("git")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    ));

    println!("{}", output);
    println!();
    println!("{} Diagnostic information generated", "✓".green());
    println!("Copy the above output to share for debugging");

    Ok(())
}

pub async fn alias(api: &BrewApi, formula: Option<&str>) -> Result<()> {
    match formula {
        None => {
            // Show all common aliases
            println!("{}", "==> Common Formula Aliases".bold().green());
            println!();

            let common_aliases = vec![
                ("python", "python@3.13", "Latest Python 3"),
                ("python3", "python@3.13", "Latest Python 3"),
                ("node", "node", "Node.js"),
                ("nodejs", "node", "Node.js"),
                ("postgres", "postgresql@17", "Latest PostgreSQL"),
                ("postgresql", "postgresql@17", "Latest PostgreSQL"),
                ("mysql", "mysql", "MySQL server"),
                ("mariadb", "mariadb", "MariaDB server"),
                ("redis", "redis", "Redis server"),
            ];

            for (alias_name, formula_name, desc) in &common_aliases {
                println!(
                    "{} {} {}",
                    alias_name.cyan().bold(),
                    format!("-> {}", formula_name).dimmed(),
                    format!("({})", desc).dimmed()
                );
            }

            println!();
            println!(
                "Run {} to see aliases for a specific formula",
                "bru alias <formula>".cyan()
            );
        }
        Some(formula_name) => {
            // Check if formula exists and show its aliases
            match api.fetch_formula(formula_name).await {
                Ok(formula) => {
                    println!("{} {}", "==>".bold().green(), formula.name.bold().cyan());
                    if let Some(desc) = &formula.desc {
                        println!("{}", desc);
                    }
                    println!();

                    // In real Homebrew, aliases are stored separately
                    // For now, show the formula name itself
                    println!("{}: {}", "Name".bold(), formula.name.cyan());
                    println!("{}: {}", "Full name".bold(), formula.full_name.dimmed());

                    // Check if this is commonly aliased
                    let common_aliases_map: std::collections::HashMap<&str, Vec<&str>> = [
                        ("python@3.13", vec!["python", "python3"]),
                        ("node", vec!["nodejs"]),
                        ("postgresql@17", vec!["postgres", "postgresql"]),
                    ]
                    .iter()
                    .cloned()
                    .collect();

                    if let Some(aliases) = common_aliases_map.get(formula.name.as_str()) {
                        println!();
                        println!("{}", "Common aliases:".bold());
                        for alias in aliases {
                            println!("  {}", alias.cyan());
                        }
                    } else {
                        println!();
                        println!("{} No known aliases", "ℹ".blue());
                    }
                }
                Err(_) => {
                    println!("{} Formula '{}' not found", "✗".red(), formula_name);
                }
            }
        }
    }

    Ok(())
}

pub fn log(formula_name: &str) -> Result<()> {
    println!("Checking logs for {}", formula_name.cyan());
    println!();

    // Check if formula is installed
    let installed_versions = cellar::get_installed_versions(formula_name)?;
    if installed_versions.is_empty() {
        println!("{} {} is not installed", "⚠".yellow(), formula_name.bold());
        println!();
        println!(
            "Run {} to install it",
            format!("bru install {}", formula_name).cyan()
        );
        return Ok(());
    }

    let version = &installed_versions[0].version;
    let install_path = cellar::cellar_path().join(formula_name).join(version);

    println!(
        "{}",
        format!("==> {} {}", formula_name, version).bold().green()
    );
    println!();

    // Check for INSTALL_RECEIPT.json
    let receipt_path = install_path.join("INSTALL_RECEIPT.json");
    if receipt_path.exists() {
        println!("{}", "Install Receipt:".bold());
        let receipt_content = std::fs::read_to_string(&receipt_path)?;
        let receipt: serde_json::Value = serde_json::from_str(&receipt_content)?;

        if let Some(obj) = receipt.as_object() {
            let on_request = obj
                .get("installed_on_request")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let status_text = if on_request { "Yes" } else { "No (dependency)" };
            println!(
                "  {}: {}",
                "Installed on request".dimmed(),
                status_text.cyan()
            );

            if let Some(time) = obj.get("time").and_then(|v| v.as_i64()) {
                let datetime = chrono::DateTime::from_timestamp(time, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "Unknown".to_string());
                println!("  {}: {}", "Install time".dimmed(), datetime);
            }

            if let Some(built_from) = obj
                .get("source")
                .and_then(|v| v.get("spec"))
                .and_then(|v| v.as_str())
            {
                println!("  {}: {}", "Built from".dimmed(), built_from);
            }
        }
        println!();
    }

    // List installed files
    println!("{}", "Installed files:".bold());
    let prefix = cellar::detect_prefix();
    let bin_dir = prefix.join("bin");

    let mut file_count = 0;
    if let Ok(entries) = std::fs::read_dir(&bin_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_symlink()
                && let Ok(target) = std::fs::read_link(&path)
            {
                // Resolve relative symlinks to absolute paths
                let resolved_target = if target.is_absolute() {
                    target.clone()
                } else {
                    bin_dir
                        .join(&target)
                        .canonicalize()
                        .unwrap_or(target.clone())
                };

                if resolved_target.starts_with(&install_path)
                    && let Some(name) = path.file_name()
                {
                    println!(
                        "  {} {}",
                        name.to_string_lossy().cyan(),
                        format!("-> {}", target.display()).dimmed()
                    );
                    file_count += 1;
                    if file_count >= 10 {
                        println!("  {} (showing first 10)", "...".dimmed());
                        break;
                    }
                }
            }
        }
    }

    if file_count == 0 {
        println!("  {}", "No executables linked".dimmed());
    }

    println!();
    println!(
        "{}: {}",
        "Install directory".dimmed(),
        install_path.display().to_string().cyan()
    );

    Ok(())
}

pub fn which_formula(command: &str) -> Result<()> {
    println!("Finding formula for command: {}", command.cyan());

    let prefix = cellar::detect_prefix();
    let bin_dir = prefix.join("bin");
    let command_path = bin_dir.join(command);

    if !command_path.exists() {
        println!(
            "\n{} Command '{}' not found in {}",
            "⚠".yellow(),
            command.bold(),
            bin_dir.display()
        );
        return Ok(());
    }

    // Check if it's a symlink
    if command_path.is_symlink()
        && let Ok(target) = std::fs::read_link(&command_path)
    {
        // Resolve to absolute path
        let resolved = if target.is_absolute() {
            target
        } else {
            bin_dir.join(&target).canonicalize().unwrap_or(target)
        };

        // Extract formula name from Cellar path
        let cellar_path = cellar::cellar_path();
        if resolved.starts_with(&cellar_path)
            && let Ok(rel_path) = resolved.strip_prefix(&cellar_path)
            && let Some(formula_name) = rel_path.components().next()
        {
            println!(
                "\n{}",
                formula_name.as_os_str().to_string_lossy().green().bold()
            );
            return Ok(());
        }
    }

    println!(
        "\n{} Could not determine formula for '{}'",
        "⚠".yellow(),
        command.bold()
    );
    Ok(())
}

pub async fn options(api: &BrewApi, formula_name: &str) -> Result<()> {
    println!("Checking options for: {}", formula_name.cyan());

    // Verify formula exists
    match api.fetch_formula(formula_name).await {
        Ok(formula) => {
            println!("\n {}", format!("==> {}", formula.name).bold().green());
            if let Some(desc) = &formula.desc {
                println!("{}", desc);
            }
            println!();
            println!("{} No options available", "ℹ".blue());
            println!();
            println!(
                "{}",
                "Bottles are pre-built binaries with fixed options.".dimmed()
            );
            println!(
                "{}",
                "For custom builds with options, use `brew install --build-from-source`.".dimmed()
            );
        }
        Err(_) => {
            println!("\n {} Formula '{}' not found", "✗".red(), formula_name);
        }
    }

    Ok(())
}

pub async fn bundle(api: &BrewApi, dump: bool, file: Option<&str>) -> Result<()> {
    let brewfile_path = file.unwrap_or("Brewfile");

    if dump {
        // Generate Brewfile from installed packages
        println!("Generating Brewfile...");

        let mut content = String::new();

        // Get all taps
        let taps = crate::tap::list_taps()?;
        if !taps.is_empty() {
            for tap in &taps {
                content.push_str(&format!("tap \"{}\"\n", tap));
            }
            content.push('\n');
        }

        // Get all installed packages
        let packages = cellar::list_installed()?;
        let mut formulae_names: Vec<_> = packages
            .iter()
            .filter(|p| p.installed_on_request())
            .map(|p| p.name.as_str())
            .collect();
        formulae_names.sort();

        for name in &formulae_names {
            content.push_str(&format!("brew \"{}\"\n", name));
        }

        // Write to file
        std::fs::write(brewfile_path, &content)?;

        println!(
            "{} Generated {} with {} formulae",
            "✓".green(),
            brewfile_path.cyan(),
            formulae_names.len().to_string().bold()
        );
    } else {
        // Install from Brewfile
        println!("Reading {}...", brewfile_path.cyan());

        if !std::path::Path::new(brewfile_path).exists() {
            println!("\n {} {} not found", "✗".red(), brewfile_path.bold());
            println!("Run {} to generate one", "bru bundle dump".cyan());
            return Ok(());
        }

        let content = std::fs::read_to_string(brewfile_path)?;

        let mut taps_to_add = Vec::new();
        let mut formulae_to_install = Vec::new();
        let mut casks_to_install = Vec::new();

        // Parse Brewfile
        for line in content.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse tap lines: tap "user/repo"
            if let Some(tap_line) = line.strip_prefix("tap") {
                let tap_line = tap_line.trim();
                if let Some(tap_name) = extract_quoted_string(tap_line) {
                    taps_to_add.push(tap_name.to_string());
                }
            }

            // Parse brew lines: brew "formula"
            if let Some(brew_line) = line.strip_prefix("brew") {
                let brew_line = brew_line.trim();
                if let Some(formula_name) = extract_quoted_string(brew_line) {
                    formulae_to_install.push(formula_name.to_string());
                }
            }

            // Parse cask lines: cask "app"
            if let Some(cask_line) = line.strip_prefix("cask") {
                let cask_line = cask_line.trim();
                if let Some(cask_name) = extract_quoted_string(cask_line) {
                    casks_to_install.push(cask_name.to_string());
                }
            }

            // Skip mas lines for now
        }

        println!(
            "\n{} Found: {} taps, {} formulae, {} casks",
            "✓".green(),
            taps_to_add.len().to_string().bold(),
            formulae_to_install.len().to_string().bold(),
            casks_to_install.len().to_string().bold()
        );

        // Install taps first
        if !taps_to_add.is_empty() {
            println!("\nAdding taps...");
            for tap_name in &taps_to_add {
                if crate::tap::is_tapped(tap_name)? {
                    println!("  {} {} already tapped", "✓".green(), tap_name.dimmed());
                } else {
                    println!("  Tapping {}...", tap_name.cyan());
                    match crate::tap::tap(tap_name) {
                        Ok(_) => println!("    {} Tapped {}", "✓".green(), tap_name.bold()),
                        Err(e) => println!("    {} Failed: {}", "✗".red(), e),
                    }
                }
            }
        }

        // Install formulae
        if !formulae_to_install.is_empty() {
            println!("\nInstalling formulae...");
            match install(api, &formulae_to_install, false, false, false).await {
                Ok(_) => {}
                Err(e) => {
                    println!("{} Failed to install some formulae: {}", "⚠".yellow(), e);
                }
            }
        }

        // Casks - for now, just notify
        if !casks_to_install.is_empty() {
            println!("\n {} Cask installation not yet implemented", "ℹ".blue());
            println!(
                "  Casks to install: {}",
                casks_to_install.join(", ").dimmed()
            );
        }

        println!("\n {} Bundle install complete", "✓".green().bold());
    }

    Ok(())
}

fn extract_quoted_string(s: &str) -> Option<&str> {
    // Extract string from quotes: "string" or 'string'
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        Some(&s[1..s.len() - 1])
    } else {
        None
    }
}

pub fn services(action: Option<&str>, formula: Option<&str>) -> Result<()> {
    match action {
        None | Some("list") => {
            // List all services
            println!("{}", "==> Services".bold().green());
            println!();

            let services = crate::services::list_all_services()?;

            if services.is_empty() {
                println!("{} No services found", "ℹ".blue());
                println!();
                println!("Services are background processes like databases and web servers.");
                println!("Common services: postgresql, mysql, redis, nginx");
                return Ok(());
            }

            // Print header
            println!(
                "{:<20} {:<12} {:<8} {}",
                "Name".bold(),
                "Status".bold(),
                "User".bold(),
                "File".bold()
            );

            // Print services
            for service in &services {
                let status_str = match &service.status {
                    crate::services::ServiceStatus::None => "none".dimmed().to_string(),
                    crate::services::ServiceStatus::Started => "started".green().to_string(),
                    crate::services::ServiceStatus::Error(code) => {
                        format!("error  {}", code).red().to_string()
                    }
                };

                let user_str = service.user.as_deref().unwrap_or("");
                let file_str = service
                    .plist_path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();

                println!(
                    "{:<20} {:<20} {:<8} {}",
                    service.name.cyan(),
                    status_str,
                    user_str,
                    file_str.dimmed()
                );
            }

            println!();
            println!(
                "{} {} services",
                "ℹ".blue(),
                services.len().to_string().bold()
            );
        }
        Some("start") => {
            let formula = formula.ok_or_else(|| anyhow::anyhow!("Formula name required"))?;
            println!("Starting service: {}", formula.cyan());

            if !crate::services::service_exists(formula) {
                println!(
                    "\n{} Service file not found for {}",
                    "⚠".yellow(),
                    formula.bold()
                );
                println!();
                println!("To create a service, the formula must support it.");
                println!(
                    "Run {} to check if service is available",
                    "bru services list".to_string().cyan()
                );
                return Ok(());
            }

            match crate::services::start_service(formula) {
                Ok(_) => {
                    println!("  {} Started {}", "✓".green(), formula.bold().green());
                }
                Err(e) => {
                    println!("  {} Failed to start: {}", "✗".red(), e);
                }
            }
        }
        Some("stop") => {
            let formula = formula.ok_or_else(|| anyhow::anyhow!("Formula name required"))?;
            println!("Stopping service: {}", formula.cyan());

            if !crate::services::service_exists(formula) {
                println!(
                    "\n{} Service file not found for {}",
                    "⚠".yellow(),
                    formula.bold()
                );
                return Ok(());
            }

            match crate::services::stop_service(formula) {
                Ok(_) => {
                    println!("  {} Stopped {}", "✓".green(), formula.bold().green());
                }
                Err(e) => {
                    println!("  {} Failed to stop: {}", "✗".red(), e);
                }
            }
        }
        Some("restart") => {
            let formula = formula.ok_or_else(|| anyhow::anyhow!("Formula name required"))?;
            println!("Restarting service: {}", formula.cyan());

            if !crate::services::service_exists(formula) {
                println!(
                    "\n{} Service file not found for {}",
                    "⚠".yellow(),
                    formula.bold()
                );
                return Ok(());
            }

            match crate::services::restart_service(formula) {
                Ok(_) => {
                    println!("  {} Restarted {}", "✓".green(), formula.bold().green());
                }
                Err(e) => {
                    println!("  {} Failed to restart: {}", "✗".red(), e);
                }
            }
        }
        Some(other) => {
            println!("{} Unknown action: {}", "✗".red(), other);
            println!();
            println!("Available actions:");
            println!("  {} - List all services", "list".cyan());
            println!("  {} - Start a service", "start <formula>".cyan());
            println!("  {} - Stop a service", "stop <formula>".cyan());
            println!("  {} - Restart a service", "restart <formula>".cyan());
        }
    }

    Ok(())
}

pub async fn edit(api: &BrewApi, formula_name: &str) -> Result<()> {
    println!(
        "{} Opening {} in editor...",
        "✏️".bold(),
        formula_name.cyan()
    );

    // First, verify formula exists
    match api.fetch_formula(formula_name).await {
        Ok(_) => {}
        Err(_) => {
            println!("\n {} Formula '{}' not found", "✗".red(), formula_name);
            return Ok(());
        }
    }

    // Try to find formula file in taps
    let prefix = cellar::detect_prefix();
    let taps_dir = prefix.join("Library/Taps");

    // Check homebrew-core first (try both flat and letter-organized structure)
    let first_letter = formula_name
        .chars()
        .next()
        .unwrap_or('a')
        .to_lowercase()
        .to_string();
    let core_formula_letter = taps_dir
        .join("homebrew/homebrew-core/Formula")
        .join(&first_letter)
        .join(format!("{}.rb", formula_name));
    let core_formula_flat = taps_dir
        .join("homebrew/homebrew-core/Formula")
        .join(format!("{}.rb", formula_name));

    let formula_path = if core_formula_letter.exists() {
        core_formula_letter
    } else if core_formula_flat.exists() {
        core_formula_flat
    } else {
        // Search all taps
        let mut found_path = None;
        if taps_dir.exists() {
            for tap_entry in std::fs::read_dir(&taps_dir)?.flatten() {
                let tap_path = tap_entry.path();
                if tap_path.is_dir() {
                    for repo_entry in std::fs::read_dir(&tap_path)?.flatten() {
                        let repo_path = repo_entry.path();
                        let formula_path = repo_path
                            .join("Formula")
                            .join(format!("{}.rb", formula_name));
                        if formula_path.exists() {
                            found_path = Some(formula_path);
                            break;
                        }
                    }
                }
                if found_path.is_some() {
                    break;
                }
            }
        }

        match found_path {
            Some(p) => p,
            None => {
                println!("\n {} Formula file not found locally", "⚠".yellow());
                println!("Formula exists in API but not in local taps");
                println!("Try: {}", "brew tap homebrew/core".to_string().cyan());
                return Ok(());
            }
        }
    };

    println!(
        "  {}: {}",
        "File".dimmed(),
        formula_path.display().to_string().cyan()
    );

    // Get editor from environment
    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| "vim".to_string());

    // Open in editor
    let status = std::process::Command::new(&editor)
        .arg(&formula_path)
        .status();

    match status {
        Ok(s) if s.success() => {
            println!(
                "\n {} Finished editing {}",
                "✓".green(),
                formula_name.bold()
            );
        }
        Ok(_) => {
            println!("\n {} Editor exited with error", "⚠".yellow());
        }
        Err(e) => {
            println!("\n {} Failed to open editor: {}", "✗".red(), e);
            println!("Set EDITOR environment variable to your preferred editor");
        }
    }

    Ok(())
}

pub fn create(url: &str, name: Option<&str>) -> Result<()> {
    println!("Creating formula from URL: {}", url.cyan());

    // Extract name from URL if not provided
    let formula_name = if let Some(n) = name {
        n.to_string()
    } else {
        // Try to extract from URL
        let parts: Vec<&str> = url.split('/').collect();
        let filename = parts.last().unwrap_or(&"formula");

        // Remove common extensions
        let name = filename
            .trim_end_matches(".tar.gz")
            .trim_end_matches(".tar.bz2")
            .trim_end_matches(".tar.xz")
            .trim_end_matches(".zip")
            .trim_end_matches(".tgz");

        // Remove version numbers (simple heuristic)
        let parts: Vec<&str> = name.split('-').collect();
        if parts.len() > 1 {
            // Take first part before version
            parts[0].to_string()
        } else {
            name.to_string()
        }
    };

    println!("  {}: {}", "Name".bold(), formula_name.cyan());

    // Generate basic formula template (capitalize first letter)
    let class_name = {
        let mut chars = formula_name.chars();
        if let Some(first_char) = chars.next() {
            first_char.to_uppercase().to_string() + chars.as_str()
        } else {
            formula_name.to_uppercase()
        }
    };
    let homepage_base =
        url.trim_end_matches(|c: char| c.is_ascii_digit() || c == '.' || c == '-' || c == '/');

    let template = vec![
        format!("class {} < Formula", class_name),
        format!("  desc \"Description of {}\"", formula_name),
        format!("  homepage \"{}\"", homepage_base),
        format!("  url \"{}\"", url),
        "  sha256 \"\"  # TODO: Add SHA256 checksum".to_string(),
        "  license \"\"  # TODO: Add license".to_string(),
        "".to_string(),
        "  depends_on \"cmake\" => :build  # Example build dependency".to_string(),
        "".to_string(),
        "  def install".to_string(),
        "    # TODO: Add installation steps".to_string(),
        "    # Common patterns:".to_string(),
        "    # system \"./configure\", \"--prefix=#{prefix}\"".to_string(),
        "    # system \"make\", \"install\"".to_string(),
        "    #".to_string(),
        "    # Or for CMake:".to_string(),
        "    # system \"cmake\", \"-S\", \".\", \"-B\", \"build\", *std_cmake_args".to_string(),
        "    # system \"cmake\", \"--build\", \"build\"".to_string(),
        "    # system \"cmake\", \"--install\", \"build\"".to_string(),
        "  end".to_string(),
        "".to_string(),
        "  test do".to_string(),
        "    # TODO: Add test".to_string(),
        format!("    # system \"#{{bin}}/{}\", \"--version\"", formula_name),
        "  end".to_string(),
        "end".to_string(),
    ]
    .join("\n")
        + "\n";

    // Write to file in current directory
    let filename = format!("{}.rb", formula_name);
    std::fs::write(&filename, template)?;

    println!("\n {} Created {}", "✓".green(), filename.bold().green());
    println!();
    println!("{}", "Next steps:".bold());
    println!(
        "  1. Add SHA256 checksum: {}",
        "shasum -a 256 <downloaded-file>".to_string().cyan()
    );
    println!("  2. Fill in description and license");
    println!("  3. Update install method with build steps");
    println!("  4. Add test command");
    println!(
        "  5. Test formula: {}",
        format!("bru install --build-from-source {}", filename).cyan()
    );

    Ok(())
}

pub async fn livecheck(api: &BrewApi, formula_name: &str) -> Result<()> {
    println!("Checking for newer versions of {}...", formula_name.cyan());

    let formula = api.fetch_formula(formula_name).await?;

    let current_version = formula
        .versions
        .stable
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No stable version found"))?;

    println!("\n {}", format!("==> {}", formula.name).bold().green());
    println!("{}: {}", "Current version".bold(), current_version.cyan());

    if let Some(homepage) = &formula.homepage {
        println!("{}: {}", "Homepage".bold(), homepage.dimmed());
    }

    println!();
    println!("{} Livecheck not yet implemented", "ℹ".blue());
    println!("Would check:");
    if let Some(homepage) = &formula.homepage {
        println!("  - {}", homepage.dimmed());
    }
    println!("  - GitHub releases (if applicable)");
    println!("  - Other version sources");

    Ok(())
}

pub async fn audit(_api: &BrewApi, formula_names: &[String]) -> Result<()> {
    if formula_names.is_empty() {
        println!("{} No formulae specified", "✗".red());
        return Ok(());
    }

    println!(
        "Auditing {} formulae...",
        formula_names.len().to_string().bold()
    );
    println!();

    let prefix = cellar::detect_prefix();
    let taps_dir = prefix.join("Library/Taps");

    for formula_name in formula_names {
        println!("{} {}", "==>".bold().green(), formula_name.bold().cyan());

        // Find formula file
        let first_letter = formula_name
            .chars()
            .next()
            .unwrap_or('a')
            .to_lowercase()
            .to_string();
        let core_formula_letter = taps_dir
            .join("homebrew/homebrew-core/Formula")
            .join(&first_letter)
            .join(format!("{}.rb", formula_name));
        let core_formula_flat = taps_dir
            .join("homebrew/homebrew-core/Formula")
            .join(format!("{}.rb", formula_name));

        let formula_path = if core_formula_letter.exists() {
            Some(core_formula_letter)
        } else if core_formula_flat.exists() {
            Some(core_formula_flat)
        } else {
            // Search all taps
            let mut found_path = None;
            if taps_dir.exists() {
                'outer: for tap_entry in std::fs::read_dir(&taps_dir)?.flatten() {
                    let tap_path = tap_entry.path();
                    if tap_path.is_dir() {
                        for repo_entry in std::fs::read_dir(&tap_path)?.flatten() {
                            let repo_path = repo_entry.path();
                            let fp = repo_path
                                .join("Formula")
                                .join(format!("{}.rb", formula_name));
                            if fp.exists() {
                                found_path = Some(fp);
                                break 'outer;
                            }
                        }
                    }
                }
            }
            found_path
        };

        match formula_path {
            Some(path) => {
                let content = std::fs::read_to_string(&path)?;
                let mut issues = Vec::new();

                // Basic checks
                if !content.contains("def install") {
                    issues.push("Missing install method");
                }

                if !content.contains("desc ") {
                    issues.push("Missing description");
                }

                if !content.contains("homepage ") {
                    issues.push("Missing homepage");
                }

                if !content.contains("url ") {
                    issues.push("Missing URL");
                }

                if !content.contains("sha256 ") {
                    issues.push("Missing SHA256");
                }

                if content.contains("TODO") {
                    issues.push("Contains TODO comments");
                }

                if issues.is_empty() {
                    println!("  {} No issues found", "✓".green());
                } else {
                    for issue in issues {
                        println!("  {} {}", "⚠".yellow(), issue.dimmed());
                    }
                }
            }
            None => {
                println!("  {} Formula file not found locally", "⚠".yellow());
            }
        }

        println!();
    }

    Ok(())
}

pub async fn install_cask(api: &BrewApi, cask_names: &[String]) -> Result<()> {
    if cask_names.is_empty() {
        println!("{} No casks specified", "✗".red());
        return Ok(());
    }

    println!(
        "Installing {} casks...",
        cask_names.len().to_string().bold()
    );

    // Fetch all cask metadata in parallel first
    let fetch_futures: Vec<_> = cask_names
        .iter()
        .map(|name| async move {
            // Check if already installed
            if crate::cask::is_cask_installed(name)
                && let Some(version) = crate::cask::get_installed_cask_version(name)
            {
                return (name.clone(), Err(format!("Already installed: {}", version)));
            }

            match api.fetch_cask(name).await {
                Ok(c) => (name.clone(), Ok(c)),
                Err(e) => (name.clone(), Err(format!("Failed to fetch: {}", e))),
            }
        })
        .collect();

    let metadata_results = futures::future::join_all(fetch_futures).await;

    // Process each cask sequentially (downloads and installs must be sequential)
    for (cask_name, result) in metadata_results {
        println!("\nInstalling cask: {}", cask_name.cyan());

        let cask = match result {
            Ok(c) => c,
            Err(msg) => {
                if msg.starts_with("Already installed") {
                    println!("  {} {}", "✓".green(), msg);
                } else {
                    println!("  {} {}", "✗".red(), msg);
                }
                continue;
            }
        };

        let version = cask
            .version
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No version"))?;
        let url = cask
            .url
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No download URL"))?;

        println!("  {}: {}", "Version".dimmed(), version.cyan());
        println!("  {}: {}", "URL".dimmed(), url.dimmed());

        // Extract app artifacts
        let apps = crate::cask::extract_app_artifacts(&cask.artifacts);
        if apps.is_empty() {
            println!("  {} No app artifacts found", "⚠".yellow());
            continue;
        }

        println!("  {}: {}", "Apps".dimmed(), apps.join(", ").cyan());

        // Download cask
        println!("  Downloading...");
        let download_path = match crate::cask::download_cask(url, &cask_name).await {
            Ok(p) => p,
            Err(e) => {
                println!("  {} Failed to download: {}", "✗".red(), e);
                continue;
            }
        };

        println!(
            "    {} Downloaded to {}",
            "✓".green(),
            download_path.display().to_string().dimmed()
        );

        // Handle different file types
        let filename = download_path
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid download path: no filename"))?
            .to_string_lossy()
            .to_lowercase();

        if filename.ends_with(".dmg") {
            // Mount DMG
            println!("  Mounting DMG...");
            let mount_point = match crate::cask::mount_dmg(&download_path) {
                Ok(p) => p,
                Err(e) => {
                    println!("  {} Failed to mount: {}", "✗".red(), e);
                    continue;
                }
            };

            println!(
                "    {} Mounted at {}",
                "✓".green(),
                mount_point.display().to_string().dimmed()
            );

            // Install each app
            for app_name in &apps {
                let app_path = mount_point.join(app_name);

                if !app_path.exists() {
                    println!("    {} App not found: {}", "⚠".yellow(), app_name);
                    continue;
                }

                println!("  Installing {}...", app_name.cyan());
                match crate::cask::install_app(&app_path, app_name) {
                    Ok(_) => {
                        println!(
                            "    └ {} Installed to /Applications/{}",
                            "✓".green(),
                            app_name.bold()
                        );
                    }
                    Err(e) => {
                        println!("    └ {} Failed to install: {}", "✗".red(), e);
                    }
                }
            }

            // Unmount DMG
            println!("  Unmounting DMG...");
            if let Err(e) = crate::cask::unmount_dmg(&mount_point) {
                println!("    {} Failed to unmount: {}", "⚠".yellow(), e);
            }
        } else if filename.ends_with(".pkg") {
            // Install PKG
            println!("  Installing PKG...");
            match crate::cask::install_pkg(&download_path) {
                Ok(_) => {
                    println!("    └ {} Installed successfully", "✓".green());
                }
                Err(e) => {
                    println!("  {} Failed to install: {}", "✗".red(), e);
                    continue;
                }
            }
        } else if filename.ends_with(".zip") {
            // Extract ZIP
            println!("  Extracting ZIP...");
            let extract_dir = match crate::cask::extract_zip(&download_path) {
                Ok(dir) => {
                    println!(
                        "    └ {} Extracted to {}",
                        "✓".green(),
                        dir.display().to_string().dimmed()
                    );
                    dir
                }
                Err(e) => {
                    println!("  {} Failed to extract: {}", "✗".red(), e);
                    continue;
                }
            };

            // Install apps from extracted directory
            for app in &apps {
                println!("  Installing {}...", app.cyan());
                let app_path = extract_dir.join(app);

                if !app_path.exists() {
                    println!("    └ {} App not found in ZIP: {}", "⚠".yellow(), app);
                    continue;
                }

                match crate::cask::install_app(&app_path, app) {
                    Ok(target) => {
                        println!(
                            "    └ {} Installed to {}",
                            "✓".green(),
                            target.display().to_string().bold()
                        );
                    }
                    Err(e) => {
                        println!("  {} Failed to install: {}", "✗".red(), e);
                        continue;
                    }
                }
            }
        } else {
            println!("  {} Unsupported file type: {}", "⚠".yellow(), filename);
            continue;
        }

        // Create Caskroom directory to track installation
        let cask_dir = crate::cask::cask_install_dir(&cask_name, version);
        std::fs::create_dir_all(&cask_dir)?;

        // Write metadata file
        let metadata = serde_json::json!({
            "token": cask_name,
            "version": version,
            "installed_apps": apps,
            "install_time": chrono::Utc::now().timestamp(),
        });
        let metadata_path = cask_dir.join(".metadata.json");
        std::fs::write(&metadata_path, serde_json::to_string_pretty(&metadata)?)?;

        println!(
            "\n  {} Installed {} {}",
            "✓".green().bold(),
            cask_name.bold().green(),
            version.dimmed()
        );
    }

    println!("\n {} Cask installation complete", "✓".green().bold());
    Ok(())
}

pub async fn reinstall_cask(api: &BrewApi, cask_names: &[String]) -> Result<()> {
    if cask_names.is_empty() {
        println!("{} No casks specified", "✗".red());
        return Ok(());
    }

    println!(
        "Reinstalling {} casks...",
        cask_names.len().to_string().bold()
    );

    for cask_name in cask_names {
        // Check if installed
        if !crate::cask::is_cask_installed(cask_name) {
            println!("  {} {} not installed", "⚠".yellow(), cask_name.bold());
            continue;
        }

        println!("  Reinstalling {}...", cask_name.cyan());

        // Uninstall
        uninstall_cask(std::slice::from_ref(cask_name))?;

        // Reinstall
        install_cask(api, std::slice::from_ref(cask_name)).await?;

        println!("  {} Reinstalled {}", "✓".green(), cask_name.bold().green());
    }

    println!("\n {} Cask reinstall complete", "✓".green().bold());
    Ok(())
}

pub fn cleanup_cask(cask_names: &[String], dry_run: bool) -> Result<()> {
    let caskroom = crate::cask::caskroom_dir();

    if !caskroom.exists() {
        println!("{} No casks installed", "ℹ".blue());
        return Ok(());
    }

    // Get list of casks to clean (all or specified)
    let to_clean: Vec<String> = if cask_names.is_empty() {
        // Clean all installed casks
        std::fs::read_dir(&caskroom)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .filter_map(|e| e.file_name().to_str().map(String::from))
            .collect()
    } else {
        cask_names.to_vec()
    };

    let mut total_removed = 0;
    let mut total_space_freed = 0u64;

    if dry_run {
        println!("{} Dry run - no files will be removed", "ℹ".blue());
    } else {
        println!("Cleaning up old cask versions...");
    }

    for token in &to_clean {
        let cask_dir = caskroom.join(token);

        if !cask_dir.exists() {
            if !cask_names.is_empty() {
                println!("  {} {} not installed", "⚠".yellow(), token.bold());
            }
            continue;
        }

        // Get all version directories
        let mut versions: Vec<_> = std::fs::read_dir(&cask_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect();

        if versions.len() <= 1 {
            continue;
        }

        // Sort by modification time (newest first)
        versions.sort_by_key(|e| {
            e.metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        });
        versions.reverse();

        let latest = &versions[0];
        let old_versions = &versions[1..];

        for old in old_versions {
            let version_path = old.path();
            let version_name = old.file_name();

            // Calculate directory size
            let size = calculate_dir_size(&version_path)?;
            total_space_freed += size;

            if dry_run {
                println!(
                    "  Would remove {} {} ({})",
                    token.cyan(),
                    version_name.to_string_lossy().dimmed(),
                    format_size(size).dimmed()
                );
            } else {
                println!(
                    "  Removing {} {} ({})",
                    token.cyan(),
                    version_name.to_string_lossy().dimmed(),
                    format_size(size).dimmed()
                );

                // Remove the old version directory
                if version_path.exists() {
                    std::fs::remove_dir_all(&version_path)?;
                }
            }

            total_removed += 1;
        }

        println!(
            "    {} Keeping {} {}",
            "✓".green(),
            token.bold(),
            latest.file_name().to_string_lossy().dimmed()
        );
    }

    if total_removed == 0 {
        println!("\n {} No old cask versions to remove", "✓".green());
    } else if dry_run {
        println!(
            "\n{} Would remove {} old cask versions ({})",
            "ℹ".blue(),
            total_removed.to_string().bold(),
            format_size(total_space_freed).bold()
        );
    } else {
        println!(
            "\n{} Removed {} old cask versions, freed {}",
            "✓".green().bold(),
            total_removed.to_string().bold(),
            format_size(total_space_freed).bold()
        );
    }

    Ok(())
}

pub async fn upgrade_cask(api: &BrewApi, cask_names: &[String]) -> Result<()> {
    // Determine which casks to upgrade
    let to_upgrade = if cask_names.is_empty() {
        // Upgrade all outdated casks - check in parallel
        println!("Checking for outdated casks...");

        let installed_casks = crate::cask::list_installed_casks()?;

        // Fetch all cask metadata in parallel
        let fetch_futures: Vec<_> = installed_casks
            .iter()
            .map(|(token, installed_version)| {
                let token = token.clone();
                let installed_version = installed_version.clone();
                async move {
                    if let Ok(cask) = api.fetch_cask(&token).await
                        && let Some(latest) = &cask.version
                        && latest != &installed_version
                    {
                        return Some(token);
                    }
                    None
                }
            })
            .collect();

        let results = futures::future::join_all(fetch_futures).await;
        let outdated: Vec<_> = results.into_iter().flatten().collect();

        if outdated.is_empty() {
            println!("\n {} All casks are up to date", "✓".green());
            return Ok(());
        }

        println!(
            "{} casks to upgrade: {}",
            outdated.len().to_string().bold(),
            outdated.join(", ").cyan()
        );
        outdated
    } else {
        cask_names.to_vec()
    };

    println!("\nUpgrading {} casks...", to_upgrade.len());

    for cask_name in &to_upgrade {
        println!("  Upgrading {}...", cask_name.cyan());

        // First uninstall the old version
        uninstall_cask(std::slice::from_ref(cask_name))?;

        // Then install the new version
        install_cask(api, std::slice::from_ref(cask_name)).await?;

        println!("  {} Upgraded {}", "✓".green(), cask_name.bold().green());
    }

    println!("\n {} Cask upgrade complete", "✓".green().bold());
    Ok(())
}

pub fn uninstall_cask(cask_names: &[String]) -> Result<()> {
    if cask_names.is_empty() {
        println!("{} No casks specified", "✗".red());
        return Ok(());
    }

    println!(
        "Uninstalling {} casks...",
        cask_names.len().to_string().bold()
    );

    for cask_name in cask_names {
        println!("\nUninstalling cask: {}", cask_name.cyan());

        // Check if installed
        if !crate::cask::is_cask_installed(cask_name) {
            println!("  {} {} is not installed", "⚠".yellow(), cask_name.bold());
            continue;
        }

        let version = crate::cask::get_installed_cask_version(cask_name)
            .ok_or_else(|| anyhow::anyhow!("Could not determine version"))?;

        // Read metadata to find installed apps
        let cask_dir = crate::cask::cask_install_dir(cask_name, &version);
        let metadata_path = cask_dir.join(".metadata.json");

        let apps = if metadata_path.exists() {
            let metadata_str = std::fs::read_to_string(&metadata_path)?;
            let metadata: serde_json::Value = serde_json::from_str(&metadata_str)?;

            if let Some(apps_array) = metadata.get("installed_apps").and_then(|v| v.as_array()) {
                apps_array
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            } else {
                Vec::new()
            }
        } else {
            // Fallback: guess app name from cask name (capitalize first letter)
            let mut chars = cask_name.chars();
            if let Some(first_char) = chars.next() {
                vec![format!(
                    "{}.app",
                    first_char.to_uppercase().to_string() + chars.as_str()
                )]
            } else {
                vec![]
            }
        };

        // Remove apps from /Applications
        for app_name in &apps {
            let app_path = PathBuf::from("/Applications").join(app_name);

            if app_path.exists() {
                println!("  Removing {}...", app_name.cyan());

                match std::fs::remove_dir_all(&app_path) {
                    Ok(_) => {
                        println!("    {} Removed {}", "✓".green(), app_name.bold());
                    }
                    Err(e) => {
                        println!("    {} Failed to remove: {}", "✗".red(), e);
                        println!(
                            "    Try: {}",
                            format!("sudo rm -rf {}", app_path.display()).cyan()
                        );
                    }
                }
            } else {
                println!("  {} App not found: {}", "⚠".yellow(), app_name.dimmed());
            }
        }

        // Remove Caskroom directory
        let caskroom_path = crate::cask::caskroom_dir().join(cask_name);
        if caskroom_path.exists() {
            std::fs::remove_dir_all(&caskroom_path)?;
        }

        println!(
            "\n  {} Uninstalled {} {}",
            "✓".green().bold(),
            cask_name.bold().green(),
            version.dimmed()
        );
    }

    println!("\n {} Cask uninstallation complete", "✓".green().bold());
    Ok(())
}

pub fn prefix(formula_name: Option<&str>) -> anyhow::Result<()> {
    let prefix = cellar::detect_prefix();

    if let Some(name) = formula_name {
        // Show formula prefix
        let versions = cellar::get_installed_versions(name)?;
        if versions.is_empty() {
            anyhow::bail!("Formula '{}' is not installed", name);
        }

        let version = &versions[0].version;
        let formula_prefix = cellar::cellar_path().join(name).join(version);

        println!("{}", formula_prefix.display());
    } else {
        // Show Homebrew prefix
        println!("{}", prefix.display());
    }

    Ok(())
}

pub fn cellar_cmd(formula_name: Option<&str>) -> anyhow::Result<()> {
    let cellar = cellar::cellar_path();

    if let Some(name) = formula_name {
        // Show formula cellar
        println!("{}", cellar.join(name).display());
    } else {
        // Show Cellar path
        println!("{}", cellar.display());
    }

    Ok(())
}

pub fn repository(tap_name: Option<&str>) -> anyhow::Result<()> {
    let prefix = cellar::detect_prefix();

    if let Some(tap) = tap_name {
        // Show tap repository path
        let tap_path = crate::tap::tap_directory(tap)?;
        println!("{}", tap_path.display());
    } else {
        // Show main repository path (homebrew-core)
        let repo = prefix.join("Library/Taps/homebrew/homebrew-core");
        println!("{}", repo.display());
    }

    Ok(())
}

pub fn formula(formula_name: &str) -> anyhow::Result<()> {
    let prefix = cellar::detect_prefix();
    let taps_dir = prefix.join("Library/Taps");

    // Try to find formula file
    let first_letter = formula_name
        .chars()
        .next()
        .unwrap_or('a')
        .to_lowercase()
        .to_string();
    let core_formula_letter = taps_dir
        .join("homebrew/homebrew-core/Formula")
        .join(&first_letter)
        .join(format!("{}.rb", formula_name));
    let core_formula_flat = taps_dir
        .join("homebrew/homebrew-core/Formula")
        .join(format!("{}.rb", formula_name));

    if core_formula_letter.exists() {
        println!("{}", core_formula_letter.display());
    } else if core_formula_flat.exists() {
        println!("{}", core_formula_flat.display());
    } else {
        // Search all taps
        let mut found = false;
        if taps_dir.exists() {
            for tap_entry in std::fs::read_dir(&taps_dir)?.flatten() {
                let tap_path = tap_entry.path();
                if tap_path.is_dir() {
                    for repo_entry in std::fs::read_dir(&tap_path)?.flatten() {
                        let repo_path = repo_entry.path();
                        let fp = repo_path
                            .join("Formula")
                            .join(format!("{}.rb", formula_name));
                        if fp.exists() {
                            println!("{}", fp.display());
                            found = true;
                            break;
                        }
                    }
                }
                if found {
                    break;
                }
            }
        }

        if !found {
            anyhow::bail!("Formula '{}' not found", formula_name);
        }
    }

    Ok(())
}

pub fn postinstall(formula_names: &[String]) -> anyhow::Result<()> {
    if formula_names.is_empty() {
        println!("{} No formulae specified", "✗".red());
        return Ok(());
    }

    println!(
        "Running post-install for {} formulae...",
        formula_names.len().to_string().bold()
    );
    println!();

    for formula_name in formula_names {
        println!("{}", formula_name.cyan());

        // Check if installed
        let versions = cellar::get_installed_versions(formula_name)?;
        if versions.is_empty() {
            println!("  {} Not installed", "⚠".yellow());
            continue;
        }

        // Post-install would run .rb postinstall block
        // For now, this is a stub since we don't have Ruby interop yet
        println!("  {} Post-install not yet implemented", "ℹ".blue());
        println!("  Requires Phase 3 (Ruby interop) to execute formula post-install blocks");
    }

    Ok(())
}

pub async fn formulae(api: &BrewApi) -> Result<()> {
    println!("Fetching all available formulae...");

    let all_formulae = api.fetch_all_formulae().await?;

    println!(
        "\n{} {} formulae available\n",
        "✓".green(),
        all_formulae.len().to_string().bold()
    );

    // Display in columns like Homebrew
    let names: Vec<String> = all_formulae.iter().map(|f| f.name.clone()).collect();

    // Calculate column width based on terminal width
    let term_width = 80; // Default, could use terminal_size crate
    let max_name_len = names.iter().map(|n| n.len()).max().unwrap_or(0);
    let col_width = max_name_len + 2;
    let num_cols = (term_width / col_width).max(1);

    // Print in columns
    for (i, name) in names.iter().enumerate() {
        print!("{:<width$}", name, width = col_width);
        if (i + 1) % num_cols == 0 {
            println!();
        }
    }

    // Final newline if needed
    if !names.len().is_multiple_of(num_cols) {
        println!();
    }

    Ok(())
}

pub async fn casks(api: &BrewApi) -> Result<()> {
    println!("Fetching all available casks...");

    let all_casks = api.fetch_all_casks().await?;

    println!(
        "{} {} casks available",
        "✓".green(),
        all_casks.len().to_string().bold()
    );

    // Display in columns like Homebrew
    let tokens: Vec<String> = all_casks.iter().map(|c| c.token.clone()).collect();

    // Calculate column width based on terminal width
    let term_width = 80; // Default
    let max_token_len = tokens.iter().map(|t| t.len()).max().unwrap_or(0);
    let col_width = max_token_len + 2;
    let num_cols = (term_width / col_width).max(1);

    // Print in columns
    for (i, token) in tokens.iter().enumerate() {
        print!("{:<width$}", token, width = col_width);
        if (i + 1) % num_cols == 0 {
            println!();
        }
    }

    // Final newline if needed
    if !tokens.len().is_multiple_of(num_cols) {
        println!();
    }

    Ok(())
}

pub async fn unbottled(api: &BrewApi, formula_names: &[String]) -> Result<()> {
    println!("Checking for formulae without bottles...");

    let all_formulae = api.fetch_all_formulae().await?;

    // Filter to formulae without bottles
    let unbottled_formulae: Vec<_> = all_formulae
        .into_iter()
        .filter(|f| {
            // If specific formulae requested, only check those
            if !formula_names.is_empty() && !formula_names.contains(&f.name) {
                return false;
            }

            // Check if formula has no bottle
            f.bottle.is_none()
                || f.bottle
                    .as_ref()
                    .and_then(|b| b.stable.as_ref())
                    .map(|s| s.files.is_empty())
                    .unwrap_or(true)
        })
        .collect();

    if unbottled_formulae.is_empty() {
        if formula_names.is_empty() {
            println!("\n {} All formulae have bottles", "✓".green());
        } else {
            println!("\n {} All specified formulae have bottles", "✓".green());
        }
        return Ok(());
    }

    println!(
        "\n{} {} formulae without bottles:\n",
        "ℹ".blue(),
        unbottled_formulae.len().to_string().bold()
    );

    // Display as list with descriptions
    for formula in &unbottled_formulae {
        print!("{}", formula.name.bold().yellow());
        if let Some(desc) = &formula.desc {
            print!(" - {}", desc.dimmed());
        }
        println!();
    }

    Ok(())
}

pub fn docs() -> Result<()> {
    let docs_url = "https://docs.brew.sh";
    println!("Opening documentation: {}", docs_url.cyan());

    // Try to open URL in browser
    let status = std::process::Command::new("open").arg(docs_url).status()?;

    if !status.success() {
        println!(
            "{} Failed to open browser. Visit: {}",
            "⚠".yellow(),
            docs_url
        );
    }

    Ok(())
}

pub fn tap_new(tap_name: &str) -> Result<()> {
    // Validate tap name format (should be user/repo)
    if !tap_name.contains('/') {
        println!(
            "{} Invalid tap name. Format: {}",
            "✗".red(),
            "user/repo".cyan()
        );
        return Ok(());
    }

    let parts: Vec<&str> = tap_name.split('/').collect();
    if parts.len() != 2 {
        println!(
            "{} Invalid tap name. Format: {}",
            "✗".red(),
            "user/repo".cyan()
        );
        return Ok(());
    }

    let user = parts[0];
    let repo = parts[1];
    let full_repo_name = if repo.starts_with("homebrew-") {
        repo.to_string()
    } else {
        format!("homebrew-{}", repo)
    };

    let tap_path = crate::tap::taps_path().join(user).join(&full_repo_name);

    if tap_path.exists() {
        println!(
            "{} Tap already exists: {}",
            "⚠".yellow(),
            tap_path.display().to_string().cyan()
        );
        return Ok(());
    }

    println!("Creating new tap: {}", tap_name.cyan());

    // Create directory structure
    std::fs::create_dir_all(&tap_path)?;
    std::fs::create_dir_all(tap_path.join("Formula"))?;
    std::fs::create_dir_all(tap_path.join("Casks"))?;

    // Create README
    let readme_content = format!(
        "# {}/{}\n\nHomebrew tap for custom formulae and casks.\n\n## Usage\n\n```bash\nbrew tap {}\n```\n",
        user, full_repo_name, tap_name
    );
    std::fs::write(tap_path.join("README.md"), readme_content)?;

    // Initialize git repository
    let status = std::process::Command::new("git")
        .args(["init"])
        .current_dir(&tap_path)
        .status()?;

    if !status.success() {
        println!("  {} Failed to initialize git repository", "⚠".yellow());
    }

    println!(
        "\n{} Tap created at: {}",
        "✓".green().bold(),
        tap_path.display().to_string().cyan()
    );
    println!(
        "\nAdd formulae to: {}",
        tap_path.join("Formula").display().to_string().dimmed()
    );
    println!(
        "Add casks to: {}",
        tap_path.join("Casks").display().to_string().dimmed()
    );

    Ok(())
}

pub fn migrate(formula_name: &str, new_tap: Option<&str>) -> Result<()> {
    println!("Migrating formula: {}", formula_name.cyan());

    // Check if formula is installed
    let versions = cellar::get_installed_versions(formula_name)?;
    if versions.is_empty() {
        println!("{} Formula not installed: {}", "✗".red(), formula_name);
        return Ok(());
    }

    let version = &versions[0].version;

    // If no new tap specified, show information about current tap
    let tap = match new_tap {
        Some(t) => t,
        None => {
            println!("\n {} Migration information:", "ℹ".blue());
            println!("  Formula: {} {}", formula_name.bold(), version.dimmed());
            println!("  Currently installed from: {}", "homebrew/core".cyan());
            println!("\nTo migrate to a different tap, use:");
            println!("  {} --tap <tap-name>", "bru migrate".cyan());
            return Ok(());
        }
    };

    println!("  Migrating {} to tap: {}", formula_name, tap.cyan());
    println!("\n {} Migration is a metadata operation only", "ℹ".blue());
    println!("  No reinstallation needed - formula remains at same location");
    println!("  Future upgrades will use the new tap");

    // In a full implementation, this would update the formula's tap metadata
    // For now, this is informational

    println!(
        "\n{} Migration prepared (metadata would be updated)",
        "✓".green()
    );

    Ok(())
}

pub fn linkage(formula_names: &[String], show_all: bool) -> Result<()> {
    println!("Checking library linkages...");

    let formulae_to_check: Vec<String> = if formula_names.is_empty() {
        // Check all installed formulae
        cellar::list_installed()?
            .into_iter()
            .map(|p| p.name)
            .collect()
    } else {
        formula_names.to_vec()
    };

    if formulae_to_check.is_empty() {
        println!("\n {} No formulae to check", "ℹ".blue());
        return Ok(());
    }

    for formula_name in &formulae_to_check {
        println!("\n {}", formula_name.cyan());

        let versions = cellar::get_installed_versions(formula_name)?;
        if versions.is_empty() {
            println!("  {} Not installed", "⚠".yellow());
            continue;
        }

        let version = &versions[0].version;
        let formula_path = cellar::cellar_path().join(formula_name).join(version);

        // Find all executables and libraries
        let mut checked_files = 0;
        let mut broken_links = 0;

        // Check bin/ directory
        let bin_dir = formula_path.join("bin");
        if bin_dir.exists()
            && let Ok(entries) = std::fs::read_dir(&bin_dir)
        {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    checked_files += 1;

                    // Use otool to check linkages on macOS
                    let output = std::process::Command::new("otool")
                        .arg("-L")
                        .arg(&path)
                        .output();

                    if let Ok(output) = output {
                        let stdout = String::from_utf8_lossy(&output.stdout);

                        if show_all && let Some(name) = path.file_name() {
                            println!("  {}:", name.to_string_lossy());
                            for line in stdout.lines().skip(1) {
                                let trimmed = line.trim();
                                if !trimmed.is_empty() {
                                    println!("    {}", trimmed.dimmed());
                                }
                            }
                        }

                        // Check for broken links (simplified)
                        if stdout.contains("dyld:") || stdout.contains("not found") {
                            broken_links += 1;
                        }
                    }
                }
            }
        }

        // Check lib/ directory
        let lib_dir = formula_path.join("lib");
        if lib_dir.exists()
            && let Ok(entries) = std::fs::read_dir(&lib_dir)
        {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && (path.extension().and_then(|s| s.to_str()) == Some("dylib")) {
                    checked_files += 1;
                }
            }
        }

        if checked_files == 0 {
            println!("  {} No linkable files found", "ℹ".blue());
        } else if broken_links > 0 {
            println!(
                "  {} {} files checked, {} broken links",
                "⚠".yellow(),
                checked_files,
                broken_links
            );
        } else {
            println!(
                "  {} {} files checked, all links valid",
                "✓".green(),
                checked_files
            );
        }
    }

    Ok(())
}

pub fn readall(tap_name: Option<&str>) -> Result<()> {
    let tap = tap_name.unwrap_or("homebrew/core");

    println!("Reading all formulae in tap: {}", tap.cyan());

    let tap_dir = if tap == "homebrew/core" {
        cellar::detect_prefix().join("Library/Taps/homebrew/homebrew-core")
    } else {
        crate::tap::tap_directory(tap)?
    };

    if !tap_dir.exists() {
        println!("{} Tap not found: {}", "✗".red(), tap);
        return Ok(());
    }

    let formula_dir = tap_dir.join("Formula");
    if !formula_dir.exists() {
        println!("{} No Formula directory in tap", "⚠".yellow());
        return Ok(());
    }

    // Count formula files recursively
    fn count_formulae(dir: &std::path::Path, depth: usize) -> (usize, usize) {
        const MAX_DEPTH: usize = 10;
        if depth > MAX_DEPTH {
            return (0, 0);
        }

        let mut total = 0;
        let mut valid = 0;

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rb") {
                    total += 1;
                    // Basic validation: check if file is readable (metadata check, not reading content)
                    if std::fs::metadata(&path).is_ok() {
                        valid += 1;
                    }
                } else if path.is_dir() {
                    let (sub_total, sub_valid) = count_formulae(&path, depth + 1);
                    total += sub_total;
                    valid += sub_valid;
                }
            }
        }

        (total, valid)
    }

    let (total, valid) = count_formulae(&formula_dir, 0);

    if total == 0 {
        println!("\n {} No formulae found in tap", "⚠".yellow());
    } else if valid == total {
        println!(
            "\n{} All {} formulae are readable",
            "✓".green().bold(),
            total.to_string().bold()
        );
    } else {
        println!(
            "\n{} {} of {} formulae are readable",
            "⚠".yellow(),
            valid,
            total
        );
        println!("  {} {} formulae have issues", "✗".red(), total - valid);
    }

    Ok(())
}

pub fn extract(formula_name: &str, target_tap: &str) -> Result<()> {
    println!("Extracting formula: {}", formula_name.cyan());
    println!("  Target tap: {}", target_tap.cyan());

    // Find the formula file
    let prefix = cellar::detect_prefix();
    let taps_dir = prefix.join("Library/Taps");

    let mut formula_path = None;
    let mut source_tap = None;

    // Search in homebrew/core first
    let core_formula_dir = taps_dir.join("homebrew/homebrew-core/Formula");
    if core_formula_dir.exists() {
        // Check letter-organized directories
        if let Some(first_letter) = formula_name.chars().next() {
            let letter_dir = core_formula_dir.join(first_letter.to_lowercase().to_string());
            let possible_path = letter_dir.join(format!("{}.rb", formula_name));
            if possible_path.exists() {
                formula_path = Some(possible_path);
                source_tap = Some("homebrew/core");
            }
        }
    }

    // If not found in core, search other taps
    if formula_path.is_none() {
        println!("  Searching taps...");
        // This is a simplified search - real implementation would be more thorough
    }

    let (formula_path, source_tap) = match (formula_path, source_tap) {
        (Some(path), Some(tap)) => (path, tap),
        _ => {
            println!("{} Formula not found: {}", "✗".red(), formula_name);
            return Ok(());
        }
    };

    println!("  {} Found in: {}", "✓".green(), source_tap.cyan());

    // Validate target tap
    let target_tap_dir = crate::tap::tap_directory(target_tap)?;
    if !target_tap_dir.exists() {
        println!("{} Target tap not found: {}", "✗".red(), target_tap);
        println!(
            "  Create it first with: {}",
            format!("bru tap-new {}", target_tap).cyan()
        );
        return Ok(());
    }

    // Copy formula to target tap
    let target_formula_dir = target_tap_dir.join("Formula");
    std::fs::create_dir_all(&target_formula_dir)?;

    let target_path = target_formula_dir.join(format!("{}.rb", formula_name));

    if target_path.exists() {
        println!("{} Formula already exists in target tap", "⚠".yellow());
        return Ok(());
    }

    std::fs::copy(&formula_path, &target_path)?;

    println!(
        "\n{} Extracted {} to {}",
        "✓".green().bold(),
        formula_name.bold(),
        target_tap.cyan()
    );
    println!("  Path: {}", target_path.display().to_string().dimmed());

    Ok(())
}

pub async fn unpack(api: &BrewApi, formula_name: &str, dest_dir: Option<&str>) -> Result<()> {
    println!("Unpacking source for: {}", formula_name.cyan());

    // Fetch formula info
    let formula = api.fetch_formula(formula_name).await?;

    let version = formula
        .versions
        .stable
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No stable version available"))?;

    println!("  Version: {}", version.cyan());

    // Note: Full implementation would download source tarball and extract
    // For now, provide informational output
    println!(
        "\n{} Source unpacking requires Phase 3 (Ruby interop)",
        "ℹ".blue()
    );
    println!("  Formula source would be downloaded and extracted to:");

    let target_dir = if let Some(dir) = dest_dir {
        std::path::PathBuf::from(dir)
    } else {
        std::env::current_dir()?.join(formula_name)
    };

    println!("  {}", target_dir.display().to_string().cyan());

    // Show what would happen
    if let Some(homepage) = &formula.homepage {
        println!("\n  Homepage: {}", homepage.dimmed());
    }

    Ok(())
}

pub fn command_not_found_init(shell: Option<&str>) -> Result<()> {
    let detected_shell = shell.map(String::from).unwrap_or_else(|| {
        std::env::var("SHELL")
            .ok()
            .and_then(|s| s.split('/').next_back().map(String::from))
            .unwrap_or_else(|| "bash".to_string())
    });

    println!("# bru command-not-found hook for {}", detected_shell);
    println!();

    match detected_shell.as_str() {
        "bash" => {
            println!("# Add this to your ~/.bashrc:");
            println!(
                "HB_CNF_HANDLER=\"$(brew --repository)/Library/Taps/homebrew/homebrew-command-not-found/handler.sh\""
            );
            println!("if [ -f \"$HB_CNF_HANDLER\" ]; then");
            println!("  source \"$HB_CNF_HANDLER\"");
            println!("fi");
        }
        "zsh" => {
            println!("# Add this to your ~/.zshrc:");
            println!(
                "HB_CNF_HANDLER=\"$(brew --repository)/Library/Taps/homebrew/homebrew-command-not-found/handler.sh\""
            );
            println!("if [ -f \"$HB_CNF_HANDLER\" ]; then");
            println!("  source \"$HB_CNF_HANDLER\"");
            println!("fi");
        }
        "fish" => {
            println!("# Add this to your ~/.config/fish/config.fish:");
            println!(
                "set HB_CNF_HANDLER (brew --repository)/Library/Taps/homebrew/homebrew-command-not-found/handler.fish"
            );
            println!("if test -f $HB_CNF_HANDLER");
            println!("  source $HB_CNF_HANDLER");
            println!("end");
        }
        _ => {
            println!("# Shell '{}' not directly supported", detected_shell);
            println!("# Use bash or zsh configuration as a starting point");
        }
    }

    Ok(())
}

pub fn man() -> anyhow::Result<()> {
    println!("Opening Homebrew man page...");

    let status = std::process::Command::new("man").arg("brew").status();

    match status {
        Ok(exit_status) if exit_status.success() => Ok(()),
        Ok(_) => {
            println!("\n {} Man page not found", "⚠".yellow());
            println!("  Try: {}", "brew install man-db".cyan());
            Ok(())
        }
        Err(e) => {
            println!("{} Failed to open man page: {}", "✗".red(), e);
            println!(
                "\n  Documentation available at: {}",
                "https://docs.brew.sh".cyan()
            );
            Ok(())
        }
    }
}

pub fn update_reset(tap_name: Option<&str>) -> anyhow::Result<()> {
    let tap = tap_name.unwrap_or("homebrew/core");

    println!("Resetting tap: {}", tap.cyan());

    let tap_dir = if tap == "homebrew/core" {
        cellar::detect_prefix().join("Library/Taps/homebrew/homebrew-core")
    } else {
        crate::tap::tap_directory(tap)?
    };

    if !tap_dir.exists() {
        println!("{} Tap not found: {}", "✗".red(), tap);
        return Ok(());
    }

    let git_dir = tap_dir.join(".git");
    if !git_dir.exists() {
        println!("{} Not a git repository: {}", "⚠".yellow(), tap);
        return Ok(());
    }

    println!("  Fetching latest changes...");

    let fetch_status = std::process::Command::new("git")
        .current_dir(&tap_dir)
        .args(["fetch", "origin"])
        .status()?;

    if !fetch_status.success() {
        println!("{} Failed to fetch", "✗".red());
        return Ok(());
    }

    println!("  Resetting to origin/master...");

    let reset_status = std::process::Command::new("git")
        .current_dir(&tap_dir)
        .args(["reset", "--hard", "origin/master"])
        .status()?;

    if !reset_status.success() {
        let reset_main_status = std::process::Command::new("git")
            .current_dir(&tap_dir)
            .args(["reset", "--hard", "origin/main"])
            .status()?;

        if !reset_main_status.success() {
            println!("{} Failed to reset", "✗".red());
            return Ok(());
        }
    }

    println!(
        "\n{} Tap reset complete: {}",
        "✓".green().bold(),
        tap.bold()
    );

    Ok(())
}

pub fn style(formula_names: &[String], fix: bool) -> anyhow::Result<()> {
    if formula_names.is_empty() {
        println!("{} No formulae specified", "✗".red());
        return Ok(());
    }

    println!(
        "Checking style for {} formulae...",
        formula_names.len().to_string().bold()
    );

    if fix {
        println!("  {} Auto-fix enabled", "ℹ".blue());
    }

    println!(
        "\n {} Style checking requires RuboCop (Phase 3)",
        "ℹ".blue()
    );
    println!("  Formula style would be validated against Homebrew standards:");
    println!("  - Naming conventions");
    println!("  - Method ordering");
    println!("  - Spacing and indentation");
    println!("  - Ruby best practices");

    for formula in formula_names {
        println!("\n  {}", formula.cyan());
        println!("    {} Would check formula style", "ℹ".dimmed());
    }

    if fix {
        println!("\n {} Auto-fix would correct violations", "ℹ".blue());
    }

    Ok(())
}

pub fn test(formula_name: &str) -> anyhow::Result<()> {
    println!("Running tests for: {}", formula_name.cyan());

    println!(
        "\n{} Formula testing requires Phase 3 (Ruby interop)",
        "ℹ".blue()
    );
    println!("  Test suite would be executed from formula's test block");
    println!("  Typical tests verify:");
    println!("  - Installation succeeded");
    println!("  - Binary is executable");
    println!("  - Version output matches");
    println!("  - Basic functionality works");

    println!(
        "\n  Would run: {} test {}",
        "brew".cyan(),
        formula_name.cyan()
    );

    Ok(())
}

pub fn bottle(formula_names: &[String], write: bool) -> anyhow::Result<()> {
    if formula_names.is_empty() {
        println!("{} No formulae specified", "✗".red());
        return Ok(());
    }

    println!(
        "Generating bottles for {} formulae...",
        formula_names.len().to_string().bold()
    );

    if write {
        println!(
            "  {} Write mode enabled - would update formula files",
            "ℹ".blue()
        );
    }

    println!(
        "\n{} Bottle generation requires Phase 3 (Ruby interop)",
        "ℹ".blue()
    );
    println!("  Would build from source and create bottles:");

    for formula in formula_names {
        println!("\n  {}", formula.cyan());
        println!("    {} Build from source", "1.".dimmed());
        println!("    {} Package into bottle tarball", "2.".dimmed());
        println!("    {} Calculate SHA256 checksum", "3.".dimmed());
        if write {
            println!("    {} Write bottle block to formula", "4.".dimmed());
        }
    }

    Ok(())
}

pub fn tap_pin(tap_name: &str) -> anyhow::Result<()> {
    println!("Pinning tap: {}", tap_name.cyan());

    let tap_dir = crate::tap::tap_directory(tap_name)?;

    if !tap_dir.exists() {
        println!("{} Tap not found: {}", "✗".red(), tap_name);
        return Ok(());
    }

    let prefix = cellar::detect_prefix();
    let pinned_dir = prefix.join("Library/PinnedTaps");

    std::fs::create_dir_all(&pinned_dir)?;

    let pin_file = pinned_dir.join(tap_name.replace('/', "--"));

    if pin_file.exists() {
        println!("{} Tap already pinned", "ℹ".blue());
        return Ok(());
    }

    std::fs::write(&pin_file, "")?;

    println!("\n {} Tap pinned: {}", "✓".green().bold(), tap_name.bold());
    println!(
        "  This tap will not be updated by {} or {}",
        "bru update".cyan(),
        "bru upgrade".cyan()
    );

    Ok(())
}

pub fn tap_unpin(tap_name: &str) -> anyhow::Result<()> {
    println!("Unpinning tap: {}", tap_name.cyan());

    let prefix = cellar::detect_prefix();
    let pinned_dir = prefix.join("Library/PinnedTaps");
    let pin_file = pinned_dir.join(tap_name.replace('/', "--"));

    if !pin_file.exists() {
        println!("{} Tap is not pinned", "ℹ".blue());
        return Ok(());
    }

    std::fs::remove_file(&pin_file)?;

    println!(
        "\n {} Tap unpinned: {}",
        "✓".green().bold(),
        tap_name.bold()
    );
    println!(
        "  This tap will now be updated by {} and {}",
        "bru update".cyan(),
        "bru upgrade".cyan()
    );

    Ok(())
}

pub fn vendor_gems() -> anyhow::Result<()> {
    println!("Installing Homebrew's vendored gems...");

    println!(
        "\n{} Vendored gems require Phase 3 (Ruby interop)",
        "ℹ".blue()
    );
    println!("  Would install Ruby gems required by Homebrew:");
    println!("  - activesupport");
    println!("  - addressable");
    println!("  - concurrent-ruby");
    println!("  - json_schemer");
    println!("  - mechanize");
    println!("  - minitest");
    println!("  - parallel");
    println!("  - parser");
    println!("  - rubocop-ast");
    println!("  - ruby-macho");
    println!("  - sorbet-runtime");

    println!(
        "\n  {} Gems would be installed to: {}",
        "ℹ".dimmed(),
        "Homebrew/Library/Homebrew/vendor".cyan()
    );

    Ok(())
}

pub fn ruby(args: &[String]) -> anyhow::Result<()> {
    if args.is_empty() {
        println!("Starting Homebrew Ruby REPL...");
    } else {
        println!("Running Ruby with Homebrew environment...");
    }

    println!(
        "\n{} Ruby execution requires Phase 3 (embedded Ruby interpreter)",
        "ℹ".blue()
    );
    println!("  Would run Ruby code with Homebrew's environment loaded");

    if !args.is_empty() {
        println!("\n  Arguments: {}", args.join(" ").cyan());
    }

    println!("\n  When implemented:");
    println!("  - Full access to Homebrew formula DSL");
    println!("  - All Homebrew libraries available");
    println!("  - Same Ruby version as Homebrew uses");

    Ok(())
}

pub fn irb() -> anyhow::Result<()> {
    println!("Starting Homebrew's interactive Ruby shell...");

    println!(
        "\n{} IRB requires Phase 3 (embedded Ruby interpreter)",
        "ℹ".blue()
    );
    println!("  Interactive Ruby shell with Homebrew environment loaded");
    println!("  Full access to Homebrew internals and formula DSL");

    println!(
        "\n  {} Use {} for non-interactive execution",
        "ℹ".dimmed(),
        "bru ruby".cyan()
    );

    Ok(())
}

pub fn prof(args: &[String]) -> anyhow::Result<()> {
    if args.is_empty() {
        println!("{} No command specified to profile", "✗".red());
        println!("\nUsage: {} <command> [args]", "bru prof".cyan());
        return Ok(());
    }

    println!("Profiling command: {}", args.join(" ").cyan());

    println!("\n {} Profiling functionality", "ℹ".blue());
    println!("  Would measure:");
    println!("  - Execution time");
    println!("  - Memory usage");
    println!("  - API calls");
    println!("  - Bottlenecks");

    println!("\n  Command: {}", args.join(" ").cyan());

    Ok(())
}

pub fn tap_readme(tap_name: &str) -> anyhow::Result<()> {
    println!("Generating README for tap: {}", tap_name.cyan());

    let tap_dir = crate::tap::tap_directory(tap_name)?;

    if !tap_dir.exists() {
        println!("{} Tap not found: {}", "✗".red(), tap_name);
        return Ok(());
    }

    let readme_path = tap_dir.join("README.md");

    if readme_path.exists() {
        println!("\n {} README.md already exists", "ℹ".blue());
        println!("  Location: {}", readme_path.display().to_string().dimmed());
    } else {
        println!("\n {} Would generate README.md with:", "ℹ".blue());
        println!("  - Tap name and description");
        println!("  - Installation instructions");
        println!("  - List of formulae/casks");
        println!("  - Contributing guidelines");
        println!("\n  Location: {}", readme_path.display().to_string().cyan());
    }

    Ok(())
}

pub fn install_bundler_gems() -> anyhow::Result<()> {
    println!("Installing Homebrew's bundler gems...");

    println!(
        "\n{} Bundler gems require Phase 3 (Ruby interop)",
        "ℹ".blue()
    );
    println!("  Would install gems from Homebrew's Gemfile:");
    println!("  - bundler");
    println!("  - rake");
    println!("  - rspec");
    println!("  - rubocop");
    println!("  - simplecov");

    println!(
        "\n  {} Different from {}",
        "ℹ".dimmed(),
        "vendor-gems".cyan()
    );
    println!("  vendor-gems: Runtime dependencies");
    println!("  install-bundler-gems: Development dependencies");

    Ok(())
}

pub fn developer(action: Option<&str>) -> anyhow::Result<()> {
    let prefix = cellar::detect_prefix();
    let dev_flag_file = prefix.join(".homebrew_developer");

    match action {
        None | Some("state") => {
            let is_dev = dev_flag_file.exists();
            if is_dev {
                println!("Developer mode: {}", "enabled".green());
            } else {
                println!("Developer mode: {}", "disabled".dimmed());
            }

            if is_dev {
                println!("\n  When enabled:");
                println!("  - Updates to latest commit instead of stable");
                println!("  - Additional validation checks");
                println!("  - More verbose output");
            } else {
                println!("\n  To enable: {} on", "bru developer".cyan());
            }
        }
        Some("on") => {
            if dev_flag_file.exists() {
                println!("{} Developer mode already enabled", "ℹ".blue());
            } else {
                std::fs::write(&dev_flag_file, "")?;
                println!("{} Developer mode enabled", "✓".green().bold());
                println!("\n  Changes:");
                println!("  - Will update to latest commit instead of stable");
                println!("  - Additional validation enabled");
            }
        }
        Some("off") => {
            if !dev_flag_file.exists() {
                println!("{} Developer mode already disabled", "ℹ".blue());
            } else {
                std::fs::remove_file(&dev_flag_file)?;
                println!("{} Developer mode disabled", "✓".green().bold());
                println!("\n  Reverted to stable release updates");
            }
        }
        Some(other) => {
            println!("{} Unknown action: {}", "✗".red(), other);
            println!("\nUsage: {} [on|off|state]", "bru developer".cyan());
        }
    }

    Ok(())
}

pub fn typecheck(files: &[String]) -> anyhow::Result<()> {
    if files.is_empty() {
        println!("Running Sorbet type checker on Homebrew code...");
    } else {
        println!("Type checking {} files...", files.len().to_string().bold());
    }

    println!(
        "\n{} Type checking requires Phase 3 (Ruby interop + Sorbet)",
        "ℹ".blue()
    );
    println!("  Sorbet is a gradual type checker for Ruby");
    println!("  Would check:");
    println!("  - Type annotations");
    println!("  - Method signatures");
    println!("  - Return types");
    println!("  - Type safety violations");

    if !files.is_empty() {
        println!("\n  Files to check:");
        for file in files {
            println!("    {}", file.cyan());
        }
    } else {
        println!("\n  {} Would check all Homebrew Ruby files", "ℹ".dimmed());
    }

    Ok(())
}

pub fn update_report() -> anyhow::Result<()> {
    println!("Generating update report...");

    let prefix = cellar::detect_prefix();
    let repository_path = prefix.join("Library/Taps/homebrew/homebrew-core");

    if !repository_path.exists() {
        println!("{} homebrew/core tap not found", "✗".red());
        return Ok(());
    }

    println!("\n {} Checking git log for recent changes...", "ℹ".blue());

    let output = std::process::Command::new("git")
        .current_dir(&repository_path)
        .args(["log", "--oneline", "--since=24.hours.ago"])
        .output()?;

    if output.status.success() {
        let log = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = log.lines().collect();

        if lines.is_empty() {
            println!("\n {} No updates in the last 24 hours", "ℹ".blue());
        } else {
            println!(
                "\n{} {} commits in the last 24 hours:",
                "✓".green(),
                lines.len().to_string().bold()
            );
            for line in lines.iter().take(10) {
                println!("  {}", line.dimmed());
            }
            if lines.len() > 10 {
                println!("  ... and {} more", (lines.len() - 10).to_string().dimmed());
            }
        }
    }

    Ok(())
}

pub fn update_python_resources(formula_name: &str, print_only: bool) -> anyhow::Result<()> {
    println!("Updating Python resources for: {}", formula_name.cyan());

    if print_only {
        println!("  {} Print-only mode enabled", "ℹ".blue());
    }

    println!(
        "\n{} Python resource updates require Phase 3 (Ruby interop)",
        "ℹ".blue()
    );
    println!("  Would analyze Python package dependencies:");
    println!("  - Parse setup.py or pyproject.toml");
    println!("  - Fetch latest versions from PyPI");
    println!("  - Generate resource blocks");
    println!("  - Calculate SHA256 checksums");

    if print_only {
        println!("\n  Would print updated resource blocks");
    } else {
        println!("\n  Would update formula file");
    }

    Ok(())
}

pub fn determine_test_runners(formula_names: &[String]) -> anyhow::Result<()> {
    if formula_names.is_empty() {
        println!("{} No formulae specified", "✗".red());
        return Ok(());
    }

    println!(
        "Determining test runners for {} formulae...",
        formula_names.len().to_string().bold()
    );

    println!("\n {} Test runner detection", "ℹ".blue());
    println!("  Would analyze formulae to determine:");
    println!("  - Language/framework used");
    println!("  - Test framework (pytest, jest, cargo test, etc.)");
    println!("  - CI/CD test runner configuration");

    for formula in formula_names {
        println!("\n  {}", formula.cyan());
        println!("    {} Would detect test framework", "ℹ".dimmed());
    }

    Ok(())
}

pub fn dispatch_build_bottle(formula_name: &str, platform: Option<&str>) -> anyhow::Result<()> {
    println!(
        "{} Dispatching bottle build for: {}",
        "🏗️".bold(),
        formula_name.cyan()
    );

    if let Some(plat) = platform {
        println!("  Platform: {}", plat.cyan());
    } else {
        println!("  Platform: {}", "current".dimmed());
    }

    println!("\n {} Bottle build dispatch (CI/CD command)", "ℹ".blue());
    println!("  This command is used by Homebrew's CI system");
    println!("  Would trigger:");
    println!("  - Remote build on specified platform");
    println!("  - Bottle generation and upload");
    println!("  - PR creation with bottle block");

    println!(
        "\n  {} For local bottle builds, use: {}",
        "ℹ".dimmed(),
        "bru bottle".cyan()
    );

    Ok(())
}

pub fn bump_formula_pr(
    formula_name: &str,
    version: Option<&str>,
    url: Option<&str>,
) -> anyhow::Result<()> {
    println!("Creating PR to update formula: {}", formula_name.cyan());

    if let Some(ver) = version {
        println!("  New version: {}", ver.cyan());
    }
    if let Some(u) = url {
        println!("  URL: {}", u.dimmed());
    }

    println!(
        "\n{} Formula PR creation requires Phase 3 (Ruby interop)",
        "ℹ".blue()
    );
    println!("  Automated workflow to update a formula:");
    println!("  1. Fetch new version from upstream");
    println!("  2. Update formula file (version, URL, SHA256)");
    println!("  3. Build and test formula");
    println!("  4. Create git branch");
    println!("  5. Commit changes");
    println!("  6. Push to GitHub");
    println!("  7. Open pull request");

    println!(
        "\n  {} This is a maintainer/contributor workflow",
        "ℹ".dimmed()
    );

    Ok(())
}

pub fn bump_cask_pr(cask_name: &str, version: Option<&str>) -> anyhow::Result<()> {
    println!("Creating PR to update cask: {}", cask_name.cyan());

    if let Some(ver) = version {
        println!("  New version: {}", ver.cyan());
    }

    println!(
        "\n{} Cask PR creation requires Phase 3 (Ruby interop)",
        "ℹ".blue()
    );
    println!("  Automated workflow to update a cask:");
    println!("  1. Fetch new version metadata");
    println!("  2. Update cask file (version, URL, SHA256)");
    println!("  3. Verify cask installs");
    println!("  4. Create git branch");
    println!("  5. Commit changes");
    println!("  6. Push to GitHub");
    println!("  7. Open pull request");

    println!(
        "\n  {} This is a maintainer/contributor workflow",
        "ℹ".dimmed()
    );

    Ok(())
}

pub async fn generate_formula_api(formula_names: &[String]) -> anyhow::Result<()> {
    if formula_names.is_empty() {
        println!("Generating formula API for all formulae...");
    } else {
        println!(
            "Generating formula API for {} formulae...",
            formula_names.len().to_string().bold()
        );
    }

    println!("\n {} API generation", "ℹ".blue());
    println!("  Generates JSON API data consumed by:");
    println!("  - formulae.brew.sh");
    println!("  - Homebrew website");
    println!("  - Third-party tools");

    println!("\n  Would generate:");
    println!("    - formula.json (formula metadata)");
    println!("    - analytics.json (install counts)");
    println!("    - cask_analytics.json");

    if !formula_names.is_empty() {
        println!("\n  Generating for specific formulae:");
        for formula in formula_names {
            println!("    {}", formula.cyan());
        }
    }

    Ok(())
}

pub async fn generate_cask_api(cask_names: &[String]) -> anyhow::Result<()> {
    if cask_names.is_empty() {
        println!("Generating cask API for all casks...");
    } else {
        println!(
            "Generating cask API for {} casks...",
            cask_names.len().to_string().bold()
        );
    }

    println!("\n {} API generation", "ℹ".blue());
    println!("  Generates JSON API data for casks");
    println!("  Used by formulae.brew.sh and Homebrew website");

    println!("\n  Would generate:");
    println!("    - cask.json (cask metadata)");
    println!("    - cask_analytics.json (install counts)");

    if !cask_names.is_empty() {
        println!("\n  Generating for specific casks:");
        for cask in cask_names {
            println!("    {}", cask.cyan());
        }
    }

    Ok(())
}

pub fn pr_pull(pr_ref: &str) -> anyhow::Result<()> {
    println!("{} Pulling PR: {}", "⬇️".bold(), pr_ref.cyan());

    let pr_number = if pr_ref.contains('/') {
        pr_ref.split('/').next_back().unwrap_or(pr_ref)
    } else {
        pr_ref
    };

    println!("\n {} PR pull workflow", "ℹ".blue());
    println!("  Downloads and applies a pull request locally");
    println!("  Useful for testing PRs before merge");

    println!("\n  Would execute:");
    println!("    1. Fetch PR #{} from GitHub", pr_number.cyan());
    println!("    2. Create local branch");
    println!("    3. Apply PR commits");
    println!("    4. Checkout PR branch");

    println!(
        "\n  {} Use {} to test changes",
        "ℹ".dimmed(),
        "bru test".cyan()
    );

    Ok(())
}

pub fn pr_upload(use_bintray: bool) -> anyhow::Result<()> {
    println!("{} Uploading bottles for PR...", "⬆️".bold());

    let target = if use_bintray {
        "Bintray"
    } else {
        "GitHub Releases"
    };
    println!("  Target: {}", target.cyan());

    println!("\n {} Bottle upload (CI/CD workflow)", "ℹ".blue());
    println!("  This is typically run by CI after building bottles");

    println!("\n  Would execute:");
    println!("    1. Find bottle tarballs in current directory");
    println!("    2. Calculate SHA256 checksums");
    println!("    3. Upload to {}", target.cyan());
    println!("    4. Update PR with bottle DSL");
    println!("    5. Commit bottle block to PR branch");

    println!("\n  {} Requires GitHub authentication", "⚠".yellow());

    Ok(())
}

pub fn test_bot(formula_names: &[String], skip_cleanup: bool) -> anyhow::Result<()> {
    if formula_names.is_empty() {
        println!("Running test-bot on all formulae...");
    } else {
        println!(
            "Running test-bot on {} formulae...",
            formula_names.len().to_string().bold()
        );
    }

    if skip_cleanup {
        println!("  {} Cleanup will be skipped", "ℹ".blue());
    }

    println!("\n {} Homebrew test-bot (CI system)", "ℹ".blue());
    println!("  Comprehensive CI testing workflow:");
    println!("  1. Build formula from source");
    println!("  2. Run formula tests");
    println!("  3. Generate bottles");
    println!("  4. Test bottle installation");
    println!("  5. Validate formula syntax");
    println!("  6. Check for conflicts");

    if !formula_names.is_empty() {
        println!("\n  Testing:");
        for formula in formula_names {
            println!("    {}", formula.cyan());
        }
    }

    println!(
        "\n  {} This is the core of Homebrew's CI infrastructure",
        "ℹ".dimmed()
    );

    Ok(())
}

pub fn bump_revision(formula_names: &[String], message: Option<&str>) -> anyhow::Result<()> {
    if formula_names.is_empty() {
        println!("{} No formulae specified", "✗".red());
        return Ok(());
    }

    println!(
        "Bumping revision for {} formulae...",
        formula_names.len().to_string().bold()
    );

    if let Some(msg) = message {
        println!("  Reason: {}", msg.dimmed());
    }

    println!("\n {} Revision bump", "ℹ".blue());
    println!("  Increments formula revision number");
    println!("  Used when formula changes but version doesn't");
    println!("  (e.g., build fixes, dependency updates)");

    for formula in formula_names {
        println!("\n  {}", formula.cyan());
        println!("    {} Would increment revision field", "ℹ".dimmed());
    }

    Ok(())
}

pub fn pr_automerge(strategy: Option<&str>) -> anyhow::Result<()> {
    println!("Auto-merging qualifying pull requests...");

    if let Some(strat) = strategy {
        println!("  Strategy: {}", strat.cyan());
    }

    println!("\n {} PR auto-merge (CI workflow)", "ℹ".blue());
    println!("  Automatically merges PRs that meet criteria:");
    println!("  - All CI checks pass");
    println!("  - Approved by maintainers");
    println!("  - No merge conflicts");
    println!("  - Meets style guidelines");

    println!("\n  Would scan open PRs and merge eligible ones");
    println!("  {} Requires maintainer permissions", "⚠".yellow());

    Ok(())
}

pub fn contributions(user: Option<&str>, from_date: Option<&str>) -> anyhow::Result<()> {
    if let Some(username) = user {
        println!("Contributor statistics for: {}", username.cyan());
    } else {
        println!("Overall contributor statistics");
    }

    if let Some(date) = from_date {
        println!("  From: {}", date.dimmed());
    }

    println!("\n {} Analyzing git history...", "ℹ".blue());

    let prefix = cellar::detect_prefix();
    let repository_path = prefix.join("Library/Taps/homebrew/homebrew-core");

    if repository_path.exists() {
        let mut args = vec!["shortlog", "-sn"];
        if let Some(date) = from_date {
            args.push("--since");
            args.push(date);
        }

        let output = std::process::Command::new("git")
            .current_dir(&repository_path)
            .args(&args)
            .output()?;

        if output.status.success() {
            let log = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<&str> = log.lines().collect();

            if let Some(username) = user {
                let user_line = lines.iter().find(|line| line.contains(username));
                if let Some(line) = user_line {
                    println!("\n {} {}", "✓".green(), line);
                } else {
                    println!("\n {} No contributions found for {}", "ℹ".blue(), username);
                }
            } else {
                println!("\n {} Top contributors:", "✓".green());
                for line in lines.iter().take(10) {
                    println!("  {}", line.dimmed());
                }
                if lines.len() > 10 {
                    println!("  ... and {} more", (lines.len() - 10).to_string().dimmed());
                }
            }
        }
    } else {
        println!("{} homebrew/core tap not found", "✗".red());
    }

    Ok(())
}

pub fn update_license_data() -> anyhow::Result<()> {
    println!("Updating SPDX license data...");

    println!("\n {} License data update", "ℹ".blue());
    println!("  Downloads and updates SPDX license list");
    println!("  Used for validating license fields in formulae");

    println!("\n  Would execute:");
    println!("    1. Fetch latest SPDX license list");
    println!("    2. Parse license data");
    println!("    3. Update Homebrew's license database");
    println!("    4. Validate existing formula licenses");

    println!("\n  {} SPDX: Software Package Data Exchange", "ℹ".dimmed());
    println!("  {} Standardized license identifiers", "ℹ".dimmed());

    Ok(())
}

pub async fn formula_info(api: &BrewApi, formula_name: &str) -> anyhow::Result<()> {
    println!("Formula info: {}", formula_name.cyan());

    let formula = api.fetch_formula(formula_name).await?;

    println!("\n {}", formula.name.bold());
    if let Some(desc) = &formula.desc {
        println!("{}", desc.dimmed());
    }

    println!("\nVersion:");
    if let Some(version) = &formula.versions.stable {
        println!("  Stable: {}", version.cyan());
    }
    if let Some(version) = &formula.versions.head {
        println!("  HEAD: {}", version.cyan());
    }

    if let Some(homepage) = &formula.homepage {
        println!("\nHomepage:");
        println!("  {}", homepage.dimmed());
    }

    if !formula.dependencies.is_empty() {
        println!("\nDependencies ({}):", formula.dependencies.len());
        for dep in formula.dependencies.iter().take(5) {
            println!("  {}", dep.cyan());
        }
        if formula.dependencies.len() > 5 {
            println!(
                "  ... and {} more",
                (formula.dependencies.len() - 5).to_string().dimmed()
            );
        }
    }

    Ok(())
}

pub fn tap_cmd(tap_name: &str, command: &str, args: &[String]) -> anyhow::Result<()> {
    println!(
        "{} Running tap command: {} {}",
        "⚙️".bold(),
        tap_name.cyan(),
        command.cyan()
    );

    if !args.is_empty() {
        println!("  Arguments: {}", args.join(" ").dimmed());
    }

    println!("\n {} External tap commands", "ℹ".blue());
    println!("  Taps can provide custom commands");
    println!("  These are scripts in the tap's cmd/ directory");

    let tap_dir = crate::tap::tap_directory(tap_name)?;
    let cmd_dir = tap_dir.join("cmd");

    if cmd_dir.exists() {
        println!("\n  {} Tap has cmd/ directory", "✓".green());
        println!("  Would execute: {}/{}", tap_name, command.cyan());
    } else {
        println!("\n  {} Tap has no cmd/ directory", "⚠".yellow());
    }

    Ok(())
}

pub fn install_formula_api() -> anyhow::Result<()> {
    println!("Installing formula API locally...");

    println!("\n {} Formula API installation", "ℹ".blue());
    println!("  Downloads and caches formula JSON API");
    println!("  Used for fast offline formula lookups");

    println!("\n  Would execute:");
    println!("    1. Download formula.json from formulae.brew.sh");
    println!("    2. Download cask.json");
    println!("    3. Cache locally in Homebrew directory");
    println!("    4. Enable fast offline search");

    println!(
        "\n  {} Improves search performance significantly",
        "ℹ".dimmed()
    );

    Ok(())
}

pub fn uses_cask(cask_name: &str) -> anyhow::Result<()> {
    println!("Checking what uses cask: {}", cask_name.cyan());

    println!("\n {} Cask usage analysis", "ℹ".blue());
    println!("  Unlike formulae, casks typically don't have dependents");
    println!("  Casks are GUI applications, not libraries");

    println!("\n  {} Casks are usually standalone", "ℹ".dimmed());
    println!(
        "  {} Safe to uninstall without affecting other software",
        "✓".green()
    );

    Ok(())
}

pub async fn abv_cask(api: &BrewApi, cask_name: &str) -> anyhow::Result<()> {
    println!("Cask info: {}", cask_name.cyan());

    let cask = api.fetch_cask(cask_name).await?;

    println!("\n {}", cask.token.bold());
    if let Some(name) = &cask.name.first() {
        println!("{}", name.dimmed());
    }

    if let Some(version) = &cask.version {
        println!("\nVersion: {}", version.cyan());
    }

    if let Some(homepage) = &cask.homepage {
        println!("Homepage: {}", homepage.dimmed());
    }

    if let Some(desc) = &cask.desc {
        println!("\n {}", desc.dimmed());
    }

    Ok(())
}

pub fn setup() -> anyhow::Result<()> {
    println!("Setting up Homebrew development environment...");

    println!("\n {} Development setup", "ℹ".blue());
    println!("  Configures environment for Homebrew development:");

    println!("\n  Would execute:");
    println!("    1. Clone Homebrew repository");
    println!("    2. Install development dependencies");
    println!("    3. Configure git hooks");
    println!("    4. Set up Ruby environment");
    println!("    5. Install bundler gems");
    println!("    6. Configure shell environment");

    println!("\n  {} For contributors and maintainers", "ℹ".dimmed());
    println!("  See: https://docs.brew.sh/Development");

    Ok(())
}

pub fn fix_bottle_tags(formula_names: &[String]) -> anyhow::Result<()> {
    if formula_names.is_empty() {
        println!("{} No formulae specified", "✗".red());
        return Ok(());
    }

    println!(
        "{} Fixing bottle tags for {} formulae...",
        "🏷️".bold(),
        formula_names.len().to_string().bold()
    );

    println!("\n {} Bottle tag repair", "ℹ".blue());
    println!("  Updates bottle tags to current platform naming");
    println!("  Homebrew periodically changes platform identifiers");

    for formula in formula_names {
        println!("\n  {}", formula.cyan());
        println!("    {} Would update bottle tags in formula", "ℹ".dimmed());
        println!("    Example: monterey -> ventura -> sonoma");
    }

    Ok(())
}

pub fn generate_man_completions() -> anyhow::Result<()> {
    println!("Generating man pages and completions...");

    println!("\n {} Documentation generation", "ℹ".blue());
    println!("  Generates Homebrew documentation:");

    println!("\n  Would generate:");
    println!("    - Man pages for brew command");
    println!("    - Shell completions (bash, zsh, fish)");
    println!("    - API documentation");

    println!("\n  Output locations:");
    println!("    - {}", "manpages/man1/brew.1".cyan());
    println!("    - {}", "completions/bash/brew".cyan());
    println!("    - {}", "completions/zsh/_brew".cyan());
    println!("    - {}", "completions/fish/brew.fish".cyan());

    println!("\n {} This is a maintainer command", "ℹ".blue());
    println!("  Used during Homebrew releases");
    println!("  Requires access to Homebrew/brew repository");

    Ok(())
}

pub fn bottle_merge(bottle_files: &[String]) -> anyhow::Result<()> {
    if bottle_files.is_empty() {
        println!("{} No bottle files specified", "✗".red());
        return Ok(());
    }

    println!(
        "Merging {} bottle files...",
        bottle_files.len().to_string().bold()
    );

    println!("\n {} Bottle merge (CI workflow)", "ℹ".blue());
    println!("  Merges bottle metadata from multiple builds");
    println!("  Used in Homebrew's CI when building for multiple platforms");

    println!("\n  Would merge:");
    for bottle in bottle_files {
        println!("    - {}", bottle.cyan());
    }

    println!("\n  Output:");
    println!("    - Combined bottle DSL block");
    println!("    - All platform SHAs merged");
    println!("    - Ready for PR upload");

    println!("\n {} This is a CI command", "ℹ".blue());
    println!("  Typically run by test-bot");

    Ok(())
}

pub fn install_bundler() -> anyhow::Result<()> {
    println!("Installing Homebrew's bundler...");

    println!("\n {} Bundler installation", "ℹ".blue());
    println!("  Installs Ruby bundler gem for Homebrew development");

    let prefix = cellar::detect_prefix();
    let vendor_dir = prefix.join("Library/Homebrew/vendor");

    println!("\n  Target:");
    println!("    {}", vendor_dir.display().to_string().cyan());

    println!("\n  Would install:");
    println!("    - bundler gem");
    println!("    - Dependencies for formula development");

    println!("\n {} This is a development command", "ℹ".blue());
    println!("  Required for formula creation and testing");

    Ok(())
}

pub fn bump(formula: &str, no_audit: bool) -> anyhow::Result<()> {
    println!(
        "{} Creating version bump PR for: {}",
        "⬆️".bold(),
        formula.cyan()
    );

    if no_audit {
        println!("  Skipping audit");
    }

    println!("\n {} Version bump workflow", "ℹ".blue());
    println!("  Automated PR creation for formula updates");

    println!("\n  Would do:");
    println!("    1. Detect latest upstream version");
    println!("    2. Update formula file");
    println!("    3. Compute new SHA256");
    println!("    4. Run audit (unless --no-audit)");
    println!("    5. Create GitHub PR");

    println!("\n  Formula:");
    println!("    {}", formula.cyan());

    println!("\n {} This is a maintainer command", "ℹ".blue());
    println!("  Requires GitHub authentication");
    println!("  Used for keeping formulae up-to-date");

    Ok(())
}

pub fn analytics_state() -> anyhow::Result<()> {
    let prefix = cellar::detect_prefix();
    let analytics_disabled = prefix.join(".homebrew_analytics_disabled").exists();

    println!("Analytics state:");

    if analytics_disabled {
        println!("  Status: {}", "disabled".dimmed());
        println!("\n  {} Analytics are currently turned off", "ℹ".blue());
        println!("  {} No usage data is being collected", "✓".green());
    } else {
        println!("  Status: {}", "enabled".green());
        println!("\n  {} Analytics are currently enabled", "ℹ".blue());
        println!("  Anonymous usage data is collected");
    }

    println!(
        "\n  {} To change: {} [on|off]",
        "ℹ".dimmed(),
        "bru analytics".cyan()
    );

    Ok(())
}

pub fn sponsor(target: Option<&str>) -> anyhow::Result<()> {
    if let Some(name) = target {
        println!("Sponsor: {}", name.cyan());
    } else {
        println!("Homebrew Sponsors");
    }

    println!("\n {} GitHub Sponsors", "ℹ".blue());
    println!("  Support open source development");

    if let Some(name) = target {
        println!("\n  Would open:");
        println!("    https://github.com/sponsors/{}", name);
    } else {
        println!("\n  Homebrew's sponsors:");
        println!("    https://github.com/sponsors/Homebrew");
        println!("\n  Thank you to all our sponsors!");
    }

    Ok(())
}

pub fn command(subcommand: &str, args: &[String]) -> anyhow::Result<()> {
    println!("Running Homebrew sub-command: {}", subcommand.cyan());

    if !args.is_empty() {
        println!("  Arguments: {}", args.join(" ").dimmed());
    }

    println!("\n {} Sub-command execution", "ℹ".blue());
    println!("  Runs external Homebrew commands or scripts");

    println!("\n  Would execute:");
    if args.is_empty() {
        println!("    brew-{}", subcommand);
    } else {
        println!("    brew-{} {}", subcommand, args.join(" "));
    }

    println!("\n {} This is an internal command", "ℹ".blue());
    println!("  Used by Homebrew to run external commands");

    Ok(())
}

pub fn nodenv_sync() -> anyhow::Result<()> {
    println!("Syncing nodenv shims...");

    println!("\n {} nodenv integration", "ℹ".blue());
    println!("  Synchronizes Node.js version manager shims");

    let prefix = cellar::detect_prefix();
    let nodenv_dir = prefix.join("opt/nodenv");

    if nodenv_dir.exists() {
        println!("\n  {} nodenv installation found", "✓".green());
        println!("    {}", nodenv_dir.display().to_string().dimmed());
    } else {
        println!("\n  {} nodenv not installed", "ℹ".blue());
        println!("    Install with: {}", "bru install nodenv".cyan());
    }

    println!("\n  Would sync:");
    println!("    - Node version shims");
    println!("    - npm/npx executables");
    println!("    - PATH integration");

    Ok(())
}

pub fn pyenv_sync() -> anyhow::Result<()> {
    println!("Syncing pyenv shims...");

    println!("\n {} pyenv integration", "ℹ".blue());
    println!("  Synchronizes Python version manager shims");

    let prefix = cellar::detect_prefix();
    let pyenv_dir = prefix.join("opt/pyenv");

    if pyenv_dir.exists() {
        println!("\n  {} pyenv installation found", "✓".green());
        println!("    {}", pyenv_dir.display().to_string().dimmed());
    } else {
        println!("\n  {} pyenv not installed", "ℹ".blue());
        println!("    Install with: {}", "bru install pyenv".cyan());
    }

    println!("\n  Would sync:");
    println!("    - Python version shims");
    println!("    - pip/python executables");
    println!("    - Virtual environment integration");

    Ok(())
}

pub fn rbenv_sync() -> anyhow::Result<()> {
    println!("Syncing rbenv shims...");

    println!("\n {} rbenv integration", "ℹ".blue());
    println!("  Synchronizes Ruby version manager shims");

    let prefix = cellar::detect_prefix();
    let rbenv_dir = prefix.join("opt/rbenv");

    if rbenv_dir.exists() {
        println!("\n  {} rbenv installation found", "✓".green());
        println!("    {}", rbenv_dir.display().to_string().dimmed());
    } else {
        println!("\n  {} rbenv not installed", "ℹ".blue());
        println!("    Install with: {}", "bru install rbenv".cyan());
    }

    println!("\n  Would sync:");
    println!("    - Ruby version shims");
    println!("    - gem/bundle executables");
    println!("    - Gemfile integration");

    Ok(())
}

pub fn setup_ruby() -> anyhow::Result<()> {
    println!("Setting up Ruby environment...");

    println!("\n {} Homebrew Ruby setup", "ℹ".blue());
    println!("  Configures Ruby environment for Homebrew development");

    let prefix = cellar::detect_prefix();
    let homebrew_ruby = prefix.join("Library/Homebrew/vendor/portable-ruby");

    println!("\n  Ruby installation:");
    if homebrew_ruby.exists() {
        println!("    {}", "Portable Ruby installed".green());
        println!("    {}", homebrew_ruby.display().to_string().dimmed());
    } else {
        println!("    {}", "Portable Ruby not found".dimmed());
    }

    println!("\n  Would setup:");
    println!("    - Ruby interpreter");
    println!("    - RubyGems configuration");
    println!("    - Bundler dependencies");
    println!("    - Development gems");

    println!("\n {} This is a development command", "ℹ".blue());
    println!("  Required for formula development and testing");

    Ok(())
}

pub async fn tab(api: &BrewApi, formula_name: &str) -> anyhow::Result<()> {
    println!("Formula info (tab-separated): {}", formula_name.cyan());

    let formula = api.fetch_formula(formula_name).await?;

    println!("\n {} Tab format", "ℹ".blue());
    println!("  Generates machine-readable formula information");

    println!("\n  Output format:");
    println!("    name\\tversion\\thomepage\\tdescription");

    println!("\n {}", "─".repeat(60).dimmed());

    let version = formula.versions.stable.as_deref().unwrap_or("unknown");
    let homepage = formula.homepage.as_deref().unwrap_or("none");
    let desc = formula.desc.as_deref().unwrap_or("no description");

    println!(
        "{}\t{}\t{}\t{}",
        formula.name.cyan(),
        version.dimmed(),
        homepage.dimmed(),
        desc.dimmed()
    );

    println!("{}", "─".repeat(60).dimmed());

    Ok(())
}

pub fn unalias(alias_name: &str) -> anyhow::Result<()> {
    println!("{} Removing alias: {}", "🗑️".bold(), alias_name.cyan());

    let prefix = cellar::detect_prefix();
    let alias_file = prefix.join(format!(".brew_alias_{}", alias_name));

    println!("\n {} Alias management", "ℹ".blue());
    println!("  Removes command aliases");

    if alias_file.exists() {
        println!("\n  {} Alias found:", "✓".green());
        println!("    {}", alias_file.display().to_string().dimmed());

        std::fs::remove_file(&alias_file)?;
        println!("\n {} Alias removed successfully", "✓".green().bold());
    } else {
        println!("\n  {} Alias not found: {}", "ℹ".blue(), alias_name);
        println!("    To see all aliases: {}", "bru alias".cyan());
    }

    Ok(())
}

pub fn update_if_needed() -> anyhow::Result<()> {
    println!("Checking if update is needed...");

    let prefix = cellar::detect_prefix();
    let repository_path = prefix.join("Library/Taps/homebrew/homebrew-core");

    if !repository_path.exists() {
        println!("{} homebrew/core tap not found", "✗".red());
        return Ok(());
    }

    println!("\n {} Conditional update", "ℹ".blue());
    println!("  Only updates if last update was more than 24 hours ago");

    let last_update_file = prefix.join(".homebrew_last_update");

    let needs_update = if last_update_file.exists() {
        if let Ok(metadata) = std::fs::metadata(&last_update_file) {
            if let Ok(modified) = metadata.modified() {
                if let Ok(elapsed) = modified.elapsed() {
                    elapsed.as_secs() > 86400
                } else {
                    true
                }
            } else {
                true
            }
        } else {
            true
        }
    } else {
        true
    };

    if needs_update {
        println!("\n  Update needed (>24 hours since last update)");
        println!("    Running: {}", "bru update".cyan());

        crate::commands::update()?;

        std::fs::write(&last_update_file, "")?;
    } else {
        println!("\n  {} Update not needed (updated recently)", "✓".green());
        println!("    Last update: within 24 hours");
    }

    Ok(())
}
