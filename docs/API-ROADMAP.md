# PackageManager API Roadmap

This document outlines potential improvements and future directions for the Kombrucha library API.

## v0.1.35 (Current)

**Status**: Production-ready ✅

Core operations:
- `install()`, `uninstall()`, `upgrade()`, `reinstall()`
- `search()`, `info()`, `list()`, `outdated()`, `dependencies()`, `uses()`
- `cleanup()`, `check()`

## v0.1.36 (Planned Improvements)

### High-Value Helpers

**`is_installed(name: &str) -> Result<bool>`**
- Quick existence check without scanning full list
- Useful for conditional setup (don't reinstall if already there)
- Trivial to implement: `!self.list()?.is_empty()`

**`install_multiple(names: &[&str]) -> Result<Vec<InstallResult>>`**
- Batch install with parallelization
- Useful for declarative setup tools (Cutler)
- Consider rate limiting to avoid hammering Homebrew API

**`upgrade_multiple(names: &[&str]) -> Result<Vec<UpgradeResult>>`**
- Parallel batch upgrades
- Same rate limiting considerations

### Performance Optimizations

**Parallelize `outdated()` queries**
- Current: Sequential API queries (~42s on 340 packages)
- Possible: Use tokio::spawn with semaphore (~10-15s possible)
- Trade-off: Slightly more complex code, significant speedup

**Cache formula metadata**
- Current: Each operation queries API independently
- Possible: LRU cache for formula metadata (api.rs already has in-memory cache)
- Benefit: Reduce API calls 10-100x in batch operations

## Future Phases (0.2.0+)

### Cask Integration

Currently only supports formulae. Cask support would enable:
- `install_cask(name)` for applications (Slack, VSCode, etc)
- `list_casks()` for installed applications
- Full declarative macOS setup (formulae + casks + system settings)

**Use Case**: Integration with Cutler for complete system setup

**Design**:
```rust
pub async fn install_cask(&self, name: &str) -> Result<CaskInstallResult>
pub async fn uninstall_cask(&self, name: &str) -> Result<UninstallResult>
pub fn list_casks(&self) -> Result<Vec<InstalledPackage>>
pub async fn upgrade_cask(&self, name: &str) -> Result<UpgradeResult>
```

### Source Build Support

Currently can't build from source (affects ~5% of formulae without bottles).

**Requirements**:
- Embed Ruby runtime (via `magnus` crate)
- Parse and execute `.rb` formula files
- Handle build dependencies and compilation

**Benefit**: 100% formula coverage

**Timeline**: Phase 5, after library API stabilized

### Advanced Query API

```rust
pub async fn search_detailed(&self, query: &str) -> Result<DetailedSearchResults>
pub fn list_by_name(&self, pattern: &str) -> Result<Vec<InstalledPackage>>
pub async fn find_dependents(&self, name: &str) -> Result<Vec<String>>
```

### Resource Callbacks

For long operations, allow progress/status callbacks:

```rust
pub struct ProgressCallback {
    pub on_download_start: Option<Box<dyn Fn(&str)>>,
    pub on_download_progress: Option<Box<dyn Fn(&str, u64, u64)>>,
    pub on_extract_start: Option<Box<dyn Fn(&str)>>,
}

impl PackageManager {
    pub async fn install_with_progress(&self, name: &str, cb: ProgressCallback) -> Result<InstallResult>
}
```

**Use Case**: TUI progress bars, integration with system managers

## API Stability

The current v0.1.35 API is stable and production-ready. The design commits to:

1. **No breaking changes in 0.1.x** - All additions are opt-in
2. **Semantic versioning** - Major changes only in 0.2.0+
3. **Backward compatibility** - Existing code continues to work

## Decision Criteria for New Features

Add to PackageManager API if:
- ✅ Common workflow (used by 2+ projects)
- ✅ Significantly simpler than low-level API
- ✅ No performance impact
- ✅ Fits natural ownership model (PackageManager responsibility)

Don't add if:
- ❌ Rarely used edge case
- ❌ Can be implemented by wrapper
- ❌ Adds complexity without value

## See Also

- [docs/library-api.md](library-api.md) - Current API reference
- [ai/STATUS.md](../ai/STATUS.md) - Project status
- [ai/DECISIONS.md](../ai/DECISIONS.md) - Architectural decisions
