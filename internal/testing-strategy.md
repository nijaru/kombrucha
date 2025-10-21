# Testing Strategy

## Philosophy

**Use Homebrew's infrastructure, test our code.**

We are not rebuilding the Homebrew ecosystem. We're building a faster client that uses their proven infrastructure. Therefore:

- ✅ Use their bottles, formulae, JSON API, taps
- ✅ Test against real Homebrew infrastructure
- ✅ Validate compatibility with brew commands
- ❌ Don't mock Homebrew API (test against real endpoints)
- ❌ Don't build bottles (consume theirs)
- ❌ Don't create test formulae (use homebrew-core)

## Test Pyramid

```
         /\
        /  \     E2E Compatibility Tests
       /----\    (Compare bru vs brew)
      /      \
     /--------\  Integration Tests
    /          \ (Real Homebrew API)
   /------------\
  /--------------\ Unit Tests
 /                \ (Pure Rust logic)
```

### Layer 1: Unit Tests (~70% of tests)

**Location**: `src/**/*.rs` with `#[cfg(test)]` modules

**What to test**:
- JSON parsing/deserialization
- Search filtering logic
- Dependency tree building
- Data structure transformations
- Error handling paths

**Example**:
```rust
// src/api.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formula_with_optional_fields() {
        let json = r#"{"name":"test","desc":null,"homepage":null}"#;
        let formula: Formula = serde_json::from_str(json).unwrap();
        assert_eq!(formula.name, "test");
        assert!(formula.desc.is_none());
    }

    #[test]
    fn test_search_results_count() {
        let results = SearchResults {
            formulae: vec![/* mock data */],
            casks: vec![/* mock data */],
        };
        assert_eq!(results.total_count(), 10);
    }
}
```

**Run**: `cargo test`

### Layer 2: Integration Tests (~25% of tests)

**Location**: `tests/*.rs`

**What to test**:
- Real API calls to formulae.brew.sh
- End-to-end command execution
- Error handling with real API responses
- Performance characteristics

**Example**:
```rust
// tests/api_integration.rs
use bru::api::BrewApi;

#[tokio::test]
async fn test_search_rust_returns_results() {
    let api = BrewApi::new().unwrap();
    let results = api.search("rust").await.unwrap();

    assert!(!results.is_empty());
    assert!(results.formulae.len() > 10);

    // Check that rust formula is in results
    assert!(results.formulae.iter().any(|f| f.name == "rust"));
}

#[tokio::test]
async fn test_info_wget_has_correct_fields() {
    let api = BrewApi::new().unwrap();
    let formula = api.fetch_formula("wget").await.unwrap();

    assert_eq!(formula.name, "wget");
    assert!(formula.desc.is_some());
    assert!(formula.versions.stable.is_some());
    assert!(!formula.dependencies.is_empty());
}

#[tokio::test]
async fn test_deps_resolves_correctly() {
    let api = BrewApi::new().unwrap();
    let formula = api.fetch_formula("wget").await.unwrap();

    // Verify all deps are valid formulae
    for dep in &formula.dependencies {
        let dep_formula = api.fetch_formula(dep).await;
        assert!(dep_formula.is_ok(), "Dependency {} not found", dep);
    }
}
```

**Run**: `cargo test --test '*'`

**Note**: These tests hit real API, so:
- May be slower
- Require network connection
- Could fail if API is down
- Use rate limiting (see caching strategy below)

### Layer 3: Compatibility Tests (~5% of tests)

**Location**: `tests/compatibility/`

**What to test**:
- `bru` output matches `brew` behavior
- Compatibility with Homebrew paths
- Installation produces same results
- Receipts are compatible

