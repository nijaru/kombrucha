# Active Tasks

**Last Updated**: January 26, 2026

## Current Sprint (Sprint 7: Formula DSL Completion)

- [ ] TASK-25: Implement `stable` block and version extraction
- [ ] TASK-26: Map `url` and `sha256` to Rust downloaders
- [ ] TASK-27: Implement resources and patches
- [ ] TASK-28: Add support for build-time options

## Future Sprints

### Sprint 8: Full Source Build Integration
- [ ] Replace `brew` source fallback with native Ruby builds
- [ ] Implement `caveats` and `post_install`
- [ ] Add comprehensive integration tests for source builds

## Completed Tasks

### Sprint 6: Source Build Execution (MVP)
- [x] TASK-22: Implement Ruby-to-Rust bridge for DSL methods
- [x] TASK-23: Execute `install` method for a formula
- [x] TASK-24: Integrate source fallback in main installation loop

### Sprint 5: Ruby Interop Foundation
- [x] TASK-19: Setup Ruby VM with `magnus`
- [x] TASK-20: Implement Formula DSL loading
- [x] TASK-21: Map Formula metadata to Ruby objects

### Sprint 4: Performance & Modularization
- [x] TASK-15: Parallelize `outdated()` queries
- [x] TASK-16: Implement parallel extraction in `install`
- [x] TASK-17: Modularize `main.rs`
- [x] TASK-18: Implement batch dependency lookups

## Planned Features

### Release
- [ ] Update CHANGELOG.md for v0.2.5
- [ ] Tag v0.2.5

### Future Improvements
- [ ] Content-addressable store for faster warm installs
- [ ] TOCTOU hardening in symlinks
- [ ] Support for non-core taps in native Rust resolution