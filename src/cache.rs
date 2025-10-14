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
fn is_cache_fresh(path: &PathBuf) -> bool {
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
