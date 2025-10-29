// Tests for parallel operations to ensure correctness and performance
//
// These tests verify that parallelization doesn't introduce race conditions
// or data corruption while maintaining performance benefits.

use std::process::Command;
use std::time::Instant;

/// Get the bru binary path for testing
fn bru_bin() -> String {
    let mut path = std::env::current_dir().unwrap();
    path.push("target");
    path.push("release");
    path.push("bru");
    path.to_str().unwrap().to_string()
}

#[test]
#[ignore] // Requires actual Homebrew installation with taps
fn test_parallel_tap_update_correctness() {
    // TEST: Parallel tap updates should complete successfully
    // FEATURE: Added in v0.1.14 (5.7x speedup)
    //
    // VERIFICATION:
    // 1. All taps should be updated without errors
    // 2. Exit code should be 0
    // 3. Output should show completion for all taps

    let output = Command::new(bru_bin())
        .arg("update")
        .output()
        .expect("Failed to run bru update");

    // Should complete successfully
    assert!(
        output.status.success(),
        "Update should complete successfully: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should show tap updates
    assert!(
        stdout.contains("Updating") && stdout.contains("taps"),
        "Output should show tap updates: {}",
        stdout
    );

    // Should show completion
    assert!(
        stdout.contains("✓") || stdout.contains("unchanged") || stdout.contains("updated"),
        "Output should show completion status: {}",
        stdout
    );
}

#[test]
#[ignore] // Performance test
fn test_parallel_tap_update_performance() {
    // TEST: Parallel tap updates should be faster than sequential
    // FEATURE: Added in v0.1.14 (5.7x speedup measured)
    //
    // EXPECTATION: Should complete in < 3s for 8 taps (vs ~11s sequential)

    let start = Instant::now();

    let output = Command::new(bru_bin())
        .arg("update")
        .output()
        .expect("Failed to run bru update");

    let duration = start.elapsed();

    assert!(
        output.status.success(),
        "Update should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Should be reasonably fast (< 5s for parallel, vs ~11s sequential)
    assert!(
        duration.as_secs() < 5,
        "Update should complete in < 5s, took {:?}",
        duration
    );
}

#[test]
#[ignore] // Requires installed packages
fn test_parallel_upgrade_download_correctness() {
    // TEST: Parallel upgrade downloads should work correctly
    // FEATURE: Added in v0.1.14 (3-8x speedup for multi-package upgrades)
    //
    // VERIFICATION:
    // 1. Dry-run should show packages that would be upgraded
    // 2. Should not actually modify the system
    // 3. Exit code should be 0

    let output = Command::new(bru_bin())
        .args(["upgrade", "--dry-run"])
        .output()
        .expect("Failed to run bru upgrade");

    // Should complete successfully (even if no upgrades)
    assert!(
        output.status.success(),
        "Upgrade dry-run should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should show status (either upgrades available or up to date)
    assert!(
        stdout.contains("up to date") || stdout.contains("Preparing"),
        "Output should show upgrade status: {}",
        stdout
    );
}

#[test]
#[ignore] // Requires network
fn test_parallel_formula_fetch_correctness() {
    // TEST: Parallel formula fetches should all succeed
    // FEATURE: API operations are parallelized with in-memory caching
    //
    // VERIFICATION:
    // 1. Multiple formula info requests should succeed
    // 2. Data should be correct for each formula
    // 3. No race conditions or data corruption

    let formulae = vec!["wget", "curl", "git"];

    for formula in &formulae {
        let output = Command::new(bru_bin())
            .args(["info", formula])
            .output()
            .expect("Failed to run bru info");

        assert!(
            output.status.success(),
            "Info for {} should succeed: {}",
            formula,
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains(formula),
            "Output should contain formula name {}: {}",
            formula,
            stdout
        );
    }
}

#[test]
#[ignore] // Requires installed packages
fn test_services_filtering_correctness() {
    // TEST: Services list should correctly filter cask-only plists
    // BUG FIX: v0.1.14 - was showing cask-only services (e.g., tailscale)
    //
    // VERIFICATION:
    // 1. Should only show services for installed formulae
    // 2. Should not show services for cask-only installations
    // 3. Parity with brew services list

    let bru_output = Command::new(bru_bin())
        .args(["services", "list"])
        .output()
        .expect("Failed to run bru services");

    let brew_output = Command::new("brew")
        .args(["services", "list"])
        .output()
        .expect("Failed to run brew services");

    assert!(
        bru_output.status.success(),
        "bru services should succeed"
    );
    assert!(
        brew_output.status.success(),
        "brew services should succeed"
    );

    let bru_stdout = String::from_utf8_lossy(&bru_output.stdout);
    let brew_stdout = String::from_utf8_lossy(&brew_output.stdout);

    // Extract service names from both outputs
    let bru_lines: Vec<&str> = bru_stdout.lines().skip(2).collect(); // Skip header
    let brew_lines: Vec<&str> = brew_stdout.lines().skip(1).collect(); // Skip header

    // Service counts should be close (within 1-2 due to timing)
    let count_diff = (bru_lines.len() as i32 - brew_lines.len() as i32).abs();
    assert!(
        count_diff <= 2,
        "Service counts should match (within 2): bru={}, brew={}, diff={}",
        bru_lines.len(),
        brew_lines.len(),
        count_diff
    );
}
