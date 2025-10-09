#!/usr/bin/env bash
set -euo pipefail

# Phase 2 Install Benchmark: bru vs brew
# Tests installation speed for packages

cd "$(dirname "$0")/.."

echo "=== Phase 2 Install Benchmark ==="
echo ""
echo "Testing package installation performance"
echo "Package: tree (no dependencies)"
echo ""

# Ensure tree is not installed
brew uninstall --force tree 2>/dev/null || true
rm -rf /opt/homebrew/Cellar/tree 2>/dev/null || true
rm -f /opt/homebrew/bin/tree 2>/dev/null || true

# Build bru
echo "Building bru..."
cargo build --release --quiet

echo ""
echo "--- Benchmarking brew install tree ---"
time brew install tree
brew uninstall --force tree

echo ""
echo "--- Benchmarking bru install tree ---"
time ./target/release/bru install tree

echo ""
echo "=== Benchmark Complete ==="
echo ""
echo "Verifying installation:"
tree --version | head -1
