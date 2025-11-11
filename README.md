# bru

**A fast, Homebrew-compatible package manager for macOS**

Drop-in replacement for `brew` with 8x faster operations. Works with Homebrew's formulae and bottles—no migration needed.

```bash
bru i ripgrep fd bat    # Install packages (shorthand)
bru up                  # Upgrade all packages
bru rm old-package      # Uninstall
```

## Why bru?

| Operation | Homebrew | bru | Speedup |
|-----------|----------|-----|---------|
| `search python` | 1.04s | **0.04s** | **26x** |
| `info wget` | 1.04s | **0.11s** | **9.6x** |
| `outdated` | 1.63s | **0.78s** | **2.1x** |
| `update` (8 taps) | 3.2s | **1.9s** | **1.7x** |
| Startup time | ~100ms | **<10ms** | **10x** |

**Same formulae, same ecosystem, dramatically faster.**

## Installation

### Via Homebrew

```bash
brew install nijaru/tap/bru
```

### Via Cargo

```bash
cargo install kombrucha
```

## Quick Start

### CLI

```bash
# Command aliases (fast typers)
bru i wget           # install
bru up              # upgrade all
bru re wget         # reinstall
bru rm wget         # uninstall

# Full commands (same as brew)
bru install wget
bru upgrade
bru search rust
bru info python
bru list
bru outdated
```

### Library (v0.1.35+)

For Rust projects that need programmatic Homebrew access:

```rust
use kombrucha::PackageManager;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let pm = PackageManager::new()?;
    
    // Install a package
    let result = pm.install("ripgrep").await?;
    println!("Installed {} {}", result.name, result.version);
    
    // Check for upgrades
    let outdated = pm.outdated().await?;
    for pkg in outdated {
        println!("{} {} → {}", pkg.name, pkg.installed, pkg.latest);
    }
    
    Ok(())
}
```

See [docs/library-api.md](docs/library-api.md) for complete API reference and examples.

## What Works ✅

**Core Operations**
- `install`, `uninstall`, `upgrade`, `reinstall` - Bottle-based formulae
- `install --cask`, `upgrade --cask` - macOS applications
- `search`, `info`, `deps`, `uses`, `list`, `outdated`
- `cleanup`, `pin`, `unpin`

**Repository Management**
- `tap`, `untap`, `tap-info`
- `update` - Refresh cache and update taps (parallelized)

**System**
- `config`, `doctor`, `env`, `shellenv`
- `services` - launchd integration
- `bundle` - Brewfile support
- Shell completions (bash, zsh, fish)

**Fallback**
- Unsupported commands automatically delegate to `brew`
- Custom taps work (delegated to brew for source builds)

## What Doesn't Work ❌

**Source Builds** (~5% of formulae)
- Formulae without precompiled bottles
- `--build-from-source` flag
- `--HEAD` installations

**Workaround**: Use `brew` for these edge cases. bru detects them automatically.

## Features

**⚡ Performance**
- Compiled Rust binary (no interpreter overhead)
- Parallel API calls (not sequential)
- In-memory + disk caching
- HTTP/2 connection pooling

**🔄 Compatibility**
- Same Cellar (`/opt/homebrew/Cellar`)
- Same formulae (Homebrew JSON API)
- Same taps (all third-party taps work)
- Can mix `brew` and `bru` commands freely
- `brew` can reinstall `bru`-installed packages

**🎯 UX**
- Command aliases (`bru i`, `bru up`, `bru re`, `bru rm`)
- Clear progress output
- Helpful error messages
- Modern CLI patterns

## Compatibility

Works with Homebrew v4.x on:
- **macOS 13+** (Ventura, Sonoma, Sequoia)
- **Apple Silicon** (M1/M2/M3)
- **Intel** (x86_64)

## Documentation

- **[Library API](docs/library-api.md)** - Programmatic access for Rust projects
- **[Architecture](docs/architecture/)** - System design and internals
- **[CHANGELOG](CHANGELOG.md)** - Version history

## Testing

```bash
# Run all tests
cargo test

# Build release binary
cargo build --release
```

## Status

**v0.1.34** - Production-ready for bottle-based workflows
- ✅ 340+ packages tested on real system
- ✅ Full Homebrew compatibility
- ✅ All core commands working
- ✅ Zero panics in error handling

**v0.1.35** (coming soon) - Library API for Rust projects
- PackageManager struct for programmatic access
- High-level interface for common workflows
- Low-level module access for advanced use cases

## FAQ

**Will it break my Homebrew setup?**  
No. bru uses the same Cellar and infrastructure. You can use both tools interchangeably.

**Can I uninstall it?**  
Yes. All packages remain intact.

```bash
brew uninstall nijaru/tap/bru  # or: cargo uninstall kombrucha
```

**Why not improve Homebrew instead?**  
Different tradeoffs. Homebrew prioritizes comprehensive features. bru prioritizes speed and simplicity.

**What about source builds?**  
Planned for a future release. For now, ~5% of formulae without bottles can be installed via `brew install`.

## Contributing

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT OR Apache-2.0

---

**Made by [nijaru](https://github.com/nijaru)**
