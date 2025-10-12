#!/usr/bin/env bash
# Benchmark complex package with dependencies
set -euo pipefail

cd "$(dirname "$0")/.."

echo "=== Phase 2: Complex Package Benchmark ==="
echo ""
echo "Testing installation with dependencies"
echo "Package: jq (has dependencies: oniguruma)"
echo ""

# Build bru
echo "Building bru..."
cargo build --release --quiet

# Clean up
echo "Cleaning up existing installations..."
brew uninstall --force --ignore-dependencies jq 2>/dev/null || true
brew uninstall --force oniguruma 2>/dev/null || true

echo ""
echo "--- Benchmarking brew install jq ---"
time brew install jq

echo ""
echo "Uninstalling for bru test..."
brew uninstall --force --ignore-dependencies jq
brew uninstall --force oniguruma

echo ""
echo "--- Benchmarking bru install jq ---"
time ./target/release/bru install jq

echo ""
echo "=== Benchmark Complete ==="
echo ""
echo "Verifying installation:"
jq --version
