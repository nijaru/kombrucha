//! Query commands for searching and retrieving formula/cask information.
//!
//! This module contains read-only commands that fetch and display information
//! about formulae and casks from the Homebrew API.

use crate::api::BrewApi;
use crate::cellar;
use crate::error::Result;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashSet;
use std::process::Command;

/// Search for formulae and casks matching a query string
pub async fn search(api: &BrewApi, query: &str, formula_only: bool, cask_only: bool) -> Result<()> {
    // Detect if stdout is a TTY (for brew-compatible behavior)
    let is_tty = std::io::IsTerminal::is_terminal(&std::io::stdout());

    let spinner = if is_tty {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        pb.set_message(format!("Searching for '{}'...", query));
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        pb
    } else {
        ProgressBar::hidden()
    };

    let results = api.search(query).await?;
    spinner.finish_and_clear();

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

/// Display detailed information about a formula or cask
pub async fn info(api: &BrewApi, formula: &str, json: bool) -> Result<()> {
    let is_tty = std::io::IsTerminal::is_terminal(&std::io::stdout());

    // Spinner for API fetching (will be shown only if we reach API call)
    let spinner = if !json && is_tty {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        pb.set_message(format!("Fetching info for {}...", formula));
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        pb
    } else {
        ProgressBar::hidden()
    };

    // Check if this is an installed tap formula
    if let Ok(versions) = cellar::get_installed_versions(formula)
        && let Some(installed_version) = versions.first()
        && let Ok(Some((tap_name, formula_path, _))) =
            crate::tap::get_package_tap_info(&installed_version.path)
    {
        // For tap formulae, parse the Ruby file natively
        match crate::tap::parse_formula_info(&formula_path, formula) {
            Ok(tap_info) => {
                // Display tap formula info in native format
                println!(
                    "{}",
                    format!("==> {}/{}", tap_name, tap_info.name).bold().green()
                );
                if let Some(desc) = &tap_info.desc {
                    println!("{}", desc);
                }
                if let Some(homepage) = &tap_info.homepage {
                    println!("{}: {}", "Homepage".bold(), homepage);
                }
                if let Some(version) = &tap_info.version {
                    println!("{}: {}", "Version".bold(), version);
                }

                // Show installed versions
                println!(
                    "{}: {} versions installed",
                    "Installed".bold(),
                    versions.len()
                );
                for v in &versions {
                    let marker = if v.version == installed_version.version {
                        "*"
                    } else {
                        ""
                    };
                    println!("  {} {}", v.version.dimmed(), marker);
                }

                println!(
                    "{}: {}",
                    "From".bold(),
                    formula_path.display().to_string().dimmed()
                );
                spinner.finish_and_clear();
                return Ok(());
            }
            Err(e) => {
                spinner.finish_and_clear();
                // If parsing fails, fall back to brew (shouldn't normally happen)
                eprintln!(
                    "Warning: Failed to parse tap formula ({}), falling back to brew",
                    e
                );
                let full_name = format!("{}/{}", tap_name, formula);
                if super::utils::check_brew_available() {
                    let _ = Command::new("brew").arg("info").arg(&full_name).status();
                }
                return Err(e.into());
            }
        }
    }

    // Try formula first, then cask
    match api.fetch_formula(formula).await {
        Ok(formula) => {
            spinner.finish_and_clear();
            if json {
                // Output as JSON
                let json_str = serde_json::to_string_pretty(&formula)?;
                println!("{}", json_str);
            } else {
                // Pretty print format
                println!("{}", format!("==> {}", formula.name).bold().green());
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
                    spinner.finish_and_clear();
                    if json {
                        let json_str = serde_json::to_string_pretty(&cask)?;
                        println!("{}", json_str);
                    } else {
                        println!("{}", format!("==> {}", cask.token).bold().cyan());
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
                    spinner.finish_and_clear();
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

/// Show dependencies for a formula
pub async fn deps(
    api: &BrewApi,
    formula: &str,
    tree: bool,
    installed_only: bool,
    direct: bool,
) -> Result<()> {
    let is_tty = std::io::IsTerminal::is_terminal(&std::io::stdout());

    // If filtering by installed, get the list of installed packages
    let installed_names: HashSet<String> = if installed_only {
        cellar::list_installed()?
            .into_iter()
            .map(|p| p.name)
            .collect()
    } else {
        HashSet::new()
    };

    if direct {
        // Direct mode: show only immediate dependencies (like brew deps --direct)
        let spinner = if is_tty {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.cyan} {msg}")
                    .unwrap(),
            );
            pb.set_message(format!("Fetching dependencies for {}...", formula));
            pb.enable_steady_tick(std::time::Duration::from_millis(100));
            pb
        } else {
            ProgressBar::hidden()
        };

        let formula_data = api.fetch_formula(formula).await?;
        spinner.finish_and_clear();

        if formula_data.dependencies.is_empty() && formula_data.build_dependencies.is_empty() {
            if is_tty {
                println!("{} No dependencies", "✓".green());
            }
            return Ok(());
        }

        if !formula_data.dependencies.is_empty() {
            let mut deps: Vec<_> = formula_data.dependencies.iter().collect();

            if installed_only {
                deps.retain(|dep| installed_names.contains(*dep));
            }

            if !deps.is_empty() {
                if is_tty {
                    println!("{}", "Runtime dependencies:".bold().green());
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
                println!("No runtime dependencies installed");
            }
        }

        // Note: brew deps --direct does NOT show build dependencies by default
        // Build deps are only shown with --include-build flag (not yet implemented)
    } else {
        // Default mode: show all transitive runtime dependencies (like brew deps)
        // Temporarily suppress the spinner output from resolve_dependencies
        unsafe {
            std::env::set_var("BRU_QUIET", "1");
        }
        let (_all_formulae, dep_order) =
            super::install::resolve_dependencies(api, &[formula.to_string()]).await?;
        unsafe {
            std::env::remove_var("BRU_QUIET");
        }

        // Remove the root formula from the dependency list
        let mut deps: Vec<_> = dep_order
            .into_iter()
            .filter(|name| name != formula)
            .collect();

        if deps.is_empty() {
            if is_tty {
                println!("{} No dependencies", "✓".green());
            }
            return Ok(());
        }

        if installed_only {
            deps.retain(|dep| installed_names.contains(dep));
        }

        if deps.is_empty() && installed_only {
            if is_tty {
                println!("No dependencies installed");
            }
            return Ok(());
        }

        // Print dependencies
        let len = deps.len();
        for (i, dep) in deps.iter().enumerate() {
            if is_tty {
                if tree {
                    let prefix = if i == len - 1 { "└─" } else { "├─" };
                    println!("{} {}", prefix, dep.cyan());
                } else {
                    println!("{}", dep.cyan());
                }
            } else {
                println!("{}", dep);
            }
        }
    }

    Ok(())
}

/// Show formulae that depend on a given formula
pub async fn uses(api: &BrewApi, formula: &str) -> Result<()> {
    let is_tty = std::io::IsTerminal::is_terminal(&std::io::stdout());

    let spinner = if is_tty {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        pb.set_message(format!("Finding formulae that depend on {}...", formula));
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        pb
    } else {
        ProgressBar::hidden()
    };

    // Fetch all formulae
    let all_formulae = api.fetch_all_formulae().await?;
    spinner.finish_and_clear();

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
            println!("{} No formulae depend on '{}'", "✓".green(), formula);
        }
        return Ok(());
    }

    if is_tty {
        println!(
            "{} Found {} formulae that depend on {}:",
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

/// Open a formula's homepage in the default browser
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
                    println!("  Please visit: {}", url);
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

/// Display descriptions for one or more formulae
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

/// Display the contents of a formula or cask as JSON
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

/// Display build options for a formula (note: bottles have fixed options)
pub async fn options(api: &BrewApi, formula_name: &str) -> Result<()> {
    println!("Checking options for: {}", formula_name.cyan());

    // Verify formula exists
    match api.fetch_formula(formula_name).await {
        Ok(formula) => {
            println!("{}", format!("==> {}", formula.name).bold().green());
            if let Some(desc) = &formula.desc {
                println!("{}", desc);
            }
            println!();
            println!("No options available");
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
            println!("{} Formula '{}' not found", "✗".red(), formula_name);
        }
    }

    Ok(())
}

/// Find and print the path to a formula file
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

/// List all available formulae
pub async fn formulae(api: &BrewApi) -> Result<()> {
    println!("Fetching all available formulae...");

    let all_formulae = api.fetch_all_formulae().await?;

    println!(
        "{} {} formulae available",
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

/// List all available casks
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

/// Show formulae that don't have pre-built bottles available
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
            println!("{} All formulae have bottles", "✓".green());
        } else {
            println!("{} All specified formulae have bottles", "✓".green());
        }
        return Ok(());
    }

    println!(
        "{} {} formulae without bottles:",
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

/// Display detailed information about a formula
pub async fn formula_info(api: &BrewApi, formula_name: &str) -> anyhow::Result<()> {
    println!("Formula info: {}", formula_name.cyan());

    let formula = api.fetch_formula(formula_name).await?;

    println!(" {}", formula.name.bold());
    if let Some(desc) = &formula.desc {
        println!("{}", desc.dimmed());
    }

    println!("Version:");
    if let Some(version) = &formula.versions.stable {
        println!("  Stable: {}", version.cyan());
    }
    if let Some(version) = &formula.versions.head {
        println!("  HEAD: {}", version.cyan());
    }

    if let Some(homepage) = &formula.homepage {
        println!("Homepage:");
        println!("  {}", homepage.dimmed());
    }

    if !formula.dependencies.is_empty() {
        println!("Dependencies ({}):", formula.dependencies.len());
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
