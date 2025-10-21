// Regression tests for critical bugs found during development
//
// These tests document real bugs that were discovered and fixed.
// Each test prevents the bug from reoccurring.

use std::process::Command;

/// Get the bru binary path for testing
fn bru_bin() -> String {
    // Use the release build for integration tests
    let mut path = std::env::current_dir().unwrap();
    path.push("target");
    path.push("release");
    path.push("bru");
    path.to_str().unwrap().to_string()
}

#[test]
#[ignore] // Requires actual Homebrew installation
fn test_regression_bottle_revision_false_positive() {
    // BUG: Bottle revision comparison causing false positives
    // DATE: 2025-10-20
    // DESCRIPTION: outdated was incorrectly flagging packages as outdated
    //              when only the bottle revision differed (e.g., 1.4.0_32 vs 1.4.0)
    // CAUSE: Version comparison did not strip bottle revision before comparing
    // FIX: Added strip_bottle_revision() function
    //
    // REPRODUCTION:
    // 1. Have a package with a bottle revision (e.g., mosh 1.4.0_32)
    // 2. API reports version 1.4.0 (no revision)
    // 3. Old code: Showed as outdated (1.4.0_32 → 1.4.0)
    // 4. Fixed code: Correctly shows as up-to-date

    let output = Command::new(bru_bin())
        .args(["outdated", "--formula"])
        .output()
        .expect("Failed to run bru outdated");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Packages that should NOT appear (they differ only in bottle revision):
    // - mosh 1.4.0_31 or 1.4.0_32 (API version: 1.4.0)
    // - ffmpeg 8.0_1 (API version: 8.0)
    // - freetype 2.14.1_1 (API version: 2.14.1)

    assert!(
        !stdout.contains("mosh") || !stdout.contains("→ 1.4.0\n"),
        "mosh should not show as outdated when only bottle revision differs"
    );

    assert!(
        !stdout.contains("ffmpeg") || !stdout.contains("→ 8.0\n"),
        "ffmpeg should not show as outdated when only bottle revision differs"
    );
}

#[test]
#[ignore] // Requires actual Homebrew installation with multiple versions
fn test_regression_multiple_versions_all_checked() {
    // BUG: Multiple installed versions all checked as outdated
    // DATE: 2025-10-20
    // DESCRIPTION: When multiple versions of a package are installed,
    //              outdated was checking ALL versions instead of just the current/linked one
    // CAUSE: list_installed() returns all versions, no deduplication
    // FIX: Added deduplication using modification time to identify current version
    //
    // REPRODUCTION:
    // 1. Install package A version 1.0
    // 2. Upgrade to version 1.1 (keeps both versions)
    // 3. Old code: Both 1.0 and 1.1 checked, false positives
    // 4. Fixed code: Only 1.1 (current) checked
    //
    // EXAMPLE: ruff had versions 0.14.0 and 0.14.1 installed
    // - API version: 0.14.1
    // - Installed: 0.14.1 (current), 0.14.0 (old)
    // - Old behavior: Showed "ruff 0.14.0 → 0.14.1" (FALSE POSITIVE)
    // - Fixed behavior: Doesn't show ruff (already current)

    let output = Command::new(bru_bin())
        .args(["outdated"])
        .output()
        .expect("Failed to run bru outdated");

    // If you have packages with multiple versions installed and all are current,
    // outdated should output nothing
    assert!(
        output.status.success(),
        "bru outdated should exit successfully"
    );

    // The critical test: if brew outdated shows N packages,
    // bru outdated should show the same N packages (not more)
    let brew_output = Command::new("brew")
        .args(["outdated"])
        .output()
        .expect("Failed to run brew outdated");

    let bru_count = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.is_empty())
        .count();

    let brew_count = String::from_utf8_lossy(&brew_output.stdout)
        .lines()
        .filter(|line| !line.is_empty())
        .count();

    assert_eq!(
        bru_count, brew_count,
        "bru and brew should report the same number of outdated packages. \
         Multiple installed versions bug may have returned."
    );
}

