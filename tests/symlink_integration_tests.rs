use anyhow::Result;
use std::fs;
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

// Mock cellar structure for testing
fn create_mock_cellar(temp_dir: &Path, formula: &str, version: &str) -> Result<PathBuf> {
    let cellar = temp_dir.join("Cellar");
    let formula_path = cellar.join(formula).join(version);

    // Create bin directory with test binaries
    let bin_dir = formula_path.join("bin");
    fs::create_dir_all(&bin_dir)?;
    fs::write(bin_dir.join("test-binary"), "#!/bin/sh\necho test")?;

    // Create share directory with nested structure
    let share_dir = formula_path.join("share");
    let man_dir = share_dir.join("man").join("man1");
    fs::create_dir_all(&man_dir)?;
    fs::write(man_dir.join("test.1"), "test man page")?;

    // Create lib directory
    let lib_dir = formula_path.join("lib");
    fs::create_dir_all(&lib_dir)?;
    fs::write(lib_dir.join("libtest.so"), "fake library")?;

    Ok(formula_path)
}

#[test]
fn test_link_formula_basic() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let prefix = temp_dir.path();
    let cellar = prefix.join("Cellar");

    // Create mock formula
    create_mock_cellar(prefix, "testpkg", "1.0.0")?;

    // Create prefix directories
    fs::create_dir_all(prefix.join("bin"))?;
    fs::create_dir_all(prefix.join("share"))?;
    fs::create_dir_all(prefix.join("lib"))?;

    // Link formula using relative paths manually
    let formula_path = cellar.join("testpkg").join("1.0.0");
    let source_bin = formula_path.join("bin").join("test-binary");
    let target_bin = prefix.join("bin").join("test-binary");

    // Create relative symlink
    unix_fs::symlink("../Cellar/testpkg/1.0.0/bin/test-binary", &target_bin)?;

    // Verify symlink exists and points to correct location
    assert!(target_bin.symlink_metadata()?.is_symlink());
    let link_target = fs::read_link(&target_bin)?;
    assert_eq!(
        link_target,
        PathBuf::from("../Cellar/testpkg/1.0.0/bin/test-binary")
    );

    // Verify target resolves correctly
    let resolved = target_bin.parent().unwrap().join(&link_target);
    let normalized = kombrucha::symlink::normalize_path(&resolved);
    assert_eq!(normalized, source_bin);

    Ok(())
}

#[test]
fn test_symlink_overwrite_existing() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let prefix = temp_dir.path();

    // Create two versions of a formula
    create_mock_cellar(prefix, "testpkg", "1.0.0")?;
    create_mock_cellar(prefix, "testpkg", "2.0.0")?;

    fs::create_dir_all(prefix.join("bin"))?;

    let target = prefix.join("bin").join("test-binary");

    // Create symlink to v1.0.0
    unix_fs::symlink("../Cellar/testpkg/1.0.0/bin/test-binary", &target)?;
    let link1 = fs::read_link(&target)?;
    assert_eq!(
        link1,
        PathBuf::from("../Cellar/testpkg/1.0.0/bin/test-binary")
    );

    // Simulate upgrading by removing old symlink and creating new one
    fs::remove_file(&target)?;
    unix_fs::symlink("../Cellar/testpkg/2.0.0/bin/test-binary", &target)?;

    // Verify it now points to v2.0.0
    let link2 = fs::read_link(&target)?;
    assert_eq!(
        link2,
        PathBuf::from("../Cellar/testpkg/2.0.0/bin/test-binary")
    );

    Ok(())
}

#[test]
fn test_symlink_skip_real_file() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let prefix = temp_dir.path();

    create_mock_cellar(prefix, "testpkg", "1.0.0")?;

    fs::create_dir_all(prefix.join("bin"))?;
    let target = prefix.join("bin").join("test-binary");

    // Create a real file (not symlink)
    fs::write(&target, "user's custom script")?;

    // Verify it's a real file
    let metadata = target.symlink_metadata()?;
    assert!(!metadata.is_symlink());
    assert!(metadata.is_file());

    // Simulate link logic - should detect it's a file and skip
    if let Ok(metadata) = target.symlink_metadata() {
        if metadata.is_symlink() {
            panic!("Should not be a symlink");
        } else {
            // Correct: skip real files
            let content = fs::read_to_string(&target)?;
            assert_eq!(content, "user's custom script");
        }
    }

    Ok(())
}

#[test]
fn test_symlink_replace_broken() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let prefix = temp_dir.path();

    create_mock_cellar(prefix, "testpkg", "1.0.0")?;

    fs::create_dir_all(prefix.join("bin"))?;
    let target = prefix.join("bin").join("test-binary");

    // Create broken symlink (points to non-existent file)
    unix_fs::symlink("/nonexistent/path", &target)?;

    // Verify it's a symlink but broken
    assert!(target.symlink_metadata().is_ok());
    assert!(target.symlink_metadata()?.is_symlink());
    assert!(!target.exists()); // Broken - target doesn't exist

    // Simulate repair - remove broken symlink and create correct one
    fs::remove_file(&target)?;
    unix_fs::symlink("../Cellar/testpkg/1.0.0/bin/test-binary", &target)?;

    // Verify new symlink works
    let link = fs::read_link(&target)?;
    assert_eq!(
        link,
        PathBuf::from("../Cellar/testpkg/1.0.0/bin/test-binary")
    );

    Ok(())
}

