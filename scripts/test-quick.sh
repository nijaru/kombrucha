#!/usr/bin/env bash
# Quick functional tests for bru - focuses on critical functionality
set -eu
set +o pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

TESTS_PASSED=0
TESTS_FAILED=0

echo "Building bru..."
cargo build --release --quiet 2>&1 | tail -1 || echo "Build done"

BRU="./target/release/bru"

pass() {
    echo -e "${GREEN}✓${NC} $1"
    ((TESTS_PASSED++))
}

fail() {
    echo -e "${RED}✗${NC} $1"
    ((TESTS_FAILED++))
}

section() {
    echo ""
    echo -e "${YELLOW}=== $1 ===${NC}"
}

# Test JSON output formats (no network required if cached)
section "JSON Output Tests"

if timeout 10 $BRU info --json wget 2>/dev/null | python3 -m json.tool > /dev/null 2>&1; then
    pass "info --json returns valid JSON"
else
    fail "info --json should return valid JSON"
fi

if timeout 10 $BRU list --json 2>/dev/null | python3 -m json.tool > /dev/null 2>&1; then
    pass "list --json returns valid JSON"
else
    fail "list --json should return valid JSON"
fi

# Test commands that don't require network
section "Local Commands"

if $BRU commands 2>/dev/null | grep -qi "install"; then
    pass "commands lists available commands"
else
    fail "commands should list available commands"
fi

if $BRU config 2>/dev/null | grep -qi "Homebrew"; then
    pass "config shows system info"
else
    fail "config should show system info"
fi

if $BRU cache 2>/dev/null | grep -qi "cache"; then
    pass "cache shows cache info"
else
    fail "cache should show cache info"
fi

if $BRU doctor 2>/dev/null | grep -qi "System\|Ready\|issue"; then
    pass "doctor runs system checks"
else
    fail "doctor should run system checks"
fi

if $BRU tap 2>/dev/null | grep -qi "homebrew"; then
    pass "tap lists taps"
else
    fail "tap should list taps"
fi

if $BRU list 2>/dev/null | grep -qi "Installed packages\|No packages"; then
    pass "list command runs"
else
    fail "list command should run"
fi

# Test one search to verify API connectivity
section "API Connectivity"

if timeout 15 $BRU search wget 2>/dev/null | grep -qi "wget"; then
    pass "search finds formulae via API"
else
    fail "search should connect to API"
fi

# Summary
section "Test Summary"
TOTAL=$((TESTS_PASSED + TESTS_FAILED))
echo ""
echo "Tests passed: $TESTS_PASSED/$TOTAL"
echo "Tests failed: $TESTS_FAILED/$TOTAL"
echo ""

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed${NC}"
    exit 1
fi
