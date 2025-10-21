// Integration tests for critical workflows
// These tests use real network calls and Homebrew infrastructure
// Run with: cargo test --test integration_tests -- --ignored --test-threads=1

use std::path::PathBuf;
use std::process::Command;

/// Get the bru binary path for testing
fn bru_bin() -> String {
    let mut path = std::env::current_dir().unwrap();
    path.push("target");
    path.push("release");
    path.push("bru");
    path.to_str().unwrap().to_string()
}

/// Get Homebrew Cellar path
fn cellar_path() -> PathBuf {
    #[cfg(target_arch = "aarch64")]
    {
        PathBuf::from("/opt/homebrew/Cellar")
    }
    #[cfg(target_arch = "x86_64")]
    {
        PathBuf::from("/usr/local/Cellar")
    }
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    {
        PathBuf::from("/usr/local/Cellar")
    }
}

/// Get Homebrew bin path
fn bin_path() -> PathBuf {
    #[cfg(target_arch = "aarch64")]
    {
        PathBuf::from("/opt/homebrew/bin")
    }
    #[cfg(target_arch = "x86_64")]
    {
        PathBuf::from("/usr/local/bin")
    }
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    {
        PathBuf::from("/usr/local/bin")
    }
}

#[test]
#[ignore] // Requires network, actual Homebrew installation, and modifies system
fn test_install_uninstall_workflow() {
    // Use "hello" - a tiny, stable formula that's perfect for testing
    let formula = "hello";
    let cellar = cellar_path();
    let bin = bin_path();

    // Clean up any existing installation first
    let _ = Command::new(bru_bin())
        .args(["uninstall", formula])
        .output();

    // Step 1: Verify formula is not installed
    let formula_dir = cellar.join(formula);
    assert!(
        !formula_dir.exists(),
        "Formula should not be installed at start"
    );

    // Step 2: Install formula
    let install_output = Command::new(bru_bin())
        .args(["install", formula])
        .output()
        .expect("Failed to run bru install");

    assert!(
        install_output.status.success(),
        "Install should succeed. stderr: {}",
        String::from_utf8_lossy(&install_output.stderr)
    );

    // Step 3: Verify installation
    assert!(
        formula_dir.exists(),
        "Formula directory should exist after install"
    );

    // Check that binary is linked
    let binary = bin.join(formula);
    assert!(binary.exists(), "Binary should be linked in bin directory");

    // Verify binary is executable and works
    let hello_output = Command::new(&binary)
        .arg("--version")
        .output()
        .expect("Failed to run hello binary");

    assert!(
        hello_output.status.success(),
        "Binary should be executable and run successfully"
    );

    // Step 4: Verify it shows in list
    let list_output = Command::new(bru_bin())
        .args(["list", "--quiet"])
        .output()
        .expect("Failed to run bru list");

    let list_stdout = String::from_utf8_lossy(&list_output.stdout);
    assert!(
        list_stdout.contains(formula),
        "Formula should appear in list output"
    );

    // Step 5: Uninstall formula
    let uninstall_output = Command::new(bru_bin())
        .args(["uninstall", formula])
        .output()
        .expect("Failed to run bru uninstall");

    assert!(
        uninstall_output.status.success(),
        "Uninstall should succeed. stderr: {}",
        String::from_utf8_lossy(&uninstall_output.stderr)
    );

    // Step 6: Verify cleanup
    assert!(
        !formula_dir.exists(),
        "Formula directory should be removed after uninstall"
    );

    assert!(
        !binary.exists() || !binary.read_link().is_ok(),
        "Binary link should be removed after uninstall"
    );

    // Verify it doesn't show in list anymore
    let list_output = Command::new(bru_bin())
        .args(["list", "--quiet"])
        .output()
        .expect("Failed to run bru list");

    let list_stdout = String::from_utf8_lossy(&list_output.stdout);
    assert!(
        !list_stdout.lines().any(|line| line == formula),
        "Formula should not appear in list after uninstall"
    );
}

#[test]
#[ignore] // Requires network and actual Homebrew installation
fn test_install_with_dependencies() {
    // Use a formula with a simple dependency chain
    // tree depends on: (none - it's standalone)
    // Let's use wget which depends on openssl and libidn2
    let formula = "wget";
    let cellar = cellar_path();

    // Clean up first
    let _ = Command::new(bru_bin())
        .args(["uninstall", formula])
        .output();

    // Install with dependencies
    let install_output = Command::new(bru_bin())
        .args(["install", formula])
        .output()
        .expect("Failed to run bru install");

    assert!(
        install_output.status.success(),
        "Install with dependencies should succeed. stderr: {}",
        String::from_utf8_lossy(&install_output.stderr)
    );

    // Verify main formula is installed
    assert!(
        cellar.join(formula).exists(),
        "Main formula should be installed"
    );

    // Verify dependencies are installed
    let stdout = String::from_utf8_lossy(&install_output.stdout);
    // Should mention installing dependencies
    assert!(
        stdout.contains("dependencies") || stdout.contains("Installing"),
        "Should show dependency installation"
    );

    // Clean up
    let _ = Command::new(bru_bin())
        .args(["uninstall", formula])
        .output();
}

