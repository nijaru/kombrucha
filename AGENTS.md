# Kombrucha (bru)

*Fast, parallel Homebrew-compatible package manager written in Rust*

## Project Overview

**Kombrucha** is a high-performance Homebrew clone with the CLI command `bru`. It's 8x faster on average while maintaining full compatibility with Homebrew's formulae and infrastructure.

**Current Status**:
- Version: v0.2.2 (ready for release)
- CLI: Production-ready for bottle-based workflows
- Library: PackageManager API available for downstream projects
- Combined: 95% formulae via bottles, 5% fallback to brew for source builds

## Project Structure

| Directory | Purpose |
|-----------|---------|
| `docs/architecture/` | Permanent specs and architecture (user documentation) |
| `ai/` | **AI session context** - Agent workspace for maintaining state across sessions |
| `src/` | Rust implementation |
| `tests/` | Unit, integration, and regression tests |
| `scripts/` | Development and testing tools |
| `benchmarks/` | Performance testing |

### AI Context Organization

**Purpose**: AI uses `ai/` to maintain continuity between sessions. Read session files every session.

**Session files** (ai/ root - read FIRST every session):
- `STATUS.md` — Current state, metrics, blockers (read FIRST)
- `TODO.md` — Active tasks only
- `DECISIONS.md` — Active architectural decisions
- `RESEARCH.md` — Research findings index

**Reference files** (subdirectories - loaded only when needed):
- `research/` — Detailed research (>200 lines per topic)
- `profiles/` — Performance profiling data

Session files kept minimal (<500 lines) for token efficiency. Detailed content in subdirectories loaded on demand. Trust git history - delete completed/historical content from session files.

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
- ✅ **Phase 3 (Library)**: PackageManager API for programmatic access (production-ready, tested on 340+ packages)
- ✅ **Phase 4**: Core command implementation (tap, update, services, bundle)
- 🔴 **Phase 5 (Source)**: Ruby interop for source builds (planned, low priority)

## Design Principles

- **Performance First**: Parallel operations, compiled binary, instant startup
- **Pragmatic Compatibility**: Leverage existing Homebrew infrastructure
- **Better UX**: Clean CLI output, clear error messages
- **No Formula Translation**: Execute existing `.rb` files via embedded Ruby
- **Drop-in Replacement**: Users should be able to alias `brew` to `bru`

## Current Status

**Library API: PRODUCTION-READY** ✅

PackageManager API fully tested on production system (macOS 15.7, M3 Max, 340+ installed packages). All operations validated with zero panics and proper error handling.

See `ai/STATUS.md` for detailed status.

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

## Development Workflow

### Build & Test Commands

| Command | Purpose |
|---------|---------|
| `cargo build --release` | Production build |
| `cargo test` | Unit + regression tests |
| `cargo test -- --ignored` | Integration tests (requires Homebrew) |
| `cargo clippy` | Linting |
| `cargo fmt` | Format code |
| `cargo doc --open` | Generate and view documentation |

### Release Process

**Versioning Strategy**:
- Use commit hashes for references
- Bump versions only when instructed
- 0.1.0+ = production ready, 1.0.0 = proven in production
- Sequential bumps only: 0.0.1 → 0.0.2 → 0.1.0 → 1.0.0

**Release Steps** (wait for CI ✅):
1. Bump version, update docs → commit → push
2. `gh run watch` (wait for pass)
3. Tag: `git tag -a vX.Y.Z -m "Description" && git push --tags`
4. `gh release create vX.Y.Z --notes-file release_notes.md`
5. ASK before publishing to crates.io (can't unpublish)

If CI fails: delete tag/release, fix, restart

### Git Conventions

- NO AI attribution in commits/PRs (strip manually)
- Ask before: PRs, publishing packages, force ops
- Commit frequently, push regularly (no ask needed)
- Never force push to main/master
- Commit message format: `type: brief description`
  - Types: feat, fix, chore, docs, test, refactor

## Code Conventions

### Rust Standards

| Category | Standard |
|----------|----------|
| **Edition** | Rust 2024 |
| **Async** | `tokio` for network, sync for filesystem |
| **Errors** | `anyhow` for library, `thiserror` for custom types |
| **Allocations** | Avoid: use `&str` not `String`, `&[T]` not `Vec<T>` |
| **Testing** | `test_<feature>_<scenario>` naming |
| **Functions** | Focused and composable |

### Naming Conventions

- Concise, context-aware, no redundancy
- Proportional to scope (local: `count`, package: `user_count`)
- Omit redundant context (`Cache` not `LRUCache_V2`)
- Omit type info (`count` not `num_users`, `timeout` not `timeout_int`)
- Booleans: `is_enabled`, `has_data`
- Constants: `MAX_RETRIES`
- Units: `timeout_ms`, `buffer_kb`

### Comments

- Only WHY, never WHAT
- Write: non-obvious decisions, external requirements, algorithm rationale
- Never: change tracking, obvious behavior, TODOs

## Resources

- **Architecture**: `docs/architecture/SPEC.md`
- **Current State**: `ai/STATUS.md`
- **Active Tasks**: `ai/TODO.md`
- **Decisions**: `ai/DECISIONS.md`
- **Research Index**: `ai/RESEARCH.md`
