# Benchmarks

Performance testing for kombrucha (bru) vs Homebrew.

## Files

- **results.md**: Current benchmark results against Homebrew
- Run benchmarks: `../scripts/benchmark.sh` (from project root)

## Running Benchmarks

```bash
# Build release binary first
cargo build --release

# Run benchmark script
./scripts/benchmark.sh
```

## Current Results Summary

**Phase 0** (read-only commands):
- `bru info`: **7.2x faster** than `brew info`
- `bru search`: Same speed, **15x less CPU**

See [results.md](results.md) for detailed analysis.
