# kombrucha (bru)

⚡ A blazingly fast Homebrew-compatible package manager written in Rust

## Features

- **Fast**: 7x faster than Homebrew for info commands
- **Efficient**: 15x less CPU usage for search operations
- **Compatible**: Targets full compatibility with Homebrew formulae and infrastructure
- **Beautiful**: Colorized output with formatted results
- **Parallel**: Concurrent API calls (parallel downloads in Phase 2)

## Status: Phase 0 Complete ✅

**Working commands**:
- `bru search <query>` - Search formulae and casks
- `bru info <formula>` - Show formula/cask information
- `bru deps <formula>` - Show dependencies
- `bru deps --tree <formula>` - Show dependency tree

**Coming soon** (Phase 1-2):
- Parallel dependency resolution
- Bottle downloads and installation
- Full Homebrew command compatibility

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
```

## Performance

See [benchmarks/results.md](benchmarks/results.md) for detailed results.

**Phase 0 Benchmarks**:
- `bru info`: **7.2x faster** than `brew info` (1.45s → 0.20s)
- `bru search`: Same speed, **15x less CPU usage**
- Phase 2 estimate: **10-20x faster** for parallel installs

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