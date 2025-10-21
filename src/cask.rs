use crate::cellar;
use colored::Colorize;
use std::path::PathBuf;
use std::process::Command;

type Result<T> = anyhow::Result<T>;

/// Get Caskroom directory
pub fn caskroom_dir() -> PathBuf {
    cellar::detect_prefix().join("Caskroom")
}

/// Get installed cask directory
pub fn cask_install_dir(token: &str, version: &str) -> PathBuf {
    caskroom_dir().join(token).join(version)
}

/// Download a cask artifact
pub async fn download_cask(url: &str, token: &str) -> Result<PathBuf> {
    let cache_dir = crate::download::cache_dir();
    std::fs::create_dir_all(&cache_dir)?;

    // Extract filename from URL
    let filename = url.split('/').next_back().unwrap_or(token);
    let dest_path = cache_dir.join(filename);

    // Skip if already downloaded
    if dest_path.exists() {
        return Ok(dest_path);
    }

    // Download file
    let client = reqwest::Client::new();
    let response = client.get(url).send().await?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to download cask: HTTP {}", response.status());
    }

    let bytes = response.bytes().await?;
    std::fs::write(&dest_path, bytes)?;

    Ok(dest_path)
}

/// Mount a DMG file and return the mount point
pub fn mount_dmg(dmg_path: &PathBuf) -> Result<PathBuf> {
    let output = Command::new("hdiutil")
        .args(["attach", "-nobrowse"])
        .arg(dmg_path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to mount DMG: {}", stderr);
    }

    // Parse output to find mount point
    // Output format: /dev/diskX<tab>filesystem<tab>/Volumes/Name
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        // Look for lines with /Volumes/
        if line.contains("/Volumes/") {
            // Split by tabs and whitespace
            let parts: Vec<&str> = line
                .split(|c: char| c == '\t' || c.is_whitespace())
                .collect();
            for part in parts {
                let trimmed = part.trim();
                if trimmed.starts_with("/Volumes/") && !trimmed.is_empty() {
                    return Ok(PathBuf::from(trimmed));
                }
            }
        }
    }

    anyhow::bail!("Could not find mount point in hdiutil output")
}

/// Unmount a DMG
pub fn unmount_dmg(mount_point: &PathBuf) -> Result<()> {
    let output = Command::new("hdiutil")
        .args(["detach", "-quiet"])
        .arg(mount_point)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to unmount DMG: {}", stderr);
    }

    Ok(())
}

/// Copy an app bundle to /Applications
pub fn install_app(app_path: &PathBuf, app_name: &str) -> Result<PathBuf> {
    let target = PathBuf::from("/Applications").join(app_name);

    // Remove existing app if present
    if target.exists() {
        println!("  {} Removing existing {}", "→".dimmed(), app_name.dimmed());
        std::fs::remove_dir_all(&target)?;
    }

    // Copy app bundle
    let status = Command::new("cp")
        .args(["-R"])
        .arg(app_path)
        .arg(&target)
        .status()?;

    if !status.success() {
        anyhow::bail!("Failed to copy app to /Applications");
    }

    Ok(target)
}

/// Install a PKG file
pub fn install_pkg(pkg_path: &PathBuf) -> Result<()> {
    println!("  {} Installing PKG (requires sudo)...", "→".bold());

    let status = Command::new("sudo")
        .args(["installer", "-pkg"])
        .arg(pkg_path)
        .arg("-target")
        .arg("/")
        .status()?;

    if !status.success() {
        anyhow::bail!("Failed to install PKG");
    }

    Ok(())
}

/// Extract artifacts from cask JSON
pub fn extract_app_artifacts(artifacts: &[serde_json::Value]) -> Vec<String> {
    let mut apps = Vec::new();

    for artifact in artifacts {
        if let Some(obj) = artifact.as_object()
            && let Some(app_array) = obj.get("app")
            && let Some(arr) = app_array.as_array()
        {
            for item in arr {
                if let Some(app_name) = item.as_str() {
                    apps.push(app_name.to_string());
                }
            }
        }
    }

    apps
}

/// Check if a cask is installed
pub fn is_cask_installed(token: &str) -> bool {
    let caskroom = caskroom_dir().join(token);
    caskroom.exists() && caskroom.is_dir()
}

/// Get installed cask version
pub fn get_installed_cask_version(token: &str) -> Option<String> {
    let caskroom = caskroom_dir().join(token);
    if !caskroom.exists() {
        return None;
    }

    // Get the first (and typically only) version directory
    if let Ok(entries) = std::fs::read_dir(&caskroom) {
        for entry in entries.flatten() {
            if entry.path().is_dir()
                && let Some(version) = entry.file_name().to_str()
            {
                return Some(version.to_string());
            }
        }
    }

    None
}

/// List all installed casks
pub fn list_installed_casks() -> Result<Vec<(String, String)>> {
    let mut casks = Vec::new();
    let caskroom = caskroom_dir();

    if !caskroom.exists() {
        return Ok(casks);
    }

    for entry in std::fs::read_dir(&caskroom)? {
        let entry = entry?;
        let token = entry.file_name().to_string_lossy().to_string();

        if let Some(version) = get_installed_cask_version(&token) {
            casks.push((token, version));
        }
    }

    casks.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(casks)
}

/// Extract a ZIP file and return the extraction directory
pub fn extract_zip(zip_path: &PathBuf) -> Result<PathBuf> {
    let cache_dir = crate::download::cache_dir();
    let extract_dir = cache_dir.join(format!(
        "{}_extracted",
        zip_path.file_stem().unwrap().to_string_lossy()
    ));

    // Remove existing extraction directory if present
    if extract_dir.exists() {
        std::fs::remove_dir_all(&extract_dir)?;
    }

    std::fs::create_dir_all(&extract_dir)?;

    // Extract ZIP file
    let status = Command::new("unzip")
        .args(["-q", "-o"]) // quiet, overwrite
        .arg(zip_path)
        .arg("-d")
        .arg(&extract_dir)
        .status()?;

    if !status.success() {
        anyhow::bail!("Failed to extract ZIP file");
    }

    Ok(extract_dir)
}

/// Quit an application before uninstalling
#[allow(dead_code)]
pub fn quit_app(bundle_id: &str) -> Result<()> {
    let output = Command::new("osascript")
        .args(["-e", &format!("quit app id \"{}\"", bundle_id)])
        .output()?;

    // Ignore errors if app isn't running
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.contains("not running") {
            eprintln!("Warning: Failed to quit app: {}", stderr);
        }
    }

    Ok(())
}
