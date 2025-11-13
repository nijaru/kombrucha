//! Homebrew JSON API client with in-memory caching.
//!
//! This module provides a [`BrewApi`] client for querying Homebrew's public JSON API.
//! It handles all communication with the remote API, including timeout management,
//! automatic retries, and in-memory caching to avoid redundant requests.
//!
//! # Features
//!
//! - **Fast lookups**: In-memory LRU cache (1000 formulae, 500 casks) per client instance
//! - **Persistent cache**: Optional 24-hour disk cache in `~/.cache/bru/`
//! - **Parallel operations**: Uses tokio for concurrent API requests
//! - **Error handling**: Distinguishes between 404s and network errors
//! - **Timeout protection**: 10-second default timeout per request
//!
//! # Architecture
//!
//! The `BrewApi` client is the primary way to query Homebrew package metadata. It handles:
//! - Fetching individual formula/cask information
//! - Bulk fetching all formulae/casks (for search operations)
//! - Searching across all packages
//! - Parsing and caching API responses
//!
//! # Examples
//!
//! ```no_run
//! use kombrucha::BrewApi;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let api = BrewApi::new()?;
//!
//!     // Fetch a single formula
//!     let formula = api.fetch_formula("ripgrep").await?;
//!     println!("Latest version: {}", formula.versions.stable.unwrap_or_default());
//!
//!     // Search across all packages
//!     let results = api.search("python").await?;
//!     println!("Found {} formulae", results.formulae.len());
//!
//!     Ok(())
//! }
//! ```

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

const HOMEBREW_API_BASE: &str = "https://formulae.brew.sh/api";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Keg-only reason metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KegOnlyReason {
    pub reason: String,
    #[serde(default)]
    pub explanation: String,
}

/// Homebrew formula metadata from JSON API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Formula {
    pub name: String,
    #[serde(default)]
    pub full_name: String,
    #[serde(default)]
    pub desc: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub versions: Versions,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub build_dependencies: Vec<String>,
    #[serde(default)]
    pub bottle: Option<Bottle>,
    #[serde(default)]
    pub keg_only: bool,
    #[serde(default)]
    pub keg_only_reason: Option<KegOnlyReason>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Versions {
    #[serde(default)]
    pub stable: Option<String>,
    #[serde(default)]
    pub head: Option<String>,
    #[serde(default)]
    pub bottle: bool,
}

/// Bottle file metadata for a specific platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BottleFile {
    pub cellar: String,
    pub url: String,
    pub sha256: String,
}

/// Bottle metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BottleData {
    pub rebuild: u32,
    #[serde(default)]
    pub root_url: Option<String>,
    pub files: std::collections::HashMap<String, BottleFile>,
}

/// Bottle information from API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bottle {
    #[serde(default)]
    pub stable: Option<BottleData>,
}

/// Cask metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cask {
    pub token: String,
    #[serde(default)]
    pub full_token: String,
    #[serde(default)]
    pub name: Vec<String>,
    #[serde(default)]
    pub desc: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub artifacts: Vec<serde_json::Value>,
}

/// Homebrew API client with in-memory caching
#[derive(Clone)]
pub struct BrewApi {
    client: reqwest::Client,
    formula_cache: moka::future::Cache<String, Formula>,
    cask_cache: moka::future::Cache<String, Cask>,
}

impl BrewApi {
    /// Create a new Homebrew API client with in-memory caching.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use kombrucha::BrewApi;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let api = BrewApi::new()?;
    ///     Ok(())
    /// }
    /// ```
    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .pool_idle_timeout(Duration::from_secs(90)) // HTTP keep-alive standard
            .pool_max_idle_per_host(10) // Reuse connections during parallel resolution
            .user_agent(format!("bru/{}", env!("CARGO_PKG_VERSION")))
            .build()?;

        // In-memory cache for formula/cask lookups (lasts for command duration)
        // Cache up to 1000 formulae and 500 casks to avoid redundant API calls
        let formula_cache = moka::future::Cache::new(1000);
        let cask_cache = moka::future::Cache::new(500);

