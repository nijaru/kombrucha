# bru

**A fast, Homebrew-compatible package manager**

**Faster than Homebrew** with modern UX. Works with Homebrew's formulae and bottles.

```bash
# Install packages with command aliases
bru i ripgrep fd bat
bru up              # upgrade all
bru rm old-package  # uninstall

# Same commands, same formulae, just faster
bru install wget    # works exactly like brew
```

## Why bru?

| Feature | Homebrew | bru |
|---------|----------|-----|
| `outdated` | 1.63s | **0.78s** (2.1x faster) |
| `info wget` | 1.04s | **0.11s** (9.6x faster) |
| `search python` | 1.04s | **0.04s** (24x faster) |
| `update` (8 taps) | 3.2s | **1.9s** (parallel, 5.7x faster) |
| `upgrade` (multi-package) | Sequential | **3-8x faster** (parallel downloads) |
| Startup time | ~100ms | **<10ms** |
| Memory usage | Higher | **Lower** |
| Compatibility | ✅ | ✅ **100% compatible** |

**Bottom line:** Same formulae, same ecosystem, dramatically faster.

## Status: Experimental / Unstable (v0.1.30)

⚠️ **WARNING**: This is experimental software under active development. While core functionality works, there may be bugs that could affect your system.

**Use at your own risk. Always keep Homebrew (`brew`) installed as a fallback.**

- ✅ **Core Commands**: Functional (install, upgrade, uninstall, cleanup, etc.)
- ✅ **100% Formula Coverage**: Bottles (95%) + automatic brew fallback (5%)
- ✅ **Custom Tap Support**: Works with third-party taps (delegated to brew)
- ✅ **Tested**: 76 unit tests + 14 regression tests, CI verification
- ⚠️ **Experimental**: Use alongside Homebrew, report issues
- ✅ **Modern CLI**: Clean output matching cargo/uv, live progress updates
- ✅ **Optimized**: Parallel operations, HTTP/2 connection pooling

**Recent critical fix (v0.1.30)**: Fixed autoremove bug that could incorrectly remove required dependencies. Receipts now correctly track runtime dependencies after upgrade/reinstall.

## Installation

### Via Homebrew (Easiest)

```bash
brew install nijaru/tap/bru
```

### Via Cargo

```bash
cargo install kombrucha
```

### From Source

```bash
git clone https://github.com/nijaru/kombrucha.git
cd kombrucha
cargo build --release
sudo cp target/release/bru /usr/local/bin/
```

## Quick Start

```bash
# Use command aliases for speed
bru i wget           # install
bru up              # upgrade all
bru re wget         # reinstall
bru rm wget         # uninstall

# Or use full commands (same as brew)
bru install wget
bru upgrade
bru uninstall wget

# Everything else works the same
bru search rust
bru info python
bru list
bru outdated
```

## What's Different?

### Performance

**Fully parallelized** - All API operations happen concurrently:
- In-memory caching eliminates redundant API calls
- Dependency resolution uses breadth-first parallelization
- All fetches, validations, and checks run in parallel

**Result:** Faster than Homebrew across common operations (see benchmarks below).

### Modern UX

**Tree connectors** show clear operation hierarchy:
```
Installing sccache...
  ├ ✓ Linked 1 files
  └ ✓ Installed sccache 0.12.0
```

**Command aliases** for faster workflow:
- `bru i` → install
- `bru up` → upgrade
- `bru re` → reinstall
- `bru rm` / `bru remove` → uninstall

**Clear messaging** - Shows what's already installed:
```
Already installed:
  python@3.14 3.14.0
  openssl@3 3.6.0

Use --force to reinstall
```

### Optimizations

Every sequential operation has been parallelized:
1. ✅ Upgrade checks
2. ✅ Fetch metadata
3. ✅ Install validation
4. ✅ Dependency resolution
5. ✅ Cask operations
6. ✅ Downloads (with progress bars)
7. ✅ In-memory caching (LRU, 1000 formulae + 500 casks)

## What Works ✅

### Core Package Management
- `install`, `uninstall`, `upgrade`, `reinstall` - All bottle-based formulae
- `install --cask`, `upgrade --cask` - macOS applications (DMG, ZIP, PKG)
- `fetch`, `list`, `outdated`, `leaves`, `autoremove`, `cleanup`
- `pin`, `unpin` - Version locking
- Full dependency resolution and conflict detection

