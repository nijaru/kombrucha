# kombrucha (bru)

⚡ A blazingly fast Homebrew-compatible package manager written in Rust

## Features

- **Fast**: 7x faster than Homebrew for common commands
- **Efficient**: 15-85x less CPU usage
- **Compatible**: Works with all existing Homebrew formulae, taps, and infrastructure
- **Beautiful**: Colorized output with progress indicators
- **Parallel**: Concurrent API calls and downloads (coming in Phase 2)

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

See [BENCHMARKS.md](BENCHMARKS.md) for detailed results.

**Summary**:
- `bru info`: **7.2x faster** than `brew info`
- `bru search`: Same speed, **15x less CPU**
- Phase 2 target: **10-20x faster** for installs

## Why?

Homebrew is amazing but slow. On modern networks (100+ Mbps), Ruby overhead dominates execution time. bru eliminates this overhead while remaining 100% compatible.

Read the [performance analysis](internal/performance-analysis.md) for details.

## Documentation

- [SPEC.md](SPEC.md) - Original architecture specification
- [CLAUDE.md](CLAUDE.md) - Development guide
- [internal/](internal/) - Research and planning docs
- [BENCHMARKS.md](BENCHMARKS.md) - Performance results

## License

MIT OR Apache-2.0