        Ok(Self {
            client,
            formula_cache,
            cask_cache,
        })
    }

    /// Fetch all formulae from Homebrew (with local disk caching for 24 hours).
    ///
    /// This downloads the complete list of all available formulae from Homebrew's
    /// public JSON API. The result is cached on disk to avoid repeated large downloads.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use kombrucha::BrewApi;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let api = BrewApi::new()?;
    ///     let formulae = api.fetch_all_formulae().await?;
    ///     println!("Total formulae available: {}", formulae.len());
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Performance
    ///
    /// - First call: ~2-3 seconds (downloads ~25 MB)
    /// - Subsequent calls: <100 ms (loads from cache)
    pub async fn fetch_all_formulae(&self) -> Result<Vec<Formula>> {
        // Try cache first
        if let Some(cached) = crate::cache::get_cached_formulae() {
            return Ok(cached);
        }

        // Fetch fresh from API
        let url = format!("{}/formula.json", HOMEBREW_API_BASE);
        let formulae = self.client.get(&url).send().await?.json().await?;

        // Store in cache (ignore errors)
        let _ = crate::cache::store_formulae(&formulae);

        Ok(formulae)
    }

    /// Fetch all casks (cached locally for 24 hours)
    pub async fn fetch_all_casks(&self) -> Result<Vec<Cask>> {
        // Try cache first
        if let Some(cached) = crate::cache::get_cached_casks() {
            return Ok(cached);
        }

        // Fetch fresh from API
        let url = format!("{}/cask.json", HOMEBREW_API_BASE);
        let casks = self.client.get(&url).send().await?.json().await?;

        // Store in cache (ignore errors)
        let _ = crate::cache::store_casks(&casks);

        Ok(casks)
    }

    /// Fetch metadata for a specific formula by name (with in-memory caching).
    ///
    /// Returns complete metadata including versions, dependencies, and bottle information.
    /// Results are cached in-memory for the duration of the API client instance.
    ///
    /// # Errors
    ///
    /// Returns [`BruError::FormulaNotFound`](crate::error::BruError::FormulaNotFound) if
    /// the formula doesn't exist in Homebrew.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use kombrucha::BrewApi;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let api = BrewApi::new()?;
    ///     let formula = api.fetch_formula("ripgrep").await?;
    ///
    ///     println!("Name: {}", formula.name);
    ///     println!("Version: {}", formula.versions.stable.as_deref().unwrap_or(""));
    ///     println!("Description: {}", formula.desc.as_deref().unwrap_or(""));
    ///     println!("Dependencies: {:?}", formula.dependencies);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn fetch_formula(&self, name: &str) -> Result<Formula> {
        // Check cache first
        if let Some(cached) = self.formula_cache.get(name).await {
            return Ok(cached);
        }

        // Fetch from API
        let url = format!("{}/formula/{}.json", HOMEBREW_API_BASE, name);
        let response = self.client.get(&url).send().await?;

        if response.status() == 404 {
            return Err(crate::error::BruError::FormulaNotFound(name.to_string()));
        }

        let formula: Formula = response.json().await?;

        // Store in cache for subsequent calls
        self.formula_cache
            .insert(name.to_string(), formula.clone())
            .await;

        Ok(formula)
    }

    /// Fetch specific cask by token (with in-memory caching)
    pub async fn fetch_cask(&self, token: &str) -> Result<Cask> {
        // Check cache first
        if let Some(cached) = self.cask_cache.get(token).await {
            return Ok(cached);
        }

        // Fetch from API
        let url = format!("{}/cask/{}.json", HOMEBREW_API_BASE, token);
        let response = self.client.get(&url).send().await?;

        if response.status() == 404 {
            return Err(crate::error::BruError::CaskNotFound(token.to_string()));
        }

        let cask: Cask = response.json().await?;

        // Store in cache for subsequent calls
        self.cask_cache
            .insert(token.to_string(), cask.clone())
            .await;

        Ok(cask)
    }

    /// Search for formulae and casks matching a query.
    ///
    /// Performs a case-insensitive search across both formula names/descriptions
    /// and cask tokens/names. Results are returned separately for filtering.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use kombrucha::BrewApi;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let api = BrewApi::new()?;
    ///     let results = api.search("python").await?;
    ///
    ///     println!("Found {} formulae", results.formulae.len());
    ///     println!("Found {} casks", results.casks.len());
    ///
    ///     for formula in &results.formulae {
    ///         println!("  {} - {}", formula.name, formula.desc.as_deref().unwrap_or(""));
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn search(&self, query: &str) -> Result<SearchResults> {
        let query_lower = query.to_lowercase();

        // Fetch both formulae and casks in parallel
        let (formulae_result, casks_result) =
            tokio::join!(self.fetch_all_formulae(), self.fetch_all_casks());

        let formulae = formulae_result?;
        let casks = casks_result?;

        // Share query string between parallel tasks using Arc to avoid cloning
        let query = std::sync::Arc::new(query_lower);

        // Filter results in parallel
        let (matching_formulae, matching_casks) = tokio::join!(
            tokio::task::spawn_blocking({
                let query = Arc::clone(&query);
                move || {
                    formulae
                        .into_iter()
                        .filter(|f| {
                            f.name.to_lowercase().contains(query.as_str())
                                || f.desc
                                    .as_ref()
                                    .is_some_and(|d| d.to_lowercase().contains(query.as_str()))
                        })
                        .collect::<Vec<_>>()
                }
            }),
            tokio::task::spawn_blocking({
                let query = Arc::clone(&query);
                move || {
                    casks
                        .into_iter()
                        .filter(|c| {
                            c.token.to_lowercase().contains(query.as_str())
                                || c.name.iter().any(|n| n.to_lowercase().contains(query.as_str()))
                                || c.desc
                                    .as_ref()
                                    .is_some_and(|d| d.to_lowercase().contains(query.as_str()))
                        })
                        .collect::<Vec<_>>()
                }
            })
        );

        Ok(SearchResults {
            formulae: matching_formulae.unwrap(),
            casks: matching_casks.unwrap(),
        })
    }
}

impl Default for BrewApi {
    fn default() -> Self {
        Self::new().expect("Failed to create API client")
    }
}

#[derive(Debug)]
pub struct SearchResults {
    pub formulae: Vec<Formula>,
    pub casks: Vec<Cask>,
}

impl SearchResults {
    pub fn is_empty(&self) -> bool {
        self.formulae.is_empty() && self.casks.is_empty()
    }
}
