use crate::api::BrewApi;
use crate::cellar;
use crate::commands::pin::read_pinned;
use crate::error::Result;
use colored::Colorize;

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
