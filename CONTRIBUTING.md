# Contributing to bru

Thanks for your interest in contributing to bru! This document provides guidelines and information for contributors.

## Quick Start

```bash
# Clone the repository
git clone https://github.com/nijaru/kombrucha.git
cd kombrucha

# Build the project
cargo build --release

# Run tests
cargo test

# Try it out
./target/release/bru search rust
```

## Development Setup

### Prerequisites
- Rust 1.90+ (uses Rust 2024 edition features)
- macOS (primary development target)
- Homebrew (for testing compatibility)

### Project Structure
```
kombrucha/
├── src/
│   ├── api.rs          # Homebrew API client
│   ├── cellar.rs       # Cellar management
│   ├── commands.rs     # CLI command implementations
│   ├── download.rs     # Bottle downloads (GHCR auth)
│   ├── extract.rs      # Tar/gzip extraction
│   ├── platform.rs     # Platform detection
│   ├── receipt.rs      # INSTALL_RECEIPT.json generation
│   ├── relocate.rs     # Bottle relocation (install_name_tool)
│   └── symlink.rs      # Symlink management
├── benchmarks/         # Performance benchmarks
├── internal/           # Architecture docs and specs
└── scripts/            # Development scripts
```

## Coding Guidelines

### Rust Style
- Follow standard Rust formatting: `cargo fmt`
- Pass clippy lints: `cargo clippy`
- No warnings in release builds
- Use descriptive variable names
- Add comments for non-obvious logic

### Commit Messages
Follow conventional commits:
```
feat: add new feature
fix: bug fix
docs: documentation changes
perf: performance improvements
test: add or update tests
chore: maintenance tasks
refactor: code restructuring
```

Example:
```
feat: implement parallel bottle downloads

Add concurrent download support using tokio for downloading multiple
bottles simultaneously. Reduces install time for packages with many
dependencies.

Benchmarks show 3x speedup for wget (4 dependencies).
```

### Testing
- Test commands with various packages
- Verify Homebrew compatibility
- Check that installed binaries work
- Test error cases
- Run benchmarks for performance changes

## What to Contribute

### High Priority
- **Phase 3**: Source builds for formulae without bottles
- **Cross-platform**: Linux support
- **Testing**: More comprehensive test coverage
- **Performance**: Further optimization opportunities

### Good First Issues
- Add more helpful error messages
- Improve progress indicators
- Add shell completions (fish, zsh, bash)
- Documentation improvements
- Add more benchmarks

### Known Issues
- Revision number handling (_1, _2 suffixes) needs improvement
- Upgrade command fails for packages with revisions
- No support for casks yet

## Pull Request Process

1. **Fork the repository** and create a feature branch
2. **Make your changes** following the coding guidelines
3. **Test thoroughly**:
   ```bash
   cargo test
   cargo clippy
   cargo fmt -- --check
   ./target/release/bru install tree  # Functional test
   ```
4. **Write clear commit messages** (see above)
5. **Update documentation** if needed (README.md, internal docs)
6. **Submit PR** with description of changes

### PR Template
```markdown
## Description
[What does this PR do?]

## Motivation
[Why is this change needed?]

## Testing
- [ ] Manual testing completed
- [ ] Tested with Homebrew for compatibility
- [ ] No compiler warnings
- [ ] Clippy passes

## Checklist
- [ ] Code follows project style
- [ ] Documentation updated
- [ ] Commit messages are clear
```

## Getting Help

- **Documentation**: See [internal/](internal/) for architecture details
- **Questions**: Open a GitHub issue with "question" label
- **Bugs**: Open a GitHub issue with detailed reproduction steps
- **Discussions**: Use GitHub Discussions for general topics

## License

By contributing, you agree that your contributions will be licensed under the same MIT OR Apache-2.0 dual license that covers the project.

## Code of Conduct

Be respectful, constructive, and professional. We're all here to make bru better.
