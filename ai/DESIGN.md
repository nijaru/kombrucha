# Architectural Design

**Last Updated**: January 26, 2026

## Overview

Kombrucha is a high-performance package manager core that mimics Homebrew behavior while utilizing Rust for speed and concurrency. It uses an embedded Ruby VM to evaluate existing Homebrew formula definitions.

## Component Structure

### CLI & Command Dispatch (`src/commands/`)

The CLI is organized into functional modules to keep the core binary small and maintainable.

- `mod.rs`: Central `Cli` struct, `Commands` enum, and the `run` dispatch function.
- `pkg/`: Core package operations (install, upgrade, uninstall).
  - `phases.rs`: Atomic installation phases (dependency resolution, extraction, relocation, symlinking).
  - `ruby.rs`: Embedded Ruby integration and DSL mapping.
- `query.rs`: Search, info, and metadata retrieval.
- `maintenance.rs`: Cleanup, update, and system health checks.

### Core Modules (`src/`)

- `api.rs`: Homebrew JSON API client with in-memory and disk caching.
- `cellar.rs`: Scans and inspects the local installation directory.
- `download.rs`: Manages parallel bottle downloads from GHCR.
- `extract.rs`: Handles archive extraction to the Cellar.
- `relocate.rs`: Fixes hardcoded paths in binaries and scripts after extraction.
- `symlink.rs`: Manages symlinks from Cellar to prefix (bin, lib, etc.).

## Ruby Integration

Kombrucha uses the `magnus` crate to embed a Ruby interpreter.

### Formula Loading Flow

1. **Locate**: Find the `.rb` file in the specified tap.
2. **Initialize**: `FormulaLoader` sets up the `Formula` base class and `Installer` bridge in Ruby.
3. **Evaluate**: Ruby VM loads the formula file, defining the package class.
4. **Instantiate**: Rust creates an instance of the formula class.
5. **Execute**: Rust calls the Ruby `install` method, which triggers Rust bridge functions (`Rust.system`, `Rust.install`) to perform actual work.

## Concurrency Model

- **Network**: `tokio` with `futures::stream` for parallel API requests and downloads.
- **CPU/IO**: `rayon` for parallel bottle extraction, relocation, and symlink processing.
- **Resource Limits**: Concurrency limits (semaphores) are applied to avoid rate limiting and file descriptor exhaustion.
