# Code Review: Resource Leak Analysis

**Date:** 2025-10-22
**Scope:** Full codebase review for resource leaks and scaling issues
**Trigger:** File descriptor leak found during upgrade (yt-dlp with 1722 files)

## Summary
Comprehensive review of bru codebase for resource leaks and scaling issues.

## Fixed Issues

### Critical: File Descriptor Leak in symlink.rs
**Problem:** `canonicalize()` opens files to resolve symlinks. Called thousands of times during upgrade.
**Impact:** "Too many open files (os error 24)" when upgrading packages with 1000+ files
**Fix:** Compare symlink targets directly without canonicalizing (commit 865c0d1)

**Details:**
- `create_relative_symlink()` called `canonicalize()` 3 times per file (lines 90, 92, 95)
- `unlink_directory()` called `canonicalize()` once per file (line 194)
- For yt-dlp with 1722 files: 1722 × 4 = 6,888 file opens
- Combined with gh (215 files) and gofumpt: cumulative leak exceeded system limit

**Solution:**
- Compare symlink targets as PathBuf without resolving
- Build expected relative path and compare directly
- Reduced file descriptor usage by ~7000 for large packages

## Remaining canonicalize() Calls (Low Risk)

### src/commands.rs:3086 - List command
```rust
.canonicalize().unwrap_or(target.clone())
```
**Context:** Showing linked files, breaks after 10 iterations
**Risk:** Low - limited iterations
**Action:** No change needed

### src/commands.rs:3145 - Which-formula command
```rust
bin_dir.join(&target).canonicalize().unwrap_or(target)
```
**Context:** Single command lookup
**Risk:** Low - single call
**Action:** No change needed

## Directory Walking Patterns (All Safe)

### Recursive Functions
- `link_directory()` - Uses fs::read_dir, processes iterators immediately
- `unlink_directory()` - Uses fs::read_dir, processes iterators immediately
- `calculate_dir_size()` - Uses walkdir crate (well-tested, streaming)

### Nested Loops
- Tap searching (3 locations) - All break early when found
- Formula searching - Breaks on match
- No unbounded loops found

### File Handle Usage
- `File::open` in download.rs - Read in loop, dropped at function end
- `File::open` in extract.rs - Ownership transferred to GzDecoder→Archive→unpack
- All files properly RAII-scoped

## Memory Usage (All Acceptable)

### Large Collections
- `list_installed()` - Vec of all packages (just paths, acceptable)
- `join_all(fetch_futures)` - Parallel API fetches (bounded by package count)
- `HashMap` for deduplication - One entry per formula (bounded)

### Cleanup Operations
- Uses walkdir for size calculation (streaming, not loaded into memory)
- Processes packages sequentially
- No accumulation issues found

## Testing Gaps

### Current Test Coverage
- Dry-run tests only (don't test actual file operations)
- Small package tests (miss resource leak issues)
- No stress tests for resource limits

### Needed Tests
1. Large package upgrade test (1000+ files like yt-dlp)
2. Multiple large packages in sequence
3. Resource limit stress test
4. File descriptor monitoring in CI

## Recommendations

1. ✅ **Fixed:** Remove canonicalize() from hot paths
2. ✅ **Verified:** All file handles properly scoped
3. ✅ **Verified:** No unbounded memory growth
4. ✅ **Verified:** No recursive iterator leaks
5. ⏳ **TODO:** Add integration test for large packages
6. ⏳ **TODO:** Add resource monitoring to CI
7. ⏳ **TODO:** Consider adding ulimit checks before operations

## Search Patterns Used

```bash
# Find all canonicalize calls
grep -rn "canonicalize()" src/

# Find recursive directory walking
grep -rn "fs::read_dir" src/

# Find file opens
grep -rn "File::open\|fs::File::open" src/

# Find recursive functions
grep -rn "fn.*_directory\|fn.*_dir" src/
```

## Conclusion

✅ **Critical bug fixed:** File descriptor leak from canonicalize()
✅ **No other resource leaks found**
✅ **Code is well-structured with proper RAII**
⏳ **Need better test coverage for large-scale operations**

The codebase is generally well-written. The canonicalize() issue was subtle because:
- It's an implicit file open (not obvious)
- Only manifests with large packages (1000+ files)
- Compounds across multiple package upgrades
- Dry-run tests don't catch it (no actual linking)
