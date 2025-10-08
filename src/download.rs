//! Bottle download manager with parallel downloads and progress tracking

use crate::api::{BrewApi, Formula};
use crate::platform;
use anyhow::{Context, Result, anyhow};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;

/// Download cache directory
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
    let mut buffer = vec![0; 8192];

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

/// Download a single bottle with progress bar
pub async fn download_bottle(
    formula: &Formula,
    progress: Option<&MultiProgress>,
) -> Result<PathBuf> {
    // Get bottle info
    let bottle = formula
        .bottle
        .as_ref()
        .and_then(|b| b.stable.as_ref())
        .ok_or_else(|| anyhow!("No bottle available for {}", formula.name))?;

    // Detect platform
    let platform_tag = platform::detect_bottle_tag()?;

    // Get bottle file for this platform
    let bottle_file = bottle
        .files
        .get(&platform_tag)
        .ok_or_else(|| anyhow!("No bottle for platform: {}", platform_tag))?;

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
                .template(
                    "{msg} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})",
                )?
                .progress_chars("#>-"),
        );
        pb.set_message(format!("⬇ {}", formula.name));
        Some(pb)
    } else {
        None
    };

    // Download
    let client = reqwest::Client::new();
    let mut response = client
        .get(&bottle_file.url)
        .send()
        .await
        .context("Failed to send request")?;

    if let Some(pb) = &pb {
        if let Some(total) = response.content_length() {
            pb.set_length(total);
        }
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

/// Download multiple bottles in parallel
pub async fn download_bottles(
    _api: &BrewApi,
    formulae: &[Formula],
) -> Result<Vec<(String, PathBuf)>> {
    let mp = MultiProgress::new();
    let mut tasks = Vec::new();

    for formula in formulae {
        let formula_clone = formula.clone();
        let mp_clone = mp.clone();

        let task = tokio::spawn(async move {
            let result = download_bottle(&formula_clone, Some(&mp_clone)).await;
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
