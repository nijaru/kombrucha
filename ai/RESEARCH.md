# Research Index

## Homebrew Performance Analysis (researched 2025-10-08)

**Sources**: Homebrew GitHub issues, performance profiling
**Key Findings**:
- Ruby overhead: ~0.6s startup + 0.4s lib loading
- Sequential operations dominant on fast networks (100+ Mbps)
- Network download NOT the bottleneck (contrary to maintainer claims)
**Relevance**: Core motivation for kombrucha
**Decision**: Build parallel-first package manager in Rust
→ Details: ai/research/performance-analysis.md

## Homebrew Compatibility Requirements (researched 2025-10-08)

**Sources**: Homebrew documentation, source code analysis
**Key Findings**:
- JSON API provides 99% of needed data
- Cellar structure well-documented
- Tap structure standardized
**Relevance**: Ensures we can be drop-in replacement
**Decision**: Follow Homebrew file layout exactly
→ Details: ai/research/homebrew-compatibility.md

## Bottle-Based Installation Coverage (researched 2025-10-08)

**Sources**: Homebrew formula analysis
**Key Finding**: 95% of formulae have pre-built bottles
**Relevance**: Can defer source builds to Phase 3
**Decision**: Bottle-first implementation strategy
→ Details: internal/research-findings.md (to be moved)

## Edge Cases and Platform Compatibility (researched 2025-10-20)

**Sources**: Testing on various macOS versions, formula edge cases
**Key Findings**:
- Keg-only formulae need special handling
- Multiple installed versions common
- Bottle revisions in version strings
**Relevance**: Caught critical bugs in outdated/upgrade commands
**Decision**: Add deduplication and version normalization
→ Details: ai/research/homebrew-edge-cases.md

## Open Questions

- [ ] Optimal cache eviction strategy for API responses
- [ ] Lock file format for reproducibility
- [ ] Ruby embedding best practices (for Phase 3)
- [ ] Progress indicator improvements (OSC 9;4 support)
