//! Homebrew tap management - adding and removing third-party repositories
//! Also handles reading and parsing tap formula files for upgrade detection

use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Get the Taps directory path
pub fn taps_path() -> PathBuf {
    crate::cellar::detect_prefix().join("Library/Taps")
}

/// Extract formula name from a tap/formula string
/// Input: "user/repo/formula" → Output: "formula"
/// Input: "formula" → Output: "formula"
pub fn extract_formula_name(tap_formula: &str) -> String {
    // If the string contains slashes, it might be tap/formula format
    if tap_formula.contains('/') {
        // Split and take the last part
        tap_formula
            .split('/')
            .next_back()
            .unwrap_or(tap_formula)
            .to_string()
    } else {
        tap_formula.to_string()
    }
}

/// Parse a tap name into (user, repo) components
/// Input: "user/repo" → Output: ("user", "homebrew-repo")
fn parse_tap_name(tap: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = tap.split('/').collect();
    if parts.len() != 2 {
        return Err(anyhow!(
            "Invalid tap name format. Expected 'user/repo', got '{}'",
            tap
        ));
    }

    let user = parts[0].to_string();
    let mut repo = parts[1].to_string();

    // Add homebrew- prefix if not present
    if !repo.starts_with("homebrew-") {
        repo = format!("homebrew-{}", repo);
    }

    Ok((user, repo))
}

/// Get the directory path for a tap
pub fn tap_directory(tap: &str) -> Result<PathBuf> {
    let (user, repo) = parse_tap_name(tap)?;
    Ok(taps_path().join(user).join(repo))
}

/// List all installed taps
pub fn list_taps() -> Result<Vec<String>> {
    let taps_dir = taps_path();

    if !taps_dir.exists() {
        return Ok(vec![]);
    }

    let mut taps = Vec::new();

    for user_entry in fs::read_dir(&taps_dir)? {
        let user_entry = user_entry?;
        let user = user_entry.file_name().to_string_lossy().to_string();

        if user.starts_with('.') {
            continue;
        }

        let user_path = user_entry.path();
        if !user_path.is_dir() {
            continue;
        }

        for repo_entry in fs::read_dir(user_path)? {
            let repo_entry = repo_entry?;
            let repo = repo_entry.file_name().to_string_lossy().to_string();

            if repo.starts_with('.') {
                continue;
            }

            // Remove "homebrew-" prefix for display
            let display_repo = repo.strip_prefix("homebrew-").unwrap_or(&repo);
            taps.push(format!("{}/{}", user, display_repo));
        }
    }

    taps.sort();
    Ok(taps)
}

/// Check if a tap is installed
pub fn is_tapped(tap: &str) -> Result<bool> {
    let tap_dir = tap_directory(tap)?;
    Ok(tap_dir.exists() && tap_dir.join(".git").exists())
}

/// Add a tap (clone the git repository)
pub fn tap(tap_name: &str) -> Result<()> {
    let (user, repo) = parse_tap_name(tap_name)?;
    let tap_dir = tap_directory(tap_name)?;

    if tap_dir.exists() {
        return Err(anyhow!("Tap {}/{} already exists", user, repo));
    }

    // Create user directory if it doesn't exist
    let user_dir = taps_path().join(&user);
    if !user_dir.exists() {
        fs::create_dir_all(&user_dir)
            .with_context(|| format!("Failed to create directory: {}", user_dir.display()))?;
    }

    // Clone the repository
    let git_url = format!("https://github.com/{}/{}.git", user, repo);

    let tap_dir_str = tap_dir
        .to_str()
        .ok_or_else(|| anyhow!("Tap directory path contains invalid UTF-8"))?;

    let output = Command::new("git")
        .args(["clone", "--depth", "1", &git_url, tap_dir_str])
        .output()
        .context("Failed to execute git clone")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Failed to clone tap: {}", stderr));
    }

    Ok(())
}

/// Remove a tap (delete the git repository)
pub fn untap(tap_name: &str) -> Result<()> {
    let tap_dir = tap_directory(tap_name)?;

    if !tap_dir.exists() {
        return Err(anyhow!("Tap {} is not installed", tap_name));
    }

    fs::remove_dir_all(&tap_dir)
        .with_context(|| format!("Failed to remove tap directory: {}", tap_dir.display()))?;

    // Remove user directory if empty
    if let Some(user_dir) = tap_dir.parent()
        && user_dir.exists()
        && fs::read_dir(user_dir)?.next().is_none()
    {
        fs::remove_dir(user_dir)?;
    }

    Ok(())
}

