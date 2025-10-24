# Kombrucha (bru)

*Fast, parallel Homebrew-compatible package manager written in Rust*

## Project Overview

**Kombrucha** is a high-performance Homebrew clone with the CLI command `bru`. The goal is to create a drop-in replacement that's 8x faster on average while maintaining full compatibility with Homebrew's formulae and infrastructure.

**Status**: v0.1.0 Beta - Production-ready for bottle-based workflows (95% of formulae)

## Project Structure

- **Documentation**: `docs/architecture/` - Permanent specs and architecture
- **AI Working Context**: `ai/` - Tasks, status, decisions, research
- **Source Code**: `src/` - Rust implementation
- **Tests**: `tests/` - Unit, integration, and regression tests
- **Scripts**: `scripts/` - Development and testing tools
- **Benchmarks**: `benchmarks/` - Performance testing

## Quick Start

```bash
# Build
cargo build --release

# Test
cargo test                          # Unit + regression tests
cargo test -- --ignored            # Integration tests (requires Homebrew)

# Run
./target/release/bru --version
./target/release/bru search rust
```

## Core Architecture

**Hybrid Rust + Ruby Design**:
- Rust core: CLI, JSON API, dependency resolution, bottle management, parallel operations
- Ruby interop (Phase 3): Execute formula DSL for source builds
- Homebrew infrastructure: Existing formulae, taps, JSON API, bottles

**Key Technologies**:
- CLI: `clap`
- Async: `tokio` for parallel operations
- HTTP: `reqwest` for concurrent downloads
- JSON: `serde`, `serde_json` for API parsing
- Archives: `tar`, `flate2` for bottle extraction

See `docs/architecture/SPEC.md` for detailed design.

## Development Phases

- ✅ **Phase 0**: Foundation (CLI scaffolding, API client)
- ✅ **Phase 1**: Read-only commands (search, info, deps, uses, list, outdated)
- ✅ **Phase 2**: Bottle-based installation (install, uninstall, upgrade)
- ✅ **Phase 4**: Core command implementation (tap, update, services, bundle)
- 🔴 **Phase 3**: Ruby interop for source builds (not started)

## Design Principles

- **Performance First**: Parallel operations, compiled binary, instant startup
- **Pragmatic Compatibility**: Leverage existing Homebrew infrastructure
- **Better UX**: Clean CLI output, clear error messages
- **No Formula Translation**: Execute existing `.rb` files via embedded Ruby
- **Drop-in Replacement**: Users should be able to alias `brew` to `bru`

## Current Focus

**CRITICAL: Testing Infrastructure Overhaul** (2025-10-24)

Our integration tests caused **system corruption** by directly modifying `/opt/homebrew/Cellar/` without isolation. This corrupted the node binary, mise shims, and Claude Code on Oct 23, 2025.

**Status**: Comprehensive remediation plan completed in `ai/TESTING_REMEDIATION.md`

**What We're Doing Wrong**:
- Tests modify real system directories (violates Homebrew best practices)
- No isolation via testcontainers or brew test-bot
- Formula test block is trivial (`--version` check - Homebrew considers this a "bad test")
- Missing GitHub Actions workflows for automated bottle building

**Remediation Plan** (State-of-the-Art):
- Phase 1 (P0): Delete dangerous tests, implement testcontainers-rs for Docker isolation
- Phase 2 (P1): Add brew test-bot --local workflow, GitHub Actions for bottles
- Phase 3 (P2): Comprehensive test suite with proper functional domain organization

See `ai/STATUS.md` for detailed status and `ai/TESTING_REMEDIATION.md` for complete plan.

## Performance

Verified benchmarks (M3 Max, macOS 15.1, 500 Mbps):
- search: 20.6x faster
- info: 12.0x faster
- deps: 8.4x faster
- Average: 8x faster

See `ai/research/performance-analysis.md` for detailed analysis.

## Known Issues

**Phase 3 Blocker**: Source builds not implemented
- Affects ~5% of formulae without bottles
- Workaround: Use `brew` for these edge cases
- Planned: Embed Ruby via `magnus` crate

See `docs/architecture/feature-parity-audit.md` for complete command coverage.

## Code Conventions

- Follow Rust 2024 edition idioms
- Use `tokio` for all async operations
- Prefer `anyhow` for error handling
- Test naming: `test_<feature>_<scenario>`
- Keep functions focused and composable

## Resources

- **Architecture**: `docs/architecture/SPEC.md`
- **Current State**: `ai/STATUS.md`
- **Active Tasks**: `ai/TODO.md`
- **Decisions**: `ai/DECISIONS.md`
- **Research Index**: `ai/RESEARCH.md`
