# kombrucha (bru)

⚡ A blazing fast Homebrew-compatible package manager

## Status: Beta

- **Core Commands**: Fully functional package management (install, uninstall, upgrade)
- **Bottle-Based**: Works with 95% of Homebrew formulae (those with bottles)
- **Well-Tested**: 32 automated tests covering critical workflows
- **Source Builds**: Not yet supported (Phase 3 feature)

See [internal/status-report.md](internal/status-report.md) for detailed status.

## Features

- **Fast**: 2-20x faster than Homebrew (8x average, verified benchmarks)
- **Efficient**: Minimal CPU usage with compiled binary
- **Compatible**: Works with Homebrew formulae and infrastructure (bottle-based)
- **Beautiful**: Colorized output with NO_COLOR support and pipe-aware formatting
- **Parallel**: Concurrent operations for maximum performance
- **Robust**: Graceful error handling and partial failure recovery

## What Works ✅

**Package Management** (fully functional):
- `install`, `uninstall`, `upgrade`, `reinstall` - Bottle-based formulae
- `install --cask`, `uninstall --cask` - macOS applications (DMG, ZIP, PKG)
- `fetch`, `list`, `outdated`, `autoremove`, `cleanup`
- `pin`, `unpin` - Version locking
- Full dependency resolution and graph traversal

**Discovery & Information** (fully functional):
- `search`, `info`, `desc`, `deps`, `uses`
- `list`, `leaves`, `missing`, `formulae`, `casks`, `unbottled`
- `which-formula`, `options`, `cat`, `log`

**Repository Management** (fully functional):
- `tap`, `untap`, `tap-info`, `tap-new`, `update`
- `extract`, `migrate`, `readall`, `tap-readme`

**System & Utilities** (fully functional):
- `config`, `doctor`, `env`, `home`, `shellenv`
- `cache`, `analytics`, `commands`, `completions`
- `services` - launchd integration for background services
- `bundle` - Brewfile install and dump

**Development Tools** (not implemented - use `brew` instead):
- `create`, `audit`, `livecheck`, `linkage`, `test`, `style`

## What Doesn't Work ❌

**Phase 3: Source Builds** (not implemented):
- Formulae without bottles (~1-5% of packages)
- Building from source (`--build-from-source` flag)
- Head installations (`--HEAD` flag)
- Custom build options
- Formula testing (`bru test`)

**Workaround**: Use `brew` for these edge cases or wait for Phase 3 (Ruby interop)

## Installation

```bash
# Clone the repo
git clone https://github.com/nijaru/kombrucha.git
cd kombrucha

# Build release binary
cargo build --release

# Binary is at: ./target/release/bru

# Optional: Set up shell completions
# For bash:
bru completions bash > ~/.local/share/bash-completion/completions/bru

# For zsh:
bru completions zsh > ~/.zfunc/_bru

# For fish:
bru completions fish > ~/.config/fish/completions/bru.fish
```

## Quick Start

```bash
# Search for packages
bru search rust

# Search only formulae or casks
bru search --formula python
bru search --cask docker

# Get info about a package
bru info wget

# Quick description lookup
bru desc wget jq curl

# Show dependencies
bru deps --tree wget

# Show only installed dependencies
bru deps --installed wget

# See what depends on a package
bru uses openssl

# Get JSON output (for scripting)
bru info --json wget

# List installed packages
bru list

# Check for outdated packages
bru outdated

# Install a package
bru install hello

# Uninstall a package
bru uninstall hello

# Remove unused dependencies
bru autoremove

# Clean up old versions
bru cleanup

# List taps (third-party repositories)
bru tap

# Add a tap
bru tap user/repo

# Remove a tap
bru untap user/repo

# Update all taps
bru update

# Download bottles without installing
bru fetch wget jq tree
```

## Performance

**Verified Benchmarks** (October 21, 2025) - M3 Max, macOS 15.1, 500 Mbps:

| Command | brew | bru | Speedup |
|---------|------|-----|---------|
| `search rust` | 1.03s | 0.050s | **20.6x** |
| `info wget` | 1.15s | 0.096s | **12.0x** |
| `deps ffmpeg` | 1.26s | 0.15s | **8.4x** |
| `install --dry-run python@3.13` | 1.20s | 0.25s | **4.8x** |
| `outdated` (251 packages) | 1.97s | 0.98s | **2.0x** |
| `list` | 0.030s | 0.020s | **1.5x** |

**Average speedup: 8x across common operations**

See [internal/performance-analysis.md](internal/performance-analysis.md) for detailed analysis and methodology.

## Why?

Homebrew is amazing but slow. On modern networks (100+ Mbps), Ruby overhead dominates execution time. bru eliminates this overhead while targeting full compatibility with Homebrew's formulae and infrastructure.

See [performance analysis](internal/performance-analysis.md) for the technical breakdown.

## Testing

```bash
# Unit tests
cargo test

# Integration tests (install/uninstall workflow)
./scripts/test-integration.sh

# Smoke tests (quick validation)
./scripts/test-smoke.sh

# Run benchmarks
./benchmarks/phase2-install.sh  # Simple package
./benchmarks/phase2-complex.sh  # Package with dependencies
```

See [scripts/README.md](scripts/README.md) for detailed testing documentation.

## Documentation

**Current Status**:
- [internal/status-report.md](internal/status-report.md) - **START HERE** - Complete status overview
- [internal/test-report.md](internal/test-report.md) - Comprehensive testing results
- [internal/feature-parity-audit.md](internal/feature-parity-audit.md) - Command-by-command tracking

**Development**:
- [CLAUDE.md](CLAUDE.md) - Development guide for AI agents
- [internal/README.md](internal/README.md) - Index of all internal documentation
- [internal/SPEC.md](internal/SPEC.md) - Technical architecture specification
- [internal/implementation-roadmap.md](internal/implementation-roadmap.md) - Phased implementation plan

**Research & Performance**:
- [internal/research-conclusions.md](internal/research-conclusions.md) - Research findings
- [internal/performance-analysis.md](internal/performance-analysis.md) - Performance breakdown
- [benchmarks/](benchmarks/) - Performance testing and results

**Testing**:
- [scripts/README.md](scripts/README.md) - Testing infrastructure documentation
- [internal/testing-strategy.md](internal/testing-strategy.md) - Testing approach

## License

MIT OR Apache-2.0