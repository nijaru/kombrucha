# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Kombrucha** is a fast Homebrew clone written in Rust, with the CLI command `bru`. The goal is to create a high-performance, user-friendly package manager that works with all existing Homebrew formulae and infrastructure, allowing users to freely swap between `brew` and `bru`.

**Documentation Structure**:
- This file: Development guide for working on kombrucha
- `internal/`: Research, specifications, and planning docs
- `benchmarks/`: Performance testing and results
- `scripts/`: Development and testing scripts

### Project Goals

- **Performance**: Dramatically faster than Ruby-based Homebrew through parallel operations and compiled code
- **Better DX**: Improved developer experience with pretty-printed CLI output and better UX
- **Better Dependency Management**: Improved resolver with clearer conflict messages and optional advanced features
- **Compatibility**: Ideally a drop-in replacement for Homebrew; at minimum, full compatibility with existing formulae, taps, and infrastructure
- **Ecosystem Integration**: Leverage all existing Homebrew resources (JSON API, bottles, formula repos)

### Key Performance Opportunities

**Homebrew's Known Bottlenecks** (from `internal/research-findings.md`):
1. **Sequential Downloads**: Homebrew only recently (4.6.0, 2025) added opt-in concurrent downloads
2. **Sequential Dependency Resolution**: Metadata fetched one-by-one
3. **No Parallel Extraction**: Bottles extracted sequentially
4. **Slow Startup**: Ruby interpreter overhead (~0.6s)

**Kombrucha's Advantages**:
- Parallel everything from day 1 (tokio + Rust)
- Instant startup (compiled binary)
- Better dependency resolver (SAT or backtracking algorithm)
- Beautiful progress indicators showing concurrent operations

## Core Architecture

### Hybrid Rust + Ruby Design

The system uses a **Rust core** for performance-critical operations and an **embedded Ruby interpreter** for formula evaluation:

- **Rust Core** (`kombrucha` binary): Handles CLI, JSON API interactions, dependency resolution, bottle management, and concurrent operations
- **Ruby Interop Layer**: Embeds Ruby (via `magnus` crate) to execute formula DSL (`.rb` files) for source builds
- **Homebrew Infrastructure**: Continues to use existing formulae, taps, JSON API (`formulae.brew.sh`), and bottles

### Key Components (from SPEC.md:8-30)

1. **CLI Parsing & Command Dispatch** - using `clap`
2. **Concurrent Operations Manager** - using `tokio`
3. **Dependency Resolution Engine** - custom parallel resolver
4. **JSON API Client & Parser** - using `reqwest` and `serde`
5. **Bottle Manager** - handles binary package downloads/extraction
6. **Ruby Interop Layer** - using `magnus` with embedded `libruby`

### Installation Workflow

When a user runs `bru install <formula>`:

1. Rust CLI parses command
2. JSON API queried for formula metadata
3. Dependency graph built in parallel
4. For each dependency:
   - If bottle exists: Rust downloads/extracts `.tar.gz` in parallel
   - If no bottle: Ruby interpreter loads `.rb` file and runs `install` method
5. Dependencies installed in correct order

## Development Phases (from SPEC.md:46-60)

**Phase 0** ✅: Foundation (CLI scaffolding, API client, basic commands)
**Phase 1** ✅: Read-only commands (`search`, `info`, `deps`, `uses`, `list`, `outdated`)
**Phase 2**: Bottle-based installation (install with bottles only)
**Phase 3**: Ruby interop for source builds (full `install`, `uninstall`, `upgrade`)
**Phase 4**: Complete command compatibility (all Homebrew commands)

## Project Structure

Follow Homebrew's main repo layout for general organization. Key directories:

- `internal/` - AI agent and dev docs (organized in subdirs by category)
- Standard Rust project structure (`src/`, `tests/`, etc.)

## Recommended Rust Crates

**Core Functionality** (from SPEC.md:61-68):
- **CLI**: `clap` for argument parsing
- **Async Runtime**: `tokio` for concurrency
- **HTTP Client**: `reqwest` for parallel downloads
- **JSON**: `serde`, `serde_json` for API parsing
- **Archive Handling**: `tar`, `flate2` for bottle extraction
- **Ruby Interop**: `magnus` (for Phase 3+)

**UX Enhancements**:
- **Progress Bars**: `indicatif` for download/install progress
- **Colors**: `colored` or `owo-colors` for output
- **Tables**: `comfy-table` or `tabled` for formatted output
- **Trees**: `ptree` or custom impl for dependency visualization

**Dependency Resolution**:
- **SAT Solver**: `resolvo` (libsolv bindings) for advanced resolution
- **Alternative**: Implement Cargo-style backtracking resolver
- **Semver**: `semver` crate for version parsing/comparison

## Development Commands

```bash
cargo build              # Build the project
cargo build --release    # Build optimized release binary
cargo run                # Run the CLI
cargo run -- <args>      # Run with arguments (e.g., cargo run -- search rust)
cargo test               # Run tests
cargo test <name>        # Run specific test
cargo clippy             # Lint
cargo fmt                # Format code
cargo check              # Quick compile check (faster than build)

# After building, binary is at:
./target/debug/bru       # Debug build
./target/release/bru     # Release build
```

## Design Principles

- **Pragmatic Compatibility**: Don't rewrite the ecosystem - leverage existing Homebrew infrastructure
- **Performance First**: Use Rust's concurrency and type safety for speed and reliability
- **Parallel by Default**: What Homebrew does sequentially, we do concurrently
- **UX Matters**: Pretty CLI output, clear error messages, and improved developer experience
- **Better Dependency Management**: Smart resolver with clear conflict resolution and optional advanced features
- **Incremental Adoption**: Phased rollout allows gradual replacement of Homebrew components
- **No Formula Translation**: Keep all `.rb` files as-is - execute them via embedded Ruby
- **Drop-in Replacement Goal**: Users should be able to alias `brew` to `bru` seamlessly

## Known Homebrew Pain Points to Address

See `internal/research-findings.md` for detailed analysis. Key issues:

1. **Dependency Management**:
   - Forces everything to latest version (no version flexibility)
   - Keg-only formulas confusing and fragile after upgrades
   - Poor conflict resolution messages
   - Doesn't cooperate with external tooling (rustup, nvm, etc.)

2. **Performance**:
   - Sequential downloads (only recently added opt-in concurrency)
   - No parallel dependency resolution
   - Sequential bottle extraction

3. **User Experience**:
   - Unclear why certain decisions made
   - Dependency tree hard to visualize
   - No reproducibility (no lock file)
