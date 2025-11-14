//! Bottle download manager with parallel downloads and optional progress tracking.
//!
//! This module handles downloading precompiled Homebrew bottles from GitHub Container Registry (GHCR),
//! with support for:
//! - **Parallel downloads**: Up to 8 concurrent downloads with semaphore control
//! - **Progress tracking**: Optional visual progress bars during downloads
//! - **Checksum verification**: SHA256 validation of downloaded files
//! - **Caching**: Avoids re-downloading bottles that already exist with correct checksum
//! - **GHCR authentication**: Automatic bearer token acquisition for public packages
//!
//! # Architecture
//!
//! Bottles are downloaded from GHCR and stored in a local cache:
//! ```text
//! ~/.cache/bru/downloads/
//!   formula-name--1.0.0.arm64_sonoma.bottle.tar.gz
//!   other-package--2.1.0.x86_64_ventura.bottle.tar.gz
//! ```
//!
//! The download process:
//! 1. Check if bottle already cached and verified
//! 2. Acquire GHCR bearer token for repository access
//! 3. Download from GHCR blob endpoint with progress tracking
//! 4. Verify SHA256 checksum matches expected value
//! 5. Return path to cached bottle
//!
//! # Examples
//!
//! ```no_run
//! use kombrucha::{BrewApi, download};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let api = BrewApi::new()?;
//!     let formula = api.fetch_formula("ripgrep").await?;
//!
//!     // Download a single bottle
//!     let cache_path = download::cache_dir();
//!     println!("Cache location: {}", cache_path.display());
//!
//!     Ok(())
//! }
//! ```

use crate::api::{BrewApi, Formula};
use crate::platform;
use anyhow::{Context, Result, anyhow};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;

/// GHCR token response
#[derive(Deserialize)]
struct GhcrToken {
    token: String,
}

/// Get anonymous bearer token for GHCR
async fn get_ghcr_token(repository: &str) -> Result<String> {
    let url = format!(
        "https://ghcr.io/token?service=ghcr.io&scope=repository:{}:pull",
        repository
    );

    let client = reqwest::Client::new();
    let response: GhcrToken = client.get(&url).send().await?.json().await?;

    Ok(response.token)
}

/// Get the download cache directory for bottles.
///
/// Returns the path where downloaded bottles are stored:
/// `~/.cache/bru/downloads/`
///
/// This directory is created automatically when needed. It's safe to manually clear
/// this directory to reclaim disk space - bottles will be re-downloaded on next use.
///
/// # Examples
///
/// ```no_run
/// use kombrucha::download;
///
/// let cache = download::cache_dir();
/// println!("Bottle cache: {}", cache.display());
/// // Output: "/Users/nick/.cache/bru/downloads"
/// ```
#[inline]
pub fn cache_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".cache/bru/downloads")
}

/// SHA256 checksum verification
async fn verify_checksum(file_path: &Path, expected: &str) -> Result<bool> {
    use sha2::{Digest, Sha256};
    use tokio::io::AsyncReadExt;

    let mut file = fs::File::open(file_path).await?;
    let mut hasher = Sha256::new();
    // Use 64KB buffer for better I/O performance (8x larger than default)
    let mut buffer = vec![0; 65536];

    loop {
        let n = file.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    let result = hasher.finalize();
    let checksum = format!("{:x}", result);

    Ok(checksum == expected)
}

/// Download a single bottle from GitHub Container Registry (GHCR).
///
/// Downloads a precompiled bottle for the current platform and verifies it via SHA256 checksum.
/// The bottle is cached locally to avoid re-downloading. If a cached bottle exists and passes
/// checksum verification, it's returned immediately without downloading.
///
/// # Arguments
///
/// * `formula` - The formula to download a bottle for
/// * `progress` - Optional progress bar for visual feedback (ignored in library mode)
/// * `client` - HTTP client for the request (should be reused across downloads)
///
/// # Returns
///
/// The local path to the cached bottle file.
///
/// # Errors
///
/// Returns an error if:
/// - The formula has no bottle for the current platform
/// - Network request fails
/// - GHCR authentication fails
/// - SHA256 checksum verification fails
/// - Cannot write to cache directory
///
/// # Examples
///
/// ```no_run
/// use kombrucha::{BrewApi, download};
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let api = BrewApi::new()?;
///     let formula = api.fetch_formula("ripgrep").await?;
///
///     let client = reqwest::Client::new();
///     let bottle_path = download::download_bottle(&formula, None, &client).await?;
///     println!("Downloaded to: {}", bottle_path.display());
///
///     Ok(())
/// }
/// ```
///
/// # Caching Behavior
///
/// - Bottles are cached in `~/.cache/bru/downloads/`
/// - Cached bottles are verified via SHA256 checksum
/// - Failed checksums trigger a re-download
/// - Safe to manually delete cache files
///
/// # Platform Selection
///
/// Bottles are matched to the current platform:
/// - macOS: `arm64_sequoia`, `x86_64_ventura`, etc.
/// - Linux: `arm64_linux`, `x86_64_linux`
/// - Falls back to universal `all` bottle if platform-specific unavailable
pub async fn download_bottle(
    formula: &Formula,
    progress: Option<&MultiProgress>,
    client: &reqwest::Client,
) -> Result<PathBuf> {
    // Get bottle info
    let bottle = formula
        .bottle
        .as_ref()
        .and_then(|b| b.stable.as_ref())
        .ok_or_else(|| anyhow!("No bottle available for {}", formula.name))?;

    // Detect platform
    let platform_tag = platform::detect_bottle_tag()?;

    // Get bottle file for this platform, with fallback to "all" (universal)
    // Matches Homebrew's fallback logic: exact platform first, then universal
    let bottle_file = bottle
        .files
        .get(&platform_tag)
        .or_else(|| bottle.files.get("all"))
        .ok_or_else(|| {
            anyhow!(
                "No bottle for platform: {} (no universal bottle available)",
                platform_tag
            )
        })?;

    // Create cache directory
    let cache = cache_dir();
    fs::create_dir_all(&cache)
        .await
        .context("Failed to create cache directory")?;

    // Determine filename
    let version = formula
        .versions
        .stable
        .as_ref()
        .ok_or_else(|| anyhow!("No stable version"))?;
    let filename = format!(
        "{}--{}.{}.bottle.tar.gz",
        formula.name, version, platform_tag
    );
    let output_path = cache.join(&filename);

    // Check if already downloaded and verified
    if output_path.exists() {
        if verify_checksum(&output_path, &bottle_file.sha256).await? {
            return Ok(output_path);
        }
        // Checksum failed, re-download
        fs::remove_file(&output_path).await?;
    }

    // Create progress bar
    let pb = if let Some(mp) = progress {
        let pb = mp.add(ProgressBar::new(0));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")?
                .progress_chars("━━╸"),
        );
        pb.set_message(format!("Downloading {}", formula.name));
        Some(pb)
    } else {
        None
    };

    // Get GHCR bearer token
    // Extract repository from bottle URL (e.g., https://ghcr.io/v2/homebrew/core/python/3.13/blobs/...)
    // Repository format: homebrew/core/{package}/{version}
    let repository = bottle_file
        .url
        .strip_prefix("https://ghcr.io/v2/")
        .and_then(|s| s.split("/blobs/").next())
        .ok_or_else(|| anyhow!("Invalid GHCR URL format: {}", bottle_file.url))?;

    let token = get_ghcr_token(repository)
        .await
        .context("Failed to get GHCR token")?;

    // Download with authentication
    let mut response = client
        .get(&bottle_file.url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .context("Failed to send request")?;

    if let Some(pb) = &pb
        && let Some(total) = response.content_length()
    {
        pb.set_length(total);
    }

    let mut file = fs::File::create(&output_path)
        .await
        .context("Failed to create output file")?;
    let mut downloaded: u64 = 0;

    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        if let Some(pb) = &pb {
            pb.set_position(downloaded);
        }
    }

    file.flush().await?;

    if let Some(pb) = &pb {
        pb.finish_with_message(format!("✓ {}", formula.name));
    }

    // Verify checksum
    if !verify_checksum(&output_path, &bottle_file.sha256).await? {
        fs::remove_file(&output_path).await?;
        anyhow::bail!("Checksum verification failed for {}", formula.name);
    }

    Ok(output_path)
}

