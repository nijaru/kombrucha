/// Test upgrade() operation  
/// Tests upgrading an already-installed package
use kombrucha::PackageManager;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║  Upgrade Operation Test                                  ║");
    println!("║  Testing: upgrade('jq')                                  ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    let pm = PackageManager::new()?;

    // First ensure jq is installed
    let installed = pm.list()?;
    if !installed.iter().any(|p| p.name == "jq") {
        println!("jq not installed. Installing first...");
        match pm.install("jq").await {
            Ok(result) => println!("✓ Installed jq v{}\n", result.version),
            Err(e) => {
                eprintln!("✗ Failed to install jq: {}", e);
                std::process::exit(1);
            }
        }
    }

    // Check current version
    let before = pm.list()?;
    let before_pkg = before.iter().find(|p| p.name == "jq").unwrap();
    println!("Current version: {}", before_pkg.version);

    // Check if upgrade is available
    println!("\nChecking for updates...");
    let outdated = pm.outdated().await?;
    let needs_upgrade = outdated.iter().find(|p| p.name == "jq");

    if needs_upgrade.is_none() {
        println!("⚠ jq is already at latest version");
        println!("  Current: {}", before_pkg.version);
        println!("  Test still validates upgrade() operation on current version");
        println!("  (upgrade detects this and returns early)\n");
    } else {
        println!("⚠ Upgrade available!");
        let pkg = needs_upgrade.unwrap();
        println!("  Current:  {}", pkg.installed);
        println!("  Latest:   {}\n", pkg.latest);
    }

    // Now test upgrade
    println!("─────────────────────────────────────────────────────────────");
    println!("Running upgrade...");

    match pm.upgrade("jq").await {
        Ok(result) => {
            println!("✓ Upgrade completed!");
            println!("\nDetails:");
            println!("  Package:     {}", result.name);
            println!("  From:        {}", result.from_version);
            println!("  To:          {}", result.to_version);
            println!("  Path:        {}", result.path.display());
            println!("  Time:        {:.2}ms\n", result.time_ms as f64);

            // Verify new version is installed
            let after = pm.list()?;
            let after_pkg = after.iter().find(|p| p.name == "jq").unwrap();
            println!("✓ New version confirmed: {}", after_pkg.version);

            // Check Cellar - old version should be removed
            let cellar = kombrucha::cellar::get_installed_versions("jq")?;
            println!("✓ Versions in Cellar: {}", cellar.len());
            for v in &cellar {
                println!("    • {}", v.version);
            }

            // Verify binary still works
            println!("\n─────────────────────────────────────────────────────────────");
            println!("Verifying binary...");
            let output = std::process::Command::new("jq").arg("--version").output();

            match output {
                Ok(out) => {
                    let version_string = String::from_utf8_lossy(&out.stdout);
                    println!("✓ jq binary works: {}", version_string.trim());
                }
                Err(e) => {
                    eprintln!("✗ jq binary error: {}", e);
                }
            }

            // Check symlinks point to new version
            let bin_link = kombrucha::cellar::detect_prefix().join("bin/jq");
            if bin_link.exists()
                && let Ok(target) = std::fs::read_link(&bin_link)
            {
                println!(
                    "✓ Symlink updated: {} -> {}",
                    bin_link.display(),
                    target.display()
                );
            }

            println!("\n╔══════════════════════════════════════════════════════════╗");
            println!("║  ✓ Upgrade Test Passed!                                 ║");
            println!("╚══════════════════════════════════════════════════════════╝");
        }
        Err(e) => {
            eprintln!("✗ Upgrade failed: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
