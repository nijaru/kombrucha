# Kombrucha Library API Specification

**Status**: Planning / Phase 5 (Future Initiative)

**Effort Estimate**: 40-60 hours (2-3 weeks, one senior engineer)

---

## Executive Summary

This document outlines the effort to expose Kombrucha's core functionality as a stable, published Rust library (`kombrucha` crate on crates.io). This enables downstream projects like Cutler to depend on Kombrucha programmatically instead of shelling out to the `bru` CLI.

**Key Benefits:**
- Zero subprocess overhead (direct Rust API calls)
- Proper error handling and typing (no string parsing)
- Composable components (dependency resolver, API client, bottle manager)
- Reusable by other Rust projects building Homebrew tooling

---

## Current State Assessment

### What's Already Good ✅
- **Modular architecture**: `api.rs`, `download.rs`, `extract.rs`, `cellar.rs`, `tap.rs` are already internal packages
- **Strong types**: `Formula`, `Cask`, `BottleFile`, `InstallReceipt`, etc. already exist
- **Error handling**: `BruError` enum with `thiserror` already defined
- **Async foundation**: Uses `tokio` consistently throughout
- **No CLI dependencies**: Core logic doesn't depend on `clap`, `colored`, or UI crates

### What Needs Work ❌
1. **Circular dependencies**: Some modules import from `commands.rs` which imports CLI-only code
2. **Incomplete public API**: Many useful functions are private (`pub fn` → only when needed)
3. **Missing documentation**: No rustdoc comments, no examples
4. **No library entry point**: `lib.rs` only exposes 2 functions (symlink utilities)
5. **CLI-specific logic mixed in**: Some functions handle error formatting/logging in verbose ways
6. **Missing core abstractions**: No `PackageManager` or `BrewContext` abstraction to tie it together

---

## Effort Breakdown

### Phase 1: Module Cleanup & Organization (10-12 hours)

**Goal**: Decouple CLI from library code, establish clear public API boundaries.

#### 1.1 Refactor Circular Dependencies (3-4 hours)
- Extract CLI dispatching logic from `commands.rs`
- Create `lib_commands.rs` (or similar) with pure library functions
- Remove `colors.rs` and `colored` crate dependency from library code
- Ensure core logic doesn't depend on `clap` or terminal UI libraries

**Files affected**: `commands.rs`, `main.rs`, potentially new `src/lib/` directory

#### 1.2 Create Module Hierarchy (2-3 hours)
```
src/
  lib.rs              # Main library entry point
  lib/
    api.rs            # Formula/Cask API client (re-export from src/api.rs)
    cellar.rs         # Cellar management (re-export from src/cellar.rs)
    download.rs       # Bottle downloads (re-export from src/download.rs)
    extract.rs        # Bottle extraction (re-export from src/extract.rs)
    tap.rs            # Tap management (re-export from src/tap.rs)
    package.rs        # NEW: High-level package operations
    error.rs          # Error types (re-export from src/error.rs)
    types.rs          # NEW: Public data types re-exports
  # CLI-specific code stays here
  cli/
    commands.rs       # CLI command handlers (not part of library)
    ui.rs             # Terminal UI code
```

#### 1.3 Stabilize Error Handling (2-3 hours)
- Ensure all error variants are publicly documented
- Add error context methods (`context`, `with_context`)
- Verify error messages don't include UI-specific formatting
- Add error recovery examples in rustdoc

**Files affected**: `src/error.rs`

---

### Phase 2: Public API Definition (12-15 hours)

**Goal**: Define and expose stable library API with comprehensive documentation.

#### 2.1 API Client Module (4-5 hours)
Make `BrewApi` public with documented methods:

```rust
// src/lib/api.rs or src/api.rs (make public)
pub struct BrewApi { /* ... */ }

impl BrewApi {
    pub fn new() -> Self { /* ... */ }
    pub async fn formula(&self, name: &str) -> Result<Formula> { /* ... */ }
    pub async fn cask(&self, name: &str) -> Result<Cask> { /* ... */ }
    pub async fn search(&self, query: &str) -> Result<Vec<Formula>> { /* ... */ }
    pub async fn formula_versions(&self, name: &str) -> Result<Versions> { /* ... */ }
    pub async fn dependencies(&self, name: &str) -> Result<Vec<String>> { /* ... */ }
    pub async fn reverse_dependencies(&self, name: &str) -> Result<Vec<String>> { /* ... */ }
}
```

