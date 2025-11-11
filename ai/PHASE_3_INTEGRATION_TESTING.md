# Phase 3: Integration Testing & Verification

**Status**: PLANNING  
**Target**: Validate PackageManager API with real system state  
**Duration**: 2-3 hours  

## Overview

Phase 2 completed the high-level PackageManager API with all 4 package management operations implemented. Phase 3 validates the implementation through:

1. **Manual integration tests** against live system
2. **Workflow verification** (install → upgrade → cleanup → uninstall)
3. **Edge case validation** (bottle revisions, keg-only, dependencies)
4. **Performance profiling** (cleanup on large Cellar)
5. **Error path testing** (network failures, missing packages)

## Test Checklist

### Core Operations (Manual Testing)

**Test 1: install() Operation**
- [ ] Install simple package (ripgrep)
  - Verify: Version matches API
  - Verify: Binary executable and works
  - Verify: INSTALL_RECEIPT.json created
  - Verify: Symlinks created in /opt/homebrew/bin/
- [ ] Install package with dependencies (graphicsmagick)
  - Verify: Dependencies also installed
  - Verify: Receipt contains runtime_dependencies
  - Verify: All binaries work
- [ ] Install keg-only formula (llvm)
  - Verify: No symlinks created to prefix
  - Verify: Warning message shown
  - Verify: /opt/homebrew/opt/llvm symlink created

**Test 2: upgrade() Operation**
- [ ] Upgrade outdated package
  - Verify: New version installed
  - Verify: Old version removed from Cellar
  - Verify: Symlinks point to new version
  - Verify: Old INSTALL_RECEIPT.json removed
  - Verify: Binary works
- [ ] Upgrade with dependencies changed
  - Verify: New dependencies installed if needed
  - Verify: Unused old dependencies remain (not autoremoved)
  - Verify: Receipt contains correct new dependencies
- [ ] Upgrade already-latest package
  - Verify: Returns early (no download)
  - Verify: No errors

**Test 3: reinstall() Operation**
- [ ] Reinstall package
  - Verify: Old version removed
  - Verify: Fresh bottle downloaded
  - Verify: New INSTALL_RECEIPT.json created
  - Verify: Binary works
  - Verify: Operation timing < 10 seconds (net from cache)
- [ ] Reinstall package with complex structure
  - Verify: All files extracted correctly
  - Verify: Scripts with shebangs work
  - Verify: All symlinks created

**Test 4: cleanup() Operation**
- [ ] cleanup(dry_run: true) with multiple versions
  - Verify: Reports what would be removed
  - Verify: No actual files deleted
  - Verify: Space calculation accurate
- [ ] cleanup(dry_run: false)
  - Verify: Actually removes old versions
  - Verify: Keeps newest version
  - Verify: Space actually freed
  - Verify: Linked version preserved (never deleted)
- [ ] cleanup() on large Cellar (100+ packages)
  - Measure: Time taken (goal: <1s)
  - Verify: No file descriptor leaks
  - Verify: All old versions cleaned

**Test 5: uninstall() Operation**
- [ ] Uninstall simple package
  - Verify: Binary no longer works
  - Verify: Cellar directory removed
  - Verify: Symlinks removed
  - Verify: /opt/homebrew/opt/<formula> removed
- [ ] Uninstall package with dependents
  - Verify: Clear error (cannot remove, has dependents)
  - Verify: Package remains installed

### Workflow Tests

**Test 6: Full Lifecycle (install → upgrade → cleanup → uninstall)**
```
1. Install package@1.0.0
   - Verify installed
   - Verify symlinks work
2. Upgrade to package@2.0.0  
   - Verify old version removed
   - Verify symlinks updated
3. Manually downgrade to package@1.5.0 via install
   - Verify cleanup preserves both 1.5.0 (linked) and 2.0.0 (keep newest)
4. cleanup() with dry_run
   - Verify reports both versions
5. cleanup() without dry_run
   - Verify keeps both (one is linked, other is newest)
6. Uninstall
   - Verify clean removal
```

**Test 7: Error Handling**
- [ ] Install nonexistent package
  - Verify: Clear "not found" error
- [ ] Upgrade with network failure
  - Verify: Graceful error (not panic)
  - Verify: No partial installation
- [ ] cleanup() on system with permission issues
  - Verify: Reports which packages failed
  - Verify: Doesn't stop at first error
