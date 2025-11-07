# Homebrew Bug Workarounds & Edge Cases Analysis

## Executive Summary

Reviewed Homebrew source code for known bugs, workarounds, and edge cases. Compared against bru's implementation to identify gaps and priorities.

**Status**: ✅ Critical issues covered, optional improvements identified

## Critical Workarounds (Implemented)

### 1. Apple Codesign Bug - IMPLEMENTED ✅

**Location**: `Library/Homebrew/extend/os/mac/keg.rb:48-65`

**Issue**: Apple's codesign utility fails randomly when signing binaries

**Homebrew Solution**:
```ruby
Dir::Tmpname.create("workaround") do |tmppath|
  FileUtils.cp file, tmppath
  FileUtils.mv tmppath, file, force: true
end
# Retry codesigning after creating new inode
```

**Our Implementation**: `src/relocate.rs:405-468`
- ✅ Copy file to temp (`.{filename}.tmp_codesign`)
- ✅ Rename back to create new inode
- ✅ Retry codesigning
- ✅ Log warnings on failure
- **Status**: Fixed in this session (Nov 6, 2025)

**Impact**: CRITICAL - Prevents SIGKILL (exit 137) crashes on binaries

---

## Edge Cases (Already Handled)

### 2. Python Virtualenv Files - OK ✅

**Homebrew**: Skips `orig-prefix.txt` during text file relocation

**Our Implementation**: `src/relocate.rs:254-273`
- Only processes files in `bin/` directories
- `orig-prefix.txt` is in `lib/pythonX.Y/`, so automatically skipped
- **Status**: Correct by design

### 3. Script Relocation - OK ✅

**Homebrew**: Only relocates executable scripts with placeholders

**Our Implementation**: `src/relocate.rs:247-324`
- ✅ Only checks `bin/` directories (max_depth=3)
- ✅ Verifies file is executable (mode & 0o111)
- ✅ Skips Mach-O binaries (already handled)
- ✅ Only processes scripts with @@HOMEBREW placeholders
- **Status**: Matches Homebrew behavior

### 4. Codesigning Warnings - OK ✅

**Homebrew**: Ignores "warning:" messages from install_name_tool

**Our Implementation**: `src/relocate.rs:175-182, 234-239`
```rust
if !stderr.contains("warning:") {
    tracing::warn!("Failed to relocate...");
}
```
- **Status**: Matches Homebrew (ignores warnings, logs errors)

---

## Non-Critical Edge Cases (Not Applicable to Relocation)

### 5. Build Prefix Patching - N/A

**Homebrew**: `Library/Homebrew/keg_relocate.rb:272-278`

**What it does**: Replaces build-time paths (e.g., `/tmp/homebrew-build-xyz`) with null bytes, validates binary size matches

**Why we don't need it**:
- Build prefix patching is for bottles built locally
- We only install pre-built bottles from Homebrew's servers
- Homebrew already stripped build paths before publishing bottles
- We only need to replace `@@HOMEBREW_PREFIX@@` and `@@HOMEBREW_CELLAR@@`

**Status**: Not needed for our use case

### 6. File Command Locale - N/A

**Homebrew**: `Library/Homebrew/keg_relocate.rb:323-326`

**What it does**: Sets `LC_ALL=C` when running `file` command to avoid locale issues

**Why we don't need it**:
- We use Rust's `std::fs::read()` + magic bytes check
- No external `file` command dependency
- Immune to locale issues

**Status**: Not applicable

### 7. Hardlink Deduplication - N/A to Relocation

**Homebrew**: `Library/Homebrew/keg_relocate.rb:378-379`

**What it does**: Tracks inodes to avoid processing hardlinks twice

**Why we don't need it (yet)**:
- Our relocation uses `WalkDir` which returns all file paths
- We process each path independently (idempotent operations)
- install_name_tool and codesign handle being run multiple times on same file
- Could optimize in future by tracking inodes