**Documentation needed:**
- What `BrewApi` does (queries Homebrew's JSON API)
- Caching behavior (with cache size docs)
- Timeout behavior
- Example: "Fetch formula metadata and print dependencies"

**Files affected**: `src/api.rs`

#### 2.2 Cellar Management Module (3-4 hours)
Public functions for reading the system cellar:

```rust
// src/lib/cellar.rs
pub fn cellar_path() -> PathBuf { /* ... */ }
pub fn installed_formulae() -> Result<Vec<InstalledFormula>> { /* ... */ }
pub fn get_installed_version(name: &str) -> Result<Option<InstalledVersion>> { /* ... */ }
pub fn read_receipt(formula: &str, version: &str) -> Result<InstallReceipt> { /* ... */ }
pub fn list_installed(pattern: Option<&str>) -> Result<Vec<InstalledPackage>> { /* ... */ }

pub struct InstalledFormula {
    pub name: String,
    pub version: String,
    pub versions: Vec<String>,
}
```

**Documentation needed:**
- What the Cellar is (Homebrew's package directory)
- How to list installed packages
- How to read installation metadata
- Example: "List all outdated packages"

**Files affected**: `src/cellar.rs`

#### 2.3 Package Management Module (4-6 hours)
HIGH-LEVEL API (the main abstraction users interact with):

```rust
// NEW: src/lib/package.rs
pub struct PackageManager {
    api: BrewApi,
    prefix: PathBuf,
}

impl PackageManager {
    pub fn new() -> Result<Self> { /* ... */ }
    pub async fn info(&self, name: &str) -> Result<PackageInfo> { /* ... */ }
    pub async fn installed_version(&self, name: &str) -> Result<Option<String>> { /* ... */ }
    pub async fn check_upgradeable(&self, name: &str) -> Result<bool> { /* ... */ }
    pub async fn dependency_tree(&self, name: &str) -> Result<DependencyTree> { /* ... */ }
    pub async fn all_outdated(&self) -> Result<Vec<OutdatedPackage>> { /* ... */ }
}

pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub installed_version: Option<String>,
    pub dependencies: Vec<String>,
    pub dependents: Vec<String>,
}

pub struct DependencyTree {
    pub root: String,
    pub children: Vec<DependencyTree>,
}
```

**Documentation needed:**
- What `PackageManager` does (main user-facing API)
- How to query package info
- How to check what's installed vs available
- Example: "Find all outdated packages and their dependencies"

**Files affected**: NEW file `src/lib/package.rs`

#### 2.4 Download & Install Module (2-3 hours)
Public bottle download/extraction API:

```rust
// src/lib/download.rs & src/lib/extract.rs
pub async fn download_bottle(
    formula: &Formula,
    platform: &str,
    progress: Option<ProgressCallback>,
) -> Result<PathBuf> { /* ... */ }

pub async fn verify_bottle(path: &Path, expected_sha256: &str) -> Result<bool> { /* ... */ }

pub fn extract_bottle(
    source: &Path,
    dest: &Path,
) -> Result<()> { /* ... */ }

pub type ProgressCallback = Box<dyn Fn(u64, u64) + Send>;
```

**Documentation needed:**
- How to download bottles programmatically
- Checksum verification process
- Progress callback usage
- Example: "Download and extract a bottle for offline installation"

**Files affected**: `src/download.rs`, `src/extract.rs`

---

### Phase 3: Documentation & Examples (10-15 hours)

**Goal**: Make the library discoverable and easy to use.

#### 3.1 Rustdoc Comments (6-8 hours)
Add to every public item:
- **`//!` module docs** - What does this module do?
- **`///` item docs** - What does this function do? When would I use it?
- **Examples** - Runnable code examples for major APIs
- **Errors** - What can go wrong?
- **Panics** - What could cause a panic? (if any)

Example structure:
```rust
/// Query installed Homebrew packages.
///
/// Returns a list of all installed formulae in the Cellar directory.
///
/// # Examples
///
/// ```
/// use kombrucha::cellar::installed_formulae;
///
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     let packages = installed_formulae()?;
///     for pkg in packages {
///         println!("{} {}", pkg.name, pkg.version);
///     }
///     Ok(())
/// }
/// ```
///
/// # Errors
///
/// Returns `Err` if the Cellar directory doesn't exist or can't be read.
pub async fn installed_formulae() -> Result<Vec<InstalledFormula>> {
    // ...
}
```

#### 3.2 Create Examples Directory (2-3 hours)
```
examples/
  basic_info.rs          # Query formula info
  check_upgradeable.rs   # Find outdated packages
  dependency_tree.rs     # Visualize dependencies
  list_installed.rs      # Show all installed packages
  download_bottle.rs     # Download a bottle offline
  custom_tap.rs          # Work with custom taps
```

Each example should be runnable with `cargo run --example <name>`.

#### 3.3 Library README (2-4 hours)
New file: `docs/library-guide.md`
- What is the Kombrucha library?
- Quick start (5 min)
- Common tasks (querying, installing, etc.)
- API overview
- Performance characteristics
- Limitations and workarounds

---

### Phase 4: Testing & Stability (10-12 hours)

**Goal**: Ensure library API is reliable and well-tested.

#### 4.1 Add Library Tests (5-6 hours)
Create `tests/library_api.rs` covering:
- API client (mock HTTP responses)
- Cellar operations
- Dependency resolution
- Error cases and recovery
- Async operation ordering

#### 4.2 Integration Tests (3-4 hours)
- Test against real Homebrew (optional, marked `#[ignore]`)
- Test with custom taps
- Test bottle downloads and extraction
- Performance benchmarks for library API

#### 4.3 BREAKING CHANGE Audit (2 hours)
- Identify any unstable types that need versioning
- Document which types are stable vs experimental
- Plan for future changes (e.g., new fields, new variants)

---

### Phase 5: Publishing (3-5 hours)

**Goal**: Release library to crates.io.

#### 5.1 Pre-publication Checklist (1-2 hours)
- [ ] Update `Cargo.toml` metadata (description, keywords, categories)
- [ ] Add crates.io documentation link to `Cargo.toml`
- [ ] Ensure all `pub` items have rustdoc comments
- [ ] Run `cargo doc --open` and verify rendering
- [ ] Create CHANGELOG entry for library release

#### 5.2 Version Planning (1 hour)
- Decide: publish as `0.2.0` (breaking from CLI v0.1.x)?
- Or use separate versioning: CLI `0.1.x`, library `0.1.x`?
- Plan semver strategy for library API stability

#### 5.3 Publish to crates.io (1-2 hours)
```bash
cargo login
cargo publish --allow-dirty  # or clean up first
```

---

## Effort Summary

| Phase | Hours | Duration | Risk |
|-------|-------|----------|------|
| 1: Module Cleanup | 10-12 | 2-3 days | Medium (refactoring) |
| 2: Public API | 12-15 | 3-4 days | Low (mostly additions) |
| 3: Documentation | 10-15 | 2-4 days | Low (tedious, not hard) |
| 4: Testing | 10-12 | 2-3 days | Low (straightforward) |
| 5: Publishing | 3-5 | 1 day | Low (mechanical) |
| **Total** | **45-59** | **2-3 weeks** | **Low-Medium** |

**Total estimate: 50 hours / 2.5 weeks (one senior engineer, or 1.5-2 weeks with two engineers)**

---

## Implementation Strategy

### Recommended Approach: Parallel Development

**Option A: Feature Branch (Recommended)**
1. Create `feature/library-api` branch
2. Work through all 5 phases without breaking `main`
3. CLI continues to work during development
4. Merge when complete, publish `v0.2.0` library + `v0.1.x` CLI update

**Option B: Experimental Crate (Alternative)**
1. Create `kombrucha-core` as separate crate in workspace
2. Both CLI and library depend on `kombrucha-core`
3. Publish `kombrucha-core` immediately (lower pressure)
4. Rename to `kombrucha` later when fully stable

---

## Risk Mitigation

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|-----------|
| Circular dependencies break CLI | Low | High | Phase 1: comprehensive testing before Phase 2 |
| Public API becomes unstable | Medium | Medium | Phase 4: full test coverage, stability audit |
| Documentation is incomplete | Medium | Low | Use `#![warn(missing_docs)]` to enforce |
| Downstream adoption is slow | Low | Low | Publish good examples, write adoption guide |

---

## Success Criteria

- ✅ All core modules are public with stable types
- ✅ All public items have rustdoc comments with examples
- ✅ `PackageManager` abstraction works for 80% of use cases
- ✅ Examples directory has 5+ runnable examples
- ✅ Test coverage for library API ≥ 80%
- ✅ Published on crates.io with 50+ documentation links
- ✅ CLI still works identically (zero breaking changes)

---

## Timeline Estimate

**Optimistic (experienced team, clear spec)**: 10-12 days
**Realistic (single engineer)**: 2-3 weeks
**Conservative (with learning)**: 3-4 weeks

---

## Next Steps

1. **Validate scope** - Does this match user expectations?
2. **Prioritize** - Start with Phase 1 & 2 (core functionality)
3. **Create tracking issue** - Break into GitHub issues
4. **Assign owner** - Who leads this work?
5. **Setup branch** - Create feature branch

---

## Related Issues

- Users wanting library API (#XX)
- Cutler integration request (#XX)
- Future: Ruby interop for library (Phase 3 blocker removal)

---

## Appendix: Example Usage Post-Implementation

```rust
// From downstream: Cutler or other Homebrew tools
use kombrucha::{PackageManager, BrewApi};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // High-level API (what most users want)
    let pm = PackageManager::new()?;
    
    let outdated = pm.all_outdated().await?;
    println!("Outdated packages: {}", outdated.len());
    
    for pkg in outdated {
        println!("  {} {} → {}", pkg.name, pkg.installed, pkg.available);
    }
    
    // Check dependencies before upgrade
    let deps = pm.dependency_tree("python").await?;
    println!("Python dependencies: {:?}", deps);
    
    // Low-level API (for advanced use cases)
    let api = BrewApi::new();
    let formula = api.formula("ripgrep").await?;
    
    println!("Ripgrep: {} ({})", formula.name, formula.versions.stable.unwrap());
    println!("Dependencies: {:?}", formula.dependencies);
    
    Ok(())
}
```

