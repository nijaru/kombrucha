# Architectural Decisions

## 2025-10-21: Use non-ignored regression tests with --dry-run

**Context**: Integration tests all marked #[ignore] didn't run in CI, missed critical bugs
**Decision**: Add regression tests using --dry-run that can run without system modification
**Rationale**:
- Tests are worthless if they don't run
- --dry-run allows testing logic without actual installs
- Caught upgrade duplicates and bottle revision bugs
**Tradeoffs**: Not full integration tests, but better than nothing

---

## 2025-10-21: Remove decorative CLI symbols (→ ⬇ ⬆)

**Context**: Modern CLIs (cargo, uv) don't use emoji/arrow symbols
**Decision**: Remove decorative arrows, keep checkmarks/warnings
**Rationale**:
- Cleaner, more professional appearance
- Easier to grep/parse output
- Follows modern CLI conventions
**Tradeoffs**: Less "fun" but more functional

---

## 2025-10-08: Use JSON API over tap parsing

**Context**: Need to fetch formula metadata
**Decision**: Use formulae.brew.sh JSON API instead of parsing local taps
**Rationale**:
- Always up-to-date (no brew update needed)
- Faster than git operations
- Simpler implementation
- Same data source as Homebrew's web interface
**Tradeoffs**: Network dependency, but acceptable for package manager

---

## 2025-10-08: Bottle-first strategy (defer source builds to Phase 3)

**Context**: 95% of formulae have bottles, 5% require source builds
**Decision**: Implement bottle support first, defer source builds
**Rationale**:
- Covers 95% of use cases immediately
- Source builds require Ruby interop (complex)
- MVP in 8 weeks vs 20 weeks for full implementation
**Tradeoffs**: Can't install unbottled formulae until Phase 3

---

## 2025-10-08: Hybrid Rust + Ruby architecture

**Context**: Need Homebrew compatibility but want Rust performance
**Decision**: Rust core with embedded Ruby for formula evaluation
**Rationale**:
- Rust: Performance, parallel operations, type safety
- Ruby: Compatibility with existing .rb formulae
- No need to rewrite 7,000+ formulae
- Leverage Homebrew ecosystem
**Tradeoffs**: Complexity of embedding Ruby, but necessary for compatibility

---

## 2025-10-08: Parallel operations by default

**Context**: Homebrew is sequential, network isn't bottleneck on modern connections
**Decision**: Use tokio for concurrent downloads, installs, dependency resolution
**Rationale**:
- 10-20x performance improvement opportunity
- Modern networks (100+ Mbps) aren't the bottleneck
- Rust + tokio make this natural
**Tradeoffs**: More complex error handling, but worth it for performance

---