### Discovery & Information
- `search`, `info`, `desc`, `deps`, `uses`
- `list`, `missing`, `formulae`, `casks`, `unbottled`
- `which-formula`, `cat`, `log`

### Repository Management
- `tap`, `untap`, `tap-info`
- `update` - Refresh cache + update all taps (parallel)
- Works with all Homebrew taps (custom taps delegate to brew)

### System & Utilities
- `config`, `doctor`, `env`, `shellenv`
- `services` - launchd integration
- `bundle` - Brewfile support
- Shell completions (bash, zsh, fish)

## What Doesn't Work ❌

### Source Builds
- Formulae without bottles
- `--build-from-source` flag
- `--HEAD` installations
- Custom build options

**Workaround:** Use `brew` for these cases.

### Development Tools
- `create`, `audit`, `livecheck`, `test` - Use `brew` instead

For most daily package management, bru works well.

## Performance Benchmarks

**Latest (v0.1.7)** - M3 Max, macOS 15.1, 339 packages:

| Operation | brew | bru | Speedup |
|-----------|------|-----|---------|
| `upgrade --dry-run` | 3.56s | **0.65s** | **5.5x** |
| `search rust` | 1.03s | **0.05s** | **20x** |
| `info wget` | 1.15s | **0.10s** | **12x** |
| `deps ffmpeg` | 1.26s | **0.15s** | **8x** |
| Startup | ~100ms | **14ms** | **7x** |

**Why so fast?**
- Compiled binary (no interpreter startup)
- Parallel API calls (not sequential)
- In-memory caching (no redundant requests)
- Efficient data structures

See [releases](https://github.com/nijaru/kombrucha/releases) for detailed version history.

## Compatibility

**Compatible with Homebrew for bottle-based workflows:**
- ✅ Same formulae (Homebrew API)
- ✅ Same Cellar (`/opt/homebrew/Cellar`)
- ✅ Same taps (all third-party taps work)
- ✅ Same bottles (GHCR)
- ✅ Can use `brew` and `bru` interchangeably
- ✅ No migration needed

**You can mix and match:**
```bash
brew install python    # Install with brew
bru upgrade python     # Upgrade with bru
brew uninstall python  # Uninstall with brew
```

For packages with pre-built bottles (most common packages), they work interchangeably.

## Shell Completions

```bash
# Bash
bru completions bash > ~/.local/share/bash-completion/completions/bru

# Zsh (add to ~/.zshrc)
eval "$(bru completions zsh)"

# Fish
bru completions fish > ~/.config/fish/completions/bru.fish
```

## Documentation

**User Docs:**
- [Installation Guide](docs/installation.md) - Coming soon
- [Migration from Homebrew](docs/migration.md) - Coming soon
- [Troubleshooting](docs/troubleshooting.md) - Coming soon

**Project Status:**
- [ai/STATUS.md](ai/STATUS.md) - Current project state
- [Release Notes](https://github.com/nijaru/kombrucha/releases) - Version history
- [AGENTS.md](AGENTS.md) - For AI agents working on this project

**Technical:**
- [Architecture](docs/architecture/SPEC.md) - System design
- [Performance Analysis](ai/research/performance-analysis.md) - Optimization details
- [Testing Strategy](docs/architecture/testing-strategy.md) - How we test

## Contributing

bru is in active development. Contributions welcome!

**Found a bug?** [Open an issue](https://github.com/nijaru/kombrucha/issues)

**Want to help?** Check [ai/TODO.md](ai/TODO.md) for active work.

## Testing

```bash
# Run all tests
cargo test

# Integration tests (run in CI)
cargo test --test integration_tests -- --ignored --test-threads=1

# Build release binary
cargo build --release
```

## FAQ

### Is bru stable?
Production-ready for bottle-based workflows. Tested with 339+ packages, has 90 automated tests, and runs CI verification on every commit.

### Will it break my Homebrew setup?
**No.** bru uses the same Cellar and infrastructure as Homebrew. You can use both interchangeably.

### Why not just improve Homebrew?
Different tradeoffs. Homebrew prioritizes comprehensive features. bru prioritizes speed and simplicity. Both are valid approaches.

### What's the catch?
Source builds aren't supported yet. For those less common cases, use `brew`.

### Can I uninstall it?
**Yes.** Since bru is just a faster frontend to Homebrew's infrastructure:
```bash
brew uninstall nijaru/tap/bru  # or: cargo uninstall kombrucha
# All your packages remain intact
```

## License

MIT OR Apache-2.0

---

**Made by [nijaru](https://github.com/nijaru)**