**Example**:
```bash
#!/usr/bin/env bash
# tests/compatibility/test_info.sh

set -euo pipefail

echo "Testing: bru info vs brew info"

test_formulae=("wget" "curl" "node" "python@3.13" "git")

for formula in "${test_formulae[@]}"; do
    echo "  Testing $formula..."

    # Get info from both
    bru_info=$(./target/release/bru info "$formula" 2>&1)
    brew_info=$(brew info "$formula" 2>&1)

    # Extract version
    bru_version=$(echo "$bru_info" | grep "Version:" | awk '{print $2}')
    brew_version=$(echo "$brew_info" | grep -E "^[a-z].*: stable" | awk '{print $3}')

    if [ "$bru_version" = "$brew_version" ]; then
        echo "    ✓ Version matches: $bru_version"
    else
        echo "    ✗ Version mismatch: bru=$bru_version brew=$brew_version"
        exit 1
    fi

    # Extract dependencies
    bru_deps=$(echo "$bru_info" | grep "Dependencies:" | cut -d: -f2 | tr ',' '\n' | sort)
    brew_deps=$(echo "$brew_info" | grep "==> Dependencies" -A 20 | tail -n +2 | head -1 | tr ' ' '\n' | sort)

    # Compare (allowing for minor formatting differences)
    if diff -w <(echo "$bru_deps") <(echo "$brew_deps") > /dev/null; then
        echo "    ✓ Dependencies match"
    else
        echo "    ⚠ Dependencies differ (might be formatting)"
    fi
done

echo "✅ All compatibility tests passed"
```

**Run**: `./tests/compatibility/run_all.sh`

## Test Formulae

### Tier 1: Simple (Use for quick tests)
- **`wget`** - Simple, few deps, good baseline
- **`curl`** - Common, well-tested
- **`jq`** - Small binary, minimal deps
- **`tree`** - Tiny, single file

### Tier 2: Medium (Use for comprehensive tests)
- **`node`** - Many runtime deps
- **`python@3.13`** - Complex dep tree
- **`git`** - Both runtime and build deps
- **`ripgrep`** - Rust formula (for dogfooding)

### Tier 3: Complex (Use for stress tests)
- **`ffmpeg`** - Huge dependency tree (~30+ deps)
- **`vim`** - Complex build options
- **`postgresql`** - Service management
- **`rust`** - Large, requires source build (Phase 3)

## CI/CD Pipeline

### GitHub Actions Workflow

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

env:
  RUST_BACKTRACE: 1

jobs:
  test:
    runs-on: macos-latest

    steps:
    - uses: actions/checkout@v4

    - name: Install Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        profile: minimal
        override: true
        components: rustfmt, clippy

    - name: Cache cargo registry
      uses: actions/cache@v3
      with:
        path: ~/.cargo/registry
        key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}

    - name: Cache cargo build
      uses: actions/cache@v3
      with:
        path: target
        key: ${{ runner.os }}-cargo-build-${{ hashFiles('**/Cargo.lock') }}

    - name: Check formatting
      run: cargo fmt --all -- --check

    - name: Clippy lints
      run: cargo clippy --all-targets --all-features -- -D warnings

    - name: Unit tests
      run: cargo test --lib

    - name: Integration tests
      run: cargo test --test '*'
      env:
        RUST_LOG: warn

    - name: Build release
      run: cargo build --release

    - name: Compatibility tests
      run: ./tests/compatibility/run_all.sh

    - name: Performance benchmarks
      run: ./benchmark.sh

    - name: Upload artifacts
      uses: actions/upload-artifact@v3
      with:
        name: bru-binary
        path: target/release/bru

  linux-test:
    runs-on: ubuntu-latest

    steps:
    - uses: actions/checkout@v4
    - uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        profile: minimal
        override: true

    # Install Homebrew on Linux
    - name: Install Homebrew
      run: |
        /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
        echo "/home/linuxbrew/.linuxbrew/bin" >> $GITHUB_PATH

    - name: Unit tests
      run: cargo test --lib

    - name: Integration tests
      run: cargo test --test '*'
```

## Test Coverage

**Target**: 80%+ coverage for critical paths

**Tools**:
- `cargo-tarpaulin` for coverage reports
- `cargo-mutants` for mutation testing (optional)

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Run coverage
cargo tarpaulin --out Html --output-dir coverage
```

## API Rate Limiting Strategy

**Problem**: Integration tests hit real Homebrew API

**Solution**: Smart caching

```rust
// tests/common/mod.rs
use std::sync::Arc;
use tokio::sync::Mutex;

lazy_static! {
    static ref API_CACHE: Arc<Mutex<HashMap<String, CachedResponse>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

pub async fn cached_api_call<F, T>(key: &str, f: F) -> Result<T>
where
    F: Future<Output = Result<T>>,
    T: Clone,
{
    let mut cache = API_CACHE.lock().await;

    if let Some(cached) = cache.get(key) {
        if cached.expires_at > SystemTime::now() {
            return Ok(cached.data.clone());
        }
    }

    let data = f.await?;
    cache.insert(key.to_string(), CachedResponse {
        data: data.clone(),
        expires_at: SystemTime::now() + Duration::from_secs(3600),
    });

    Ok(data)
}
```