#[test]
#[ignore] // Requires network
fn test_error_handling_nonexistent_formula() {
    let output = Command::new(bru_bin())
        .args(["install", "this-formula-definitely-does-not-exist-xyz-123"])
        .output()
        .expect("Failed to run bru install");

    // Should fail gracefully
    assert!(
        !output.status.success(),
        "Should fail when formula doesn't exist"
    );

    // Check both stdout and stderr for error message
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    // Should have a clear error message
    assert!(
        combined.contains("not found") || combined.contains("No formula"),
        "Should have clear error message, got stdout: {}, stderr: {}",
        stdout,
        stderr
    );

    // Should NOT panic (stack backtraces from anyhow are okay)
    assert!(
        !combined.contains("panic"),
        "Should not panic on missing formula"
    );
}

#[test]
#[ignore] // Requires network
fn test_concurrent_operations_install_multiple() {
    // Install multiple small formulae at once
    let formulae = vec!["hello", "tree", "jq"];
    let cellar = cellar_path();

    // Clean up first
    for formula in &formulae {
        let _ = Command::new(bru_bin())
            .args(["uninstall", formula])
            .output();
    }

    // Install all at once
    let mut args = vec!["install"];
    args.extend(formulae.iter().map(|s| *s));

    let install_output = Command::new(bru_bin())
        .args(&args)
        .output()
        .expect("Failed to run bru install");

    assert!(
        install_output.status.success(),
        "Concurrent install should succeed. stderr: {}",
        String::from_utf8_lossy(&install_output.stderr)
    );

    // Verify all are installed
    for formula in &formulae {
        assert!(
            cellar.join(formula).exists(),
            "Formula {} should be installed",
            formula
        );
    }

    // Clean up
    for formula in &formulae {
        let _ = Command::new(bru_bin())
            .args(["uninstall", formula])
            .output();
    }
}

#[test]
#[ignore] // Requires network
fn test_reinstall_workflow() {
    let formula = "hello";
    let cellar = cellar_path();
    let formula_dir = cellar.join(formula);

    // Ensure it's installed first
    let _ = Command::new(bru_bin()).args(["install", formula]).output();

    assert!(formula_dir.exists(), "Formula should be installed");

    // Reinstall
    let reinstall_output = Command::new(bru_bin())
        .args(["reinstall", formula])
        .output()
        .expect("Failed to run bru reinstall");

    assert!(
        reinstall_output.status.success(),
        "Reinstall should succeed. stderr: {}",
        String::from_utf8_lossy(&reinstall_output.stderr)
    );

    // Should still be installed
    assert!(
        formula_dir.exists(),
        "Formula should still be installed after reinstall"
    );

    // Clean up
    let _ = Command::new(bru_bin())
        .args(["uninstall", formula])
        .output();
}

#[test]
#[ignore] // Requires network
fn test_fetch_without_install() {
    let formula = "hello";

    // Fetch the bottle without installing
    let fetch_output = Command::new(bru_bin())
        .args(["fetch", formula])
        .output()
        .expect("Failed to run bru fetch");

    assert!(
        fetch_output.status.success(),
        "Fetch should succeed. stderr: {}",
        String::from_utf8_lossy(&fetch_output.stderr)
    );

    let stdout = String::from_utf8_lossy(&fetch_output.stdout);
    // Should mention downloading
    assert!(
        stdout.contains("Downloaded") || stdout.contains("Fetching"),
        "Should show download progress"
    );

    // Should NOT be installed
    let cellar = cellar_path();
    assert!(
        !cellar.join(formula).exists(),
        "Fetch should not install the formula"
    );
}

#[test]
#[ignore] // Requires network
fn test_upgrade_workflow() {
    // This test is tricky because we need a formula that's actually outdated
    // For now, just verify the command doesn't crash
    let output = Command::new(bru_bin())
        .args(["upgrade", "--dry-run"])
        .output()
        .expect("Failed to run bru upgrade");

    // Should succeed (even if nothing to upgrade)
    assert!(
        output.status.success() || output.status.code() == Some(0),
        "Upgrade dry-run should not crash. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[ignore] // Requires network
fn test_info_real_formula() {
    // Test that info actually fetches and parses real data
    let output = Command::new(bru_bin())
        .args(["info", "wget"])
        .output()
        .expect("Failed to run bru info");

    assert!(
        output.status.success(),
        "Info should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should contain key information
    assert!(stdout.contains("wget"), "Should show formula name");
    assert!(
        stdout.contains("retriever") || stdout.contains("Internet file"),
        "Should show description (wget is 'Internet file retriever')"
    );
    assert!(
        stdout.contains("Homepage") || stdout.contains("http"),
        "Should show homepage"
    );
}

#[test]
#[ignore] // Requires network
fn test_deps_real_formula() {
    // Test dependency resolution with a real formula
    let output = Command::new(bru_bin())
        .args(["deps", "wget"])
        .output()
        .expect("Failed to run bru deps");

    assert!(
        output.status.success(),
        "Deps should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    // wget depends on openssl and libidn2
    assert!(
        stdout.contains("openssl") || stdout.contains("libidn"),
        "Should show dependencies"
    );
}

#[test]
#[ignore] // Requires network
fn test_search_real_query() {
    // Test search with real API
    let output = Command::new(bru_bin())
        .args(["search", "wget"])
        .output()
        .expect("Failed to run bru search");

    assert!(
        output.status.success(),
        "Search should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("wget"),
        "Should find wget in search results"
    );
}
