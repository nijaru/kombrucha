#!/usr/bin/env bash
# Benchmark bru vs brew

set -euo pipefail

echo "🔥 Benchmarking bru vs brew"
echo "============================="
echo

# Build release binary
echo "Building release binary..."
cargo build --release --quiet
echo

# Test queries
queries=("rust" "python" "node" "docker" "wget")

echo "Running benchmarks (3 runs each)..."
echo

for query in "${queries[@]}"; do
    echo "Query: '$query'"
    echo "  Homebrew:"

    # Warm up
    brew search "$query" > /dev/null 2>&1 || true

    # Benchmark brew (3 runs)
    brew_total=0
    for i in {1..3}; do
        start=$(gdate +%s.%N 2>/dev/null || date +%s)
        brew search "$query" > /dev/null 2>&1
        end=$(gdate +%s.%N 2>/dev/null || date +%s)
        duration=$(echo "$end - $start" | bc)
        brew_total=$(echo "$brew_total + $duration" | bc)
        printf "    Run $i: %.3fs\n" "$duration"
    done
    brew_avg=$(echo "scale=3; $brew_total / 3" | bc)
    printf "    Average: %.3fs\n" "$brew_avg"

    echo "  bru:"

    # Warm up
    ./target/release/bru search "$query" > /dev/null 2>&1

    # Benchmark bru (3 runs)
    bru_total=0
    for i in {1..3}; do
        start=$(gdate +%s.%N 2>/dev/null || date +%s)
        ./target/release/bru search "$query" > /dev/null 2>&1
        end=$(gdate +%s.%N 2>/dev/null || date +%s)
        duration=$(echo "$end - $start" | bc)
        bru_total=$(echo "$bru_total + $duration" | bc)
        printf "    Run $i: %.3fs\n" "$duration"
    done
    bru_avg=$(echo "scale=3; $bru_total / 3" | bc)
    printf "    Average: %.3fs\n" "$bru_avg"

    # Calculate speedup
    speedup=$(echo "scale=2; $brew_avg / $bru_avg" | bc)
    printf "  ⚡ Speedup: %.2fx faster\n" "$speedup"
    echo
done

echo "✅ Benchmark complete!"