**Usage**:
```rust
#[tokio::test]
async fn test_with_caching() {
    let api = BrewApi::new().unwrap();

    let result = cached_api_call("formula/wget", api.fetch_formula("wget")).await;

    // Subsequent calls use cache
}
```

## Phase-Specific Tests

### Phase 0 (Current): Read-Only Tests
- [x] API client tests
- [x] Search filtering tests
- [x] JSON parsing tests
- [x] Command output tests

### Phase 1: Dependency Resolution
- [ ] Recursive dependency resolution
- [ ] Circular dependency detection
- [ ] Version conflict handling
- [ ] Keg-only formula handling

### Phase 2: Installation
- [ ] Bottle download and extraction
- [ ] Symlink creation
- [ ] Install receipt generation
- [ ] Cellar compatibility
- [ ] Uninstall cleanup

### Phase 3: Source Builds
- [ ] Ruby interop tests
- [ ] Formula DSL execution
- [ ] Build environment setup
- [ ] Compiler toolchain tests

## Test Data Management

### Mock Data
For unit tests that don't need real API:

```rust
// tests/fixtures/formulae.json
{
  "name": "test-formula",
  "desc": "Test formula for unit tests",
  "homepage": "https://example.com",
  "versions": {
    "stable": "1.0.0",
    "bottle": true
  },
  "dependencies": ["dep1", "dep2"]
}
```

Load in tests:
```rust
fn load_fixture(name: &str) -> Formula {
    let json = include_str!(concat!("fixtures/", name, ".json"));
    serde_json::from_str(json).unwrap()
}
```

## Continuous Benchmarking

Track performance over time:

```yaml
# .github/workflows/benchmark.yml
name: Benchmark

on:
  push:
    branches: [ main ]

jobs:
  benchmark:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo build --release
      - run: ./benchmark.sh > benchmark-results.txt
      - name: Store benchmark result
        uses: benchmark-action/github-action-benchmark@v1
        with:
          tool: 'customSmallerIsBetter'
          output-file-path: benchmark-results.txt
```

## Manual Testing Checklist

Before each release:

- [ ] Test on clean macOS install
- [ ] Test on macOS with Homebrew already installed
- [ ] Test on Linux with Linuxbrew
- [ ] Test with slow network (simulate with `tc` or `pf`)
- [ ] Test with no network (offline mode)
- [ ] Test with 100+ formulae search results
- [ ] Test formulae with many dependencies (ffmpeg)
- [ ] Test keg-only formulae (openssl@3)
- [ ] Compare output format with `brew` command
- [ ] Verify colored output works on different terminals

## Test Maintenance

**Keep tests fast**:
- Unit tests: <100ms total
- Integration tests: <10s total
- Compatibility tests: <30s total

**Keep tests reliable**:
- Retry flaky network tests (3 attempts)
- Cache API responses in CI
- Use stable test formulae (not HEAD versions)

**Keep tests valuable**:
- Remove redundant tests
- Focus on critical paths
- Test user-facing behavior, not implementation

## Edge Case Findings (2025-10-21)

### Keg-Only Formulae

**Discovery**: bru does not capture or display `keg_only` status from the Homebrew API.

**Missing API fields** (src/api.rs:10-26):
- `keg_only: bool`
- `keg_only_reason: Option<KegOnlyReason>`

**Known keg-only formulae**:
- `sqlite` - keg-only (`:provided_by_macos`)
- `readline` - keg-only (`:shadowed_by_macos`)
- `ncurses` - keg-only (`:provided_by_macos`)
- Many others (check API: `curl -s 'https://formulae.brew.sh/api/formula/<name>.json' | jq '.keg_only'`)

**Critical safety issue**:
- Uninstalling keg-only formulae can break system dependencies
- Example: removing `ncurses` breaks `zsh` (dynamic library dependency)
- **DO NOT test keg-only uninstalls locally** - use CI with throwaway environments

**Testing approach**:
- ✅ Use `--dry-run` mode to test install/upgrade logic
- ✅ Test dependency resolution with already-installed keg-only formulae
- ✅ Verify keg-only formulae appear in dependency trees correctly
- ❌ Do NOT uninstall keg-only formulae locally for testing
- 🤖 Use CI for destructive keg-only tests (install/uninstall/upgrade)