#[test]
#[ignore] // Requires actual Homebrew installation
fn test_regression_broken_pipe_panic() {
    // BUG: Broken pipe causes panic when output is piped
    // DATE: 2025-10-20
    // DESCRIPTION: When output is piped and the pipe closes early (e.g., | head -1),
    //              program panicked instead of exiting gracefully
    // CAUSE: No SIGPIPE handler, Rust default behavior is to panic
    // FIX: Added SIGPIPE handler in main() to reset to SIG_DFL
    //
    // REPRODUCTION:
    // 1. Run: bru list | head -1
    // 2. head closes pipe after first line
    // 3. Old code: Panicked with broken pipe error
    // 4. Fixed code: Exits cleanly with code 0

    let output = Command::new("sh")
        .arg("-c")
        .arg(&format!("{} list | head -1", bru_bin()))
        .output()
        .expect("Failed to run piped command");

    // Should exit without panic (exit code 0 or 141/SIGPIPE)
    let exit_code = output.status.code().unwrap_or(0);
    assert!(
        exit_code == 0 || exit_code == 141,
        "Should exit gracefully on broken pipe, got exit code: {}",
        exit_code
    );

    // Should not have panic messages in stderr
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("panic") && !stderr.contains("thread"),
        "Should not panic on broken pipe"
    );
}

#[test]
#[ignore] // Requires actual Homebrew installation
fn test_regression_cask_version_detection_metadata() {
    // BUG: Cask version detection returned ".metadata" as version
    // DATE: 2025-10-20
    // DESCRIPTION: get_installed_cask_version() was reading ".metadata" directory
    //              as a version for casks that store version in subdirectory
    // CAUSE: Didn't skip hidden directories when scanning Caskroom
    // FIX: Updated to read version from .metadata/{version}/ subdirectory
    //
    // REPRODUCTION:
    // 1. Install a font cask (e.g., font-hack-nerd-font)
    // 2. Version stored as: Caskroom/font-hack-nerd-font/.metadata/3.2.1/
    // 3. No version directory at root level
    // 4. Old code: Returned ".metadata" as version
    // 5. Fixed code: Reads "3.2.1" from subdirectory

    // This is hard to test without installing specific casks,
    // but we can at least verify the command doesn't crash
    let output = Command::new(bru_bin())
        .args(["outdated", "--cask"])
        .output()
        .expect("Failed to run bru outdated --cask");

    assert!(
        output.status.success(),
        "bru outdated --cask should not crash"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should never show ".metadata" as a version
    assert!(
        !stdout.contains(".metadata"),
        "Version detection should not return .metadata as version"
    );
}

#[test]
#[ignore] // Requires network and Homebrew API
fn test_regression_api_404_error_messages() {
    // BUG: Generic API error messages for 404 responses
    // DATE: 2025-10-20
    // DESCRIPTION: When querying non-existent packages, showed generic
    //              "API error" instead of specific "Formula not found"
    // CAUSE: No HTTP status code checking before error handling
    // FIX: Added 404 detection to return BruError::FormulaNotFound
    //
    // REPRODUCTION:
    // 1. Run: bru info nonexistent-package-xyz-123
    // 2. Old code: "Error: API request failed: HTTP 404"
    // 3. Fixed code: "Error: Formula 'nonexistent-package-xyz-123' not found"

    let output = Command::new(bru_bin())
        .args(["info", "nonexistent-package-xyz-123-definitely-does-not-exist"])
        .output()
        .expect("Failed to run bru info");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should show "not found" message
    assert!(
        stderr.contains("not found") || stderr.contains("No formula or cask found"),
        "Should show specific not found error, got: {}",
        stderr
    );

    // Should NOT show generic API error
    assert!(
        !stderr.contains("API request failed"),
        "Should not show generic API error for 404"
    );
}

#[test]
fn test_parity_outdated_count() {
    // PARITY TEST: Verify bru and brew report same outdated count
    // This test ensures our outdated detection matches Homebrew's exactly

    // Note: This is a property that should ALWAYS hold:
    // bru and brew should always report the same packages as outdated
    // (though the output format may differ)

    // Skip if brew not available
    if Command::new("brew").arg("--version").output().is_err() {
        return;
    }

    let brew_output = Command::new("brew")
        .args(["outdated", "--quiet"])
        .output()
        .expect("Failed to run brew outdated");

    let bru_output = Command::new(bru_bin())
        .args(["outdated"])
        .output()
        .expect("Failed to run bru outdated");

    let brew_count = String::from_utf8_lossy(&brew_output.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();

    let bru_count = String::from_utf8_lossy(&bru_output.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();

    assert_eq!(
        bru_count, brew_count,
        "bru and brew must report the same number of outdated packages.\n\
         brew count: {}, bru count: {}",
        brew_count,
        bru_count
    );
}
