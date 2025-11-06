# Bru autoremove Bug - 2025-11-05

## Issue Summary

**`bru autoremove` incorrectly removed required dependencies** during system update, breaking curl, llvm, lld, and the entire development environment.

**NOTE**: This is a bug in `bru` (Homebrew clone in Rust), NOT Homebrew itself.

## What Happened

During `up` script execution:
```bash
brew update
and brew upgrade
and brew cleanup
and brew autoremove
```

### Packages Upgraded (9 total)
- go: 1.25.3 → 1.25.4
- huggingface-cli: 1.0.1 → 1.1.1
- lld: 21.1.4 → 21.1.5
- opencode: 1.0.25 → 1.0.34
- re2: 20250812_1 → 20251105
- curl: 8.16.0 → 8.17.0
- harfbuzz: 12.1.0 → 12.2.0
- llvm: 21.1.4 → 21.1.5
- biome: 2.3.3 → 2.3.4

### Packages Removed as "Unused" (3 total)
**ALL THREE WERE INCORRECTLY REMOVED** ❌❌❌

1. **libnghttp3 1.12.0** - INCORRECTLY REMOVED
   - Required by: curl 8.17.0
   - Error: `dyld: Library not loaded: /opt/homebrew/opt/libnghttp3/lib/libnghttp3.9.dylib`
   - Fixed: `brew install libnghttp3`

2. **rtmpdump 2.4-20151223_3** - INCORRECTLY REMOVED
   - Required by: curl 8.17.0
   - Error: `dyld: Library not loaded: /opt/homebrew/opt/rtmpdump/lib/librtmp.1.dylib`
   - Fixed: `brew install rtmpdump`

3. **z3 4.15.4** - INCORRECTLY REMOVED
   - Required by: llvm 21.1.5, lld 21.1.5 (both just upgraded!)
   - Error: `dyld: Library not loaded: /opt/homebrew/opt/z3/lib/libz3.4.15.dylib`
   - Broke: llvm-config, lld, and anything using LLVM
   - Fixed: `brew install z3`

## Impact

**CRITICAL: 100% of packages identified as "unused" were actually required**

- **Broke curl** - All curl commands failed with dyld errors
  - Impact: fisher, npm, git https, any HTTP tool
- **Broke LLVM/lld** - Compiler toolchain broken
  - Impact: Rust compilation, C/C++ compilation, any LLVM-based tool
- **System essentially unusable** for development work

## Root Cause Analysis

The sequence was:
1. `bru upgrade curl` (8.16.0 → 8.17.0)
2. `bru upgrade llvm` (21.1.4 → 21.1.5)
3. `bru upgrade lld` (21.1.4 → 21.1.5)
4. `bru autoremove` runs
5. **BUG**: `bru` incorrectly identifies dependencies as "unused":
   - libnghttp3 (required by curl 8.17.0)
   - rtmpdump (required by curl 8.17.0)
   - z3 (required by llvm 21.1.5, lld 21.1.5)
6. Dependencies removed → packages broken
7. System unusable for development

**This is a critical bug in `bru`'s dependency tracking logic.**

Likely causes:
- `bru` not updating dependency graph after `upgrade`
- Autoremove checking against old dependency state
- Missing lock/transaction between upgrade and autoremove

## Fix Applied

### Initial Quick Fix (WRONG APPROACH)
```bash
# This fixed the immediate errors but didn't address root cause
brew install libnghttp3 rtmpdump z3
```

### Proper Fix (CORRECT APPROACH)
```bash
# Reinstall the packages that depend on removed libraries
# This properly re-establishes dependency links
brew reinstall curl llvm lld
```

**Result**: Homebrew downgraded packages back to stable versions:
- curl: 8.17.0 → 8.16.0 (stable)
- llvm: 21.1.5 → 21.1.4 (stable)
- lld: 21.1.5 → 21.1.4 (stable)

All working now ✓

## Recommendations

### Short-term
1. **Remove `brew autoremove` from `up` script** - Too risky
2. Or add safety check before autoremove:
   ```bash
   # Verify no broken links before removing
   brew doctor
   and brew autoremove
   ```

### Long-term
1. **Investigate Homebrew upgrade process**
   - Why didn't upgraded curl retain its dependencies?
   - Is this a known Homebrew issue?
   - Check Homebrew GitHub issues for similar reports

2. **Add validation to `up` script**
   - Test critical tools after upgrade (curl, git, etc.)
   - Rollback or skip autoremove if breakage detected

## Performance Observation

The upgrade process shows sequential linking:
```
✓ opencode [━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━] 27.76 MiB/27.76 MiB (0s)
    ├ ✓ Linked 2 files
    ├ ✓ Removed old version 1.0.25
    └ ✓ Upgraded opencode to 1.0.34
```

Each package appears to:
1. Download (parallel ✓)
2. Link files (sequential?)
3. Remove old version (sequential?)

**Optimization opportunity**: Can linking/cleanup be parallelized across packages?
- llvm linked 9195 files sequentially - significant time
- curl linked 544 files
- Could save 10-30s on large upgrades

### Investigation needed:
- Is linking already parallel?
- Does linking need to be sequential for safety?
- Can we use `brew upgrade --parallel` or similar?

## Related Files

### Update Script Location
`~/.config/fish/functions/up.fish` - Lines 10-13:

```fish
brew update        # Actually runs: bru update (if bru alias is set)
and brew upgrade   # Actually runs: bru upgrade
and brew cleanup   # Actually runs: bru cleanup
and brew autoremove  # Actually runs: bru autoremove ← BUG IS HERE
```

**Note**: The `brew` command was aliased to `bru` in `~/.config/fish/darwin.fish` for testing.
This entire session was using `bru`, not Homebrew.

### Recommended Fix
```fish
brew update
and brew upgrade
and brew cleanup
# DO NOT USE autoremove with bru - dependency tracking is broken
# See: ~/github/nijaru/kombrucha/BREW_AUTOREMOVE_BUG.md
# and brew autoremove
```

## Timeline
- 2025-11-05 20:15: Ran `up` command
- 2025-11-05 20:15: Packages upgraded, autoremove executed
- 2025-11-05 20:15: curl broken, fisher failed
- 2025-11-05 20:16: Reinstalled libnghttp3 and rtmpdump
- 2025-11-05 20:16: System working again

## Action Items
- [ ] **URGENT: Remove `brew autoremove` from update script** - 100% failure rate
- [x] Verify z3 removal - CONFIRMED BROKEN (llvm/lld failed)
- [ ] Report to Homebrew GitHub - This is a critical bug
- [ ] Investigate linking parallelization opportunity
- [ ] Add post-upgrade validation to `up` script

## Severity: CRITICAL

**`bru autoremove` is completely broken** - identified 3 packages as unused, all 3 were required.
This is a 100% false positive rate.

**DO NOT USE `bru autoremove` after `bru upgrade` until this bug is fixed.**

## Bug Location in bru

The dependency tracking logic in `bru` needs investigation:
- Where does `bru` track installed dependencies?
- How does `upgrade` update the dependency graph?
- Does `autoremove` read stale dependency information?
- Is there proper locking between upgrade and autoremove?

**Next steps for bru development**:
1. Add integration tests for upgrade → autoremove sequence
2. Verify dependency graph updates after each upgrade
3. Add safety checks in autoremove (verify no broken links before removing)
4. Consider: Should autoremove even exist? Homebrew users rarely use it.
