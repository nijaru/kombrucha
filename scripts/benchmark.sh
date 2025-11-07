#!/usr/bin/env bash
# Comprehensive bru vs brew benchmark

set -euo pipefail

echo "🔥 Benchmarking bru vs brew"
echo "============================="
echo

# Build release binary
echo "Building release binary..."
cargo build --release --quiet
echo

# Number of runs for each benchmark
RUNS=5

# Helper function to benchmark a command
benchmark_command() {
    local cmd_name="$1"
    local brew_cmd="$2"
    local bru_cmd="$3"

    echo "Command: $cmd_name"
    echo "  Homebrew:"

    # Warm up
    eval "$brew_cmd" > /dev/null 2>&1 || true

    # Benchmark brew
    brew_total=0
    for i in $(seq 1 $RUNS); do
        start=$(gdate +%s.%N 2>/dev/null || date +%s)
        eval "$brew_cmd" > /dev/null 2>&1
        end=$(gdate +%s.%N 2>/dev/null || date +%s)
        duration=$(echo "$end - $start" | bc)
        brew_total=$(echo "$brew_total + $duration" | bc)
        printf "    Run $i: %.3fs\n" "$duration"
    done
    brew_avg=$(echo "scale=3; $brew_total / $RUNS" | bc)
    printf "    Average: %.3fs\n" "$brew_avg"

    echo "  bru:"

    # Warm up
    eval "$bru_cmd" > /dev/null 2>&1 || true

    # Benchmark bru
    bru_total=0
    for i in $(seq 1 $RUNS); do
        start=$(gdate +%s.%N 2>/dev/null || date +%s)
        eval "$bru_cmd" > /dev/null 2>&1
        end=$(gdate +%s.%N 2>/dev/null || date +%s)
        duration=$(echo "$end - $start" | bc)
        bru_total=$(echo "$bru_total + $duration" | bc)
        printf "    Run $i: %.3fs\n" "$duration"
    done
    bru_avg=$(echo "scale=3; $bru_total / $RUNS" | bc)
    printf "    Average: %.3fs\n" "$bru_avg"

    # Calculate speedup
    speedup=$(echo "scale=2; $brew_avg / $bru_avg" | bc)
    printf "  ⚡ Speedup: %.2fx faster\n" "$speedup"
    echo

    # Store results for summary table
    results+=("$cmd_name|$brew_avg|$bru_avg|$speedup")
}

# Array to store results
results=()

echo "Running benchmarks ($RUNS runs each)..."
echo

# Search benchmarks
benchmark_command "search python" \
    "brew search python" \
    "./target/release/bru search python"

benchmark_command "search rust" \
    "brew search rust" \
    "./target/release/bru search rust"

# Info benchmarks
benchmark_command "info wget" \
    "brew info wget" \
    "./target/release/bru info wget"

# Deps benchmarks
benchmark_command "deps wget" \
    "brew deps wget" \
    "./target/release/bru deps wget"

# List benchmark
benchmark_command "list" \
    "brew list" \
    "./target/release/bru list"

# Outdated benchmark
benchmark_command "outdated" \
    "brew outdated" \
    "./target/release/bru outdated"

# Upgrade dry-run benchmark
benchmark_command "upgrade --dry-run" \
    "brew upgrade --dry-run" \
    "./target/release/bru upgrade --dry-run"

# Autoremove dry-run benchmark
benchmark_command "autoremove --dry-run" \
    "brew autoremove --dry-run" \
    "./target/release/bru autoremove --dry-run"

echo "✅ Benchmarks complete!"
echo
echo "============================="
echo "Summary Table"
echo "============================="
echo

# Print markdown table
printf "| Command | brew (s) | bru (s) | Speedup |\n"
printf "|---------|----------|---------|----------|\n"
for result in "${results[@]}"; do
    IFS='|' read -r cmd brew_time bru_time speedup <<< "$result"
    printf "| %-27s | %8s | %7s | **%5sx** |\n" "$cmd" "$brew_time" "$bru_time" "$speedup"
done
echo

# Calculate overall average speedup
total_speedup=0
count=0
for result in "${results[@]}"; do
    IFS='|' read -r cmd brew_time bru_time speedup <<< "$result"
    total_speedup=$(echo "$total_speedup + $speedup" | bc)
    count=$((count + 1))
done
avg_speedup=$(echo "scale=2; $total_speedup / $count" | bc)
echo "Average speedup across all commands: ${avg_speedup}x faster"
