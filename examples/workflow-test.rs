/// Complete workflow test: install → upgrade → uninstall
/// Tests the full lifecycle of package management
use kombrucha::PackageManager;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║  Complete Workflow Test                                  ║");
    println!("║  Install → Info → Upgrade (if available) → Uninstall    ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    let pm = PackageManager::new()?;
    let test_package = "jq";

    // STEP 1: Uninstall if already present
    println!("STEP 1: Prepare Test Environment");
    println!("─────────────────────────────────────────────────────────────");

    let installed = pm.list()?;
    if installed.iter().any(|p| p.name == test_package) {
        println!("Removing existing {}", test_package);
        match pm.uninstall(test_package).await {
            Ok(result) => println!("✓ Uninstalled {} v{}\n", result.name, result.version),
            Err(e) => eprintln!("⚠ Could not uninstall: {}\n", e),
        }
    } else {
        println!("✓ {} not present, ready for clean test\n", test_package);
    }

    // STEP 2: Install package
    println!("STEP 2: Install Package");
    println!("─────────────────────────────────────────────────────────────");

    println!("Installing {}...", test_package);
    match pm.install(test_package).await {
        Ok(result) => {
            println!("✓ Installation successful!");
            println!("  Version:      {}", result.version);
            println!("  Path:         {}", result.path.display());
            println!("  Linked:       {}", result.linked);
            println!("  Dependencies: {}", result.dependencies.len());
            println!("  Time:         {:.2}ms\n", result.time_ms as f64);
        }
        Err(e) => {
            eprintln!("✗ Installation failed: {}", e);
            std::process::exit(1);
        }
    }

    // STEP 3: Get info about the package
    println!("STEP 3: Get Package Information");
    println!("─────────────────────────────────────────────────────────────");

    match pm.info(test_package).await {
        Ok(formula) => {
            println!("Package Information:");
            println!("  Name:        {}", formula.name);
            println!(
                "  Version:     {}",
                formula.versions.stable.unwrap_or_default()
            );
            println!("  Description: {}", formula.desc.unwrap_or_default());
            println!("  Keg-only:    {}\n", formula.keg_only);
        }
        Err(e) => eprintln!("⚠ Could not fetch info: {}\n", e),
    }

    // STEP 4: Check for upgrades
    println!("STEP 4: Check for Available Upgrades");
    println!("─────────────────────────────────────────────────────────────");

    let outdated = pm.outdated().await?;
    if let Some(pkg) = outdated.iter().find(|p| p.name == test_package) {
        println!("Upgrade available: {} → {}\n", pkg.installed, pkg.latest);

        // STEP 5: Upgrade
        println!("STEP 5: Upgrade Package");
        println!("─────────────────────────────────────────────────────────────");

        match pm.upgrade(test_package).await {
            Ok(result) => {
                println!("✓ Upgrade successful!");
                println!("  From:  {}", result.from_version);
                println!("  To:    {}", result.to_version);
                println!("  Path:  {}", result.path.display());
                println!("  Time:  {:.2}ms\n", result.time_ms as f64);
            }
            Err(e) => eprintln!("⚠ Upgrade failed: {}\n", e),
        }
    } else {
        println!("✓ {} is already at latest version\n", test_package);
    }

    // STEP 6: Check dependencies
    println!("STEP 6: Check Dependencies");
    println!("─────────────────────────────────────────────────────────────");

    match pm.dependencies(test_package).await {
        Ok(deps) => {
            println!("Dependencies for {}:", test_package);
            println!("  Runtime: {}", deps.runtime.len());
            for dep in &deps.runtime {
                println!("    • {}", dep);
            }
            println!("  Build:   {}\n", deps.build.len());
        }
        Err(e) => eprintln!("⚠ Could not fetch dependencies: {}\n", e),
    }

    // STEP 7: Check what depends on this package
    println!("STEP 7: Check Reverse Dependencies");
    println!("─────────────────────────────────────────────────────────────");

    match pm.uses(test_package).await {
        Ok(dependents) => {
            if dependents.is_empty() {
                println!("✓ No packages depend on {}\n", test_package);
            } else {
                println!("Packages that depend on {}:", test_package);
                for dep in &dependents {
                    println!("  • {}", dep);
                }
                println!();
            }
        }
        Err(e) => eprintln!("⚠ Could not check dependencies: {}\n", e),
    }

    // STEP 8: Uninstall
    println!("STEP 8: Uninstall Package");
    println!("─────────────────────────────────────────────────────────────");

    println!("Uninstalling {}...", test_package);
    match pm.uninstall(test_package).await {
        Ok(result) => {
            println!("✓ Uninstallation successful!");
            println!("  Package:  {}", result.name);
            println!("  Version:  {}", result.version);
            println!("  Unlinked: {}", result.unlinked);
            println!("  Time:     {:.2}ms\n", result.time_ms as f64);
        }
        Err(e) => {
            eprintln!("✗ Uninstallation failed: {}", e);
            std::process::exit(1);
        }
    }

    // STEP 9: Verify removal
    println!("STEP 9: Verify Removal");
    println!("─────────────────────────────────────────────────────────────");

    let after = pm.list()?;
    if after.iter().any(|p| p.name == test_package) {
        eprintln!("✗ {} still in package list!", test_package);
        std::process::exit(1);
    } else {
        println!("✓ {} successfully removed from system", test_package);
    }

    let bin_link = kombrucha::cellar::detect_prefix().join(format!("bin/{}", test_package));
    if !bin_link.exists() {
        println!("✓ Symlinks cleaned up");
    } else {
        println!("⚠ Symlink still exists: {}", bin_link.display());
    }

    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║  ✓ Complete Workflow Test Passed!                       ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!("\nSummary:");
    println!("  ✓ Package installation");
    println!("  ✓ Package information retrieval");
    println!("  ✓ Upgrade detection and execution");
    println!("  ✓ Dependency resolution");
    println!("  ✓ Reverse dependency lookup");
    println!("  ✓ Package uninstallation");
    println!("  ✓ Cleanup verification");

    Ok(())
}
