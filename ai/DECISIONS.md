# Architectural Decisions

**Last Updated**: January 26, 2026

## Active Decisions

### Library API Design (v0.1.35)

**Decision**: High-level `PackageManager` API wrapping low-level modules

**Rationale**:
- Downstream projects (like Cutler) need simple, unified interface
- Don't expose all internal complexity (cellar, download, extract, symlink modules)
- Maintain backward compatibility with low-level API for advanced users
- Shared HTTP client enables connection pooling and efficient resource usage

---

### Command Modularization (v0.2.4)

**Decision**: Split `main.rs` and `install.rs` into specialized sub-modules and phases.

**Rationale**:
- `main.rs` exceeded 1400 lines, making it difficult to maintain.
- `install.rs` contained monolithic command implementations with duplicated logic.
- Modularization allows for better code reuse (e.g., sharing installation phases between `install` and `reinstall`).
- Centralizing CLI structure in `src/commands/mod.rs` simplifies argument parsing and dispatching.

**Implementation**:
- CLI `Cli` and `Commands` moved to `src/commands/mod.rs`.
- Large `match` block in `main.rs` replaced by a central `run` function in `commands`.
- Core installation phases moved to `src/commands/pkg/phases.rs`.

---

### Parallel Outdated() Optimization (v0.2.4)

**Decision**: Use `futures::stream` with a concurrency limit for `outdated()`.

**Rationale**:
- Sequential API queries took ~40s for 300+ packages.
- Parallelizing with a limit (15) provides a significant speedup (~4x) without hitting rate limits.
- `buffer_unordered` provides the best throughput for independent network requests.

---

### Ruby VM Integration (Phase 5 Foundation)

**Decision**: Embed Ruby VM using `magnus` and initialize once in `main.rs`.

**Rationale**:
- Required for Phase 5 (Source Builds) to execute `.rb` formulae.
- `magnus` provides high-performance, safe bindings.
- Initializing in `main.rs` and passing a reference to `run` ensures the VM lives for the process duration and avoids thread-safety issues with `Sync/Send`.

---

### Ruby-to-Rust Bridge (v0.2.4)

**Decision**: Map Ruby DSL methods (e.g., `system`, `bin.install`) directly to Rust functions.

**Rationale**:
- Allows executing Homebrew formula logic while maintaining control within the Rust core.
- Simplifies the Ruby `Formula` class by offloading implementation to Rust.
- Provides a path for capturing output and progress from source builds.

---

## See Also

- [STATUS.md](./STATUS.md) - Current project state
- [TODO.md](./TODO.md) - Active tasks
- [DESIGN.md](./DESIGN.md) - Architectural design