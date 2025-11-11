# PackageManager Library API

The Kombrucha library provides a high-level `PackageManager` API for programmatic Homebrew package management. This is useful for downstream projects that need to integrate package management without shelling out to the CLI.

**Current Version**: v0.2.0  
**Status**: Production-ready for bottle-based workflows  
**Platform**: macOS (Apple Silicon and Intel)

## Table of Contents

1. [Installation](#installation)
2. [Quick Start](#quick-start)
3. [Core Operations](#core-operations)
4. [Discovery Operations](#discovery-operations)
5. [Maintenance Operations](#maintenance-operations)
6. [Result Types](#result-types)
7. [Error Handling](#error-handling)
8. [Performance](#performance)
9. [Examples](#examples)

## Installation

Add Kombrucha to your `Cargo.toml`:

```toml
[dependencies]
kombrucha = "0.2.0"
tokio = { version = "1", features = ["full"] }
anyhow = "1"
```

## Quick Start

```rust
use kombrucha::PackageManager;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let pm = PackageManager::new()?;
    
    // Install a package
    let result = pm.install("ripgrep").await?;
    println!("Installed {} {}", result.name, result.version);
    
    Ok(())
}
```

## Core Operations

### install(name)

Install a package from a precompiled bottle.

```rust
let result = pm.install("ripgrep").await?;

// Access installation details
println!("Package: {}", result.name);
println!("Version: {}", result.version);
println!("Path: {}", result.path.display());
println!("Dependencies: {:?}", result.dependencies);
println!("Time: {}ms", result.time_ms);
```

**Returns**: `InstallResult` with version, path, and dependencies  
**Async**: Yes (downloads, extracts, creates symlinks)  
**Errors**:
- Formula not found
- No bottle available for current platform
- Download or extraction fails
- Symlink creation fails

### uninstall(name)

Remove a package completely, including symlinks.

```rust
let result = pm.uninstall("ripgrep").await?;
println!("Removed {} {}", result.name, result.version);
```

**Returns**: `UninstallResult` with version and timing  
**Async**: Yes (removes files and symlinks)  
**Errors**: Package not installed, permission denied

### upgrade(name)

Upgrade a package to the latest version.

```rust
let result = pm.upgrade("python").await?;
if result.from_version == result.to_version {
    println!("Already at latest version");
} else {
    println!("Upgraded from {} to {}", result.from_version, result.to_version);
}
```

**Returns**: `UpgradeResult` with before/after versions  
**Async**: Yes (downloads new version, installs, updates symlinks)  
**Behavior**:
- Returns early if already at latest (no download)
- Preserves old version during installation (allows rollback)
- Updates symlinks to new version after successful extraction

### reinstall(name)

Fresh installation (uninstall then install).

```rust
let result = pm.reinstall("wget").await?;
println!("Reinstalled {} {}", result.name, result.version);
```

**Returns**: `ReinstallResult` with path and timing  
**Async**: Yes (removes old version, installs fresh)

### cleanup(dry_run)

Remove old package versions, preserving space.

```rust
// Preview what would be removed
let result = pm.cleanup(true)?;
println!("Would remove: {}", result.removed.len());
println!("Would free: {:.1} MB", result.space_freed_mb);

// Actually clean up
let result = pm.cleanup(false)?;
if !result.errors.is_empty() {
    println!("Errors: {:?}", result.errors);
}
```

**Returns**: `CleanupResult` with removed versions and space freed  
**Sync**: Yes (scans local filesystem only)  
**Behavior**:
- Preserves the currently linked version
- Preserves the newest version
- Removes all other older versions
- Reports errors but continues processing

## Discovery Operations

### search(query)

Search for packages.

```rust
let results = pm.search("python").await?;
println!("Found {} formulae", results.formulae.len());
for formula in &results.formulae {
    println!("  {} - {}", formula.name, formula.desc.unwrap_or_default());
}
```

**Returns**: `SearchResults` (formulae and casks)  
**Async**: Yes (queries Homebrew API)  
**Caching**: Results cached in-memory and on disk (24 hours)

### info(name)

Get detailed package metadata.

```rust
let formula = pm.info("wget").await?;
println!("Name: {}", formula.name);
println!("Version: {}", formula.versions.stable.unwrap_or_default());
println!("Description: {}", formula.desc.unwrap_or_default());
println!("Dependencies: {}", formula.dependencies.len());
println!("Homepage: {}", formula.homepage.unwrap_or_default());
```

**Returns**: `Formula` with complete metadata  
**Async**: Yes (queries Homebrew API)  
**Fields**:
- name, homepage, description (desc)
- versions (stable, head, devel)
- dependencies, build_dependencies
- keg_only, bottle availability

### list()

List all installed packages.

```rust
let installed = pm.list()?;
for pkg in installed {
    println!("{} {}", pkg.name, pkg.version);
}
```

**Returns**: `Vec<InstalledPackage>`  
**Sync**: Yes (scans local Cellar)  
**Performance**: <50ms on typical systems

### outdated()

Find packages with available upgrades.

```rust
let outdated = pm.outdated().await?;
for pkg in outdated {
    if pkg.changeable {  // Not pinned
        println!("{} {} → {}", pkg.name, pkg.installed, pkg.latest);
    }
}
```

**Returns**: `Vec<OutdatedPackage>`  
**Async**: Yes (queries API for each installed package)  
**Note**: Can take ~40 seconds on systems with 300+ packages (queries all against API)

### dependencies(name)

Get package dependencies.

```rust
let deps = pm.dependencies("ffmpeg").await?;
println!("Runtime: {:?}", deps.runtime);
println!("Build: {:?}", deps.build);
```

**Returns**: `Dependencies` with runtime and build deps  
**Async**: Yes (queries API for dependency metadata)

### uses(name)

Find packages that depend on this one.

```rust
let dependents = pm.uses("openssl").await?;
println!("Packages depending on openssl: {:?}", dependents);
```

**Returns**: `Vec<String>` (formula names)  
**Async**: Yes  
**Useful**: Determine impact of uninstalling a package

## Maintenance Operations

### check()

Check system health.

```rust
let health = pm.check()?;
println!("Homebrew available: {}", health.homebrew_available);
println!("Cellar exists: {}", health.cellar_exists);
println!("Prefix writable: {}", health.prefix_writable);

if !health.issues.is_empty() {
    println!("Issues: {:?}", health.issues);
}
```

**Returns**: `HealthCheck` with system state  
**Sync**: Yes (checks filesystem)

## Result Types

### InstallResult

```rust
pub struct InstallResult {
    pub name: String,              // Package name
    pub version: String,           // Installed version
    pub path: PathBuf,             // Path in Cellar
    pub dependencies: Vec<String>, // Direct runtime dependencies
    pub linked: bool,              // Symlinks created
    pub time_ms: u64,              // Installation time
}
```

### UpgradeResult

```rust
pub struct UpgradeResult {
    pub name: String,          // Package name
    pub from_version: String,  // Previous version
    pub to_version: String,    // New version
    pub path: PathBuf,         // Path in Cellar
    pub time_ms: u64,          // Upgrade time
}
```

### UninstallResult

```rust
pub struct UninstallResult {
    pub name: String,     // Package name
    pub version: String,  // Uninstalled version
    pub unlinked: bool,   // Symlinks removed
    pub time_ms: u64,     // Uninstall time
}
```

### OutdatedPackage

```rust
pub struct OutdatedPackage {
    pub name: String,       // Package name
    pub installed: String,  // Current version
    pub latest: String,     // Latest available
    pub changeable: bool,   // Not pinned
}
```

### CleanupResult

```rust
pub struct CleanupResult {
    pub removed: Vec<String>,         // "formula/version" entries
    pub space_freed_mb: f64,          // Space in MB
    pub errors: Vec<(String, String)>, // (formula, error message)
}
```

### HealthCheck

```rust
pub struct HealthCheck {
    pub homebrew_available: bool,  // CLI available
    pub cellar_exists: bool,       // Cellar directory exists
    pub prefix_writable: bool,     // Can write to prefix
    pub issues: Vec<String>,       // Problem descriptions
}
```

## Error Handling

All operations return `anyhow::Result<T>` with detailed error context.

### Common Error Patterns

```rust
use kombrucha::{PackageManager, BruError};

let pm = PackageManager::new()?;

// Pattern 1: Handle specific error types
match pm.install("nonexistent").await {
    Ok(result) => println!("Installed {}", result.version),
    Err(e) => eprintln!("Failed: {}", e),
}

// Pattern 2: Unwrap with context
let formula = pm.info("wget")
    .await
    .expect("Failed to fetch package metadata");

// Pattern 3: Propagate with question mark operator
async fn install_multiple(names: &[&str]) -> anyhow::Result<()> {
    let pm = PackageManager::new()?;
    for name in names {
        let result = pm.install(name).await?;  // Fails fast on first error
        println!("Installed {}", result.name);
    }
    Ok(())
}

// Pattern 4: Recover from errors
let installed = match pm.install("ripgrep").await {
    Ok(result) => result.version,
    Err(_) => {
        eprintln!("Falling back to system package");
        "system".to_string()
    }
};
```

### Common Error Cases

| Operation | Error | Recovery |
|-----------|-------|----------|
| `install("unknown")` | Formula not found | Check package name, search() first |
| `install("keg-only-pkg")` | No bottle | Use `brew install` for source builds |
| Network timeout | API unreachable | Retry or check internet connection |
| Permission denied | Cellar not writable | Run with elevated privileges |
| Already installed | Formula exists | Use `upgrade()` or `--force` |

## Performance

### Operation Timing

**Non-destructive** (no file modifications):

| Operation | Time | Notes |
|-----------|------|-------|
| `list()` | <50ms | Scans local Cellar (339 packages tested) |
| `check()` | 5-10ms | Filesystem checks only |
| `search(query)` | 30-50ms | Cached API query |
| `info(name)` | 200-300ms | Single API request |
| `dependencies(name)` | 0-50ms | Cached after first call |
| `uses(name)` | 20-100ms | Filters all installed packages |
| `cleanup(dry_run)` | 10-20ms | Scans Cellar only |
| `outdated()` | 10,000-50,000ms | Queries all installed packages against API |

**Destructive** (modify system):

| Operation | Time | Notes |
|-----------|------|-------|
| `install()` | 100-500ms | Bottle cached, fast extraction |
| `upgrade()` | 100-500ms | If upgrade available; 0ms if already latest |
| `uninstall()` | 1,000-3,000ms | Removes files and symlinks |
| `cleanup()` | <50ms | Removes old versions |

### Caching Strategy

The library implements multiple caching layers:

1. **In-memory caching** - Formula/cask metadata cached during session (LRU, 1000 formulae)
2. **Disk caching** - API responses cached to `~/.cache/kombrucha/` (24-hour TTL)
3. **Bottle caching** - Downloaded bottles cached to `~/.cache/bru/downloads/`
4. **Connection pooling** - HTTP/2 connections reused (10 per host)

**Note**: `outdated()` is slow because it queries the API for each installed package. For batch operations, consider caching the results:

```rust
let outdated = pm.outdated().await?;
for pkg in &outdated {
    println!("{} can be upgraded", pkg.name);
}
// Reuse 'outdated' list instead of calling outdated() again
```

## Examples

### Example 1: Basic Install Workflow

```rust
use kombrucha::PackageManager;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let pm = PackageManager::new()?;
    
    // Install package
    println!("Installing ripgrep...");
    let result = pm.install("ripgrep").await?;
    println!("✓ Installed {} {}", result.name, result.version);
    
    // Verify it's installed
    let installed = pm.list()?;
    if installed.iter().any(|p| p.name == "ripgrep") {
        println!("✓ Verified installed");
    }
    
    Ok(())
}
```

### Example 2: Batch Upgrade

```rust
async fn upgrade_packages(names: &[&str]) -> anyhow::Result<usize> {
    let pm = PackageManager::new()?;
    let mut count = 0;
    
    for name in names {
        match pm.upgrade(name).await {
            Ok(result) if result.from_version != result.to_version => {
                println!("✓ {} {} → {}", result.name, result.from_version, result.to_version);
                count += 1;
            }
            Ok(_) => println!("  {} already at latest", name),
            Err(e) => eprintln!("✗ {} failed: {}", name, e),
        }
    }
    
    Ok(count)
}
```

### Example 3: Dependency Analysis

```rust
async fn find_dependents(package: &str) -> anyhow::Result<()> {
    let pm = PackageManager::new()?;
    
    let dependents = pm.uses(package).await?;
    
    if dependents.is_empty() {
        println!("No packages depend on {}", package);
    } else {
        println!("{} dependents:", dependents.len());
        for dep in dependents {
            println!("  - {}", dep);
        }
    }
    
    Ok(())
}
```

### Example 4: Safe Cleanup

```rust
async fn safe_cleanup() -> anyhow::Result<()> {
    let pm = PackageManager::new()?;
    
    // Preview changes
    let result = pm.cleanup(true)?;
    println!("Would remove {} versions", result.removed.len());
    println!("Would free {:.1} MB", result.space_freed_mb);
    
    if result.removed.is_empty() {
        println!("Nothing to clean up");
        return Ok(());
    }
    
    // User confirmation would go here...
    let confirmed = true;  // Get from user input
    
    if confirmed {
        let result = pm.cleanup(false)?;
        println!("Cleaned up {} versions", result.removed.len());
        if !result.errors.is_empty() {
            println!("Errors: {:?}", result.errors);
        }
    }
    
    Ok(())
}
```

### Example 5: Health Check & Recovery

```rust
async fn ensure_healthy() -> anyhow::Result<()> {
    let pm = PackageManager::new()?;
    
    let health = pm.check()?;
    
    if !health.issues.is_empty() {
        println!("System issues detected:");
        for issue in &health.issues {
            println!("  ⚠ {}", issue);
        }
        return Err(anyhow::anyhow!("System not healthy"));
    }
    
    println!("✓ System healthy");
    println!("  Cellar: {}", pm.cellar().display());
    println!("  Prefix: {}", pm.prefix().display());
    
    Ok(())
}
```

## Resource Management

The `PackageManager` holds a single HTTP client for the entire session. This enables connection pooling and efficient resource usage. Keep the same instance alive for multiple operations:

```rust
// Good: Reuse PM instance
let pm = PackageManager::new()?;
pm.install("ripgrep").await?;
pm.install("fd").await?;  // Reuses HTTP client from first call
pm.upgrade("ripgrep").await?;

// Less efficient: Creates new client each time
PackageManager::new()?.install("ripgrep").await?;
PackageManager::new()?.install("fd").await?;  // New HTTP client
```

## Compatibility

- **Homebrew**: 100% compatible. Packages installed by the library can be used with `brew`.
- **Platform**: macOS (Apple Silicon and Intel)
- **Rust Edition**: 2024

## See Also

- [Package Manager Source](../src/package_manager.rs) - Implementation
- [Library Module](../src/lib.rs) - Public API exports
- [Examples](../examples/) - Complete working examples
- [Test Report](../ai/PHASE_3_TEST_REPORT.md) - Integration test results