/// Get the formula file path for a package in a tap
/// Returns the path even if the file doesn't exist
pub fn formula_path(tap_name: &str, formula_name: &str) -> Result<PathBuf> {
    let tap_dir = tap_directory(tap_name)?;
    Ok(tap_dir.join("Formula").join(format!("{}.rb", formula_name)))
}

/// Parse version from a Ruby formula file
/// Looks for: version "X.Y.Z"
pub fn parse_formula_version(formula_path: &Path) -> Result<Option<String>> {
    if !formula_path.exists() {
        return Ok(None);
    }

    let contents = fs::read_to_string(formula_path)
        .with_context(|| format!("Failed to read formula: {}", formula_path.display()))?;

    // Parse version from Ruby formula file
    for line in contents.lines() {
        let line = line.trim();

        // Look for: version "X.Y.Z"
        if line.starts_with("version ") && line.contains('"') {
            // Extract version string between quotes
            if let Some(start) = line.find('"')
                && let Some(end) = line[start + 1..].find('"') {
                    let version = &line[start + 1..start + 1 + end];
                    return Ok(Some(version.to_string()));
                }
        }
    }

    Ok(None)
}

/// Get the latest version for a tap formula
/// Returns None if the formula file doesn't exist or version can't be parsed
pub fn get_tap_formula_version(tap_name: &str, formula_name: &str) -> Result<Option<String>> {
    let path = formula_path(tap_name, formula_name)?;
    parse_formula_version(&path)
}

/// Tap formula metadata extracted from Ruby file
#[derive(Debug, Clone)]
pub struct TapFormulaInfo {
    pub name: String,
    pub desc: Option<String>,
    pub homepage: Option<String>,
    pub version: Option<String>,
}

/// Parse formula metadata from a Ruby formula file
pub fn parse_formula_info(formula_path: &Path, formula_name: &str) -> Result<TapFormulaInfo> {
    if !formula_path.exists() {
        return Err(anyhow::anyhow!(
            "Formula file not found: {}",
            formula_path.display()
        ));
    }

    let contents = fs::read_to_string(formula_path)
        .with_context(|| format!("Failed to read formula: {}", formula_path.display()))?;

    let mut desc = None;
    let mut homepage = None;
    let mut version = None;

    for line in contents.lines() {
        let line = line.trim();

        // Parse: desc "Description text"
        if line.starts_with("desc ") && line.contains('"')
            && let Some(start) = line.find('"')
                && let Some(end) = line[start + 1..].rfind('"') {
                    desc = Some(line[start + 1..start + 1 + end].to_string());
                }

        // Parse: homepage "https://..."
        if line.starts_with("homepage ") && line.contains('"')
            && let Some(start) = line.find('"')
                && let Some(end) = line[start + 1..].rfind('"') {
                    homepage = Some(line[start + 1..start + 1 + end].to_string());
                }

        // Parse: version "X.Y.Z"
        if line.starts_with("version ") && line.contains('"')
            && let Some(start) = line.find('"')
                && let Some(end) = line[start + 1..].find('"') {
                    version = Some(line[start + 1..start + 1 + end].to_string());
                }
    }

    Ok(TapFormulaInfo {
        name: formula_name.to_string(),
        desc,
        homepage,
        version,
    })
}

/// Check if an installed package is from a tap (based on receipt)
/// Returns (tap_name, formula_path, installed_version) if from a tap, None otherwise
pub fn get_package_tap_info(cellar_path: &Path) -> Result<Option<(String, PathBuf, String)>> {
    use crate::receipt::InstallReceipt;

    let receipt = InstallReceipt::read(cellar_path)?;

    let source = match receipt.source {
        Some(s) => s,
        None => return Ok(None),
    };

    // homebrew/core is not considered a "tap" for upgrade purposes
    if source.tap == "homebrew/core" {
        return Ok(None);
    }

    let path = match source.path {
        Some(p) => PathBuf::from(p),
        None => return Ok(None),
    };

    let installed_version = source.versions.and_then(|v| v.stable).unwrap_or_default();

    Ok(Some((source.tap, path, installed_version)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tap_name() {
        let (user, repo) = parse_tap_name("user/repo").unwrap();
        assert_eq!(user, "user");
        assert_eq!(repo, "homebrew-repo");

        let (user, repo) = parse_tap_name("user/homebrew-repo").unwrap();
        assert_eq!(user, "user");
        assert_eq!(repo, "homebrew-repo");
    }

    #[test]
    fn test_parse_tap_name_invalid() {
        assert!(parse_tap_name("invalid").is_err());
        assert!(parse_tap_name("too/many/slashes").is_err());
    }

    #[test]
    fn test_tap_directory() {
        let dir = tap_directory("user/repo").unwrap();
        assert!(dir.to_string_lossy().contains("Taps/user/homebrew-repo"));
    }
}