**Status**: Works correctly, could optimize for performance

---

## Potential Future Improvements (Low Priority)

### 8. Hardlink Inode Tracking (Performance)

**Priority**: Low
**Impact**: Minor performance improvement for packages with many hardlinks

**Implementation**:
```rust
use std::collections::HashSet;
use std::os::unix::fs::MetadataExt;

let mut processed_inodes = HashSet::new();

for file in mach_o_files {
    let metadata = fs::metadata(&file)?;
    let inode = metadata.ino();

    if processed_inodes.contains(&inode) {
        continue; // Skip hardlinks
    }
    processed_inodes.insert(inode);

    // Process file...
}
```

**Benefit**: Avoid redundant codesigning/relocation on hardlinked files
**Cost**: Extra HashSet lookups, minimal memory overhead
**Recommendation**: Profile first - likely not worth complexity

### 9. Relative Symlink Detection (Symlinking Phase)

**Priority**: Low - Not relevant to relocation phase

**What Homebrew does**: Skips fixing relative symlinks during dynamic linkage

**Our status**: We don't modify symlinks during relocation (correct)

**Recommendation**: No action needed

### 10. Locale Handling for External Tools

**Priority**: Very Low

**Current status**: We don't use locale-sensitive external tools

**Recommendation**: If we ever add `file` command usage, set `LC_ALL=C`

---

## Testing Gaps Identified

### Scripts with Placeholders

**Current testing**: None for script relocation
**Recommendation**: Add test in `tests/relocation_tests.rs`:

```rust
#[test]
fn test_script_shebang_relocation() {
    // Create test script with @@HOMEBREW_PREFIX@@/bin/python shebang
    // Run relocate_script_shebang()
    // Verify shebang is updated to /opt/homebrew/bin/python
}
```

### Codesign Retry Workaround

**Current testing**: None for retry mechanism
**Recommendation**: Hard to test (requires Apple's codesign bug to trigger)
**Alternative**: Integration test that verifies git/other packages work after install

### Hardlink Handling

**Current testing**: None
**Recommendation**: Low priority - add if we implement inode tracking

---

## Summary of Findings

| Issue | Homebrew Workaround | Our Status | Priority |
|-------|---------------------|------------|----------|
| Apple codesign bug | Copy to temp, retry | ✅ **Fixed today** | CRITICAL |
| Python virtualenv | Skip orig-prefix.txt | ✅ Correct by design | N/A |
| Script relocation | Only bin/ + executable | ✅ Matches Homebrew | N/A |
| Build prefix patching | Size validation | ❌ Not needed | N/A |
| File locale issues | LC_ALL=C | ✅ Don't use file command | N/A |
| Hardlink deduplication | Track inodes | ⚠️ Works, could optimize | LOW |

---

## Recommendations

### Immediate (v0.1.31)
1. ✅ **Done**: Implement codesign retry workaround
2. ✅ **Done**: Test git reinstall works
3. **Optional**: Add script relocation test

### Future Optimization (v0.2.x)
1. Profile hardlink inode tracking (only if performance issue found)
2. Add integration tests for edge cases

### Not Needed
- Build prefix patching (we only install pre-built bottles)
- File command locale handling (we don't use file command)
- Binary size validation (only needed for build prefix patching)

---

## References

- Homebrew codesigning: `Library/Homebrew/extend/os/mac/keg.rb`
- Homebrew relocation: `Library/Homebrew/keg_relocate.rb`
- Platform-specific: `Library/Homebrew/extend/os/mac/keg_relocate.rb`
- Our implementation: `src/relocate.rs`

---

## Conclusion

**We've implemented the only critical workaround**: Apple's codesign bug fix.

All other Homebrew edge cases either:
- Don't apply to our use case (build prefix patching)
- Are already handled correctly by our implementation (virtualenv, scripts)
- Are minor optimizations (hardlink tracking)

**Current state**: Production-ready for v0.1.31 release.
