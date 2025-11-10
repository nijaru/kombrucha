//! Local disk caching of Homebrew API data.
//!
//! This module manages a persistent cache of Homebrew formula and cask lists on disk.
//! This avoids repeated API requests for bulky data and provides fast startup times.
//! The cache is stored in `~/.cache/bru/` with a 24-hour TTL.
//!
//! # Architecture
//!
//! Two main cache files:
//! ```text
//! ~/.cache/bru/
//!   formulae.json    # All formula metadata from API
//!   casks.json       # All cask metadata from API
//! ```
//!
//! Each file is refreshed automatically when:
//! - 24 hours have passed since last update
//! - The file doesn't exist
//! - The cache is explicitly cleared with [`clear_caches`]
//!
//! # Performance Impact
//!
//! - **First run**: Downloads ~25 MB of data, takes 2-3 seconds
//! - **Cached runs**: Loads from disk cache, takes <100 ms
//! - **API fallback**: If cache is stale, fetches fresh data and updates cache
//!
//! # Examples
//!
//! ```no_run
//! use kombrucha::cache;
//!
//! fn main() -> anyhow::Result<()> {
//!     // Check if cached data exists
//!     if let Some(formulae) = cache::get_cached_formulae() {
//!         println!("Cache has {} formulae", formulae.len());
//!     } else {
//!         println!("Cache is stale or missing, will fetch from API");
//!     }
//!
//!     Ok(())
//! }
//! ```

use crate::api::{Cask, Formula};
use crate::error::Result;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

const CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60); // 24 hours

/// Get the cache directory (~/.cache/bru/ or equivalent)
pub fn cache_dir() -> PathBuf {
    if let Some(cache_home) = std::env::var_os("XDG_CACHE_HOME") {
        PathBuf::from(cache_home).join("bru")
    } else if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join(".cache/bru")
    } else {
        PathBuf::from(".cache/bru")
    }
}

/// Check if a cached file is still fresh (less than TTL old)
pub fn is_cache_fresh(path: &PathBuf) -> bool {
    if !path.exists() {
        return false;
    }

    let metadata = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return false,
    };

    let modified = match metadata.modified() {
        Ok(t) => t,
        Err(_) => return false,
    };

    let age = match SystemTime::now().duration_since(modified) {
        Ok(d) => d,
        Err(_) => return false,
    };

    age < CACHE_TTL
}

/// Get cached formulae list or None if stale/missing
pub fn get_cached_formulae() -> Option<Vec<Formula>> {
    let cache_path = cache_dir().join("formulae.json");

    if !is_cache_fresh(&cache_path) {
        return None;
    }

    let content = std::fs::read_to_string(&cache_path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Store formulae list in cache
pub fn store_formulae(formulae: &Vec<Formula>) -> Result<()> {
    let cache_path = cache_dir().join("formulae.json");

    // Create cache directory if needed
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string(formulae)?;
    std::fs::write(&cache_path, json)?;

    Ok(())
}

/// Get cached casks list or None if stale/missing
pub fn get_cached_casks() -> Option<Vec<Cask>> {
    let cache_path = cache_dir().join("casks.json");

    if !is_cache_fresh(&cache_path) {
        return None;
    }

    let content = std::fs::read_to_string(&cache_path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Store casks list in cache
pub fn store_casks(casks: &Vec<Cask>) -> Result<()> {
    let cache_path = cache_dir().join("casks.json");

    // Create cache directory if needed
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string(casks)?;
    std::fs::write(&cache_path, json)?;

    Ok(())
}

/// Clear all caches
#[allow(dead_code)]
pub fn clear_caches() -> Result<()> {
    let cache_path = cache_dir();

    if cache_path.exists() {
        for entry in std::fs::read_dir(&cache_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("json") {
                std::fs::remove_file(&path)?;
            }
        }
    }

    Ok(())
}
