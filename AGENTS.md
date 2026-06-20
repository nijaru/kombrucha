# bru

*Go package manager for the brew ecosystem*

## Project Overview

**bru** is a package manager that uses brew's package universe (formula.json API,
GHCR bottles) but owns its own content-addressable store with transactional installs,
atomic upgrades, and rollback. Not a drop-in `brew` replacement — a better tool for
the same packages.

The "pnpm for system packages" model: pnpm reads npm's `package.json` and downloads
from npm's registry, but has its own CAS+symlink store. Same idea, for macOS/Linux
system packages.

**Current Status**: Design phase. Go implementation not yet started.

## Project Structure

| Directory | Purpose |
|-----------|---------|
| `ai/` | AI session context (local-only, not committed) |

### AI Context Organization

**Session files** (ai/ root — read FIRST every session):
- `STATUS.md` — Current state, metrics, blockers
- `TODO.md` — Active tasks only
- `decisions.md` — Active architectural decisions
- `RESEARCH.md` — Research findings index
- `brief.md` — Quick context snapshot

**Reference files** (subdirectories — loaded only when needed):
- `research/` — Detailed research (>200 lines per topic)

Session files kept minimal (<500 lines). Detailed content in subdirectories.
Trust git history — delete completed/historical content from session files.

## Quick Start

```bash
# (TBD — Go implementation not yet started)
```

## Core Architecture

**Go, layered, content-addressable**:

- CLI layer: user-facing commands (cobra or urfave/cli)
- Service layer: business logic (resolve → fetch → store → materialize → register)
- CAS store: hash-addressed storage, dedup, integrity verification
- Metadata: SQLite with FTS5 for fast search and delta sync
- Transaction engine: staged writes, journal, atomic commit, rollback
- Fetcher: parallel bottle downloads from GHCR with SHA256 verification

Brew's formula.json API provides 8,300+ formulae. Brew's GHCR bottles provide
prebuilt binaries. We borrow these. We own the store, the transactions, and the UX.

## Design Principles

- **Same packages**: reads brew's API, downloads brew's bottles
- **No bloat**: content-addressable store, dedup across versions
- **No breakage**: transactional installs with atomic rollback
- **Fast**: Go binary, milliseconds to start, parallel everything
- **Honest**: falls back to brew for source builds (~5%), doesn't pretend otherwise
- **Simple**: MVS resolution, no SAT solver, no Ruby, no surprises

## Known Design Decisions

| Decision | Choice |
|----------|--------|
| Language | Go |
| Compatibility | pnpm-hybrid (brew's packages, own store) |
| Storage | Content-addressable (CAS) with clonefile/hardlink |
| Transactions | Staged writes → atomic rename → journal |
| Metadata | SQLite with FTS5 |
| Resolution | MVS (Minimal Version Selection) |
| Source builds | Clean brew fallback |

## Resources

- **Architecture**: `ai/architecture.md`
- **Current State**: `ai/STATUS.md`
- **Active Tasks**: `ai/TODO.md`
- **Decisions**: `ai/decisions.md`
- **Research Index**: `ai/RESEARCH.md`
- **Landscape Research**: `ai/research/package-manager-landscape-2026.md`