**API verification**:
```bash
# Check if a formula is keg-only
curl -s 'https://formulae.brew.sh/api/formula/sqlite.json' | jq '{name, keg_only, reason: .keg_only_reason.reason}'
# Output: {"name": "sqlite", "keg_only": true, "reason": ":provided_by_macos"}
```

**Future work**:
- Add `keg_only` fields to Formula struct
- Display keg-only status in `bru info` output
- Warn users when installing/upgrading keg-only formulae
- Add CI tests for keg-only handling

### Error Handling Inconsistencies

**Discovery**: Commands handle API errors inconsistently.

**Issue**: HTTP 404 responses return HTML, but some commands try to parse as JSON (src/api.rs:135-138).

**Affected commands**:
- `install` - Shows ugly stack backtrace for non-existent formulae
- `upgrade` - Shows ugly stack backtrace for non-existent formulae
- `deps` - Shows ugly stack backtrace for non-existent formulae
- `info` - Shows clean error message ✓ (catches errors at command level)
- `search` - Handles empty queries well ✓

**Example error**:
```bash
$ bru install nonexistent-12345
Error: API request failed: error decoding response body
Caused by:
    0: error decoding response body
    1: expected value at line 1 column 1
Stack backtrace:
   0: __mh_execute_header
   ...
```

**Root cause**:
```rust
// src/api.rs:135-138
pub async fn fetch_formula(&self, name: &str) -> Result<Formula> {
    let url = format!("{}/formula/{}.json", HOMEBREW_API_BASE, name);
    let formula = self.client.get(&url).send().await?.json().await?;
    Ok(formula)
}
```

Problem: `.send().await?` doesn't check HTTP status before `.json().await?`

**Fix strategy**:
Either:
1. Check HTTP status in API layer before parsing JSON
2. Catch errors at command layer like `info` does (lines 163-173)

**Testing**:
```bash
# Clean error (info command)
$ bru info nonexistent-12345
❌ No formula or cask found for 'nonexistent-12345'

# Ugly error (install/upgrade/deps)
$ bru install nonexistent-12345
Error: API request failed... [stack trace]
```

### Performance Testing Results

**Search performance** (2025-10-21):
```bash
$ time bru search rust
Found 145 results
Real: 0.050s (user: 0.04s, sys: 0.01s)

$ time brew search rust
Found ~40 results
Real: 1.030s (user: 0.78s, sys: 0.11s)

Speedup: 20x faster
```

**Key advantages**:
- Parallel formulae + casks fetch (tokio::join!)
- Compiled binary (no Ruby interpreter startup)
- Efficient filtering with spawn_blocking

### Dependency Resolution Testing

**Tested formulae**:
- `node` - 12 runtime deps, 2 build deps - ✓ All resolved correctly
- `python@3.13` - 4 runtime deps, 1 build dep - ✓ All resolved correctly
- `ffmpeg` - 44 runtime deps, 1 build dep - ✓ All resolved correctly
- `vim` - 6 runtime deps - ✓ Correctly skipped (already installed)

**Multi-formula install**:
```bash
$ bru install --dry-run tree htop wget curl ripgrep fd bat fzf
→ 2 formulae to install: htop, tree
✓ Correctly identified 6 already installed
```

**Transitive dependencies**:
- ffmpeg → aom → jpeg-xl, libvmaf ✓
- Recursive resolution working correctly

**Edge cases**:
- ~~Non-existent formula in list causes entire install to fail~~ FIXED (2025-10-21)
  - Now skips invalid formulae with warnings, continues with valid ones
  - Shows ⚠ for skipped formulae, ❌ if all invalid
- No partial progress reporting during resolution (future enhancement)
- No indication of keg-only status in dependency tree (cosmetic)

## Resources

- Homebrew's test suite: https://github.com/Homebrew/brew/tree/master/Library/Homebrew/test
- cargo test docs: https://doc.rust-lang.org/book/ch11-00-testing.html
- Integration testing: https://doc.rust-lang.org/rust-by-example/testing/integration_testing.html

---

**Status**: Phase 0 complete, basic test structure pending
**Next**: Add unit tests for API module, create integration test suite
