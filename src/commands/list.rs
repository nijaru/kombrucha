use crate::api::BrewApi;
use crate::cellar;
use crate::commands::format_columns;
use crate::error::Result;
use colored::Colorize;

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
                    println!("No packages installed");
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
                    println!("No packages installed");
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
                    "{} {} packages installed",
                    "✓".green(),
                    by_name.len().to_string().bold()
                );
            }
        }
    }

    Ok(())
}
