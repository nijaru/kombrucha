//! Example: Explore package dependencies
//!
//! This example demonstrates how to fetch a formula and explore its dependency
//! tree, distinguishing between runtime and build dependencies.
//!
//! Usage: cargo run --example dependency_tree [formula_name]

use kombrucha::BrewApi;
use std::collections::{HashMap, HashSet};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let formula_name = if args.len() > 1 {
        args[1].clone()
    } else {
        "python".to_string()
    };

    println!("Dependency tree for: {}\n", formula_name);

    let api = BrewApi::new()?;

    // Fetch the formula
    println!("Fetching formula metadata...\n");
    let formula = api.fetch_formula(&formula_name).await?;

    println!("Package: {}", formula.name);
    if let Some(desc) = &formula.desc {
        println!("Description: {}", desc);
    }
    if let Some(version) = &formula.versions.stable {
        println!("Version: {}", version);
    }
    println!();

    // Display dependencies
    if !formula.dependencies.is_empty() {
        println!("RUNTIME DEPENDENCIES ({}):", formula.dependencies.len());
        for (idx, dep) in formula.dependencies.iter().enumerate() {
            let marker =
                if idx == formula.dependencies.len() - 1 && formula.build_dependencies.is_empty() {
                    "└──"
                } else {
                    "├──"
                };
            println!("{} {}", marker, dep);
        }
    } else {
        println!("RUNTIME DEPENDENCIES: None");
    }

    println!();

    // Display build dependencies
    if !formula.build_dependencies.is_empty() {
        println!("BUILD DEPENDENCIES ({}):", formula.build_dependencies.len());
        for (idx, dep) in formula.build_dependencies.iter().enumerate() {
            let marker = if idx == formula.build_dependencies.len() - 1 {
                "└──"
            } else {
                "├──"
            };
            println!("{} {}", marker, dep);
        }
    } else {
        println!("BUILD DEPENDENCIES: None");
    }

    // Analyze dependency structure
    println!("\n--- Dependency Analysis ---");

    let mut all_deps = HashSet::new();
    let mut dep_count: HashMap<&str, u32> = HashMap::new();

    for dep in &formula.dependencies {
        all_deps.insert(dep.as_str());
        *dep_count.entry(dep.as_str()).or_insert(0) += 1;
    }

    for dep in &formula.build_dependencies {
        if !formula.dependencies.contains(dep) {
            all_deps.insert(dep.as_str());
        }
        *dep_count.entry(dep.as_str()).or_insert(0) += 1;
    }

    println!("Total unique dependencies: {}", all_deps.len());
    println!(
        "Total dependency references: {}",
        formula.dependencies.len() + formula.build_dependencies.len()
    );

    // Categorize dependencies
    let only_runtime = formula
        .dependencies
        .iter()
        .filter(|d| !formula.build_dependencies.contains(d))
        .count();
    let only_build = formula
        .build_dependencies
        .iter()
        .filter(|d| !formula.dependencies.contains(d))
        .count();
    let shared = formula
        .dependencies
        .iter()
        .filter(|d| formula.build_dependencies.contains(d))
        .count();

    println!("\nDependency breakdown:");
    println!("  Runtime only: {}", only_runtime);
    println!("  Build only: {}", only_build);
    println!("  Both: {}", shared);

    // Keg-only info
    if formula.keg_only {
        println!("\n⚠️  This formula is keg-only (not linked to prefix)");
        if let Some(reason) = &formula.keg_only_reason {
            println!("   Reason: {}", reason.reason);
            if !reason.explanation.is_empty() {
                println!("   Explanation: {}", reason.explanation);
            }
        }
    }

    // Bottle availability
    println!(
        "\nBottle availability: {}",
        if formula.versions.bottle { "Yes" } else { "No" }
    );
    if let Some(bottle) = &formula.bottle
        && let Some(stable) = &bottle.stable
    {
        println!("  Rebuild: {}", stable.rebuild);
        println!("  Platforms available: {}", stable.files.len());
        for platform in stable.files.keys() {
            println!("    - {}", platform);
        }
    }

    Ok(())
}
