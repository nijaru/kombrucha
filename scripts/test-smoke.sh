#!/usr/bin/env bash
# Smoke tests - quick validation that core commands work
set -eu

GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

echo "Building bru..."
cargo build --release --quiet

BRU="./target/release/bru"
PASSED=0
FAILED=0

test_cmd() {
    local name="$1"
    shift
    if timeout 10 "$@" > /dev/null 2>&1; then
        echo -e "${GREEN}✓${NC} $name"
        ((PASSED++))
    else
        echo -e "${RED}✗${NC} $name"
        ((FAILED++))
    fi
}

echo ""
echo "Running smoke tests..."
echo ""

# Test commands that should complete quickly
test_cmd "commands" $BRU commands
test_cmd "config" $BRU config
test_cmd "cache" $BRU cache
test_cmd "doctor" $BRU doctor
test_cmd "tap (list)" $BRU tap
test_cmd "list" $BRU list

# Test commands that require API
test_cmd "info" $BRU info wget
test_cmd "deps" $BRU deps wget
test_cmd "search" $BRU search wget

echo ""
echo "Results: $PASSED passed, $FAILED failed"

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed${NC}"
    exit 1
fi
