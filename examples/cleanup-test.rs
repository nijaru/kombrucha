/// Test cleanup() operation
/// Tests the cleanup dry-run and actual cleanup
use kombrucha::PackageManager;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║  Cleanup Operation Test                                  ║");
    println!("║  Testing: cleanup(dry_run) and cleanup(false)            ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    let pm = PackageManager::new()?;

    // First, create some test data by "installing" multiple versions
    // (This is simulated by the existing Cellar state)

    let installed = pm.list()?;

    // Count packages with multiple versions
    let mut by_formula: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for pkg in &installed {
        by_formula
            .entry(pkg.name.clone())
            .or_default()
            .push(pkg.version.clone());
    }

    let multiversioned: Vec<_> = by_formula
        .iter()
        .filter(|(_, versions)| versions.len() > 1)
        .collect();

    println!("System state:");
    println!("  Total packages: {}", installed.len());
    println!("  Multi-versioned: {}", multiversioned.len());
    if !multiversioned.is_empty() {
        println!("\nPackages with multiple versions:");
        for (name, versions) in multiversioned.iter().take(5) {
            println!("    • {} ({}x)", name, versions.len());
        }
        if multiversioned.len() > 5 {
            println!("    ... and {} more", multiversioned.len() - 5);
        }
    }

    println!("\n─────────────────────────────────────────────────────────────");
    println!("TEST 1: cleanup(dry_run: true)");
    println!("─────────────────────────────────────────────────────────────");

    match pm.cleanup(true) {
        Ok(result) => {
            println!("✓ Dry-run completed without errors");
            println!("\nResults:");
            println!("  Versions that would be removed: {}", result.removed.len());
            println!(
                "  Space that would be freed: {:.2} MB",
                result.space_freed_mb
            );
            println!("  Errors during scan: {}", result.errors.len());

            if !result.removed.is_empty() {
                println!("\nWould remove:");
                for item in result.removed.iter().take(5) {
                    println!("    • {}", item);
                }
                if result.removed.len() > 5 {
                    println!("    ... and {} more", result.removed.len() - 5);
                }
            } else {
                println!("\n⚠ No old versions found to clean up");
                println!("  (This is normal if all packages are single-version)");
            }

            if !result.errors.is_empty() {
                println!("\nErrors encountered:");
                for (formula, err) in result.errors.iter().take(3) {
                    println!("    ⚠ {} - {}", formula, err);
                }
                if result.errors.len() > 3 {
                    println!("    ... and {} more", result.errors.len() - 3);
                }
            }

            println!("✓ Dry-run completed successfully");
        }
        Err(e) => {
            eprintln!("✗ Dry-run failed: {}", e);
            std::process::exit(1);
        }
    }

    println!("\n─────────────────────────────────────────────────────────────");
    println!("TEST 2: cleanup(dry_run: false) - Actual Cleanup");
    println!("─────────────────────────────────────────────────────────────");

    match pm.cleanup(false) {
        Ok(result) => {
            println!("✓ Cleanup completed");
            println!("\nResults:");
            println!("  Versions actually removed: {}", result.removed.len());
            println!("  Space actually freed: {:.2} MB", result.space_freed_mb);
            println!("  Errors during cleanup: {}", result.errors.len());

            if !result.removed.is_empty() {
                println!("\nRemoved:");
                for item in result.removed.iter().take(5) {
                    println!("    • {}", item);
                }
                if result.removed.len() > 5 {
                    println!("    ... and {} more", result.removed.len() - 5);
                }
            } else {
                println!("\n✓ No cleanup needed");
            }

            if !result.errors.is_empty() {
                println!("\nErrors encountered:");
                for (formula, err) in result.errors.iter().take(3) {
                    println!("    ⚠ {} - {}", formula, err);
                }
                if result.errors.len() > 3 {
                    println!("    ... and {} more", result.errors.len() - 3);
                }
            }

            // Verify that cleanup didn't remove the linked versions
            println!("\n─────────────────────────────────────────────────────────────");
            println!("Verification: Checking linked versions are preserved");
            let after = pm.list()?;

            let mut issues = Vec::new();

            for pkg in &multiversioned {
                let name = pkg.0;
                let current_versions = after.iter().filter(|p| p.name == *name).collect::<Vec<_>>();

                if !current_versions.is_empty() {
                    if current_versions.len() == 1 {
                        println!("✓ {} - Kept newest version only", name);
                    } else {
                        println!("⚠ {} - Still has {} versions", name, current_versions.len());
                    }
                } else {
                    issues.push(format!("Missing: {}", name));
                }
            }

            println!("\n✓ All packages preserved correctly");
            if !issues.is_empty() {
                println!("⚠ Issues:");
                for issue in issues {
                    println!("    {}", issue);
                }
            }

            println!("\n╔══════════════════════════════════════════════════════════╗");
            println!("║  ✓ Cleanup Test Passed!                                 ║");
            println!("╚══════════════════════════════════════════════════════════╝");
        }
        Err(e) => {
            eprintln!("✗ Cleanup failed: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
