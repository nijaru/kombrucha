# TODO

## Phase 2 Completion (v0.2.0) - COMPLETE ✅

- [x] Implement uninstall() operation
- [x] Implement upgrade() operation  
- [x] Implement reinstall() operation
- [x] Implement cleanup() operation
- [x] Create example programs (6 total)
- [x] Compile and test all operations
- [x] Document Phase 2 in STATUS.md

## Phase 3: Integration Testing & Optimization - COMPLETE ✅

### Testing Completed ✅

- [x] Integration test PackageManager with live system (340+ packages)
- [x] Test install → upgrade → cleanup → uninstall workflow (all passed)
- [x] Performance profile on large installations (acceptable characteristics)
- [x] Verify error handling paths (zero panics, proper Results returned)
- [x] Test edge cases (multi-versioned packages, "already at latest", cleanup dry-run)
- [x] Create comprehensive test report (ai/PHASE_3_TEST_REPORT.md)

### Documentation Added ✅

- [x] Created test artifacts (5 example test programs)
- [x] Documented performance findings in test report
- [x] Documented edge cases and their handling
- [x] Added test artifacts to examples/

## Phase 4: Release & Documentation (CURRENT)

### High Priority

- [ ] Update CHANGELOG with v0.2.0 entry
- [ ] Tag release: `git tag -a v0.2.0 -m "PackageManager library release"`
- [ ] Update README with library usage section
- [ ] Add PackageManager examples to docs/
- [ ] Publish to crates.io: `cargo publish`

### Documentation

- [ ] Create docs/library-api.md with usage patterns
- [ ] Document PackageManager result types
- [ ] Add performance characteristics section
- [ ] Update AGENTS.md with Phase 3 completion
- [ ] Add examples to README (install, upgrade, cleanup workflows)

### Code Quality

- [ ] Review test artifacts (keep in examples/ for reference)
- [ ] Consider code cleanup in tests if needed
- [ ] Verify no debug code left in main branch

## Phase 5+: Advanced Features (Backlog)

### Phase 3: Ruby Interop
- [ ] Design Ruby interop approach for source builds
- [ ] Implement source build support (~5% remaining formulae)
- [ ] Test source build integration

### Testing Improvements (Lower Priority)
- [ ] Large package stress tests (1000+ files like yt-dlp)
- [ ] Resource limit monitoring tests
- [ ] File descriptor leak detection tests
- [ ] Download module tests (parallel downloads, checksum verification)
- [ ] Commands module tests (127 functions currently untested)

### Future Enhancements
- [ ] Performance optimizations for large dependency graphs
- [ ] Better error messages for common failure modes
- [ ] Lock file support for reproducibility
