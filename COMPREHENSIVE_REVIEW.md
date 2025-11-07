# Comprehensive Code Review - Autoremove Bug & Performance Issues

**Date:** 2025-11-06
**Scope:** Recent autoremove fix (6c8d8c0) and related code

## 🔴 CRITICAL BUG FOUND: Root Cause Analysis

### The Real Problem

The autoremove bug was NOT caused by stale receipts. The root cause is in **upgrade** and **reinstall** commands:

**Location:** `src/commands.rs:1890-1894` and `src/commands.rs:2097-2101`

```rust
// WRONG - only includes the formula being upgraded, not its dependencies!
let runtime_deps = build_runtime_deps(&formula.dependencies, &{
    let mut map = HashMap::new();
    map.insert(formula.name.clone(), formula.clone());
    map
});
```

### What Happens

1. User runs `bru upgrade curl` (curl 8.16.0 → 8.17.0)
2. curl has dependencies: `["libnghttp3", "rtmpdump", "libidn2", ...]`
3. `build_runtime_deps()` tries to look up each dependency in `all_formulae`
4. But `all_formulae` only contains `{curl: Formula}` ❌
5. **Result:** All lookups return None → **EMPTY runtime_dependencies in receipt!**
6. Receipt written: `curl/8.17.0/INSTALL_RECEIPT.json` with `runtime_dependencies: []`
7. User runs `bru autoremove`
8. Autoremove sees: curl (on_request) has NO dependencies
9. libnghttp3 (installed as dep) is not in required set
10. **Incorrectly removes libnghttp3** → curl breaks

### Proof

**Install command** (src/commands.rs:1280, 1430):
```rust
// ✅ CORRECT - resolves ALL dependencies
let (all_formulae, dep_order) = resolve_dependencies(api, &valid_formulae).await?;
// ...
let runtime_deps = build_runtime_deps(&formula.dependencies, &all_formulae);
```

**Upgrade command** (src/commands.rs:1890-1894):
```rust
// ❌ WRONG - only single formula
let runtime_deps = build_runtime_deps(&formula.dependencies, &{
    let mut map = HashMap::new();
    map.insert(formula.name.clone(), formula.clone());
    map
});
```

## 📊 Current "Fix" Analysis

### What the API-based fix did (commit 6c8d8c0)

Changed autoremove to fetch formula data from API instead of reading receipts:

```rust
// Fetch all formulae in this batch in parallel
let formula_futures: Vec<_> = batch.iter().map(|name| api.fetch_formula(name)).collect();
let formula_results = futures::future::join_all(formula_futures).await;

match result {
    Ok(formula) => {
        // Use fresh API data (not broken receipts)
        for dep in &formula.dependencies { ... }
    }
    Err(_) => {
        // Fallback to receipt if API fails
        if let Some(pkg) = all_packages.iter().find(|p| p.name == *name) { ... }
    }
}
```

**This works BUT:**
- ❌ Masks the root cause (receipts are still broken after upgrade)
- ❌ 1-3 seconds of network calls (vs <10ms with receipts)
- ❌ Doesn't work offline
- ❌ Unnecessary file descriptor/connection overhead
- ❌ Doesn't match Homebrew's behavior

### What Homebrew Actually Does

Source: `Library/Homebrew/cleanup.rb:autoremove`

```ruby
formulae = Formula.installed  # Read local receipts only
removable_formulae = Utils::Autoremove.removable_formulae(formulae, casks)
# Uses installed_runtime_formula_dependencies from receipts
```

**Homebrew uses receipts-only - ZERO network calls!**

## ✅ PROPER FIX

### Fix 1: Upgrade Command (P0 - Critical)

**File:** `src/commands.rs:1830-1894`

```rust
// Phase 2: Download all bottles in parallel
println!("Downloading {} bottles...", candidates.len());
let formulae: Vec<_> = candidates.iter().map(|c| c.formula.clone()).collect();
let downloaded = download::download_bottles(api, &formulae).await?;
let download_map: HashMap<_, _> = downloaded.into_iter().collect();

// ✅ NEW: Resolve dependencies for ALL candidates to build complete formula map
let candidate_names: Vec<String> = candidates.iter().map(|c| c.name.clone()).collect();
let (all_formulae, _) = resolve_dependencies(api, &candidate_names).await?;

// Phase 3: Install sequentially
for candidate in &candidates {
    // ... extraction, linking, etc ...

    // ✅ FIXED: Pass complete all_formulae map
    let runtime_deps = build_runtime_deps(&formula.dependencies, &all_formulae);

    // ... write receipt ...
}
```

