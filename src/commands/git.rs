//! Git and logging commands
//!
//! Commands for viewing installation logs, generating diagnostic information,
//! and inspecting formula metadata from git history.

use crate::api::BrewApi;
use crate::cellar;
use crate::error::Result;
use colored::Colorize;

use super::utils::read_pinned;

/// Show installation logs and information for a formula
///
/// Displays detailed information about an installed formula including
/// installation receipt, linked files, and install directory.
///
/// # Arguments
/// * `formula_name` - The formula to show logs for
pub fn log(formula_name: &str) -> Result<()> {
    println!("Checking logs for {}", formula_name.cyan());
    println!();

    // Check if formula is installed
    let installed_versions = cellar::get_installed_versions(formula_name)?;
    if installed_versions.is_empty() {
        println!("{} {} is not installed", "⚠".yellow(), formula_name.bold());
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
                && let Ok(target) = std::fs::read_link(&path) {
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
                        && let Some(name) = path.file_name() {
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

/// Generate diagnostic information for debugging
///
/// Creates a comprehensive system and package report useful for
/// debugging issues or sharing with maintainers. Includes system
/// info, installed packages, taps, and formula-specific details.
///
/// # Arguments
/// * `api` - The Homebrew API client
/// * `formula` - Optional formula name to include specific details for
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
