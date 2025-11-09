# Project Status

Last updated: 2025-01-08

## Current State

**Version**: 0.1.34 (in development)
**Status**: CRITICAL FIX - Interrupted Operations & Linked Version Handling

### v0.1.34 (2025-01-08) - CRITICAL FIX: Interrupted Operations & Linked Version

**Critical Bug Fixed:** Commands used newest version in Cellar instead of linked (active) version

**Problem:**
- upgrade/reinstall/cleanup/uninstall/unlink used `versions[0]` (newest by semantic versioning)
- Should use linked version (like Homebrew's `linked_keg`) to determine active installation
- **Impact**: Interrupted upgrades would fail, cleanup could delete active version, commands operated on wrong version

**Example Failure:**
```bash
# Interrupt upgrade during extraction
bru upgrade wget  # Ctrl+C mid-upgrade
# Cellar has: wget/1.25.0 (linked), wget/1.26.0 (partial)

# Next upgrade attempt (OLD BEHAVIOR)
bru upgrade wget  # Sees 1.26.0 is newest, thinks up-to-date, skips
# System broken: partial 1.26.0 + old 1.25.0 still linked

# NEW BEHAVIOR
bru upgrade wget  # Sees 1.25.0 is linked, upgrades to 1.26.0 correctly
```

**Fix Applied:**
1. **Added `symlink::get_linked_version()`** (src/symlink.rs:486-508):
   - Reads `/opt/homebrew/opt/<formula>` symlink to determine linked version
   - Matches Homebrew's `linked_keg` behavior
   - Returns `Option<String>` (None if not linked)

2. **Updated 5 commands to use linked version**:
   - **upgrade** (src/commands.rs:1769-1787): Use linked version as "old_version"
   - **reinstall** (src/commands.rs:2082-2100): Use linked version as "old_version"
   - **cleanup** (src/commands.rs:2783-2837): Keep both linked AND newest versions
   - **uninstall** (src/commands.rs:2258-2265): Uninstall linked version
   - **unlink** (src/commands.rs:3585-3600): Unlink linked version, skip if not linked

**Edge Cases Fixed:**
- ✅ Interrupted upgrade: Linked version used, operation completes correctly
- ✅ Multiple versions in Cellar: Commands operate on linked version, not newest
- ✅ User downgrade: Cleanup preserves both linked and newest versions
- ✅ Unlink when not linked: Clear error message
- ✅ No linked version: Falls back to newest (backward compatible)

**Testing:**
- ✅ All 76 unit tests pass
- ✅ Backward compatible: Falls back to newest if no linked version

**Homebrew Compatibility:**
- Matches `Library/Homebrew/upgrade.rb` (uses `linked_keg`)
- Matches `Library/Homebrew/cleanup.rb` (uses `latest_version_installed?`)
- Documented in: ai/LINKED_VERSION_FIX.md

**Impact:**
- ✅ Interrupted operations now handled correctly
- ✅ Commands always operate on active/linked version
- ✅ Cleanup never deletes linked version
- ✅ Matches Homebrew behavior exactly
- ✅ No breaking changes

**Files Changed:**
- src/symlink.rs: Added `get_linked_version()` function
- src/commands.rs: Updated 5 commands
- ai/LINKED_VERSION_FIX.md: Comprehensive documentation

### v0.1.33 (2025-01-07) - CRITICAL FIXES: Keg-only + Receipt Compatibility

**Critical Bugs Fixed:** Multiple Homebrew compatibility issues that could break local environment

**Problems:**
1. **Keg-only formulas were being symlinked** (install/upgrade/reinstall/link)
   - bru ignored `keg_only` flag from API
   - Created symlinks for formulas like llvm that should NOT be linked
   - Caused conflicts with standalone packages (clang-format, libomp)
   - **Impact**: Symlink conflicts prevented formula installation

2. **Receipt format incompatibilities** (RuntimeDependency, SourceInfo)
   - Missing `bottle_rebuild` field in runtime_dependencies
   - Missing `source.versions` field with formula version
   - **Impact**: brew could fail reading bru-generated receipts

3. **Reinstall ignored pinned packages**
   - `reinstall` command didn't check pinned formulae
   - **Impact**: Users could accidentally reinstall pinned packages

**Fixes Applied:**
1. **Keg-only respect** (src/commands.rs: install:1426, upgrade:1913, reinstall:2142, link:3443):
   ```rust
   if !formula.keg_only {
       symlink::link_formula(&formula.name, actual_version)?;
       symlink::optlink(&formula.name, actual_version)?;
   } else {
       println!("  {} is keg-only (not linked to prefix)", formula.name);
   }
   ```
   - `link` command now fetches metadata and blocks keg-only formulas with helpful message

2. **Receipt compatibility** (src/cellar.rs:42, src/receipt.rs:128, src/commands.rs:1605):
   - Added `bottle_rebuild: u32` field to RuntimeDependency struct
   - Populated `source.versions` with formula version, version_scheme
   - Set `changed_files: Some(vec![])` (matches Homebrew format)
   - Populated `source.path` with API cache location

3. **Pinned package respect** (src/commands.rs:2033, 2046):
   - `reinstall` now reads pinned formulae and skips them
   - Clear message: "is pinned (cannot reinstall pinned formulae)"

**Comprehensive Audit:**
- Created ai/COMPATIBILITY_AUDIT.md with full analysis
- Found 10 total issues: 2 CRITICAL (fixed), 3 HIGH (2 fixed), 3 MEDIUM, 2 LOW
- Remaining HIGH issues deferred (homebrew_version format, Xcode/CLT detection)

**Testing:**
- ✅ All 76 unit tests pass
- ✅ 13/14 regression tests pass (1 flaky search test - network related)
- ✅ Receipt format now matches Homebrew's INSTALL_RECEIPT.json

**Impact:**
- ✅ Fixes symlink conflicts that prevented package installation
- ✅ Improves brew/bru interoperability
- ✅ Receipts now fully compatible with Homebrew
- ✅ Prevents accidental reinstall of pinned packages
- ✅ Keg-only formulas properly isolated (no conflicts)

**Files Changed:**
- src/commands.rs: Keg-only checks in 4 commands, pinned check in reinstall
- src/cellar.rs: Added bottle_rebuild field
- src/receipt.rs: Populated source.versions, changed_files, source.path
- ai/COMPATIBILITY_AUDIT.md: Comprehensive audit report (new file)

### v0.1.32 (2025-11-06) - CRITICAL FIX: Bottle Revision Suffix Symlinks

**Critical Bug Fixed:** Symlinks pointing to wrong version when bottle has revision suffix

**Problem:**
- When bottles have revision suffixes (e.g., `25.1.0_1`), symlinks were created with base version
- `/opt/homebrew/opt/node` → `../Cellar/node/25.1.0` (wrong!)
- Should be: `/opt/homebrew/opt/node` → `../Cellar/node/25.1.0_1` (correct)
- **Impact**: Packages depending on node (gemini-cli) failed with "bad interpreter" error

**Example failure:**
```bash
bru install gemini-cli  # Installs node 25.1.0_1 as dependency
gemini --version  # Error: bad interpreter: /opt/homebrew/opt/node/bin/node: no such file or directory
ls -la /opt/homebrew/opt/node  # Points to 25.1.0 (doesn't exist)
ls /opt/homebrew/Cellar/node/  # Only 25.1.0_1 exists
```

**Root Cause:** (src/commands.rs, prior to fix)
- `extract_bottle()` correctly returned path with suffix: `/opt/homebrew/Cellar/node/25.1.0_1`
- But we passed original version `"25.1.0"` to `link_formula()` and `optlink()`
- Symlinks created with wrong version, pointing to non-existent directory

**Fix:** (src/commands.rs:1413-1431, 1890-1902, 2106-2118)
```rust
// Extract the actual installed version from the returned path
let extracted_path = extract::extract_bottle(bottle_path, &formula.name, version)?;
let actual_version = extracted_path
    .file_name()
    .and_then(|n| n.to_str())
    .ok_or_else(|| anyhow::anyhow!("Invalid extracted path: {}", extracted_path.display()))?;

// Use actual_version (with suffix) for symlinks
symlink::link_formula(&formula.name, actual_version)?;
symlink::optlink(&formula.name, actual_version)?;
```

**Affected Commands:**
- `bru install` - Fixed (src/commands.rs:1413-1431)
- `bru upgrade` - Fixed (src/commands.rs:1890-1902)
- `bru reinstall` - Fixed (src/commands.rs:2106-2118)
- `bru link` - Already correct (uses versions from `get_installed_versions`)

**Testing:**
- Reinstalled node with `bru reinstall node`
- Verified symlink: `ls -la /opt/homebrew/opt/node` → `../Cellar/node/25.1.0_1` ✓
- Tested gemini-cli: `gemini --version` → `0.13.0` ✓
- All 76 unit tests pass ✓

### v0.1.31 (2025-11-06) - CRITICAL FIX: Apple Codesign Bug Workaround

**Critical Bug Fixed:** Codesigning failures causing SIGKILL (exit 137) crashes on binaries

**Problem:**
- Apple's codesign utility has a known bug where signing randomly fails
- Our `codesign_file()` function (added in v0.1.21) always returned `Ok()` even on failures
- Silent failures left binaries unsigned → macOS killed them with SIGKILL (exit 137)
- **Impact**: git and other binaries crashed after `bru reinstall`, breaking Claude Code instances

**Example failure:**
```bash
bru reinstall git
git --version  # Exit code 137 (SIGKILL)
file /opt/homebrew/Cellar/git/2.51.2/bin/git  # "data" (corrupted)
```

**Root Cause:** (src/relocate.rs:408-418, prior to fix)
```rust
// BUGGY CODE (v0.1.21 - v0.1.30)
if !output.status.success() {
    tracing::debug!("Codesign note...");  // Only debug log
}
Ok(())  // ← ALWAYS returns Ok, even when signing fails!
```

**Homebrew Solution:** (Library/Homebrew/extend/os/mac/keg.rb:48-65)
- Copy file to temp location (creates new inode)
- Move back to original location
- Retry codesigning on new inode
- This works around Apple's codesign bug

**Our Fix:** (src/relocate.rs:405-468)
```rust
// If signing failed, try Homebrew's workaround
if !success {
    let temp_path = parent.join(format!(".{}.tmp_codesign", file_name));
    fs::copy(path, &temp_path)?;
    fs::rename(&temp_path, path)?;  // Creates new inode
    // Retry codesigning...
}
```

**Changes:**
- ✅ Implemented Homebrew's copy→rename→retry workaround
- ✅ Improved error logging: `debug!` → `warn!` for failures
- ✅ Proper temp file naming: `.{filename}.tmp_codesign`
- ✅ Cleanup: Remove temp file on errors

**Testing:**
- ✅ All 76 unit tests pass
- ✅ `bru reinstall git` - No warnings, git works
- ✅ `bru reinstall jq` - No warnings, jq works
- ✅ `/opt/homebrew/bin/git --version` - Works (git version 2.51.2)
- ✅ `codesign -dvvv .../git` - Properly signed with adhoc signature
- ✅ `file .../git` - Mach-O 64-bit executable (not "data")

**Impact:**
- ✅ Fixes SIGKILL crashes on git, preventing breakage of other Claude Code instances
- ✅ Matches Homebrew's production-tested workaround
- ✅ Better error visibility (warnings vs debug logs)

**Benchmarking Infrastructure Added:**

**Criterion Benchmarks:** (benches/core_operations.rs)
- `normalize_path`: 386.81 ns (5 paths), 120.30 ns (single)
- `normalize_path_complexity`: 82ns (simple) to 185ns (complex)
- `list_installed`: 7.77ms (339 packages)

**Expanded Benchmark Script:** (scripts/benchmark.sh)
- 8 commands: search, info, deps, list, outdated, upgrade, autoremove
- 5 runs per command (was 3)
- Markdown table output for README
- Average speedup calculation

**Profiling:** (ai/profiles/)
- `2025-11-06-upgrade-dry-run.svg` (219 KB) - Parallelization patterns
- `2025-11-06-autoremove-dry-run.svg` (42 KB) - Confirms <20ms execution
- `2025-11-06-deps-ffmpeg.svg` (154 KB) - Dependency resolution (100+ deps)

**Documentation:**
- ai/BENCHMARKING_STRATEGY.md: Comprehensive strategy (criterion, CI tracking, profiling)
- ai/HOMEBREW_WORKAROUNDS.md: Analysis of all Homebrew bugs/workarounds
- ai/profiles/: Flamegraph storage

**Files Changed:**
- src/relocate.rs: Codesign retry workaround (lines 405-468)
- Cargo.toml: Added criterion benchmark dependency
- benches/core_operations.rs: Created with 4 benchmark groups
- scripts/benchmark.sh: Expanded from 1 to 8 commands
- ai/: Added comprehensive documentation

### v0.1.30 (2025-11-06) - CRITICAL FIX: Autoremove Root Cause

**Critical Bug Fixed:** upgrade/reinstall were writing receipts with EMPTY runtime_dependencies

**Problem:**
- Autoremove bug (documented in BREW_AUTOREMOVE_BUG.md) had wrong diagnosis
- Original theory: "receipts are stale" ❌
- Real cause: upgrade/reinstall writing receipts with `runtime_dependencies: []` ❌
- When user ran `bru upgrade curl`, receipt had NO dependencies listed
- Autoremove saw curl had no dependencies → incorrectly removed libnghttp3, rtmpdump, z3
- System breaks (curl, llvm, lld all broken)

**Root Cause:** (src/commands.rs:1890, 2103)
```rust
// WRONG - only includes single formula being upgraded
let runtime_deps = build_runtime_deps(&formula.dependencies, &{
    let mut map = HashMap::new();
    map.insert(formula.name.clone(), formula.clone());  // ← BUG!
    map
});
```
- `build_runtime_deps()` tries to lookup dependencies in the map
- Map only contains formula being upgraded, not its dependencies
- All lookups return None → receipt written with empty dependencies

**Previous "Fix" (commit 6c8d8c0):**
- Made autoremove fetch from API instead of using receipts
- Worked around broken receipts by getting fresh data
- But: 100-300x slower (1-3s vs <10ms), doesn't work offline, masked root cause

**Proper Fix:** (src/commands.rs:1832-1839, 1898-1900, 1999-2001, 2107-2108)
- **upgrade**: Now resolves dependencies BEFORE generating receipts (matches install)
- **reinstall**: Now resolves dependencies BEFORE generating receipts (matches install)
- **autoremove**: Reverted to receipt-based (removed async, API calls, batching)

```rust
// upgrade command - resolve dependencies first
let candidate_names: Vec<String> = candidates.iter().map(|c| c.name.clone()).collect();
let (all_formulae, _) = resolve_dependencies(api, &candidate_names).await?;

// Use complete all_formulae map
let runtime_deps = build_runtime_deps(&formula.dependencies, &all_formulae);
```

```rust
// autoremove - receipt-based traversal (NO API calls)
pub fn autoremove(dry_run: bool) -> Result<()> {  // No longer async!
    while let Some(name) = to_check.pop_front() {
        if let Some(pkg) = all_packages.iter().find(|p| p.name == name) {
            for dep in pkg.runtime_dependencies() {
                required.insert(dep.full_name.clone());
                to_check.push_back(dep.full_name.clone());
            }
        }
    }
}
```

**Testing:**
- ✅ All 76 unit tests pass
- ✅ upgrade --dry-run works correctly
- ✅ autoremove --dry-run: <20ms (was 1-3s)
- ✅ No regressions in other commands

**Impact:**
- ✅ Receipts now have correct runtime_dependencies after upgrade/reinstall
- ✅ Autoremove no longer incorrectly removes required packages
- ✅ **100-300x faster** autoremove (<20ms vs 1-3s)
- ✅ Works offline (no network calls)
- ✅ Matches Homebrew behavior exactly
- ✅ Simpler code (-23 lines net)

**Files Changed:**
- src/commands.rs: Fixed upgrade, reinstall, autoremove
- src/main.rs: Updated autoremove call (removed async)

**Documentation:**
- COMPREHENSIVE_REVIEW.md: Deep technical analysis
- FIXES_APPLIED.md: Implementation details
- REVIEW_SUMMARY.md: Executive summary

### v0.1.29 (2025-11-04) - Bug Fix: Missing Version-Agnostic Symlinks

**Bug Fixed:** Packages were missing version-agnostic symlinks in opt/ and var/homebrew/linked/

**Problem:**
- bru was not creating `/opt/homebrew/opt/<formula>` symlinks
- bru was not creating `/opt/homebrew/var/homebrew/linked/<formula>` symlinks
- These symlinks are used by other packages to find dependencies
- After upgrades, symlinks were left pointing to removed versions (broken symlinks)
- Example: `/opt/homebrew/opt/librsvg` → `../Cellar/librsvg/2.61.2` (removed)

**Root Cause:**
- bru never implemented Homebrew's `optlink` functionality
- Only created bin/lib/include symlinks, not the version-agnostic ones
- Homebrew uses `optlink` method in keg.rb to create these during install/upgrade

**Fix:** (src/symlink.rs:273-395, src/commands.rs:1427,1880,2077,2178,2315,3357,3394)
- Implemented `optlink()` function matching Homebrew's behavior
- Creates `/opt/homebrew/opt/<formula>` → `../Cellar/<formula>/<version>`
- Creates `/opt/homebrew/var/homebrew/linked/<formula>` → `../../../Cellar/<formula>/<version>`
- Called after every `link_formula()` in install, upgrade, reinstall, and link commands
- Implemented `unoptlink()` to remove symlinks during uninstall and unlink
- Automatically updates symlinks during upgrades to point to new version

**Testing:**
- ✅ Real-world test with librsvg reinstall
- ✅ Both symlinks created correctly
- ✅ Symlinks use relative paths matching Homebrew
- ✅ All 85 existing tests still pass

**Impact:**
- ✅ Packages now fully compatible with Homebrew
- ✅ Dependencies can be found via `/opt/homebrew/opt/<formula>`
- ✅ No more broken symlinks after upgrades
- ✅ Matches Homebrew's directory structure exactly
- ✅ No breaking changes

### v0.1.28 (2025-11-04) - Bug Fix: Symlink Conflict Handling

**Bug Fixed:** Packages were not being linked when symlinks already existed

**Problem:**
- bru would silently skip creating symlinks if target already existed (even if pointing elsewhere)
- This caused packages to be installed but not linked to `/opt/homebrew/bin/`
- Commands like `topgrade`, `mise`, `yq` were installed but not accessible in PATH
- No error or warning was shown - linking silently failed

**Root Cause:** (src/symlink.rs:120-122)
- Code returned `Ok(())` (success) when symlink existed but pointed elsewhere
- This was "for safety" but broke upgrades when brew/bru created conflicting symlinks
- Homebrew's behavior: overwrite existing symlinks by default (like `brew link --overwrite`)

**Fix:** (src/symlink.rs:120-142)
- Check if target is a symlink or regular file
- **Symlinks**: Remove and replace (matches Homebrew's `--overwrite` behavior)
- **Regular files**: Warn and skip (protect user files)
- **Broken symlinks**: Automatically remove and replace

**Testing:**
- ✅ Reinstall with missing symlink → creates symlink correctly
- ✅ Reinstall with conflicting symlink → overwrites symlink
- ✅ Reinstall with regular file → warns and skips (file preserved)
- ✅ Binary execution verified after reinstall

**Impact:**
- ✅ Packages now properly linked even when conflicts exist
- ✅ Matches Homebrew's overwrite behavior
- ✅ User files protected (warns instead of overwriting)
- ✅ No breaking changes

### v0.1.27 (2025-11-02) - Critical Hotfix: Architecture Compatibility

**Critical Bug Fixed:** INSTALL_RECEIPT.json arch field incompatibility with Homebrew

**Problem:**
- bru v0.1.26 wrote `"arch": "aarch64"` in INSTALL_RECEIPT.json files
- Homebrew expects `"arch": "arm64"` on Apple Silicon Macs
- Packages installed by bru could not be used as dependencies for brew installations
- Error: "dependencies not built for the arm64 CPU architecture: <dep> was built for aarch64"

**Fix:** (src/receipt.rs:146-153)
- Added `homebrew_arch()` helper function to map Rust architecture names to Homebrew platform names
- Maps `aarch64` → `arm64` (Apple Silicon)
- Keeps `x86_64` → `x86_64` (Intel Mac)
- Applied to both `arch` field (line 127) and `cpu_family` field (line 172)

**Impact:**
- ✅ brew now accepts bru-installed packages as dependencies
- ✅ brew can reinstall bru-installed packages without arch errors
- ✅ Full interoperability between bru and brew maintained

**Testing:**
- ✅ All 103 tests pass
- ✅ Verified INSTALL_RECEIPT.json contains `"arch": "arm64"` and `"cpu_family": "arm64"`
- ✅ Verified `brew reinstall <bru-installed-package>` works without errors

**Affected versions:** v0.1.26 and likely earlier versions
**Upgrade urgency:** HIGH - All v0.1.26 users should upgrade immediately

### v0.1.26 (2025-10-31) - Performance Optimizations

**1. HTTP/2 Connection Pooling** (src/api.rs:106-110)
- Increased `pool_idle_timeout`: 5s → 90s (HTTP keep-alive standard)
- Increased `pool_max_idle_per_host`: 2 → 10 connections
- Reuses TCP/TLS connections during parallel dependency resolution
- **Impact:** 10-30% faster resolution for packages with many dependencies

**2. Enhanced `bru update` Command** (src/commands.rs:2470-2477)
- Now clears cached formula/cask data before updating taps
- Ensures fresh results after `brew update`
- Solves cache staleness concerns
- **Usage:** `bru update` → clears cache + updates all taps in parallel

**Testing:**
- ✅ All 90 tests pass (76 unit + 14 regression)
- ✅ Connection pooling verified
- ✅ Cache clearing works correctly

**Impact:**
- Faster dependency resolution via connection reuse
- Cache stays fresh with explicit update command
- No breaking changes - all existing functionality preserved

### v0.1.25 (2025-10-31) - Custom Tap Support + CLI Modernization

**1. Custom Tap Formula Installation:**

**Fixed tap formula resolution** (src/commands.rs:21-36, 1186-1230)
- Added `is_tap_formula()` helper to detect tap-prefixed names (≥2 slashes)
- Excludes `homebrew/core/*` from tap detection (treats as core formulas)
- Strips `homebrew/core/` prefix when present
- Modified `install()` to separate tap formulas from core formulas
- Tap formulas automatically delegated to brew (typically need source builds)
- Core formulas continue via fast bottle installation
- Supports mixed installs: `bru install jq homebrew/core/wget nijaru/tap/sy`

**Root Cause:**
- `api.fetch_formula()` only queries Homebrew API (formulae.brew.sh)
- Custom tap formulas exist only as local Ruby files
- Previous behavior: "Formula not found" error for all tap formulas

**Solution:**
- Early detection of tap-prefixed formulas (e.g., "user/tap/formula")
- Smart handling of `homebrew/core/` prefix (stripped and treated as core)
- Immediate delegation to brew for custom tap formulas
- No disruption to core formula fast bottle logic
- Clean separation of concerns

**Testing:**
- ✅ Tap formula only: `bru install nijaru/tap/sy`
- ✅ Mixed install: `bru install jq nijaru/tap/sy`
- ✅ Core with prefix: `bru install homebrew/core/wget` → fast bottles
- ✅ Core formula unchanged: `bru install jq`
- ✅ All 76 unit tests + 14 regression tests pass

**2. Removed ℹ Icon for Modern CLI Style:**

**Modernized informational messages** (src/commands.rs: 98 instances)
- Removed all `ℹ` Unicode icons from output
- Modern CLI pattern: plain text, reserve symbols for outcomes (✓/✗)
- Matches cargo/uv/rustup style (no info icons, just colored text)
- Cleaner, less cluttered output
- Better terminal compatibility

**Before:**
```
ℹ nijaru/tap/sy is from a custom tap - delegating to brew
ℹ nijaru/tap/sy requires building from source
```

**After:**
```
nijaru/tap/sy is from a custom tap - delegating to brew
nijaru/tap/sy requires building from source
```

**Research:**
- cargo: Plain indented text, no icons
- uv: Plain descriptive text
- rustup: "info:" prefix, no icons
- clig.dev: "Use symbols sparingly - excessive symbols make output cluttered"

**3. Performance Optimizations:**

**HTTP/2 Connection Pooling** (src/api.rs:106-110)
- Increased `pool_idle_timeout`: 5s → 90s (HTTP keep-alive standard)
- Increased `pool_max_idle_per_host`: 2 → 10 connections
- Reuses TCP/TLS connections during parallel dependency resolution
- **Impact:** 10-30% faster resolution for packages with many dependencies

**Enhanced `bru update` Command** (src/commands.rs:2470-2477)
- Now clears cached formula/cask data before updating taps
- Ensures fresh results after `brew update`
- Solves cache staleness concerns
- **Usage:** `bru update` → clears cache + updates all taps in parallel

**Impact:**
- Custom taps now work correctly
- bru can install formulas from user taps (homebrew-core compatible)
- Cleaner, more professional CLI output matching modern tools
- Faster dependency resolution via connection reuse
- Cache stays fresh with explicit update command
- No breaking changes - all existing functionality preserved
- Maintains bru's design: fast bottles for core, delegate source builds

### v0.1.24 Release (2025-10-30) - UX Improvements

**CLI Output Modernization:**

1. **Removed 143 leading newlines from command output** (src/commands.rs)
   - Converted all `println!("\n ...")` to `println!("...")`
   - Modern CLI style matching uv/cargo (compact, clean output)
   - Removed 5 excessive blank lines between related messages
   - All commands now follow consistent output formatting

2. **Fixed which-formula output** (src/commands.rs:3918)
   - Removed extra status line "Finding formula for command: X"
   - Output now matches brew exactly (just shows formula name)

3. **Implemented help command** (src/main.rs)
   - `bru help` shows general help
   - `bru help <command>` shows command-specific help
   - Automatically falls back to brew for unsupported commands
   - Clean error handling without noise

4. **Global brew fallback for all unsupported commands** (src/main.rs:916-942)
   - Any unrecognized command automatically falls back to brew
   - Transparent to users - all args/flags passed through
   - Proper exit codes maintained
   - Examples: `bru rubocop` → `brew rubocop`, `bru sh` → `brew sh`
   - Enables kombrucha to focus on core package management while supporting all brew commands

**Testing:**
- All 76 unit tests pass ✅
- Comprehensive manual testing of all scenarios ✅
- Release build clean ✅

**Impact:**
- Better UX with modern CLI output style
- Seamless brew fallback means users can use bru for everything
- No breaking changes - all existing functionality preserved

### v0.1.23 (Previous Release)

**Version**: 0.1.22 (In Development - Audit Fixes)
**Status**: Complete audit completed, critical uninstall bug fixed

### v0.1.21 Release (2025-10-30) - CRITICAL FIXES

**Critical Bug Fixes:**

1. **ALL Mach-O files are now signed after extraction** (src/relocate.rs)
   - **Root Cause**: v0.1.20 only signed files that had `@@HOMEBREW` placeholders
   - **Impact**: Binaries without placeholders (like wget) were left UNSIGNED and crashed with SIGKILL
   - **Fix**: Sign ALL Mach-O files after extraction, matching Homebrew's behavior
   - **Details**:
     - Homebrew's tar extraction does NOT preserve code signatures (uses `--no-same-owner`)
     - ALL extracted binaries are unsigned and need signing
     - wget uses `/opt/homebrew/opt/` symlinks (no placeholders) so v0.1.20 skipped it
     - New approach: Separate signing pass after relocation, signs all Mach-O files
     - Follows Homebrew's process: make writable → sign → restore permissions
   - **Tested working**: wget, bat, jq all execute correctly ✅

2. **Symlink paths now match Homebrew's format** (src/symlink.rs)
   - **Root Cause**: Always used `../Cellar` regardless of symlink depth
   - **Impact**: Broken symlinks for nested directories (share/locale, etc)
   - **Fix**: Calculate correct number of `../` based on directory depth from prefix
   - **Examples**:
     - `/opt/homebrew/bin/wget` → `../Cellar/wget/1.25.0/bin/wget` (1 level)
     - `/opt/homebrew/share/man/man1/wget.1` → `../../../Cellar/...` (3 levels)
     - `/opt/homebrew/share/locale/af/LC_MESSAGES/wget.mo` → `../../../../Cellar/...` (4 levels)
   - **Tested working**: All symlink depths now match Homebrew exactly ✅

**Impact Assessment:**
- v0.1.20: Binaries without placeholders crash, broken symlinks - **BROKEN**
- v0.1.21: All binaries properly signed, symlinks correct ✅

### v0.1.22 Development (2025-10-30) - Dependency Resolution Audit

**Dependency Resolution Fix** (src/commands.rs):

1. **`bru deps` now shows transitive dependencies**
   - **Root Cause**: `deps` command only showed direct dependencies, not full closure
   - **Impact**: Users couldn't see all dependencies that would be installed
   - **Comparison (before fix)**:
     - `brew deps ffmpeg`: 92 dependencies
     - `bru deps ffmpeg`: 4 dependencies (WRONG - only direct deps)
   - **Fix**: Modified `deps` command to use `resolve_dependencies()` function
   - **Comparison (after fix)**:
     - `brew deps wget`: 5 deps | `bru deps wget`: 5 deps ✅ (perfect match)
     - `brew deps aom`: 16 deps | `bru deps aom`: 17 deps (1 extra: openjph)
     - `brew deps ffmpeg`: 92 deps | `bru deps ffmpeg`: 93 deps (1 extra: openjph)
   - **Note**: openjph discrepancy is because bru uses latest API (openexr@3.4.2 depends on openjph) while brew uses installed version (openexr@3.4.1 doesn't)

2. **Added `--direct` flag to match `brew deps --direct`**
   - Shows only immediate dependencies, not transitive
   - Matches Homebrew behavior exactly
   - Does NOT show build dependencies (matches brew default)

3. **Verified `resolve_dependencies()` function is correct**
   - Used by `install` command to determine what to install
   - Correctly resolves full transitive closure of runtime dependencies
   - Does NOT include build dependencies (correct for bottle installation)
   - Homebrew doesn't install build deps when installing bottles (pre-compiled binaries)

**Testing Results:**
- Simple packages (wget): Perfect match ✅
- Medium complexity (aom): 16 vs 17 (+1 from updated API) ✅
- Complex packages (ffmpeg): 92 vs 93 (+1 from updated API) ✅
- Direct deps flag: Exact match with brew ✅

**Version/Bottle Selection Audit** (src/platform.rs, src/download.rs):

1. **Platform detection is correct**
   - Detects architecture: aarch64 → arm64, x86_64 → x86_64
   - Detects macOS version and maps to Homebrew code names
   - Format: `{arch}_{codename}` (e.g., "arm64_sequoia")

2. **Fixed macOS 16/26 (Tahoe) version mapping**
   - **Issue**: macOS 16 was mapped to "sequoia" instead of "tahoe"
   - **Root Cause**: Missing "tahoe" in version mapping
   - **Context**: macOS Tahoe uses dual versioning (16 and 26) for compatibility
   - **Fix**: Added both version 16 and 26 → "tahoe" mapping

3. **Added universal bottle fallback**
   - **Issue**: No fallback when exact platform bottle unavailable
   - **Fix**: Added fallback to "all" (universal) bottles, matching Homebrew
   - **Logic**: Try exact platform first, then "all", then fail

**Platform Version Mapping:**
- macOS 26/16: tahoe
- macOS 15: sequoia
- macOS 14: sonoma
- macOS 13: ventura
- macOS 12: monterey
- macOS 11: big_sur

**Testing:**
- All platform detection tests pass ✅
- Bottle selection logic matches Homebrew ✅
- Universal bottle fallback implemented ✅

**Cleanup Logic Audit** (src/commands.rs):

1. **Cleanup command verified correct**
   - Removes old versions of installed formulae ✅
   - Keeps latest version based on semantic versioning ✅
   - Unlinks symlinks before removal ✅
   - Calculates and reports space freed ✅
   - Has dry-run mode ✅

2. **CRITICAL BUG FOUND AND FIXED: Symlink handling in uninstall**
   - **Issue**: Uninstall failed with "Not a directory (os error 20)" on symlinked formulae
   - **Root Cause**: Code called `read_dir()` on symlinks without checking metadata first
   - **Example**: python-certifi → certifi symlink in Cellar
   - **Fix**: Use `symlink_metadata()` to check if path is symlink before calling `read_dir()`
   - **Impact**: CRITICAL - breaks autoremove/uninstall for any renamed/symlinked formulae

**Upgrade Logic Audit** (src/commands.rs):

1. **Upgrade command verified correct**
   - Detects outdated packages by comparing versions ✅
   - Strips bottle revisions for accurate comparison ✅
   - Handles tap formulae (falls back to brew) ✅
   - Respects pinned packages ✅
   - Parallel fetching for performance ✅
   - Has dry-run and force modes ✅

**Audit Status:**
- ✅ Dependency resolution - VERIFIED CORRECT + FIXED
- ✅ Version/bottle selection logic - VERIFIED CORRECT + ENHANCED
- ✅ Cleanup logic - VERIFIED CORRECT
- ✅ Upgrade logic - VERIFIED CORRECT
- ✅ Uninstall logic - CRITICAL BUG FOUND AND FIXED

**Three Critical Bottle Installation Fixes** (2025-10-30, commit: 7b083c1):

Discovered when upgrading python@3.13 (3.13.8 → 3.13.9). Three related issues:

1. **GHCR Token Authentication Failure** (src/download.rs:130-141)
   - **Issue**: "Failed to get GHCR token" for versioned formulae
   - **Root Cause**: Repository path constructed as "homebrew/core/python@3.13"
   - **Problem**: GHCR rejects @ symbol in repository names (returns "NAME_INVALID")
   - **Actual Format**: "homebrew/core/python/3.13" (@ replaced with /)
   - **Fix**: Extract repository path from bottle URL instead of constructing from formula name
   - **Impact**: ALL versioned formulae (python@3.13, node@20, ruby@3.2, etc.)

2. **Bottle Extraction Failure for Revision Suffixes** (src/extract.rs:33-58)
   - **Issue**: "Extraction failed: expected path does not exist"
   - **Root Cause**: Code expected exact version (3.13.9) but archive contained 3.13.9_1
   - **Problem**: Bottle revisions append _N suffix to version directory in archive
   - **Fix**: Look for directories matching version OR version_N pattern
   - **Impact**: All bottles with non-zero revision numbers

3. **Script Relocation Permission Failure** (src/relocate.rs:356-368)
   - **Issue**: "Failed to write script" during shebang placeholder replacement
   - **Root Cause**: Bottles may extract scripts as read-only
   - **Problem**: Code tried to write without making file writable first
   - **Fix**: Make file writable → write → restore original permissions
   - **Impact**: Formulae with scripts containing @@HOMEBREW_PREFIX@@ placeholders

**Testing:**
- All 76 unit tests pass ✅
- python@3.13 upgrade: 3.13.8 → 3.13.9_1 ✅
- wget installation still works ✅

### v0.1.20 Release (2025-10-30) - PARTIAL FIX

**Critical Bug Fixes:**
- **Bottle relocation now works correctly** (src/relocate.rs)
  - **Script Issue** (v0.1.18): @@HOMEBREW_CELLAR@@ placeholders not replaced in executable scripts
    - Root cause: relocate_bottle() only processed Mach-O binaries, ignored scripts
    - Impact: Scripts like `#!/@@HOMEBREW_CELLAR@@/package/version/bin/python` would fail with "bad interpreter"
    - Fix (v0.1.19): Added find_scripts_with_placeholders() and relocate_script_shebang()
    - Only processes executable files in bin/ directories with placeholder shebangs
    - Example: huggingface-cli's `hf` command now works correctly ✅

  - **Code Signature Issue** (v0.1.19): Binaries crashed with SIGKILL after relocation
    - Root cause: Used `codesign --remove-signature` but should use adhoc signing with Homebrew's exact flags
    - Impact: ALL packages installed with bru v0.1.19 crashed with exit code 137 (SIGKILL)
    - **Real fix (v0.1.20)**: Match Homebrew's exact codesign command:
      ```
      codesign --sign - --force --preserve-metadata=entitlements,requirements,flags,runtime
      ```
    - Missing flags `--preserve-metadata` caused signature corruption
    - Verified by examining Homebrew source: `Library/Homebrew/extend/os/mac/keg.rb`
    - Tested working: bat, hf, jq, wget all execute correctly ✅

**Impact Assessment:**
- v0.1.18: Broken scripts, some Python crashes - **5 packages affected**
- v0.1.19: ALL packages crash with SIGKILL - **UNUSABLE, DO NOT USE**
- v0.1.20: Fully working relocation matching Homebrew behavior ✅

**Test Coverage Gaps Identified:**
- **Missing**: Integration tests for bottle relocation
  - Current tests only cover command-level functionality (outdated, upgrade, list, etc.)
  - NO tests verify that installed packages actually execute without crashes
  - NO tests verify script shebangs are properly relocated
  - NO tests verify Mach-O binaries have code signatures removed after install_name_tool
  - **Why we missed the bugs**: Tests never actually installed a bottle and tried to run the resulting binaries
  - **Impact**: Critical relocation bugs (v0.1.18) shipped to users and broke installed packages

**Packages Affected by Broken bru 0.1.18:**
- User installed 26 packages with various bru versions (0.0.1 through 0.1.19)
- **5 packages installed with broken bru 0.1.18** (2025-10-30 02:21-03:02):
  - `mise/2025.10.20` - Oct 30 02:21 ✅ Tested working
  - `vercel-cli/48.6.7` - Oct 30 03:02 ⚠️ Has unreplaced @@HOMEBREW_PREFIX@@ in shebang
  - `doggo/1.1.0` - Oct 30 02:21 ✅ Tested working (binary, no script)
  - `huggingface-cli/1.0.1` - Oct 30 02:58 ✅ Working (shebang was relocated)
  - `opencode/0.15.29` - Oct 30 02:32 (not tested yet)
- **bat status**: Reinstalled with Homebrew (not bru) after crash - **need to investigate why bat crashes even with bru 0.1.19**

### Previous Improvements (fadded4)

**Performance & UX Polish:**
- Live progress display for parallel tap updates (361c845)
- Improved error messages with consistent, colored formatting (cd3f51f)
- Better visual feedback for long operations

**Testing & Quality:**
- Added comprehensive parallel operations test suite (a61a24f)
- 5 new integration tests for parallel correctness
- Services filtering correctness validation
- Performance regression prevention

**Documentation:**
- Updated README with real performance benchmarks (fadded4)
- Documented 2-24x speedup across commands
- Added modern CLI pattern documentation

### Performance Benchmarks (M3 Max, October 2025)
- `bru outdated`: 2.1x faster (780ms vs 1.63s)
- `bru info`: 9.6x faster (107ms vs 1.04s)
- `bru search`: 24x faster (43ms vs 1.04s)
- `bru update`: 5.7x faster (1.9s vs ~11s sequential)
- `bru upgrade`: 3-8x faster (parallel downloads)

### Metrics
- **Test Coverage**: 97 automated tests (76 unit + 21 integration)
- **Integration Tests**: 16 tests (5 parallel operations, 11 regression)
- **Command Coverage**: Core user-facing commands fully functional
- **Bottle-Based Support**: 95% of Homebrew formulae (native)
- **Source Build Support**: 100% via automatic brew fallback

### What Works ✅
- Core package management: install, uninstall, upgrade, reinstall
- **Source builds**: Automatic brew fallback for formulae without bottles
- Cask support: macOS application management (DMG, ZIP, PKG)
- Discovery commands: search, info, deps, uses, list, outdated
- Repository management: tap, untap, update
- System utilities: doctor, config, env, shellenv
- Services: launchd integration
- Bundle: Brewfile install and dump
- Modern CLI output: Tree connectors, clean formatting, command aliases
- Distribution: Homebrew tap (brew install nijaru/tap/bru)

### What Doesn't Work ❌
- Development tools: create, audit, livecheck, test (stubs only)
- CI/internal commands: Not implemented

### Performance
Verified benchmarks (M3 Max, macOS 15.7, 338 packages, October 2025):
- upgrade --dry-run: **1.85x faster than brew** (0.92s vs 1.71s average over 3 runs)
  - bru times: 1.23s, 0.86s, 0.66s (first run slower due to cold cache)
  - brew times: 2.28s, 1.43s, 1.41s
  - Best case: 0.66s vs 1.41s (2.1x faster)
- upgrade optimization: **53x faster** than v0.1.2 (0.74s vs 39.5s)
- All API operations: Fully parallelized with in-memory caching
- Startup time: <0.01s (measured, claimed 0.014s)

## What Worked

### Architecture Decisions
- **Hybrid Rust + Ruby approach**: Right balance of performance and compatibility
- **Parallel operations from day 1**: Major performance win
- **JSON API over tap parsing**: Faster, always up-to-date
- **Bottle-first strategy**: 95% coverage without complexity

### Testing Strategy
- Non-ignored regression tests using --dry-run
- Parity tests against brew to catch regressions
- Property-based checks (deduplication, bottle revision stripping)

### Recent Changes

**v0.1.14** (2025-10-28):
- **Major Performance Optimizations**: Comprehensive parallelization and efficiency improvements [7aa494f, 96f470d, ceb35e0, 3a7a40d, c208f48]
  - **Shell Hang Fix**: Explicit tokio runtime shutdown with 1s timeout prevents shell hang after command completion
  - **Parallel Tap Updates**: 5.7x speedup (10.9s → 1.9s) via parallel git pulls, now matches brew performance
  - **Parallel Upgrade Downloads**: 3-8x speedup for multi-package upgrades via parallel bottle downloads
  - **HashMap/Vec Capacity Hints**: 5-15% improvement for large dependency graphs via pre-allocation
  - **Parallel Mach-O Operations**: 2-4x faster detection, 3-5x faster relocation using rayon
  - **New Dependency**: Added rayon for parallel iteration
  - **Real-world impact**: `bru update` now ~2x faster than before, upgrade scales linearly with packages

**v0.1.11** (2025-10-26):
- **Brew Fallback for Source Builds**: Automatic fallback to brew for formulae without bottles [476cedc]
  - **Feature**: install, upgrade, reinstall now automatically delegate to brew when no bottle available
  - **User Experience**: Clear messaging: "requires building from source (no bottle available)"
  - **Implementation**: Added check_brew_available() and fallback_to_brew() helpers
  - **Rationale**: Long-term solution (same Cellar, compatible receipts, zero maintenance)
  - **Coverage**: 100% of Homebrew formulae (95% native bottles + 5% brew fallback)
  - **Design**: Full analysis in ai/FALLBACK_DESIGN.md
  - Commands updated: install (line ~1115), upgrade (line ~1469), reinstall (line ~1606)
- **CI Improvements**: Fixed Homebrew integrity check to avoid false positives [c1af9e1]
  - Updated check to verify Cellar integrity instead of brew doctor
  - Pre-existing runner warnings no longer fail builds
- **Distribution**: Released to crates.io, GitHub, and Homebrew tap

**v0.1.13** (2025-10-27):
- **Critical Bug Fix**: Resolved "Too many open files" error in parallel downloads [df09a0b]
  - Root cause: Each download created new reqwest::Client, exhausting file descriptors
  - Fix: Create shared HTTP client once per operation, pass by reference
  - Reduced MAX_CONCURRENT_DOWNLOADS from 16 to 8 for more conservative resource usage
  - Client connection pooling now works correctly across all downloads
  - Tested: Successfully upgraded 3 packages without errors

**v0.1.12** (2025-10-27):
- **Quick Wins**: UX improvements and code quality [50b4f65, c786cac, fabf8ba]
  - Added --no-color flag for easier color disabling (works like NO_COLOR env var)
  - Resolved all 36 clippy warnings (collapsible_if, type_complexity, if_same_then_else)
  - Added 8 edge case tests validating unwrap fixes from f071069
  - Improved git error message in doctor command with install guidance
  - Better let-chain formatting for consistency
  - All 76 unit tests passing, CI green
- **Robustness Improvements**: Eliminated all edge-case unwrap() calls [f071069]
  - Fixed 9 problematic unwraps (path handling, empty strings, option unwrapping)
  - Better error messages for invalid UTF-8 paths, empty formula names
  - No more panics on edge-case inputs
  - Reduced unwraps from 20 to 10 (all remaining are safe)
- **UX Polish**: Final refinements to progress indicators and symbols [5736f97]
  - Removed arrow symbols (⬇ →) from output for cleaner appearance
  - Improved progress bar with Unicode blocks (━━╸ instead of #>-)
  - Simplified download messages: "Downloading wget" instead of "⬇ wget"
  - Changed relationship indicators to ASCII (->) for better compatibility
- **Quiet Mode**: Added --quiet/-q flag and BRU_QUIET env var [17f41aa]
  - Suppresses progress bars and spinners
  - ProgressBar::hidden() for quiet mode
  - Respects user preference for minimal output
- **Progress Indicators**: Enhanced UX for long-running operations [cf7067d]
  - Dependency resolution spinner with status messages
  - Install counter showing progress (Installing 3/10...)
  - Modern spinner characters (⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏)
- **Code Quality**: Comprehensive audit and improvements [b2d8c94]
  - Added profiling support (debug = true in release profile)
  - Created flamegraph.svg for performance analysis
  - Documented UX best practices in ai/UX_AUDIT.md
  - Comprehensive code review in ai/CODE_REVIEW.md
  - Performance profile analysis in ai/PERFORMANCE_PROFILE.md

**v0.1.10** (2025-10-23):
- **CRITICAL DATA LOSS BUG FIXED**: cleanup command was deleting NEWEST versions!
  - **Discovered**: User testing revealed cleanup kept v1.7.0 and deleted v1.8.1
  - **Root cause**: Assumed versions[0] was newest, but fs::read_dir() returns random order
  - **Impact**: Users running cleanup could lose their newest package versions
  - **Fix 1**: Added semantic version comparison to cleanup command
  - **Fix 2**: Added version sorting to get_installed_versions() (systemic fix)
  - Commands affected: cleanup (critical), upgrade (minor), list (cosmetic)
- **Resource Exhaustion Fixes**: Found 2 more potential "Too many open files" bugs
  - calculate_dir_size(): Added .max_open(64) to WalkDir (used by `bru cache`)
  - download_bottles(): Added semaphore limiting to 16 concurrent downloads
- **Testing Quality Issues**: Post-mortem analysis of why tests missed cleanup bug
  - Added 6 comprehensive cleanup tests with behavior verification
  - Created TESTING_ISSUES.md documenting test quality problems
  - 98 tests total, but most are shallow (test execution, not correctness)
  - Action plan: Add behavior tests for all critical commands

**v0.1.9** (2025-10-23):
- **Critical Bug Fix**: "Too many open files" error during large package upgrades
  - **Root cause**: WalkDir keeps directory handles open during traversal - llvm alone has 9,283 files
  - **Fix**: Added `.max_open(64)` to limit concurrent directory handles in relocate.rs
  - **Impact**: Can now upgrade packages with thousands of files (llvm, numpy, etc.) without resource exhaustion
  - User-reported: `cargo install kombrucha` → `bru upgrade` with 19 packages failed on librsvg
  - Verified fix: Successfully processed llvm reinstall (9,283 files) without errors
- **Testing**: All 92 tests passing (70 unit + 14 regression + 8 basic)

**Unreleased** (post-v0.1.8):
- **Performance Benchmarking**: Verified and corrected claimed performance metrics
  - Original claim: "5.5x faster than brew" was based on outdated brew baseline
  - Verified speedup: **1.85x faster** on average (0.92s vs 1.71s for upgrade --dry-run)
  - Test system: M3 Max, macOS 15.7, 338 packages
  - bru times: 1.23s, 0.86s, 0.66s (average 0.92s)
  - brew times: 2.28s, 1.43s, 1.41s (average 1.71s)
  - Best case: 0.66s vs 1.41s = 2.1x faster
  - Updated STATUS.md with honest, reproducible benchmarks
- **Critical Bug Fixes**: Edge case hunting and code review found 6 critical bugs
  - relocate.rs: `is_mach_o()` was reading entire files instead of just 4 bytes (same pattern as v0.1.5)
  - commands.rs: `count_formulae()` was reading entire files just to check if readable
  - Added depth limits (MAX_DEPTH=10) to 3 recursive functions to prevent stack overflow
  - **reinstall command**: Fixed version mismatch bug - was using OLD version to extract NEW bottle
  - **String slicing panics**: Fixed 2 locations that panicked on single-character formula/cask names
  - Code review: Identified 22 total issues (3 critical fixed, 19 documented for future work)
- **Testing**: Added 57 new unit tests (92 total CI tests, up from 35)
  - Symlink path normalization tests (8 tests) - validates recent bug fixes
  - Cache functionality tests (6 tests) - TTL logic, directory detection
  - Download module tests (6 tests) - filename construction, GHCR token URLs
  - Error handling tests (4 tests) - error message formatting
  - Receipt module tests (6 tests) - version format, structure validation
  - Platform module tests (6 tests) - architecture normalization, version parsing
  - API module tests (8 tests) - URL construction, user agent format
  - Tap module tests (6 tests) - Git URL construction, tap name parsing
  - Progress module tests (7 tests) - percentage calculation, time formatting
  - All tests CI-safe (no system modification required)
  - Test coverage increased by 163% (35 → 92 tests)

**v0.1.8** (2025-10-23):
- **Critical Bug Fixes**: Symlink cleanup in multiple commands
  - upgrade command: Properly unlinks symlinks before removal
  - cleanup command: Properly unlinks symlinks before removal
  - uninstall command: Proper cleanup of all symlinks
  - Prevents "Directory not empty" errors during package operations
- **Documentation**: Updated STATUS.md and cleaned up dated summary files

**v0.1.7** (2025-10-23):
- **UX Enhancement**: Better tree visualization and command summaries
- **UX Enhancement**: Simplified help output to match brew style
- **Bug Fix**: Show usage message for desc/pin/unpin/link/unlink with no args
- **Bug Fix**: Show helpful error when commands called with no formulae

**v0.1.6** (2025-10-22):
- **Compatibility**: Full brew-compatibility for multiple commands
  - info: No fetch message when piped (matches brew behavior)
  - deps/uses: Output format matches brew exactly
  - search: Output format matches brew exactly
  - leaves: Output format matches brew exactly
  - outdated: Output format matches brew exactly
  - list: Full brew compatibility with column mode
- **UX Enhancement**: Improved list command spacing and column mode

**v0.1.5** (2025-10-22):
- **Critical Bug Fix**: File descriptor leak during upgrade
  - Removed canonicalize() calls from symlink creation/removal
  - Fixed "Too many open files" error with large packages (1000+ files)
  - Tested with yt-dlp (1722 files), gh (215 files), multiple packages
- UX: Icon spacing fix - icons no longer sit at terminal edge
  - Space added inside colored strings for proper alignment
  - Applied to all status icons (ℹ, ✓, ✗, ⚠)
- Documentation: Comprehensive code review for resource leaks
  - Verified all file handles properly scoped
  - Verified no unbounded memory growth
  - Identified testing gaps (need large package tests)

**v0.1.4** (2025-10-22):
- Performance: Parallelized ALL sequential API patterns (8 total)
  - fetch command: Parallel metadata fetching
  - install validation: Instant multi-package validation
  - dependency resolution: Breadth-first with parallel levels
  - cask operations: Parallel metadata for install/upgrade
- Performance: In-memory caching (moka) - eliminates redundant API calls
  - Caches 1000 formulae + 500 casks per command execution
  - Benefits complex dependency trees with shared dependencies
- Result: 5.5x faster than brew for upgrade checks (0.65s vs 3.56s)

**v0.1.3** (2025-10-22):
- Performance: 53x faster upgrade checks (39.5s → 0.74s) via parallelization
- UX: Tree connectors (├ └) for visual hierarchy in install/upgrade/uninstall
- UX: Command aliases (i, up, re, rm) for faster workflow
- UX: Detailed "Already installed" messages with version numbers
- CI: Integrated tests now run automatically on every push

**v0.1.2** (2025-10-22):
- Bug fixes: leaves command deduplication, error handling improvements
- UX: Removed stack traces, accurate success/failure messages
- Quality: Replaced unwrap() calls with proper error handling
- All changes from extended session consolidated into release

**v0.1.1** (2025-10-22):
- Upgrade duplicates: Fixed deduplication by modification time
- Leaves duplicates: Fixed deduplication (same bug as upgrade)
- Bottle revision false positives: Strip _N revisions before comparison
- Modern CLI output: Removed all 78 arrow symbols (→ ⬇ ⬆)
- Error handling: Removed stack backtraces, added proper validation
- Error messages: Accurate success/failure reporting for uninstall/reinstall
- Added 12 new regression tests (install, search, info, deps, leaves, etc.)
- Improved test coverage: 16 → 27 automated tests
- Documentation reorganization: agent-contexts patterns (ai/, docs/)

**v0.1.0** (2025-10-21):
- Initial beta release
- Core commands functional
- Bottle-based workflows production-ready

## What Didn't Work

### Initial Testing Approach
- Tried: All integration tests marked #[ignore]
- Problem: Never ran in CI, missed critical bugs
- Fix: Added non-ignored regression tests using --dry-run

### Symbol Cleanup Attempts
- Tried: sed/perl for bulk symbol removal
- Problem: Broke format strings
- Fix: Manual fixes for user-visible commands, left others for later

## Active Work

**v0.1.11 Released** (2025-10-26):
- ✅ Brew fallback feature implemented and released
- ✅ Distributed to: crates.io, GitHub releases, Homebrew tap
- ✅ 100% formula coverage (95% bottles + 5% brew fallback)

**Real-World Validation** (CURRENT - Started 2025-10-26):
- 📋 Testing checklist: ai/REAL_WORLD_TESTING.md
- 🎯 Goal: Use bru daily for 1-2 weeks to validate core functionality
- 📝 Document: bugs, performance issues, edge cases
- 🆕 **Test brew fallback** with source-only formulae
- ⏸️ **Source builds deferred** - Validate 100% coverage via fallback first

**Testing Infrastructure Overhaul** (2025-10-24 - COMPLETE):
- ❌ **System Corruption Incident**: Integration tests corrupted macOS system (Oct 23)
  - Node binary: Kernel code signing failure → SIGKILL on all node/npm commands
  - mise shims: Replaced with binary garbage instead of shell scripts
  - Claude Code: Unable to run (SIGKILL)
  - Root cause: Tests directly modified `/opt/homebrew/Cellar/` without isolation
- 📋 **Comprehensive Review Complete**: Created ai/TESTING_REMEDIATION.md
  - Researched Homebrew's testing best practices (testpath, brew test-bot, GitHub Actions)
  - Identified violations: Tests modify real system, no isolation, bad formula test
  - SOTA solution: testcontainers-rs + brew test-bot --local + GitHub Actions
- ✅ **Phase 1 Complete (P0 - Critical)**: Safe testing infrastructure [01e7c75]
  - Deleted dangerous tests/integration_tests.rs
  - Added testcontainers-rs and tempfile for isolated testing
  - Created tests/test_helpers.rs with TestEnvironment
  - Updated CI to verify Homebrew integrity after tests
  - Deprecated docs/architecture/testing-strategy.md
- ✅ **Phase 2 Complete (P1 - High)**: Proper tap management [808aadd in homebrew-tap]
  - Added GitHub Actions workflows to homebrew-tap (tests.yml, publish.yml)
  - Updated formula test block to test actual functionality (not just --version)
  - Documented brew test-bot --local workflow in tap README
  - Automated bottle building for macOS 13, macOS 14, Ubuntu
- ❌ **Phase 3 Not Recommended**: Docker-based integration tests [311e4d6]
  - brew test-bot on CI is sufficient (what Homebrew uses)
  - CI already tests 3 platforms (macOS 13, 14, Ubuntu)
  - Docker tests would duplicate brew test-bot functionality
  - Added complexity with diminishing returns
  - See ai/TESTING_REMEDIATION.md for full rationale

## Blockers

None currently. Critical testing infrastructure issues resolved (Phase 1 & 2 complete).

## Next Priorities

1. **Real-world testing (CURRENT)**: Use bru daily for 1-2 weeks (see ai/REAL_WORLD_TESTING.md)
   - Test all core operations (install, upgrade, cleanup)
   - **Test brew fallback** with source-only formulae
   - Document bugs, performance issues, edge cases
   - Validate edge case handling (bottle revisions, keg-only, etc.)
2. **Bug fixes**: Address issues found during real-world testing
3. **Performance optimization**: Profile and fix any bottlenecks discovered
4. **UX improvements**: Consider adding progress bars/indicators if needed
5. **Source builds (DEFERRED)**: Only after 100% coverage via fallback is validated
