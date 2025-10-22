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

See `ai/TODO.md` for active tasks and `ai/STATUS.md` for project state.

**Recent work** (2025-10-21):
- Fixed upgrade bugs (duplicates, bottle revision false positives)
- Improved testing (16 tests run automatically)
- Modernized CLI output (removed decorative symbols)

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
