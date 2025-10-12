# kombrucha (bru)

⚡ A blazingly fast Homebrew-compatible package manager written in Rust

## Features

- **Fast**: 7x faster than Homebrew for info commands
- **Efficient**: 15x less CPU usage for search operations
- **Compatible**: Targets full compatibility with Homebrew formulae and infrastructure
- **Beautiful**: Colorized output with formatted results
- **Parallel**: Concurrent API calls (parallel downloads in Phase 2)

## Status: Phase 2 Complete + Phase 3/4 Features ✅

**Working commands**:
- `bru search <query>` - Search formulae and casks
- `bru info <formula>` - Show formula/cask information
- `bru deps <formula>` - Show dependencies
- `bru deps --tree <formula>` - Show dependency tree
- `bru uses <formula>` - Show formulae that depend on a formula
- `bru list` - List installed packages
- `bru list --versions` - Show all installed versions
- `bru outdated` - Show outdated packages
- `bru fetch <formula>...` - Download bottles with parallel downloads
- `bru install <formula>...` - Install packages from bottles with full dependency resolution
- `bru upgrade [formula...]` - Upgrade installed packages (all or specific)
- `bru reinstall <formula>...` - Reinstall packages
- `bru uninstall <formula>...` - Uninstall packages (with dependency checking)
- `bru cleanup [formula...]` - Remove old versions of installed packages
- `bru tap [user/repo]` - Add or list third-party repositories
- `bru untap <user/repo>` - Remove a third-party repository
- `bru config` - Show system configuration and statistics
- `bru doctor` - Check system for potential problems
- `bru home <formula>` - Open formula homepage in browser
- `bru leaves` - List packages not required by others
- `bru pin <formula>...` - Pin formulae to prevent upgrades
- `bru unpin <formula>...` - Unpin formulae to allow upgrades

**Coming soon** (Phase 3):
- Source builds for formulae without bottles

## Installation

```bash
# Clone the repo
git clone https://github.com/nijaru/kombrucha.git
cd kombrucha

# Build release binary
cargo build --release

# Binary is at: ./target/release/bru
```

## Quick Start

```bash
# Search for packages
bru search rust

# Get info about a package
bru info wget

# Show dependencies
bru deps --tree wget

# See what depends on a package
bru uses openssl

# List installed packages
bru list

# Check for outdated packages
bru outdated

# Install a package
bru install hello

# Uninstall a package
bru uninstall hello

# Clean up old versions
bru cleanup

# List taps (third-party repositories)
bru tap

# Add a tap
bru tap user/repo

# Remove a tap
bru untap user/repo

# Download bottles without installing
bru fetch wget jq tree
```

## Performance

See [benchmarks/results.md](benchmarks/results.md) and [benchmarks/phase2-results.md](benchmarks/phase2-results.md) for detailed results.

**Phase 0 Benchmarks** (read-only commands):
- `bru info`: **7.2x faster** than `brew info` (1.45s → 0.20s)
- `bru search`: Same speed, **15x less CPU usage**

**Phase 2 Benchmarks** (installation):
- `bru install`: **21-60x faster** than `brew install`
  - Normal usage (with auto-update): **60x faster** (8.3s → 0.14s)
  - Pure install (no auto-update): **21x faster** (2.9s → 0.14s)
  - **100x less CPU usage** (2.1s → 0.02s user time)

## Why?

Homebrew is amazing but slow. On modern networks (100+ Mbps), Ruby overhead dominates execution time. bru eliminates this overhead while targeting full compatibility with Homebrew's formulae and infrastructure.

See [performance analysis](internal/performance-analysis.md) for the technical breakdown.

## Documentation

- [CLAUDE.md](CLAUDE.md) - Development guide
- [internal/](internal/) - Research, planning, and specifications
  - [SPEC.md](internal/SPEC.md) - Original architecture specification
  - [implementation-roadmap.md](internal/implementation-roadmap.md) - Phased implementation plan
  - [research-conclusions.md](internal/research-conclusions.md) - All research findings
- [benchmarks/](benchmarks/) - Performance testing and results
  - [results.md](benchmarks/results.md) - Benchmark results vs Homebrew
- [scripts/](scripts/) - Development and testing scripts

## License

MIT OR Apache-2.0