### Fix 2: Reinstall Command (P0 - Critical)

**File:** `src/commands.rs:2054-2102`

Same fix - resolve dependencies before generating receipt:

```rust
pub async fn reinstall(api: &BrewApi, names: &[String], cask: bool) -> Result<()> {
    // ... validation ...

    // ✅ NEW: Resolve dependencies to build complete formula map
    let (all_formulae, _) = resolve_dependencies(api, names).await?;

    for formula_name in names {
        let formula = all_formulae.get(formula_name).unwrap();

        // ... extraction, linking, etc ...

        // ✅ FIXED: Pass complete all_formulae map
        let runtime_deps = build_runtime_deps(&formula.dependencies, &all_formulae);

        // ... write receipt ...
    }
}
```

### Fix 3: Autoremove - Revert to Receipts-Only (P0 - Critical)

**File:** `src/commands.rs:2240-2306`

```rust
pub fn autoremove(dry_run: bool) -> Result<()> {  // Remove async, remove api param
    if dry_run {
        println!("Dry run - no packages will be removed");
    } else {
        println!("Removing unused dependencies...");
    }

    let all_packages = cellar::list_installed()?;

    // Build set of packages installed on request
    let mut on_request: HashSet<String> = all_packages
        .iter()
        .filter(|p| p.installed_on_request())
        .map(|p| p.name.clone())
        .collect();

    // Build set of all dependencies required by on_request packages
    let mut required = HashSet::new();
    let mut to_check: VecDeque<String> = on_request.iter().cloned().collect();
    let mut checked = HashSet::new();

    // ✅ FIXED: Use receipts only - NO API calls
    while let Some(name) = to_check.pop_front() {
        if !checked.insert(name.clone()) {
            continue;  // Already processed
        }

        // Find package and add its dependencies from receipt
        if let Some(pkg) = all_packages.iter().find(|p| p.name == name) {
            for dep in pkg.runtime_dependencies() {
                required.insert(dep.full_name.clone());
                to_check.push_back(dep.full_name.clone());
            }
        }
    }

    // Find unrequired packages
    let to_remove: Vec<_> = all_packages
        .iter()
        .filter(|p| !p.installed_on_request() && !required.contains(&p.name))
        .collect();

    // ... rest of removal logic ...
}
```

**Also update:** `src/main.rs:1056`
```rust
Some(Commands::Autoremove { dry_run }) => {
    commands::autoremove(dry_run)?;  // Remove .await, remove &api
}
```

## 🚀 PERFORMANCE COMPARISON

| Approach | Speed | Network | Offline | Correctness |
|----------|-------|---------|---------|-------------|
| **Current (API batched)** | 1-3s | Required | No | ✅ Fresh data |
| **Proposed (receipts)** | <10ms | None | Yes | ✅ If receipts correct |
| **Homebrew** | <10ms | None | Yes | ✅ Receipts |

**Expected improvement:** **100-300x faster** autoremove

## 🔍 SYMLINK PARALLELIZATION REVIEW

### Current Implementation

**File:** `src/symlink.rs:57-98`

```rust
// Collect all operations
let mut operations = Vec::new();
collect_link_operations(source, target, cellar_root, &mut operations)?;

// Create directories (sequential)
let dir_results: Vec<Result<()>> = operations
    .iter()
    .filter_map(|op| { ... })
    .collect();

// Create symlinks (parallel with rayon)
let symlink_results: Vec<Result<PathBuf>> = operations
    .into_par_iter()  // ⚠️ Unbounded parallelism (relies on rayon thread pool)
    .filter_map(|op| { ... })
    .collect();
```

### Issues

1. **Memory overhead:** Collects ALL operations before processing
   - llvm: 9,195 files → Vec with 9,195 entries in memory

2. **Likely I/O bound:** Symlink creation is ~1.2ms (filesystem metadata ops)
   - Parallelizing I/O operations rarely helps (filesystem may serialize anyway)
   - rayon thread pool (CPU count) vs 9,195 operations → most wait in queue

3. **Complexity:** Two-pass approach (collect then process) vs streaming

4. **File descriptors:** Each symlink operation opens 2-4 FDs temporarily
   - rayon thread pool ~16 threads × 4 FDs = 64 FDs concurrent
   - Should be safe but unnecessary risk

### Recommendation: REMOVE PARALLELIZATION

