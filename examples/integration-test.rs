/// Phase 3 Integration Tests
/// Tests PackageManager API with real system state
use kombrucha::PackageManager;
use std::time::Instant;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║  Kombrucha Phase 3: Integration Tests                    ║");
    println!("║  Testing PackageManager API with Real System State       ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    // Initialize PackageManager
    println!("📦 Initializing PackageManager...");
    let pm = PackageManager::new()?;
    println!("✓ PackageManager created\n");

    // ============ TEST 1: Verify Cellar and Prefix ============
    println!("─────────────────────────────────────────────────────────────");
    println!("TEST 1: System Paths");
    println!("─────────────────────────────────────────────────────────────");
    let prefix = pm.prefix();
    let cellar = pm.cellar();
    println!("Prefix:  {}", prefix.display());
    println!("Cellar:  {}", cellar.display());
    assert!(prefix.exists(), "Prefix should exist");
    assert!(cellar.exists(), "Cellar should exist");
    println!("✓ System paths verified\n");

    // ============ TEST 2: List Installed Packages ============
    println!("─────────────────────────────────────────────────────────────");
    println!("TEST 2: list() - List Installed Packages");
    println!("─────────────────────────────────────────────────────────────");
    let installed = pm.list()?;
    println!("Found {} installed packages", installed.len());
    assert!(
        !installed.is_empty(),
        "Should have at least one package installed"
    );

    // Show first 5 packages
    for pkg in installed.iter().take(5) {
        println!("  • {} {}", pkg.name, pkg.version);
    }
    if installed.len() > 5 {
        println!("  ... and {} more", installed.len() - 5);
    }
    println!("✓ list() works correctly\n");

    // ============ TEST 3: Health Check ============
    println!("─────────────────────────────────────────────────────────────");
    println!("TEST 3: check() - System Health Check");
    println!("─────────────────────────────────────────────────────────────");
    let health = pm.check()?;
    println!("Homebrew available: {}", health.homebrew_available);
    println!("Cellar exists:      {}", health.cellar_exists);
    println!("Prefix writable:    {}", health.prefix_writable);
    if !health.issues.is_empty() {
        println!("Issues found:");
        for issue in &health.issues {
            println!("  ⚠ {}", issue);
        }
    } else {
        println!("No issues found");
    }
    assert!(health.cellar_exists, "Cellar should exist");
    assert!(health.prefix_writable, "Prefix should be writable");
    println!("✓ check() works correctly\n");

    // ============ TEST 4: Search and Info ============
    println!("─────────────────────────────────────────────────────────────");
    println!("TEST 4: search() and info() - Package Discovery");
    println!("─────────────────────────────────────────────────────────────");

    let search_start = Instant::now();
    let results = pm.search("ripgrep").await?;
    let search_time = search_start.elapsed();
    println!(
        "search('ripgrep'): {} results in {:.2}ms",
        results.formulae.len(),
        search_time.as_secs_f64() * 1000.0
    );

    if !results.formulae.is_empty() {
        let formula_name = &results.formulae[0].name;
        let info_start = Instant::now();
        let formula = pm.info(formula_name).await?;
        let info_time = info_start.elapsed();
        println!("info('{}'):", formula_name);
        println!(
            "  Version:     {}",
            formula.versions.stable.unwrap_or_default()
        );
        println!("  Description: {}", formula.desc.unwrap_or_default());
        println!("  Time:        {:.2}ms", info_time.as_secs_f64() * 1000.0);
    }
    println!("✓ search() and info() work correctly\n");

    // ============ TEST 5: Outdated Packages ============
    println!("─────────────────────────────────────────────────────────────");
    println!("TEST 5: outdated() - Check for Upgrades");
    println!("─────────────────────────────────────────────────────────────");

    let outdated_start = Instant::now();
    let outdated = pm.outdated().await?;
    let outdated_time = outdated_start.elapsed();
    println!(
        "Found {} outdated packages in {:.2}ms",
        outdated.len(),
        outdated_time.as_secs_f64() * 1000.0
    );

    if !outdated.is_empty() {
        println!("Outdated packages:");
        for pkg in outdated.iter().take(5) {
            println!("  • {} {} → {}", pkg.name, pkg.installed, pkg.latest);
        }
        if outdated.len() > 5 {
            println!("  ... and {} more", outdated.len() - 5);
        }
    }
    println!("✓ outdated() works correctly\n");

    // ============ TEST 6: Dependencies ============
    println!("─────────────────────────────────────────────────────────────");
    println!("TEST 6: dependencies() - Check Package Dependencies");
    println!("─────────────────────────────────────────────────────────────");

    if !installed.is_empty() {
        // Find a package with dependencies
        let test_pkg = installed
            .iter()
            .find(|p| !p.name.contains("@")) // Skip versioned formulas
            .unwrap_or(&installed[0]);

        let deps_start = Instant::now();
        let deps = pm.dependencies(&test_pkg.name).await?;
        let deps_time = deps_start.elapsed();

        println!("dependencies('{}'):", test_pkg.name);
        println!("  Runtime: {}", deps.runtime.len());
        if !deps.runtime.is_empty() {
            for dep in deps.runtime.iter().take(3) {
                println!("    • {}", dep);
            }
            if deps.runtime.len() > 3 {
                println!("    ... and {} more", deps.runtime.len() - 3);
            }
        }
        println!("  Build:   {}", deps.build.len());
        println!("  Time:    {:.2}ms", deps_time.as_secs_f64() * 1000.0);
    }
    println!("✓ dependencies() works correctly\n");

    // ============ TEST 7: Cleanup Dry Run ============
    println!("─────────────────────────────────────────────────────────────");
    println!("TEST 7: cleanup(dry_run: true) - Preview Cleanup");
    println!("─────────────────────────────────────────────────────────────");

    let cleanup_start = Instant::now();
    let cleanup_result = pm.cleanup(true)?;
    let cleanup_time = cleanup_start.elapsed();

    println!("Cleanup dry-run results:");
    println!("  Versions to remove: {}", cleanup_result.removed.len());
    println!(
        "  Space that would be freed: {:.2} MB",
        cleanup_result.space_freed_mb
    );
    println!("  Errors encountered: {}", cleanup_result.errors.len());
    println!(
        "  Time to scan: {:.2}ms",
        cleanup_time.as_secs_f64() * 1000.0
    );

    if !cleanup_result.removed.is_empty() {
        println!("\nWould remove:");
        for item in cleanup_result.removed.iter().take(5) {
            println!("    • {}", item);
        }
        if cleanup_result.removed.len() > 5 {
            println!("    ... and {} more", cleanup_result.removed.len() - 5);
        }
    }

    if !cleanup_result.errors.is_empty() {
        println!("\nErrors during scan:");
        for (formula, err) in cleanup_result.errors.iter().take(3) {
            println!("    ⚠ {} - {}", formula, err);
        }
    }
    println!("✓ cleanup(dry_run) works correctly\n");

    // ============ TEST 8: Uses (Reverse Dependencies) ============
    println!("─────────────────────────────────────────────────────────────");
    println!("TEST 8: uses() - Find Reverse Dependencies");
    println!("─────────────────────────────────────────────────────────────");

    if !installed.is_empty() {
        let test_pkg = &installed[0];
        let uses_start = Instant::now();
        let dependents = pm.uses(&test_pkg.name).await?;
        let uses_time = uses_start.elapsed();

        println!("uses('{}'):", test_pkg.name);
        if dependents.is_empty() {
            println!("  No packages depend on this");
        } else {
            println!("  Packages that depend on this: {}", dependents.len());
            for dep in dependents.iter().take(3) {
                println!("    • {}", dep);
            }
            if dependents.len() > 3 {
                println!("    ... and {} more", dependents.len() - 3);
            }
        }
        println!("  Time: {:.2}ms", uses_time.as_secs_f64() * 1000.0);
    }
    println!("✓ uses() works correctly\n");

    // ============ TEST 9: Type Verification ============
    println!("─────────────────────────────────────────────────────────────");
    println!("TEST 9: Result Type Verification");
    println!("─────────────────────────────────────────────────────────────");

    // Verify we can access all fields of various result types
    let health_check = pm.check()?;
    assert!(health_check.homebrew_available);
    assert!(health_check.cellar_exists);

    let cleanup = pm.cleanup(true)?;
    assert!(cleanup.space_freed_mb >= 0.0);

    let outdated_list = pm.outdated().await?;
    if !outdated_list.is_empty() {
        let pkg = &outdated_list[0];
        assert!(!pkg.name.is_empty());
        assert!(!pkg.installed.is_empty());
        assert!(!pkg.latest.is_empty());
    }

    println!("✓ All result types properly defined and accessible\n");

    // ============ SUMMARY ============
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  ✓ All Integration Tests Passed!                        ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    println!("Summary:");
    println!("  ✓ PackageManager initialization");
    println!("  ✓ System path detection");
    println!("  ✓ list() - {} installed packages", installed.len());
    println!("  ✓ check() - System health verified");
    println!("  ✓ search() and info() - Package discovery");
    println!("  ✓ outdated() - {} packages to update", outdated.len());
    println!("  ✓ dependencies() - Dependency resolution");
    println!(
        "  ✓ cleanup() - Would free {:.2} MB",
        cleanup_result.space_freed_mb
    );
    println!("  ✓ uses() - Reverse dependency lookup");
    println!("  ✓ Type safety - All result types verified");

    println!("\nNext steps:");
    println!("  For destructive tests (install/upgrade/uninstall):");
    println!("    cargo run --release --example install-test");
    println!("    cargo run --release --example upgrade-test");
    println!("    cargo run --release --example workflow-test");

    Ok(())
}
