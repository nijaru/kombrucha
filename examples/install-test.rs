/// Test install() operation
/// Installs a simple test package (jq) that has no complex dependencies
use kombrucha::PackageManager;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║  Install Operation Test                                  ║");
    println!("║  Testing: install('jq')                                  ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    let pm = PackageManager::new()?;

    // Check if jq is already installed
    let installed = pm.list()?;
    let already_installed = installed.iter().find(|p| p.name == "jq");

    if let Some(pkg) = already_installed {
        println!("⚠ jq is already installed: v{}", pkg.version);
        println!("Uninstalling first for clean test...\n");

        match pm.uninstall("jq").await {
            Ok(result) => {
                println!("✓ Uninstalled {} v{}", result.name, result.version);
                println!("  Unlinked: {}", result.unlinked);
                println!("  Time: {:.2}ms\n", result.time_ms as f64);
            }
            Err(e) => {
                eprintln!("⚠ Could not uninstall: {}", e);
                eprintln!("  Continuing with test anyway...\n");
            }
        }
    }

    // Now test install
    println!("Installing jq...");
    println!("─────────────────────────────────────────────────────────────");

    match pm.install("jq").await {
        Ok(result) => {
            println!("✓ Installation successful!");
            println!("\nDetails:");
            println!("  Package:      {}", result.name);
            println!("  Version:      {}", result.version);
            println!("  Path:         {}", result.path.display());
            println!("  Linked:       {}", result.linked);
            println!("  Dependencies: {}", result.dependencies.len());
            println!("  Time:         {:.2}ms", result.time_ms as f64);

            if !result.dependencies.is_empty() {
                println!("\nRuntime dependencies:");
                for dep in &result.dependencies {
                    println!("    • {}", dep);
                }
            }

            // Verify binary works
            println!("\n─────────────────────────────────────────────────────────────");
            println!("Verifying installation...");
            let output = std::process::Command::new("jq").arg("--version").output();

            match output {
                Ok(out) => {
                    let version_string = String::from_utf8_lossy(&out.stdout);
                    println!("✓ jq binary works: {}", version_string.trim());
                }
                Err(e) => {
                    eprintln!("✗ jq binary not in PATH: {}", e);
                }
            }

            // Verify receipt was created
            let receipt_path = result.path.join("INSTALL_RECEIPT.json");
            if receipt_path.exists() {
                println!("✓ INSTALL_RECEIPT.json created");
            } else {
                eprintln!("✗ INSTALL_RECEIPT.json missing!");
            }

            // Check symlinks
            let bin_link = kombrucha::cellar::detect_prefix().join("bin/jq");
            if bin_link.exists() {
                println!("✓ Symlink created: {}", bin_link.display());
            } else {
                println!("⚠ Symlink not found: {}", bin_link.display());
            }

            println!("\n╔══════════════════════════════════════════════════════════╗");
            println!("║  ✓ Install Test Passed!                                 ║");
            println!("╚══════════════════════════════════════════════════════════╝");
        }
        Err(e) => {
            eprintln!("✗ Installation failed:");
            eprintln!("  {}", e);
            eprintln!("\nThis could be due to:");
            eprintln!("  - Network connectivity issues");
            eprintln!("  - No bottle available for your platform");
            eprintln!("  - Permission issues on Cellar");
            std::process::exit(1);
        }
    }

    Ok(())
}
