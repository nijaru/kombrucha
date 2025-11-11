# bru

**A fast, Homebrew-compatible package manager for macOS**

⚠️ **Experimental**: Production-ready for bottle-based workflows, but under active development.

Drop-in replacement for `brew` with 8x faster operations. Works with Homebrew's formulae and bottles—no migration needed.

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

## Usage

**CLI**: Works like `brew`. See `bru --help` for all commands. Unsupported commands automatically fall back to `brew`.

**Library**: For Rust projects that need programmatic Homebrew access. See [docs/library-api.md](docs/library-api.md) for complete API reference and examples.

## Limitations

Source builds (~5% of formulae without precompiled bottles) automatically fall back to `brew`.

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

## Contributing

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT OR Apache-2.0

---

**Made by [nijaru](https://github.com/nijaru)**
