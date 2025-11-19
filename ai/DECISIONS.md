# Architectural Decisions

**Last Updated**: November 10, 2025

## Active Decisions

### Library API Design (v0.1.35)

**Decision**: High-level `PackageManager` API wrapping low-level modules

**Rationale**:
- Downstream projects (like Cutler) need simple, unified interface
- Don't expose all internal complexity (cellar, download, extract, symlink modules)
- Maintain backward compatibility with low-level API for advanced users
- Shared HTTP client enables connection pooling and efficient resource usage

**Implementation**:
- `PackageManager` struct holds `BrewApi` and `reqwest::Client`
- All operations return rich result types (InstallResult, UpgradeResult, etc)
- Async for network operations, sync for local operations
- Errors wrapped in `anyhow::Result` with context

**Trade-offs**:
- ✅ Simple for common cases
- ✅ Performant (connection pooling)
- ✅ Type-safe (no stringly-typed results)
- ⚠️ Doesn't expose all advanced options (OK for MVP)

**Future Extensions**:
- Add `is_installed()` helper (trivial, high-value)
- Add batch operations (`install_multiple`, `upgrade_multiple`)
- Add cask support (requires wrapping `Cask` type similarly)

---

### Version Strategy (v0.2.0 for Library API)

**Decision**: Bumped to v0.2.0 for library API addition

**Rationale**:
- Library API is substantial feature addition (~730 lines, 9+ core operations)
- Introduces new public API surface area with PackageManager interface
- Full programmatic access to package management (install, upgrade, uninstall, etc.)
- Comprehensive refactored documentation and examples
- While additive and non-breaking, the library API represents significant new functionality

**Semantic Versioning**:
- 0.1.x = CLI-focused feature additions
- 0.2.0 = Library API addition with programmatic access
- Future 0.3.0+ = Major features like source builds or architectural shifts

---

### Async/Sync Boundary

**Decision**: Async for bottle operations, sync for local operations

**Operations by Category**:

**Async** (network/download-heavy):
- `install()` - Downloads bottle
- `uninstall()` - Async symlink cleanup (future)
- `upgrade()` - Downloads new bottle
- `reinstall()` - Combo of above
- `search()` - API query
- `info()` - API query
- `outdated()` - API queries (one per package)
- `dependencies()` - API query
- `uses()` - API queries (filtered)

**Sync** (local filesystem only):
- `list()` - Cellar scan
- `cleanup()` - Filesystem traversal
- `check()` - System health checks
- `cellar()` / `prefix()` - Path queries

**Rationale**:
- Tokio is lightweight, but don't use async for unnecessary blocking ops
- Local filesystem is fast enough
- Makes API simpler (no blocking_on context needed)

---

### Error Handling Strategy

**Decision**: Use `anyhow::Result<T>` throughout library

**Rationale**:
- Allows error context to be added at each level
- Downstream projects can match on specific errors via `.downcast()`
- BruError enum still available for low-level modules
- Library consumers typically want "something went wrong" with context

**Example**:
```rust
pm.install("ripgrep").await
  .context("failed to install ripgrep")?
```

---

### HTTP Client Resource Management

**Decision**: Single shared `reqwest::Client` in PackageManager

**Rationale**:
- Enables HTTP/2 connection pooling
- Reduces TCP/TLS overhead during parallel operations
- Matches performance characteristics of Homebrew optimizations
- Users reuse same `PackageManager` instance for multiple operations

**Configuration**:
```rust
timeout: 10s
pool_idle_timeout: 90s (HTTP keep-alive standard)
pool_max_idle_per_host: 10
```

---

## Deferred Decisions (Future Phases)

### Cask Support in PackageManager

**Status**: Not implemented yet

**Design sketch**:
```rust
impl PackageManager {
    pub async fn install_cask(&self, name: &str) -> Result<InstallResult>
    pub async fn uninstall_cask(&self, name: &str) -> Result<UninstallResult>
    pub fn list_casks(&self) -> Result<Vec<InstalledPackage>>
}
```

**Considerations**:
- Cask operations are mostly download + extract (different paths than formulae)
- May have different dependency model
- Requires testing with applications (Slack, VSCode, etc)

---

### Batch Operations Optimization

**Status**: Not implemented yet

**Design sketch**:
```rust
pub async fn install_multiple(&self, names: &[&str]) -> Result<Vec<InstallResult>> {
    futures::future::try_join_all(
        names.iter().map(|n| self.install(n))
    ).await
}
```

**Considerations**:
- Would improve Cutler's declarative setup workflow
- Need to consider rate limiting (don't hammer Homebrew API)
- Error handling: fail-fast vs collect-all-errors

---

### Parallel Outdated() Optimization

**Status**: Known limitation (~42s on 340 packages)

**Current**: Sequential API queries per package
**Possible**: Batch queries to API or parallelize with semaphore

**Trade-offs**:
- Current: Simple, reliable, matches Homebrew behavior
- Parallel: Could be 3-5x faster, but adds complexity
- Deferred: OK for interactive tool (users typically run once)

---

### Source Build Support (Phase 5)

**Status**: Planned, not started

**Design considerations**:
- Embed Ruby runtime (via `magnus` crate)
- Execute formula DSL for building from source
- Would unlock remaining ~5% formulae without bottles
- Major undertaking, deferred until library API stabilized

---

### Installation History & Rollback Tracking

**Status**: Future feature (v0.3.0+)

**Problem**: No record of package operations for rollback/audit
- User reinstalls `gettext`, needs to know which dependent packages were recently reinstalled
- No way to rollback to previous package state after upgrade
- Can't audit "what did I install yesterday?"

**Proposed Design**:

| Approach | Pros | Cons |
|----------|------|------|
| **SQLite** | Fast queries, transactions, mature | 300KB binary size |
| **DuckDB** | Analytics-friendly, fast aggregates | 30MB+ binary, overkill |
| **JSONL log** | Simple, grep-able, zero deps | Slow for large histories |

**Recommendation**: SQLite (`rusqlite` crate)
- Store in `~/.bru/history.db`
- Schema: `operations(timestamp, action, package, version, prev_version, metadata)`
- Operations tracked: install, uninstall, upgrade, reinstall
- Retention: configurable (default: 90 days)

**Features**:
```bash
bru history                    # Show recent operations
bru history gettext            # Operations for specific package
bru rollback gettext           # Restore previous version
bru undo                       # Undo last operation
```

**Integration Points**:
- Hook into `install()`, `upgrade()`, `uninstall()` operations
- Record before/after versions
- Store dependency tree snapshots for full rollback

**Trade-offs**:
- ✅ Powerful audit/rollback capabilities
- ✅ Small overhead (SQLite is fast)
- ⚠️ New dependency (rusqlite)
- ⚠️ Homebrew doesn't have this (not a compatibility blocker)

**Defer Until**: After source build support (Phase 5) - core compatibility first

---

## See Also

- [STATUS.md](./STATUS.md) - Current project state
- [TODO.md](./TODO.md) - Active tasks
- [docs/library-api.md](../docs/library-api.md) - API documentation
