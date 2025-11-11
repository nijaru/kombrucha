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

/// Get the local disk cache directory for Homebrew API data.
///
/// Returns the path where cached formula and cask lists are stored.
/// Uses XDG standards: `$XDG_CACHE_HOME/bru` or `~/.cache/bru`
///
/// # Returns
///
/// The cache directory path. Directory is created automatically when caching data.
///
/// # Examples
///
/// ```no_run
/// use kombrucha::cache;
///
/// let cache = cache::cache_dir();
/// println!("Cache directory: {}", cache.display());
/// // Output: "/Users/nick/.cache/bru" or "$XDG_CACHE_HOME/bru"
/// ```
///
/// # Cache Files
///
/// The directory contains:
/// - `formulae.json` - Cached list of all Homebrew formulae (24-hour TTL)
/// - `casks.json` - Cached list of all Homebrew casks (24-hour TTL)
/// - `downloads/` - Cached downloaded bottles
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

/// Get cached formulae list if it's still fresh (less than 24 hours old).
///
/// Returns the cached list of all Homebrew formulae if the cache exists and is fresh.
/// Returns `None` if the cache is missing or older than 24 hours.
///
/// # Returns
///
/// - `Some(formulae)` if cache is fresh and valid
/// - `None` if cache is stale, missing, or corrupted
///
/// # Examples
///
/// ```no_run
/// use kombrucha::cache;
///
/// if let Some(formulae) = cache::get_cached_formulae() {
///     println!("Cache hit: {} formulae cached", formulae.len());
/// } else {
///     println!("Cache miss: will fetch from API");
/// }
/// ```
pub fn get_cached_formulae() -> Option<Vec<Formula>> {
    let cache_path = cache_dir().join("formulae.json");

    if !is_cache_fresh(&cache_path) {
        return None;
    }

    let content = std::fs::read_to_string(&cache_path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Store the complete formulae list to cache.
///
/// Saves the formulae list for fast lookup on subsequent runs.
/// Cache is automatically refreshed after 24 hours.
///
/// # Errors
///
/// Returns an error if the cache directory cannot be created or the file cannot be written.
///
/// # Examples
///
/// ```no_run
/// use kombrucha::{cache, BrewApi};
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let api = BrewApi::new()?;
///     let formulae = api.fetch_all_formulae().await?;
///     cache::store_formulae(&formulae)?;
///     println!("Cached {} formulae", formulae.len());
///
///     Ok(())
/// }
/// ```
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

/// Get cached casks list if it's still fresh (less than 24 hours old).
///
/// Returns the cached list of all Homebrew casks if the cache exists and is fresh.
/// Returns `None` if the cache is missing or older than 24 hours.
///
/// # Returns
///
/// - `Some(casks)` if cache is fresh and valid
/// - `None` if cache is stale, missing, or corrupted
///
/// # Examples
///
/// ```no_run
/// use kombrucha::cache;
///
/// if let Some(casks) = cache::get_cached_casks() {
///     println!("Cache hit: {} casks cached", casks.len());
/// } else {
///     println!("Cache miss: will fetch from API");
/// }
/// ```
pub fn get_cached_casks() -> Option<Vec<Cask>> {
    let cache_path = cache_dir().join("casks.json");

    if !is_cache_fresh(&cache_path) {
        return None;
    }

    let content = std::fs::read_to_string(&cache_path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Store the complete casks list to cache.
///
/// Saves the casks list for fast lookup on subsequent runs.
/// Cache is automatically refreshed after 24 hours.
///
/// # Errors
///
/// Returns an error if the cache directory cannot be created or the file cannot be written.
///
/// # Examples
///
/// ```no_run
/// use kombrucha::{cache, BrewApi};
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let api = BrewApi::new()?;
///     let casks = api.fetch_all_casks().await?;
///     cache::store_casks(&casks)?;
///     println!("Cached {} casks", casks.len());
///
///     Ok(())
/// }
/// ```
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
