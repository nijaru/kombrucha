# Benchmarking & Testing Strategy

## Current State Assessment

### Testing Infrastructure ✅

**Unit Tests**: 76 tests passing
- Location: `tests/unit_tests.rs`
- Coverage: Core logic, version comparison, tap handling
- Status: Good

**Integration Tests**: Limited
- `tests/regression_tests.rs` - 14 tests
- `tests/cleanup_tests.rs` - Cleanup logic
- `tests/relocation_tests.rs` - Bottle relocation
- `tests/symlink_integration_tests.rs` - Symlink operations
- `tests/parallel_tests.rs` - Parallel operations

**Test Fixtures**: Present
- `tests/fixtures/formulae/` - Formula JSON samples
- `tests/fixtures/casks/` - Cask JSON samples
- `tests/fixtures/receipts/` - Receipt samples

**Gaps Identified**: See `TESTING_IMPROVEMENTS.md`
- No receipt validation tests
- No command sequence tests (install → upgrade → autoremove)
- No binary execution tests
- No E2E tests with actual bottles

### Benchmark Infrastructure ⚠️

**Existing**:
- `scripts/benchmark.sh` - Basic shell script
  - Only tests `search` command
  - 3 runs, calculates average
  - Compares brew vs bru

**Missing**:
- ❌ Rust criterion benchmarks
- ❌ Standardized benchmark suite
- ❌ CI benchmark tracking
- ❌ Performance regression detection
- ❌ Comprehensive command coverage

### Profiling Infrastructure ✅

**Tools Available**:
- ✅ `cargo-flamegraph` (installed)
- ✅ `samply` (installed)
- ✅ Debug symbols enabled in release (`Cargo.toml:74`)

**Documentation**:
- ✅ `ai/PERFORMANCE_PROFILE.md` - Initial flamegraph analysis
- ✅ `ai/research/performance-analysis.md` - Research findings

**Status**: Set up, used once (October 2025), not regularly tracked

## Recommendations

### Priority 0: Standardized Benchmark Suite

Create comprehensive benchmarks using **criterion.rs** (Rust standard):

```toml
# Cargo.toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "core_operations"
harness = false
```

**Benchmark Coverage**:

1. **Command Operations** (`benches/commands.rs`)
   - `search` - Already in scripts/benchmark.sh
   - `info` - Single formula lookup
   - `deps` - Dependency resolution
   - `outdated` - Version checking
   - `list` - Installed packages enumeration
   - `autoremove` - Dependency graph traversal

2. **Install/Upgrade Operations** (`benches/install.rs`)
   - Dependency resolution (100 formulae)
   - Receipt parsing (100 receipts)
   - Symlink creation (1000 files)
   - Bottle relocation (Mach-O processing)

3. **API Operations** (`benches/api.rs`)
   - Formula fetch (single)
   - Formula fetch (batch of 100, parallel)
   - Cask fetch
   - Cache hit vs miss performance

4. **Data Structures** (`benches/data_structures.rs`)
   - Version comparison
   - Dependency graph construction
   - Receipt parsing

**Example Criterion Benchmark**:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use kombrucha::commands;

fn bench_autoremove(c: &mut Criterion) {
    c.bench_function("autoremove dry-run", |b| {
        b.iter(|| {
            commands::autoremove(black_box(true)).unwrap()
        })
    });
}

fn bench_search(c: &mut Criterion) {
    c.bench_function("search python", |b| {
        b.iter(|| {
            // Benchmark search operation
        })
    });
}

criterion_group!(benches, bench_autoremove, bench_search);
criterion_main!(benches);
```

**Benefits**:
- Statistical rigor (outlier detection, confidence intervals)
- HTML reports with graphs
- Regression detection
- CI integration
- Micro-benchmark specific functions

### Priority 1: E2E Benchmark Script

Expand `scripts/benchmark.sh` to cover all commands:

```bash
#!/usr/bin/env bash
# Comprehensive bru vs brew benchmark

commands=(
    "search:python"
    "info:wget"
    "deps:ffmpeg"
    "outdated"
    "list"
    "upgrade:--dry-run"
    "autoremove:--dry-run"
)

# Run each command 5 times, report median + stddev
# Output: Markdown table for README
```

**Why shell script in addition to criterion?**
- Tests against actual `brew` binary (comparison)
- End-to-end timing including process startup
- Easy to run manually
- Results go directly to README benchmarks

### Priority 2: CI Benchmark Tracking

**GitHub Actions Workflow** (`.github/workflows/benchmark.yml`):

```yaml
name: Benchmark

on:
  push:
    branches: [main]
  pull_request:

