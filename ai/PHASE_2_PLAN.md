# Phase 2: High-Level API Abstraction - PackageManager

**Status**: PLANNING  
**Target**: Create unified `PackageManager` interface wrapping low-level modules  
**Dependency**: Phase 1.4 (examples) - COMPLETE ✅

## Overview

Phase 2 creates a high-level `PackageManager` abstraction that wraps all low-level modules (api, cellar, download, extract, symlink, receipt, tap). This provides downstream projects like Cutler with a single, unified interface instead of juggling multiple modules.

**Key Principle**: Simplify library usage without exposing internal complexity or CLI dependencies.

## Goals

1. **Single unified interface** for common package operations
2. **Automatic resource management** (single HTTP client, connection pooling, caching)
3. **Simplified error handling** with consistent error types
4. **Backward compatible** with existing low-level module API
5. **Production-ready** with comprehensive error handling and logging

## Design

### PackageManager Structure

```rust
pub struct PackageManager {
    api: BrewApi,
    client: reqwest::Client,
    // ... other shared resources
}

impl PackageManager {
    // Creation
    pub fn new() -> Result<Self>;
    
    // Core operations
    pub async fn install(&self, name: &str) -> Result<InstallResult>;
    pub async fn uninstall(&self, name: &str) -> Result<UninstallResult>;
    pub async fn upgrade(&self, name: &str) -> Result<UpgradeResult>;
    pub async fn reinstall(&self, name: &str) -> Result<ReinstallResult>;
    
    // Discovery
    pub async fn search(&self, query: &str) -> Result<SearchResults>;
    pub async fn info(&self, name: &str) -> Result<Formula>;
    pub async fn dependencies(&self, name: &str) -> Result<Dependencies>;
    pub async fn uses(&self, name: &str) -> Result<Vec<String>>;
    pub async fn list(&self) -> Result<Vec<InstalledPackage>>;
    pub async fn outdated(&self) -> Result<Vec<OutdatedPackage>>;
    
    // Maintenance
    pub fn cleanup(&self, dry_run: bool) -> Result<CleanupResult>;
    pub async fn update(&self) -> Result<()>;
    
    // Utilities
    pub fn prefix(&self) -> PathBuf;
    pub fn cellar(&self) -> PathBuf;
    pub async fn check(&self) -> Result<HealthCheck>;
}
```

### Outcome Types

Each operation returns a rich result type with detailed information:

```rust
pub struct InstallResult {
    pub name: String,
    pub version: String,
    pub path: PathBuf,
    pub dependencies: Vec<String>,
    pub linked: bool,
    pub time_ms: u64,
}

pub struct UpgradeResult {
    pub name: String,
    pub from_version: String,
    pub to_version: String,
    pub path: PathBuf,
    pub time_ms: u64,
}

pub struct OutdatedPackage {
    pub name: String,
    pub installed: String,
    pub latest: String,
    pub changeable: bool,
}

pub struct CleanupResult {
    pub removed: Vec<String>,
    pub space_freed_mb: f64,
    pub errors: Vec<(String, String)>,
}

pub struct HealthCheck {
    pub homebrew_available: bool,
    pub cellar_exists: bool,
    pub prefix_writable: bool,
    pub issues: Vec<String>,
}
```

## Implementation Plan

### Phase 2.1: Core Structure (P0)
- [ ] Create `src/package_manager.rs`
- [ ] Implement `PackageManager::new()` with resource initialization
- [ ] Create result types (InstallResult, UpgradeResult, etc.)
- [ ] Implement basic operations: install, uninstall, upgrade, reinstall
- [ ] Add comprehensive error handling

### Phase 2.2: Discovery Operations (P1)
- [ ] Implement search, info, dependencies, uses
- [ ] Implement list, outdated
- [ ] Comprehensive error messages
- [ ] Test with live API

### Phase 2.3: Maintenance Operations (P1)
- [ ] Implement cleanup with result type
- [ ] Implement update with progress tracking
- [ ] Implement check with health reporting
- [ ] Error recovery strategies

### Phase 2.4: Documentation & Examples (P2)
- [ ] Comprehensive module documentation
- [ ] Usage examples for each operation
- [ ] Error handling patterns
- [ ] Performance tips
- [ ] Update examples/ to use PackageManager

### Phase 2.5: Testing & Validation (P2)
- [ ] Unit tests for all operations
- [ ] Integration tests with live API
- [ ] Error path testing
- [ ] Backward compatibility tests

## Key Design Decisions

### 1. Single HTTP Client
**Why**: Connection pooling, reduced memory overhead, shared keep-alives
```rust
pub struct PackageManager {
    client: reqwest::Client,  // Shared across all operations
}
```

### 2. Async-First API
**Why**: Modern Rust, supports concurrent operations naturally
```rust
pub async fn install(&self, name: &str) -> Result<InstallResult>;
```

### 3. Rich Result Types
**Why**: Callers get detailed information without multiple queries
```rust
pub struct InstallResult {
    pub name: String,
    pub version: String,
    pub path: PathBuf,
    pub dependencies: Vec<String>,
    pub linked: bool,
    pub time_ms: u64,
}
```

### 4. Backward Compatibility
**Why**: Existing code using low-level APIs continues to work
- Low-level modules (api, cellar, download, etc.) remain unchanged
- PackageManager is an optional wrapper, not replacement

### 5. Ownership Model
**Why**: Clear resource responsibility
```rust
// PackageManager owns its resources
let pm = PackageManager::new()?;
pm.install("ripgrep").await?;  // pm stays alive for connection reuse
```

## Integration with Examples

After Phase 2 completes, examples will be updated to show both:

1. **Low-level API** (for advanced use cases)
   ```rust
   let api = BrewApi::new()?;
   let formula = api.fetch_formula("ripgrep").await?;
   ```

2. **High-level API** (for common workflows)
   ```rust
   let pm = PackageManager::new()?;
   let result = pm.install("ripgrep").await?;
   ```

## Success Criteria

- [ ] Single PackageManager interface covers 80% of common workflows
- [ ] All operations have comprehensive error handling
- [ ] Documentation with examples for each operation
- [ ] Backward compatible with existing API
- [ ] Performance on par with CLI (no unnecessary overhead)
- [ ] Ready for production use by downstream projects

## Testing Strategy

1. **Unit tests**: Each operation in isolation
2. **Integration tests**: Against live API (non-destructive)
3. **Error path tests**: Network failures, missing packages, etc.
4. **Performance tests**: No regression from low-level API

## Blockers & Dependencies

- ✅ Phase 1.4 (examples) complete
- No external dependencies needed
- No breaking changes required

## Timeline

- **Phase 2.1**: Core structure (2-3 hours)
- **Phase 2.2**: Discovery operations (2-3 hours)
- **Phase 2.3**: Maintenance operations (1-2 hours)
- **Phase 2.4**: Documentation (1-2 hours)
- **Phase 2.5**: Testing & validation (2-3 hours)

**Total estimate**: 8-13 hours

## Related Documentation

- `LIBRARY_API_SPEC.md` - Initial API design (may reference)
- `PHASE_1.4_COMPLETION.md` - Completed examples foundation
- `examples/README.md` - Example usage patterns

## Next After Phase 2

**Phase 3: Advanced Features**
- Batch operations (install multiple, upgrade multiple)
- Async/concurrent operations
- Custom tap support enhancements
- Caching strategies and invalidation

**Phase 4: Integration**
- Real-world testing with Cutler
- Performance optimization
- Production hardening
- Release as stable library
