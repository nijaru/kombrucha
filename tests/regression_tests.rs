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
        .args([
            "info",
            "nonexistent-package-xyz-123-definitely-does-not-exist",
        ])
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
fn test_regression_upgrade_duplicates() {
    // BUG: upgrade showed duplicate packages when multiple versions installed
    // DATE: 2025-10-21
    // DESCRIPTION: When running `bru upgrade` without arguments, packages appeared
    //              multiple times in the "packages to upgrade" list
    // CAUSE: list_installed() returns all installed versions, no deduplication
    // FIX: Added deduplication using modification time in upgrade command
    //
    // REPRODUCTION:
    // 1. Have multiple versions of packages installed (common after upgrades)
    // 2. Run: bru upgrade (without formula names)
    // 3. Old code: "74 packages to upgrade: mosh, mosh, gh, gh, mise, mise, ..."
    // 4. Fixed code: "2 packages to upgrade: mosh, gh"

    // Skip if brew not available
    if Command::new("brew").arg("--version").output().is_err() {
        return;
    }

    let output = Command::new(bru_bin())
        .args(["upgrade", "--dry-run"])
        .output()
        .expect("Failed to run bru upgrade");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse the upgrade list - look for "N packages to upgrade: ..."
    if let Some(line) = stdout.lines().find(|l| l.contains("packages to upgrade:")) {
        // Extract the list of packages after the colon
        if let Some(packages_str) = line.split(':').nth(1) {
            let packages: Vec<&str> = packages_str
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();

            // Count occurrences of each package
            let mut package_counts = std::collections::HashMap::new();
            for pkg in &packages {
                *package_counts.entry(*pkg).or_insert(0) += 1;
            }

            // Check for duplicates
            for (pkg, count) in package_counts {
                assert_eq!(
                    count, 1,
                    "Package '{}' appears {} times in upgrade list (should be 1). \
                     Deduplication bug may have returned. Full line: {}",
                    pkg, count, line
                );
            }
        }
    }
}

#[test]
fn test_regression_upgrade_bottle_revision() {
    // BUG: upgrade attempted to "upgrade" packages differing only in bottle revision
    // DATE: 2025-10-21
    // DESCRIPTION: upgrade showed "mosh 1.4.0_31 → 1.4.0" and attempted upgrade
    // CAUSE: Version comparison in upgrade didn't strip bottle revisions
    // FIX: Added strip_bottle_revision() call before version comparison in upgrade
    //
    // REPRODUCTION:
    // 1. Have package with bottle revision (e.g., mosh 1.4.0_31)
    // 2. API reports 1.4.0 (no revision)
    // 3. Old code: Tried to upgrade 1.4.0_31 → 1.4.0 (failed extraction)
    // 4. Fixed code: Recognizes same version, skips upgrade

    // Skip if brew not available
    if Command::new("brew").arg("--version").output().is_err() {
        return;
    }

    let output = Command::new(bru_bin())
        .args(["upgrade", "--dry-run"])
        .output()
        .expect("Failed to run bru upgrade");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Look for bottle revision "upgrades" like "1.4.0_32 → 1.4.0"
    // Check each line for version_N → version pattern (same version, different revision)
    for line in stdout.lines() {
        // Match pattern: "something X.Y.Z_N → X.Y.Z"
        if line.contains(" → ") {
            let parts: Vec<&str> = line.split(" → ").collect();
            if parts.len() == 2 {
                // Extract version from first part (after last space)
                if let Some(old_ver) = parts[0].split_whitespace().last() {
                    let new_ver = parts[1].trim();

                    // Check if old version has _N and matches new version when stripped
                    if let Some(pos) = old_ver.rfind('_') {
                        let base_ver = &old_ver[..pos];
                        let suffix = &old_ver[pos + 1..];

                        // If suffix is all digits and base matches new version
                        if suffix.chars().all(|c| c.is_ascii_digit()) && base_ver == new_ver {
                            panic!(
                                "Found bottle revision false positive: {}\n\
                                 Trying to upgrade {} → {} (only bottle revision differs)\n\
                                 This suggests the bottle revision bug has returned.",
                                line, old_ver, new_ver
                            );
                        }
                    }
                }
            }
        }
    }
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

    // IMPORTANT: Update taps first to ensure brew and bru use the same data
    // bru uses the online API (formulae.brew.sh) which is always current
    // brew uses local tap data which needs to be updated
    let update_result = Command::new("brew")
        .args(["update", "--quiet"])
        .output()
        .expect("Failed to run brew update");

    // Don't fail the test if update fails (might be network issues)
    if !update_result.status.success() {
        eprintln!("Warning: brew update failed, test may be inaccurate");
    }

    let brew_output = Command::new("brew")
        .args(["outdated", "--quiet"])
        .output()
        .expect("Failed to run brew outdated");

    let bru_output = Command::new(bru_bin())
        .args(["outdated", "--quiet"])
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
        brew_count, bru_count
    );
}