jobs:
  benchmark:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable

      - name: Run criterion benchmarks
        run: cargo bench --bench core_operations

      - name: Store benchmark results
        uses: benchmark-action/github-action-benchmark@v1
        with:
          tool: 'cargo'
          output-file-path: target/criterion/*/estimates.json

      - name: Upload results
        uses: actions/upload-artifact@v4
        with:
          name: benchmark-results
          path: target/criterion/
```

**Benefits**:
- Detect performance regressions in PRs
- Track performance over time
- Visualize trends (github-action-benchmark generates site)

### Priority 3: Regular Profiling

**Schedule profiling runs** to catch performance regressions:

**Commands to Profile**:

1. **upgrade --dry-run** (already done Oct 2025)
   ```bash
   cargo flamegraph --bin=bru -- upgrade --dry-run -q
   ```

2. **install with dependencies**
   ```bash
   # Install formula with many deps (ffmpeg has ~40)
   cargo flamegraph --bin=bru -- install ffmpeg --dry-run
   ```

3. **autoremove** (now <20ms, verify stays fast)
   ```bash
   cargo flamegraph --bin=bru -- autoremove --dry-run
   ```

4. **Memory profiling** (check for leaks)
   ```bash
   # macOS Instruments
   cargo instruments -t Allocations --bin=bru -- upgrade --dry-run

   # Or samply with heap tracking
   samply record --profile time-and-heap ./target/release/bru upgrade --dry-run
   ```

**Store profiles in**: `ai/profiles/YYYY-MM-DD-command.svg`

**Review Schedule**: Monthly or after major changes

### Priority 4: Performance Regression Tests

Add to `tests/performance_tests.rs`:

```rust
#[test]
fn test_autoremove_performance() {
    let start = std::time::Instant::now();
    autoremove(true).unwrap();
    let duration = start.elapsed();

    // Should be <100ms (currently ~20ms)
    assert!(duration < std::time::Duration::from_millis(100),
        "autoremove took {:?}, expected <100ms", duration);
}

#[test]
fn test_upgrade_dry_run_performance() {
    let start = std::time::Instant::now();
    // Run upgrade --dry-run
    let duration = start.elapsed();

    // Should be <2s for typical case
    assert!(duration < std::time::Duration::from_secs(2),
        "upgrade --dry-run took {:?}, expected <2s", duration);
}
```

**Benefits**:
- Catch major performance regressions in CI
- Complement criterion microbenchmarks
- Fast to run (no statistical sampling)

## Implementation Plan

### Phase 1 (This Week)
- [x] Assess current state
- [ ] Set up criterion benchmarks
  - [ ] Add criterion to Cargo.toml
  - [ ] Create `benches/core_operations.rs`
  - [ ] Benchmark: autoremove, search, info, deps
- [ ] Expand `scripts/benchmark.sh`
  - [ ] Add: info, deps, outdated, list, upgrade, autoremove
  - [ ] Output markdown table

### Phase 2 (Next Week)
- [ ] Add performance regression tests
  - [ ] `tests/performance_tests.rs`
  - [ ] Critical: autoremove <100ms
  - [ ] Critical: upgrade --dry-run <2s
- [ ] CI benchmark tracking
  - [ ] `.github/workflows/benchmark.yml`
  - [ ] Store results as artifacts

### Phase 3 (Ongoing)
- [ ] Monthly profiling runs
  - [ ] Flamegraph: upgrade, install, autoremove
  - [ ] Memory: Instruments/samply
  - [ ] Store in `ai/profiles/`
- [ ] Update README with latest benchmarks
- [ ] Track performance trends over releases

## Success Metrics

**Before** (Current):
- 76 unit tests ✅
- 1 benchmark script (search only) ⚠️
- Profiling: Ad-hoc, last run Oct 2025 ⚠️
- No regression detection ❌

**Target** (Phase 3 Complete):
- 76+ unit tests ✅
- 10+ integration tests (from TESTING_IMPROVEMENTS.md)
- 15+ criterion benchmarks covering all commands
- Comprehensive E2E benchmark script (8+ commands)
- CI benchmark tracking with regression alerts
- Monthly profiling schedule
- Performance regression tests in CI

## Profiling Command Reference

### CPU Profiling

```bash
# Flamegraph (interactive SVG)
cargo flamegraph --bin=bru -- upgrade --dry-run -q

# samply (Firefox Profiler format)
samply record ./target/release/bru upgrade --dry-run
```

### Memory Profiling

```bash
# macOS Instruments (GUI required)
cargo instruments -t Allocations --bin=bru -- upgrade --dry-run

# samply with heap tracking
samply record --profile time-and-heap ./target/release/bru upgrade --dry-run

# Valgrind (Linux only)
valgrind --tool=massif ./target/release/bru upgrade --dry-run
```

### Analyzing Results

**Flamegraph**:
- Wide bars = high CPU time functions
- Look for: JSON parsing, allocations, sync points

**samply**:
- Upload to profiler.firefox.com
- Analyze call stacks, thread activity

**Instruments**:
- View allocations over time
- Detect memory leaks
- Track high-watermark memory usage

## References

- **criterion.rs**: https://github.com/bheisler/criterion.rs
- **cargo-flamegraph**: https://github.com/flamegraph-rs/flamegraph
- **samply**: https://github.com/mstange/samply
- **github-action-benchmark**: https://github.com/benchmark-action/github-action-benchmark
- **Rust Performance Book**: https://nnethercote.github.io/perf-book/

## Related Documents

- `TESTING_IMPROVEMENTS.md` - Testing gaps and strategy
- `PERFORMANCE_PROFILE.md` - Initial flamegraph analysis (Oct 2025)
- `ai/research/performance-analysis.md` - Performance research
- `README.md` - Current benchmarks section