**Rationale:**
- Symlinks are I/O bound, not CPU bound
- Sequential: ~11s for llvm (9,195 files) - acceptable
- Parallelization adds complexity with marginal/zero benefit
- Need benchmarks to prove parallelization actually helps

**Proposed:** Streaming approach (like WalkDir)

```rust
fn link_directory_streaming(
    source: &Path,
    target: &Path,
    cellar_root: &Path,
) -> Result<Vec<PathBuf>> {
    let mut linked = Vec::new();

    for entry in WalkDir::new(source) {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(source)?;
        let target_path = target.join(relative);

        if path.is_dir() {
            fs::create_dir_all(&target_path)?;
        } else {
            create_relative_symlink(path, &target_path, cellar_root)?;
            linked.push(target_path);
        }
    }

    Ok(linked)
}
```

**Benefits:**
- ✅ Zero memory overhead (streaming)
- ✅ Simpler code (single pass)
- ✅ No file descriptor concerns
- ✅ Likely same performance (I/O bound)
- ✅ Easier to reason about

## 📋 OTHER ISSUES FOUND

### 1. Unnecessary API Calls

**Pattern:** Many commands fetch formula/cask data when they could read from:
- Local receipts (for installed packages)
- Local cache (`~/Library/Caches/Homebrew/api/formula.jws.json`)

**Examples:**
- `outdated` (line 1032): Fetches API for each installed package
- `list` (line 937): Could read from receipts
- `leaves` (line 392): Could optimize

### 2. Missing Dependency Resolution in Commands

**Commands that should call `resolve_dependencies`:**
- `reinstall` - currently doesn't resolve deps ❌
- `upgrade` - currently doesn't resolve deps ❌

### 3. Receipt Reading Pattern

Multiple commands read receipts with similar code - should extract helper:

```rust
pub fn get_installed_package_with_receipt(name: &str) -> Result<(InstalledPackage, InstallReceipt)> {
    // ... DRY helper ...
}
```

## 🎯 ACTION PLAN

### Phase 1: Critical Fixes (P0)
1. ✅ Fix upgrade command to resolve dependencies
2. ✅ Fix reinstall command to resolve dependencies
3. ✅ Revert autoremove to receipt-based (remove async/API calls)
4. ✅ Update main.rs autoremove invocation
5. ✅ Test: upgrade → autoremove sequence

### Phase 2: Simplification (P1)
6. ✅ Remove symlink parallelization (revert to streaming)
7. ✅ Benchmark before/after to verify no regression
8. ✅ Remove rayon dependency if not used elsewhere

### Phase 3: Optimization (P2)
9. Audit unnecessary API calls across all commands
10. Add local cache usage for offline support
11. Extract common receipt-reading patterns

### Phase 4: Testing (P0)
12. Add integration test: upgrade → autoremove
13. Add test: verify receipts have correct runtime_dependencies
14. Add benchmark: autoremove performance

## 📈 EXPECTED RESULTS

### Performance
- autoremove: 1-3s → <10ms (**100-300x faster**)
- Works offline ✅
- Zero network overhead ✅

### Correctness
- Receipts will have correct dependencies after upgrade ✅
- autoremove won't incorrectly remove required packages ✅
- Matches Homebrew behavior exactly ✅

### Code Quality
- Simpler autoremove (no async, no batching, no API)
- Simpler symlink code (streaming vs two-pass)
- Better separation of concerns

## 🔗 RELATED COMMITS

- `6c8d8c0` - Current autoremove API fix (workaround)
- `1428915` - Bug documentation
- `54fe421` - optlink implementation

## ✅ VERIFICATION STEPS

After fixes:

```bash
# 1. Verify receipts are correct
bru install wget
cat /opt/homebrew/Cellar/wget/*/INSTALL_RECEIPT.json | jq .runtime_dependencies

# 2. Verify upgrade preserves deps
bru upgrade curl
cat /opt/homebrew/Cellar/curl/*/INSTALL_RECEIPT.json | jq .runtime_dependencies

# 3. Verify autoremove doesn't remove required deps
bru upgrade llvm lld curl
bru autoremove --dry-run  # Should NOT list z3, libnghttp3, rtmpdump

# 4. Benchmark autoremove
time bru autoremove --dry-run  # Should be <100ms

# 5. Test offline
# Disconnect network
bru autoremove --dry-run  # Should still work
```

## 📝 NEXT STEPS

1. Implement Phase 1 fixes (critical)
2. Run comprehensive tests
3. Benchmark performance
4. Update STATUS.md with findings
5. Consider Phase 2 (simplification) based on benchmarks
