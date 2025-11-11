//! Example: Check for available package upgrades
//!
//! This example demonstrates how to:
//! 1. List all installed packages
//! 2. Check each one against the API for newer versions
//! 3. Report outdated packages
//! 4. Show what versions are available
//!
//! This is useful for implementing `bru outdated` functionality.
//!
//! Usage: cargo run --example check_upgrades

use kombrucha::{BrewApi, cellar};
use std::collections::BTreeMap;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Checking for package upgrades...\n");

    // Step 1: List installed packages
    println!("Step 1: Reading installed packages from Cellar...");
    let installed = cellar::list_installed()?;

    if installed.is_empty() {
        println!("No packages installed.");
        return Ok(());
    }

    println!("✓ Found {} package installations\n", installed.len());

    // Group by formula name (taking newest version of each)
    let mut latest_versions: BTreeMap<String, String> = BTreeMap::new();
    for pkg in &installed {
        // Keep only the newest version
        if let Some(existing) = latest_versions.get(&pkg.name) {
            let is_newer = compare_versions(&pkg.version, existing) > 0;
            if is_newer {
                latest_versions.insert(pkg.name.clone(), pkg.version.clone());
            }
        } else {
            latest_versions.insert(pkg.name.clone(), pkg.version.clone());
        }
    }

    println!(
        "Step 2: Checking {} formulae for updates...\n",
        latest_versions.len()
    );

    let api = BrewApi::new()?;

    let mut upgradeable = Vec::new();
    let mut up_to_date = Vec::new();
    let mut not_found = Vec::new();
    let mut errors = Vec::new();

    for (formula_name, installed_version) in &latest_versions {
        match api.fetch_formula(formula_name).await {
            Ok(formula) => {
                if let Some(latest_version) = &formula.versions.stable {
                    if compare_versions(latest_version, installed_version) > 0 {
                        upgradeable.push((
                            formula_name.clone(),
                            installed_version.clone(),
                            latest_version.clone(),
                        ));
                    } else {
                        up_to_date.push((formula_name.clone(), installed_version.clone()));
                    }
                } else {
                    not_found.push(formula_name.clone());
                }
            }
            Err(e) => {
                errors.push((formula_name.clone(), format!("{}", e)));
            }
        }

        // Print progress
        print!(".");
        std::io::Write::flush(&mut std::io::stdout())?;
    }
    println!("\n");

    // Display results
    println!("═══════════════════════════════════════");
    println!("UPGRADE REPORT");
    println!("═══════════════════════════════════════\n");

    if !upgradeable.is_empty() {
        println!("⬆️  UPGRADEABLE ({}):", upgradeable.len());
        for (name, installed, latest) in &upgradeable {
            println!("  {} {} → {}", name, installed, latest);
        }
        println!();
    }

    if !up_to_date.is_empty() && upgradeable.len() < 10 {
        println!("✓ UP TO DATE ({}):", up_to_date.len());
        for (name, version) in up_to_date.iter().take(5) {
            println!("  {} {}", name, version);
        }
        if up_to_date.len() > 5 {
            println!("  ... and {} more", up_to_date.len() - 5);
        }
        println!();
    } else if !up_to_date.is_empty() {
        println!("✓ {} packages are up to date\n", up_to_date.len());
    }

    if !not_found.is_empty() {
        println!("⚠️  NOT FOUND IN API ({}):", not_found.len());
        for name in not_found.iter().take(5) {
            println!("  {}", name);
        }
        if not_found.len() > 5 {
            println!(
                "  ... and {} more (possibly from taps)",
                not_found.len() - 5
            );
        }
        println!();
    }

    if !errors.is_empty() {
        println!("❌ ERRORS ({}):", errors.len());
        for (name, err) in errors.iter().take(3) {
            println!("  {}: {}", name, err);
        }
        if errors.len() > 3 {
            println!("  ... and {} more", errors.len() - 3);
        }
        println!();
    }

    // Summary
    println!("═══════════════════════════════════════");
    println!("Summary:");
    println!("  Total installed: {}", latest_versions.len());
    println!("  Upgradeable: {}", upgradeable.len());
    println!("  Up to date: {}", up_to_date.len());
    println!("  Not found: {}", not_found.len());
    println!("  Errors: {}", errors.len());

    if !upgradeable.is_empty() {
        println!("\n💡 Tip: Use 'bru upgrade <formula>' to update specific packages");
        println!("    or 'bru upgrade' to update all packages");
    }

    Ok(())
}

/// Compare two version strings semantically
fn compare_versions(a: &str, b: &str) -> i32 {
    let a_parts: Vec<u32> = a.split('.').filter_map(|s| s.parse().ok()).collect();
    let b_parts: Vec<u32> = b.split('.').filter_map(|s| s.parse().ok()).collect();

    for i in 0..a_parts.len().max(b_parts.len()) {
        let a_part = a_parts.get(i).unwrap_or(&0);
        let b_part = b_parts.get(i).unwrap_or(&0);

        match a_part.cmp(b_part) {
            std::cmp::Ordering::Greater => return 1,
            std::cmp::Ordering::Less => return -1,
            std::cmp::Ordering::Equal => continue,
        }
    }

    0
}
