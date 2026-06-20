# bru

**A package manager for the brew ecosystem, written in Go**

Same packages as brew. Doesn't bloat your disk. Doesn't break on upgrade.

## What is this?

bru uses brew's package universe — the same formula.json API, the same prebuilt
bottles — but stores them differently. Content-addressable storage means packages are
deduplicated across versions. Transactional installs mean interrupted upgrades can be
rolled back. No Ruby startup tax.

It's the [pnpm](https://pnpm.io/) model applied to system packages: pnpm reads npm's
`package.json` and downloads from npm's registry, but has its own store. Same idea.

## Status

**Design phase.** Architecture spec and research complete. Go implementation not yet started.

## Why not brew?

brew is the default way to get CLI tools on macOS. It works. But:

- **Slow startup** — Ruby interpreter adds hundreds of milliseconds to every command
- **"Updating Homebrew..."** — auto-update runs on every `brew install`, blocking for 10-30s
- **Disk bloat** — old versions pile up silently, no dedup across shared dependencies
- **Upgrade breakage** — interrupted upgrade leaves half-linked packages (brew issue #20427)
- **No rollback** — if an upgrade breaks something, you're on your own

bru fixes these at the storage layer, not by being a faster wrapper.

## Why not Stout?

[Stout](https://github.com/neul-labs/stout) is a mature Rust-based brew client with
10+ crates, SQLite indexing, signed indexes, and CVE auditing. It's good. It solves
the speed problem by being a drop-in brew replacement.

bru isn't drop-in. It's a different tool with a different store. The speed improvement
is a side effect of the architecture, not the pitch.

## Architecture

```
CLI → Service → Store (CAS) → Metadata (SQLite) → Fetch (parallel) → Transaction (atomic)
```

Key properties:
- **Content-addressable store**: packages stored by content hash, deduplicated
- **APFS clonefile**: instant materialization on macOS (copy-on-write)
- **Transactional installs**: staged writes, atomic commit, crash-safe rollback
- **SQLite + FTS5**: fast search, incremental metadata sync
- **MVS resolution**: simple, reproducible dependency resolution

## Building

```bash
# (TBD — implementation not yet started)
```

## License

MIT