/// Download multiple bottles in parallel with automatic concurrency control.
///
/// Downloads multiple bottles concurrently (limited to 8 simultaneous downloads) to balance
/// speed and resource consumption. Each bottle is cached and verified independently.
///
/// # Arguments
///
/// * `_api` - Homebrew API client (currently unused, kept for future extension)
/// * `formulae` - Slice of formulas to download bottles for
///
/// # Returns
///
/// A vector of `(formula_name, bottle_path)` tuples for successfully downloaded bottles.
///
/// # Errors
///
/// Returns an error if any bottle fails to download (stops on first error).
/// Use with error recovery in production to skip failed packages.
///
/// # Examples
///
/// ```no_run
/// use kombrucha::{BrewApi, download};
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let api = BrewApi::new()?;
///     let formulae = vec![
///         api.fetch_formula("ripgrep").await?,
///         api.fetch_formula("bat").await?,
///     ];
///
///     let bottles = download::download_bottles(&api, &formulae).await?;
///     for (name, path) in bottles {
///         println!("Downloaded {} to {}", name, path.display());
///     }
///
///     Ok(())
/// }
/// ```
///
/// # Concurrency
///
/// - Limited to 8 concurrent downloads (configurable via `MAX_CONCURRENT_DOWNLOADS`)
/// - Progress bars shown for each download if `BRU_QUIET` env var is not set
/// - HTTP connections reused across downloads for efficiency
///
/// # Performance
///
/// On a 500 Mbps connection, downloading 10 bottles takes ~5-10 seconds.
pub async fn download_bottles(
    _api: &BrewApi,
    formulae: &[Formula],
) -> Result<Vec<(String, PathBuf)>> {
    use std::sync::Arc;
    use tokio::sync::Semaphore;

    let mp = MultiProgress::new();
    let mut tasks = Vec::new();

    // Create shared HTTP client (reused across all downloads)
    let client = Arc::new(reqwest::Client::new());

    // Limit concurrent downloads to prevent resource exhaustion
    // Reduced from 16 to 8 to be more conservative with file descriptors
    const MAX_CONCURRENT_DOWNLOADS: usize = 8;
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_DOWNLOADS));

    for formula in formulae {
        let formula_clone = formula.clone();
        let mp_clone = mp.clone();
        let sem = semaphore.clone();
        let client_clone = client.clone();

        let task = tokio::spawn(async move {
            // Acquire semaphore permit before downloading
            let _permit = sem.acquire().await.unwrap();
            // Pass progress only if not in quiet mode
            let progress = if std::env::var("BRU_QUIET").is_ok() {
                None
            } else {
                Some(&mp_clone)
            };
            let result = download_bottle(&formula_clone, progress, &client_clone).await;
            (formula_clone.name.clone(), result)
        });

        tasks.push(task);
    }

    let mut results = Vec::new();
    for task in tasks {
        let (name, result) = task.await?;
        match result {
            Ok(path) => results.push((name, path)),
            Err(e) => return Err(e),
        }
    }

    Ok(results)
}