- [ ] uninstall() package with no installation
  - Verify: Clear error

### Edge Cases

**Test 8: Bottle Revisions**
- [ ] Package with bottle revision (e.g., python@3.13/3.13.9_1)
  - Verify: Extracted correctly
  - Verify: Symlinks point to correct revision
  - Verify: upgrade finds matching revision

**Test 9: Keg-only Formulas**
- [ ] Install keg-only (llvm)
  - Verify: No prefix symlinks created
  - Verify: /opt/homebrew/opt/llvm created
  - Verify: Message shows "is keg-only"
- [ ] Uninstall keg-only
  - Verify: /opt/homebrew/opt/llvm removed

**Test 10: Dependencies**
- [ ] Install package with shared dependency
  - Example: mesa (depends on llvm) + llvm
  - Verify: llvm installed once
  - Verify: Both use same llvm
  - Verify: cleanup never removes llvm (has dependent)

### Performance Tests

**Test 11: Cleanup Performance**
- [ ] Measure cleanup on 100+ package Cellar
  - Goal: < 1 second
  - Measure: Directory traversal time
  - Measure: File removal time
  - Check: No file descriptor leaks

**Test 12: Operations Timing**
- [ ] install: < 5 seconds (cached bottle)
- [ ] upgrade: < 10 seconds (download + extract + symlink)
- [ ] reinstall: < 5 seconds (cached bottle)
- [ ] cleanup: < 1 second (100+ packages)

## Test Environment

**System**:
- macOS 15.7 (Sequoia)
- M3 Max
- ~339 packages installed

**Network**:
- 500+ Mbps connection (for download tests)
- Should test with network degradation if possible

**Test Packages**:
- Simple: ripgrep, wget, bat (no dependencies)
- Complex: graphicsmagick, librsvg (multiple deps)
- Keg-only: llvm, gcc
- Large: ffmpeg (100+ deps)
- Versioned: python@3.13, node@20

## Success Criteria

✅ **All 4 operations work without panics**
✅ **Full lifecycle workflow (install → upgrade → cleanup → uninstall) succeeds**
✅ **All edge cases (revisions, keg-only, dependencies) handled correctly**
✅ **Error paths graceful (no panics, clear messages)**
✅ **Performance acceptable (cleanup < 1s on 100+ packages)**
✅ **Binary compatibility** (all installed packages execute)
✅ **Receipt compatibility** (brew can read bru-generated receipts)
✅ **Symlink integrity** (all symlinks valid after operations)

## Failure Criteria (Blockers)

❌ Any panic or unwrap() triggered during normal operation
❌ Partial installations left after failed operations
❌ File descriptor leaks (increasing FDs during repeated operations)
❌ Cleanup deletes linked/newest version
❌ Symlinks point to non-existent paths
❌ cleanup on 100+ packages takes > 5 seconds

## Investigation Plan for Failures

If any test fails:

1. **Capture full output** with RUST_BACKTRACE=1
2. **Check Cellar state** before and after
3. **Verify symlink targets** (ls -la /opt/homebrew/opt/*)
4. **Check file counts** in formula directories
5. **Review INSTALL_RECEIPT.json** for correctness
6. **Profile if performance issue** using flamegraph

## Documentation Updates (Post-Testing)

After all tests pass:

- [ ] Update README with PackageManager usage examples
- [ ] Document error handling patterns
- [ ] Add performance characteristics to docs/
- [ ] Update AGENTS.md with v0.2.0 summary
- [ ] Create release notes for v0.2.0

## Timeline

- **Test setup**: 15 minutes
- **Core operations**: 45 minutes (tests 1-5)
- **Workflows**: 30 minutes (tests 6-7)
- **Edge cases**: 20 minutes (tests 8-10)
- **Performance**: 15 minutes (tests 11-12)
- **Documentation**: 30 minutes
- **Total**: ~2.5 hours

## Known Issues from Phase 2

None identified. All operations implement correct patterns:
- Use linked version (via symlink::get_linked_version)
- Respect keg-only flag
- Handle bottle revisions
- Generate proper receipts
- Create correct symlinks

## Next Phase (Phase 4)

After successful Phase 3:

1. **Batch operations** (install_multiple, upgrade_multiple)
2. **Performance optimization** if needed
3. **Metadata caching** (reduce API calls)
4. **Documentation polish**
5. **Release v0.2.0**
