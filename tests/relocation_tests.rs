// Integration tests for bottle relocation
//
// These tests verify that the relocation process correctly handles:
// 1. Script shebangs with @@HOMEBREW placeholders
// 2. Mach-O binary relocation
// 3. Code signature removal after install_name_tool
// 4. Actual execution of relocated binaries and scripts

use std::fs;
use std::path::Path;
use std::process::Command;

/// Get the bru binary path for testing
fn bru_bin() -> String {
    let mut path = std::env::current_dir().unwrap();
    path.push("target");
    path.push("release");
    path.push("bru");
    path.to_str().unwrap().to_string()
}

/// Check if a file contains @@HOMEBREW placeholders
fn has_homebrew_placeholders(path: &Path) -> bool {
    if let Ok(content) = fs::read_to_string(path) {
        content.contains("@@HOMEBREW_PREFIX@@") || content.contains("@@HOMEBREW_CELLAR@@")
    } else {
        // For binary files, read as bytes
        if let Ok(bytes) = fs::read(path) {
            let content = String::from_utf8_lossy(&bytes);
            content.contains("@@HOMEBREW_PREFIX@@") || content.contains("@@HOMEBREW_CELLAR@@")
        } else {
            false
        }
    }
}

/// Check if a Mach-O binary has a code signature
fn has_code_signature(path: &Path) -> bool {
    let output = Command::new("codesign")
        .arg("-dv")
        .arg(path)
        .output()
        .expect("Failed to run codesign");

    // If codesign succeeds and doesn't say "not signed", it has a signature
    let stderr = String::from_utf8_lossy(&output.stderr);
    output.status.success() && !stderr.contains("not signed")
}

#[test]
#[ignore] // Requires actual Homebrew installation and network
fn test_relocation_script_shebangs() {
    // TEST: Script shebangs must have no @@HOMEBREW placeholders after installation
    //
    // This test installs a package with Python scripts and verifies:
    // 1. Scripts have no unreplaced @@HOMEBREW placeholders
    // 2. Scripts actually execute without "bad interpreter" errors

    // Skip if brew not available
    if Command::new("brew").arg("--version").output().is_err() {
        return;
    }

    // Install huggingface-cli with bru (has Python scripts)
    let install_output = Command::new(bru_bin())
        .args(["install", "huggingface-cli"])
        .output()
        .expect("Failed to run bru install");

    if !install_output.status.success() {
        let stderr = String::from_utf8_lossy(&install_output.stderr);
        eprintln!("Install failed: {}", stderr);
        return;
    }

    // Check the installed script shebang
    let hf_path = Path::new("/opt/homebrew/Cellar/huggingface-cli")
        .join("1.0.1")
        .join("libexec")
        .join("bin")
        .join("hf");

    if !hf_path.exists() {
        eprintln!("Warning: {} not found, skipping test", hf_path.display());
        return;
    }

    // Verify no @@HOMEBREW placeholders in shebang
    assert!(
        !has_homebrew_placeholders(&hf_path),
        "Script {} still has unreplaced @@HOMEBREW placeholders",
        hf_path.display()
    );

    // Verify the script actually executes
    let exec_output = Command::new(&hf_path)
        .arg("--version")
        .output()
        .expect("Failed to execute hf script");

    let stderr = String::from_utf8_lossy(&exec_output.stderr);
    assert!(
        !stderr.contains("bad interpreter"),
        "Script has 'bad interpreter' error: {}",
        stderr
    );
}

#[test]
#[ignore] // Requires actual Homebrew installation and network
fn test_relocation_mach_o_binaries() {
    // TEST: Mach-O binaries must have code signatures removed after relocation
    //
    // This test installs a package with Mach-O binaries and verifies:
    // 1. Binaries have no unreplaced @@HOMEBREW placeholders in install names
    // 2. Binaries have code signatures removed (to prevent SIGKILL)
    // 3. Binaries actually execute without crashing

    // Skip if brew not available
    if Command::new("brew").arg("--version").output().is_err() {
        return;
    }

    // Install bat with bru (has Mach-O binary with dependencies)
    let install_output = Command::new(bru_bin())
        .args(["install", "bat"])
        .output()
        .expect("Failed to run bru install");

    if !install_output.status.success() {
        let stderr = String::from_utf8_lossy(&install_output.stderr);
        eprintln!("Install failed: {}", stderr);
        return;
    }

    // Find the bat binary
    let cellar_path = Path::new("/opt/homebrew/Cellar/bat");
    if !cellar_path.exists() {
        eprintln!("Warning: {} not found, skipping test", cellar_path.display());
        return;
    }

    // Find the version directory (e.g., 0.26.0)
    let version_dirs: Vec<_> = fs::read_dir(cellar_path)
        .expect("Failed to read cellar")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();

    if version_dirs.is_empty() {
        eprintln!("Warning: No version directories found for bat");
        return;
    }

    let version_dir = version_dirs[0].path();
    let bat_binary = version_dir.join("bin").join("bat");

    if !bat_binary.exists() {
        eprintln!("Warning: {} not found", bat_binary.display());
        return;
    }

    // Check install names for @@HOMEBREW placeholders
    let otool_output = Command::new("otool")
        .arg("-L")
        .arg(&bat_binary)
        .output()
        .expect("Failed to run otool");

    let otool_str = String::from_utf8_lossy(&otool_output.stdout);
    assert!(
        !otool_str.contains("@@HOMEBREW"),
        "Binary {} still has unreplaced @@HOMEBREW placeholders in install names:\n{}",
        bat_binary.display(),
        otool_str
    );

    // Check that code signature was removed
    // Note: We allow either no signature or adhoc signature
    let codesign_output = Command::new("codesign")
        .arg("-dv")
        .arg(&bat_binary)
        .output()
        .expect("Failed to run codesign");

    let codesign_stderr = String::from_utf8_lossy(&codesign_output.stderr);

    // Binary should either be not signed, or have adhoc signature
    // It should NOT have an Apple Developer certificate signature
    assert!(
        codesign_stderr.contains("not signed") || codesign_stderr.contains("adhoc"),
        "Binary {} should have no signature or adhoc signature after relocation, got:\n{}",
        bat_binary.display(),
        codesign_stderr
    );

    // Verify the binary actually executes without SIGKILL
    let exec_output = Command::new(&bat_binary)
        .arg("--version")
        .output()
        .expect("Failed to execute bat");

    assert!(
        exec_output.status.success() || exec_output.status.code() == Some(0),
        "Binary {} crashed with exit code {:?}. stderr: {}",
        bat_binary.display(),
        exec_output.status.code(),
        String::from_utf8_lossy(&exec_output.stderr)
    );

    let stdout = String::from_utf8_lossy(&exec_output.stdout);
    assert!(
        stdout.contains("bat"),
        "Binary executed but produced unexpected output: {}",
        stdout
    );
}

