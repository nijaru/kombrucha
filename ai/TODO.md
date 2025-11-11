# TODO

## Phase 2 Completion (v0.2.0) - COMPLETE ✅

- [x] Implement uninstall() operation
- [x] Implement upgrade() operation  
- [x] Implement reinstall() operation
- [x] Implement cleanup() operation
- [x] Create example programs (6 total)
- [x] Compile and test all operations
- [x] Document Phase 2 in STATUS.md

## Phase 3: Integration Testing & Optimization

### High Priority

- [ ] Integration test PackageManager with live system
- [ ] Test install → upgrade → cleanup → uninstall workflow
- [ ] Performance profile cleanup on large installations (100+ packages)
- [ ] Verify error handling paths (network failures, missing packages)
- [ ] Test edge cases (bottle revisions, keg-only, pinned packages)

### Documentation

- [ ] Update README with library usage section
- [ ] Add PackageManager examples to docs/
- [ ] Document API for downstream projects
- [ ] Add performance characteristics section
- [ ] Update AGENTS.md with Phase 2 results

### Code Quality

- [ ] Add integration tests for all 4 operations
- [ ] Profile cleanup directory traversal
- [ ] Consider optimization (parallel walks, early termination)
- [ ] Review error messages for clarity

## Phase 4+: Advanced Features (Backlog)

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
