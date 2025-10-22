use crate::error::Result;
use serde::{Deserialize, Serialize};
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
    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
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

    /// Fetch all formulae (cached locally for 24 hours)
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

    /// Fetch specific formula by name (with in-memory caching)
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

    /// Search formulae and casks in parallel
    pub async fn search(&self, query: &str) -> Result<SearchResults> {
        let query_lower = query.to_lowercase();

        // Fetch both formulae and casks in parallel
        let (formulae_result, casks_result) =
            tokio::join!(self.fetch_all_formulae(), self.fetch_all_casks());

        let formulae = formulae_result?;
        let casks = casks_result?;

        // Filter results in parallel
        let (matching_formulae, matching_casks) = tokio::join!(
            tokio::task::spawn_blocking({
                let query = query_lower.clone();
                move || {
                    formulae
                        .into_iter()
                        .filter(|f| {
                            f.name.to_lowercase().contains(&query)
                                || f.desc
                                    .as_ref()
                                    .map(|d| d.to_lowercase().contains(&query))
                                    .unwrap_or(false)
                        })
                        .collect::<Vec<_>>()
                }
            }),
            tokio::task::spawn_blocking({
                let query = query_lower.clone();
                move || {
                    casks
                        .into_iter()
                        .filter(|c| {
                            c.token.to_lowercase().contains(&query)
                                || c.name.iter().any(|n| n.to_lowercase().contains(&query))
                                || c.desc
                                    .as_ref()
                                    .map(|d| d.to_lowercase().contains(&query))
                                    .unwrap_or(false)
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