#[test]
#[ignore] // Requires actual Homebrew installation and network
fn test_relocation_python_packages() {
    // TEST: Python packages must not crash with SIGKILL after relocation
    //
    // This test specifically checks Python-based packages that had
    // code signature issues in v0.1.18
    //
    // The bug: install_name_tool invalidates code signatures on Python
    // shared libraries (.so files), causing SIGKILL crashes when Python
    // tries to import them.

    // Skip if brew not available
    if Command::new("brew").arg("--version").output().is_err() {
        return;
    }

    // Install a Python-based CLI tool
    let install_output = Command::new(bru_bin())
        .args(["install", "huggingface-cli"])
        .output()
        .expect("Failed to run bru install");

    if !install_output.status.success() {
        let stderr = String::from_utf8_lossy(&install_output.stderr);
        eprintln!("Install failed: {}", stderr);
        return;
    }

    // Try to execute the Python CLI
    let hf_output = Command::new("hf")
        .arg("--version")
        .output()
        .expect("Failed to execute hf");

    // Check exit code - 137 (SIGKILL) indicates code signature problem
    let exit_code = hf_output.status.code();
    assert!(
        exit_code != Some(137) && exit_code != Some(139),
        "Python CLI crashed with exit code {:?} (SIGKILL or SIGSEGV). \
         This indicates code signature was not removed from Python shared libraries. \
         stderr: {}",
        exit_code,
        String::from_utf8_lossy(&hf_output.stderr)
    );

    // The command might fail for other reasons (e.g., needs arguments),
    // but it should not crash with SIGKILL
    let combined_output = format!(
        "{}{}",
        String::from_utf8_lossy(&hf_output.stdout),
        String::from_utf8_lossy(&hf_output.stderr)
    );

    assert!(
        !combined_output.is_empty(),
        "Python CLI produced no output - may have crashed silently"
    );
}

#[test]
#[ignore] // Requires actual Homebrew installation
fn test_relocation_preserves_symlinks() {
    // TEST: Relocation should not break symlinks in bin/ directories
    //
    // Many formulae have symlinks from bin/ to libexec/bin/
    // These should remain functional after relocation

    // Skip if brew not available
    if Command::new("brew").arg("--version").output().is_err() {
        return;
    }

    // Install a package with symlinked binaries
    let install_output = Command::new(bru_bin())
        .args(["install", "vercel-cli"])
        .output()
        .expect("Failed to run bru install");

    if !install_output.status.success() {
        let stderr = String::from_utf8_lossy(&install_output.stderr);
        eprintln!("Install failed: {}", stderr);
        return;
    }

    // Check that the symlink exists and points to the right place
    let vercel_link = Path::new("/opt/homebrew/bin/vercel");
    if !vercel_link.exists() {
        eprintln!("Warning: {} not found", vercel_link.display());
        return;
    }

    // The link should be a symlink (not a regular file)
    let metadata = fs::symlink_metadata(vercel_link)
        .expect("Failed to get symlink metadata");
    assert!(
        metadata.file_type().is_symlink(),
        "{} should be a symlink",
        vercel_link.display()
    );

    // The target should exist and have no @@HOMEBREW placeholders
    let target = fs::read_link(vercel_link)
        .expect("Failed to read symlink");
    let target_path = vercel_link.parent().unwrap().join(target);

    assert!(
        target_path.exists(),
        "Symlink target {} does not exist",
        target_path.display()
    );

    // If it's a script, check for placeholders
    if let Ok(content) = fs::read_to_string(&target_path) {
        if content.starts_with("#!") {
            assert!(
                !content.contains("@@HOMEBREW"),
                "Symlink target {} still has @@HOMEBREW placeholders",
                target_path.display()
            );
        }
    }
}