#[test]
fn test_unlink_formula() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let prefix = temp_dir.path();
    let cellar = prefix.join("Cellar");

    create_mock_cellar(prefix, "testpkg", "1.0.0")?;

    fs::create_dir_all(prefix.join("bin"))?;
    fs::create_dir_all(prefix.join("lib"))?;

    let formula_path = cellar.join("testpkg").join("1.0.0");

    // Create symlinks
    let bin_target = prefix.join("bin").join("test-binary");
    let lib_target = prefix.join("lib").join("libtest.so");

    unix_fs::symlink("../Cellar/testpkg/1.0.0/bin/test-binary", &bin_target)?;
    unix_fs::symlink("../Cellar/testpkg/1.0.0/lib/libtest.so", &lib_target)?;

    // Verify symlinks exist
    assert!(bin_target.symlink_metadata()?.is_symlink());
    assert!(lib_target.symlink_metadata()?.is_symlink());

    // Simulate unlinking - find and remove symlinks pointing to formula
    for entry in fs::read_dir(prefix.join("bin"))? {
        let entry = entry?;
        let path = entry.path();
        if let Ok(metadata) = path.symlink_metadata() {
            if metadata.is_symlink() {
                if let Ok(link_target) = fs::read_link(&path) {
                    let resolved = if link_target.is_relative() {
                        path.parent().unwrap().join(&link_target)
                    } else {
                        link_target
                    };
                    let normalized = kombrucha::symlink::normalize_path(&resolved);
                    if normalized.starts_with(&formula_path) {
                        fs::remove_file(&path)?;
                    }
                }
            }
        }
    }

    // Verify bin symlink removed
    assert!(!bin_target.exists());

    // lib symlink should still exist (we only processed bin)
    assert!(lib_target.exists());

    Ok(())
}

#[test]
fn test_relative_path_calculation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let prefix = temp_dir.path();

    // Test depth 1: /prefix/bin/binary -> ../Cellar/pkg/1.0.0/bin/binary
    let bin_dir = prefix.join("bin");
    fs::create_dir_all(&bin_dir)?;

    // Expected: 1 level deep, need 1 ".."
    let _expected = PathBuf::from("../Cellar/testpkg/1.0.0/bin/test-binary");

    // Test depth 2: /prefix/share/man/binary -> ../../Cellar/pkg/1.0.0/share/man/binary
    let man_dir = prefix.join("share").join("man");
    fs::create_dir_all(&man_dir)?;

    // Expected: 2 levels deep, need 2 ".."
    let _expected2 = PathBuf::from("../../Cellar/testpkg/1.0.0/share/man/test.1");

    // Verify path structure
    assert_eq!(bin_dir.strip_prefix(prefix)?.components().count(), 1);
    assert_eq!(man_dir.strip_prefix(prefix)?.components().count(), 2);

    Ok(())
}

#[test]
fn test_directory_recursion() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let prefix = temp_dir.path();

    create_mock_cellar(prefix, "testpkg", "1.0.0")?;

    let cellar = prefix.join("Cellar");
    let formula_path = cellar.join("testpkg").join("1.0.0");

    // Create nested directory structure
    let share_dir = formula_path.join("share");
    fs::create_dir_all(share_dir.join("doc").join("testpkg"))?;
    fs::write(
        share_dir.join("doc").join("testpkg").join("README"),
        "readme",
    )?;

    // Verify nested structure exists
    assert!(
        share_dir
            .join("doc")
            .join("testpkg")
            .join("README")
            .exists()
    );

    // Test directory traversal
    fn count_files(dir: &Path) -> usize {
        let mut count = 0;
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() {
                        count += 1;
                    } else if path.is_dir() {
                        count += count_files(&path);
                    }
                }
            }
        }
        count
    }

    let file_count = count_files(&formula_path);
    // We have: bin/test-binary, share/man/man1/test.1, lib/libtest.so, share/doc/testpkg/README
    assert_eq!(file_count, 4);

    Ok(())
}

#[test]
fn test_normalize_path_with_symlinks() {
    // Test that normalize_path correctly handles .. and . components
    let path = Path::new("/opt/homebrew/bin/../Cellar/wget/1.21.4/bin/wget");
    let normalized = kombrucha::symlink::normalize_path(path);
    assert_eq!(
        normalized,
        PathBuf::from("/opt/homebrew/Cellar/wget/1.21.4/bin/wget")
    );
}

#[test]
fn test_skip_already_correct_symlink() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let prefix = temp_dir.path();

    create_mock_cellar(prefix, "testpkg", "1.0.0")?;

    fs::create_dir_all(prefix.join("bin"))?;
    let target = prefix.join("bin").join("test-binary");

    // Create correct symlink
    unix_fs::symlink("../Cellar/testpkg/1.0.0/bin/test-binary", &target)?;

    // Record original metadata
    let original_link = fs::read_link(&target)?;

    // Simulate re-linking - should detect it's already correct and skip
    if let Ok(existing) = fs::read_link(&target) {
        let expected = PathBuf::from("../Cellar/testpkg/1.0.0/bin/test-binary");
        if existing == expected {
            // Correct: skip, don't recreate
        } else {
            panic!("Should have detected correct symlink");
        }
    }

    // Verify symlink unchanged
    let current_link = fs::read_link(&target)?;
    assert_eq!(original_link, current_link);

    Ok(())
}
