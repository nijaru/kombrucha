# TODO

## High Priority

- [x] Add more non-ignored regression tests
- [x] Improve CI test coverage
- [x] Add symlink module tests (path normalization)
- [x] Add cache module tests (directory detection, TTL logic)

## Testing

- [x] Add test for install command (--dry-run based)
- [x] Add tests for search, info, deps, list commands
- [x] Add tests for autoremove and cleanup (--dry-run)
- [x] Add test for fetch error handling
- [x] Add test for help and version commands
- [x] Set up CI to run integration tests on macOS runner

## Documentation

- [x] Create ai/ directory structure
- [x] Create STATUS.md with current state
- [x] Create DECISIONS.md with architectural decisions
- [x] Create RESEARCH.md index
- [x] Move permanent specs to docs/architecture/
- [x] Archive old session files
- [x] Rename CLAUDE.md → AGENTS.md
- [x] Update AGENTS.md to follow new template
- [x] Update README.md with new structure
- [x] Update STATUS.md with v0.1.6 and v0.1.7 changes
- [x] Remove dated summary files (CODE_REVIEW_2025_10_22.md)
- [x] Clean up release notes from root directory

## Future (Backlog)

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