#[test]
fn test_install_dry_run_validation() {
    // TEST: Install --dry-run validates formulae without modification
    // Ensures install command properly validates inputs in dry-run mode

    // Skip if brew not available
    if Command::new("brew").arg("--version").output().is_err() {
        return;
    }

    // Test 1: Valid formula should succeed
    let output = Command::new(bru_bin())
        .args(["install", "--dry-run", "hello"])
        .output()
        .expect("Failed to run bru install --dry-run");

    assert!(
        output.status.success(),
        "Install dry-run should succeed for valid formula"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Dry run") && stdout.contains("hello"),
        "Should indicate dry-run and show formula name"
    );

    // Test 2: Invalid formula should fail gracefully
    let output = Command::new(bru_bin())
        .args(["install", "--dry-run", "nonexistent-formula-xyz-123"])
        .output()
        .expect("Failed to run bru install --dry-run");

    assert!(
        !output.status.success(),
        "Install dry-run should fail for invalid formula"
    );

    // Error might be in stdout or stderr
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    assert!(
        combined.contains("not found") || combined.contains("No formula"),
        "Should show clear error for nonexistent formula. Got:\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );
}

#[test]
fn test_search_basic_functionality() {
    // TEST: Search command should work without Homebrew installation
    // Search uses the API directly, no local state required

    let output = Command::new(bru_bin())
        .args(["search", "rust"])
        .output()
        .expect("Failed to run bru search");

    assert!(output.status.success(), "Search should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should find common Rust-related formulae
    assert!(
        stdout.contains("rust") || stdout.contains("Rust"),
        "Search for 'rust' should return results containing 'rust'"
    );

    // Should show result count
    assert!(
        stdout.contains("Found") || stdout.contains("results"),
        "Should indicate number of results"
    );
}

#[test]
fn test_info_basic_functionality() {
    // TEST: Info command should work for common formulae
    // Info uses the API directly, no local state required

    let output = Command::new(bru_bin())
        .args(["info", "wget"])
        .output()
        .expect("Failed to run bru info");

    assert!(
        output.status.success(),
        "Info should succeed for valid formula"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should show key information
    assert!(stdout.contains("wget"), "Should show formula name");

    assert!(
        stdout.contains("Homepage:") || stdout.contains("Version:"),
        "Should show formula metadata"
    );
}

#[test]
fn test_deps_basic_functionality() {
    // TEST: Deps command should work for common formulae
    // Deps uses the API directly, no local state required

    let output = Command::new(bru_bin())
        .args(["deps", "wget"])
        .output()
        .expect("Failed to run bru deps");

    assert!(
        output.status.success(),
        "Deps should succeed for valid formula"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // wget has dependencies like openssl, so should show them
    assert!(
        stdout.contains("dependencies") || stdout.contains("openssl"),
        "Should show dependencies"
    );
}

#[test]
fn test_list_no_crash() {
    // TEST: List command should never crash
    // Even if no packages installed, should return gracefully

    // Skip if brew not available
    if Command::new("brew").arg("--version").output().is_err() {
        return;
    }

    let output = Command::new(bru_bin())
        .args(["list"])
        .output()
        .expect("Failed to run bru list");

    // Should always succeed (even if empty)
    assert!(output.status.success(), "List should never crash");

    // Should not panic
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("panic") && !stderr.contains("thread"),
        "List should not panic"
    );
}

#[test]
fn test_autoremove_dry_run() {
    // TEST: Autoremove --dry-run should detect unused deps without removing
    // This tests the dependency analysis without system modification

    // Skip if brew not available
    if Command::new("brew").arg("--version").output().is_err() {
        return;
    }

    let output = Command::new(bru_bin())
        .args(["autoremove", "--dry-run"])
        .output()
        .expect("Failed to run bru autoremove --dry-run");

    assert!(output.status.success(), "Autoremove dry-run should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should indicate dry-run mode
    assert!(
        stdout.contains("Dry run") || stdout.contains("Would remove"),
        "Should indicate dry-run mode"
    );

    // If there are unused deps, should list them
    // If none, should say "No unused dependencies"
    assert!(
        stdout.contains("unused") || stdout.contains("dependencies"),
        "Should mention dependencies"
    );
}

#[test]
fn test_cleanup_dry_run() {
    // TEST: Cleanup --dry-run should detect old versions without removing
    // This tests old version detection without system modification

    // Skip if brew not available
    if Command::new("brew").arg("--version").output().is_err() {
        return;
    }

    let output = Command::new(bru_bin())
        .args(["cleanup", "--dry-run"])
        .output()
        .expect("Failed to run bru cleanup --dry-run");

    assert!(output.status.success(), "Cleanup dry-run should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should indicate dry-run mode
    assert!(
        stdout.contains("Dry run") || stdout.contains("Would remove"),
        "Should indicate dry-run mode"
    );
}

#[test]
fn test_fetch_validation() {
    // TEST: Fetch should show errors for nonexistent formulae
    // Tests error handling for nonexistent formulae

    let output = Command::new(bru_bin())
        .args(["fetch", "nonexistent-formula-xyz-123"])
        .output()
        .expect("Failed to run bru fetch");

    // Fetch may succeed even if individual formulae fail (it continues processing)
    // Check that error message is shown
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    assert!(
        combined.contains("not found") || combined.contains("Failed to fetch"),
        "Should show error for nonexistent formula. Got:\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );
}

#[test]
fn test_help_command() {
    // TEST: Help should always work and show all commands
    // Basic sanity check for help output

    let output = Command::new(bru_bin())
        .args(["help"])
        .output()
        .expect("Failed to run bru help");

    assert!(output.status.success(), "Help should always succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should show common commands
    assert!(
        stdout.contains("install") && stdout.contains("search") && stdout.contains("upgrade"),
        "Help should list core commands"
    );

    // Should show usage
    assert!(
        stdout.contains("Usage") || stdout.contains("USAGE"),
        "Help should show usage information"
    );
}

#[test]
fn test_version_flag() {
    // TEST: Version flag should always work
    // Basic sanity check

    let output = Command::new(bru_bin())
        .args(["--version"])
        .output()
        .expect("Failed to run bru --version");

    assert!(output.status.success(), "Version flag should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should show version number
    assert!(
        stdout.contains("0.1") || stdout.contains("kombrucha"),
        "Should show version information"
    );
}
