// Test helpers for isolated testing
// Provides safe test environments that don't modify the system

use std::path::PathBuf;
use tempfile::TempDir;

/// Isolated test environment using temporary directories
/// Automatically cleaned up when dropped (RAII pattern)
///
/// # Example
/// ```
/// use test_helpers::TestEnvironment;
///
/// let env = TestEnvironment::new();
/// // Use env.prefix, env.cellar, etc.
/// // Automatically cleaned up when env goes out of scope
/// ```
pub struct TestEnvironment {
    pub temp_dir: TempDir,
    pub prefix: PathBuf,
    pub cellar: PathBuf,
    pub cache: PathBuf,
    pub bin: PathBuf,
}

impl TestEnvironment {
    /// Create a new isolated test environment
    ///
    /// Creates a temporary directory structure mimicking Homebrew:
    /// - temp/
    ///   - Cellar/     (package installations)
    ///   - bin/        (symlinks to executables)
    ///   - cache/      (downloaded bottles)
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let prefix = temp_dir.path().to_path_buf();
        let cellar = prefix.join("Cellar");
        let cache = prefix.join("cache");
        let bin = prefix.join("bin");

        // Create directory structure
        std::fs::create_dir_all(&cellar).unwrap();
        std::fs::create_dir_all(&cache).unwrap();
        std::fs::create_dir_all(&bin).unwrap();

        Self {
            temp_dir,
            prefix,
            cellar,
            cache,
            bin,
        }
    }

    /// Get the path to the Cellar directory
    pub fn cellar_path(&self) -> &PathBuf {
        &self.cellar
    }

    /// Get the path to the prefix directory
    pub fn prefix_path(&self) -> &PathBuf {
        &self.prefix
    }

    /// Get the path to the cache directory
    pub fn cache_path(&self) -> &PathBuf {
        &self.cache
    }

    /// Get the path to the bin directory
    pub fn bin_path(&self) -> &PathBuf {
        &self.bin
    }
}

impl Default for TestEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

// Temp directory automatically cleaned up when TestEnvironment is dropped
// No explicit Drop implementation needed - TempDir handles it

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_creates_directories() {
        let env = TestEnvironment::new();

        assert!(env.cellar.exists());
        assert!(env.cache.exists());
        assert!(env.bin.exists());
        assert!(env.prefix.exists());
    }

    #[test]
    fn test_environment_cleanup() {
        let cellar_path = {
            let env = TestEnvironment::new();
            env.cellar.clone()
        };

        // After env is dropped, temp directory should be cleaned up
        assert!(!cellar_path.exists());
    }

    #[test]
    fn test_multiple_environments_isolated() {
        let env1 = TestEnvironment::new();
        let env2 = TestEnvironment::new();

        // Each environment has its own isolated directory
        assert_ne!(env1.prefix, env2.prefix);
        assert!(env1.prefix.exists());
        assert!(env2.prefix.exists());
    }
}
