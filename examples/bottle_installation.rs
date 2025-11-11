//! Example: Download and install a package from a bottle
//!
//! This example demonstrates the complete bottle-based installation workflow:
//! 1. Fetch formula metadata from the API
//! 2. Download the bottle for current platform
//! 3. Extract to the Cellar
//! 4. Generate installation receipt
//! 5. Create symlinks for system accessibility
//!
//! Usage: cargo run --example bottle_installation [formula_name]

use kombrucha::{BrewApi, cellar, download, extract, receipt::InstallReceipt, symlink};
use std::fs;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let formula_name = if args.len() > 1 {
        args[1].clone()
    } else {
        // Default to a small, safe package for demonstration
        "curl".to_string()
    };

    println!("Installing {} from bottle...\n", formula_name);
    println!("Step 1: Fetching formula metadata...");

    let api = BrewApi::new()?;

    // Step 1: Fetch formula metadata
    let formula = match api.fetch_formula(&formula_name).await {
        Ok(f) => {
            println!("✓ Found {}", f.name);
            f
        }
        Err(e) => {
            eprintln!("✗ Formula not found: {}", e);
            std::process::exit(1);
        }
    };

    let version = formula
        .versions
        .stable
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No stable version available"))?
        .clone();

    println!("  Version: {}", version);
    println!("  Dependencies: {}", formula.dependencies.len());

    // Step 2: Download bottle
    println!("\nStep 2: Downloading bottle...");
    let client = reqwest::Client::new();

    let bottle_path = match download::download_bottle(&formula, None, &client).await {
        Ok(path) => {
            let size_mb = fs::metadata(&path)
                .map(|m| m.len() as f64 / 1_000_000.0)
                .unwrap_or(0.0);
            println!("✓ Downloaded to: {}", path.display());
            println!("  Size: {:.1} MB", size_mb);
            path
        }
        Err(e) => {
            eprintln!("✗ Download failed: {}", e);
            std::process::exit(1);
        }
    };

    // Step 3: Extract to Cellar
    println!("\nStep 3: Extracting to Cellar...");
    let cellar_path = match extract::extract_bottle(&bottle_path, &formula_name, &version) {
        Ok(path) => {
            println!("✓ Extracted to: {}", path.display());
            path
        }
        Err(e) => {
            eprintln!("✗ Extraction failed: {}", e);
            std::process::exit(1);
        }
    };

    // Step 4: Create installation receipt
    println!("\nStep 4: Creating installation receipt...");

    // Convert formula dependencies to RuntimeDependencies
    // In a real scenario, we'd resolve actual installed versions
    let runtime_deps = formula
        .dependencies
        .iter()
        .enumerate()
        .map(|(idx, dep)| kombrucha::cellar::RuntimeDependency {
            full_name: dep.clone(),
            version: "0.0.0".to_string(), // Would be resolved from cellar
            revision: 0,
            bottle_rebuild: 0,
            pkg_version: "0.0.0".to_string(),
            declared_directly: idx == 0,
        })
        .collect();

    let receipt = InstallReceipt::new_bottle(&formula, runtime_deps, true);

    match receipt.write(&cellar_path) {
        Ok(_) => {
            println!("✓ Receipt created");
            println!("  Homebrew version: {}", receipt.homebrew_version);
        }
        Err(e) => {
            eprintln!("✗ Receipt creation failed: {}", e);
            std::process::exit(1);
        }
    }

    // Step 5: Create symlinks
    println!("\nStep 5: Creating symlinks...");

    match symlink::link_formula(&formula_name, &version) {
        Ok(linked) => {
            println!("✓ Created {} symlinks", linked.len());
            for link in linked.iter().take(5) {
                println!("    {}", link.display());
            }
            if linked.len() > 5 {
                println!("    ... and {} more", linked.len() - 5);
            }
        }
        Err(e) => {
            eprintln!("✗ Symlink creation failed: {}", e);
            std::process::exit(1);
        }
    }

    // Create version-agnostic symlink
    match symlink::optlink(&formula_name, &version) {
        Ok(_) => {
            println!("✓ Created opt symlink: /opt/homebrew/opt/{}", formula_name);
        }
        Err(e) => {
            eprintln!("✗ Opt symlink failed: {}", e);
        }
    }

    // Summary
    println!("\n✓ Installation complete!");
    println!("\nInstalled package details:");
    println!("  Name:    {}", formula_name);
    println!("  Version: {}", version);
    println!("  Path:    {}", cellar_path.display());
    println!("  Prefix:  {}", cellar::detect_prefix().display());

    // Verify installation
    println!("\nVerifying installation...");
    let installed = cellar::get_installed_versions(&formula_name)?;
    if !installed.is_empty() {
        println!("✓ Verified in Cellar: {} version(s) found", installed.len());
    }

    Ok(())
}
