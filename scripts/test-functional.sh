#!/usr/bin/env bash
# Functional tests to validate bru produces correct results
set -euo pipefail

# Disable pipefail for tests since we use pipes with grep
set +o pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

TESTS_PASSED=0
TESTS_FAILED=0

# Build release binary
echo "Building bru..."
cargo build --release --quiet

BRU="./target/release/bru"

# Helper functions
pass() {
    echo -e "${GREEN}✓${NC} $1"
    ((TESTS_PASSED++))
}

fail() {
    echo -e "${RED}✗${NC} $1"
    ((TESTS_FAILED++))
}

test_section() {
    echo ""
    echo -e "${YELLOW}=== $1 ===${NC}"
}

# Test: search returns results
test_section "Search Command"

if $BRU search wget 2>/dev/null | grep -qi "wget"; then
    pass "search finds wget"
else
    fail "search should find wget"
fi

if $BRU search --formula python 2>/dev/null | grep -qi "python"; then
    pass "search --formula filters work"
else
    fail "search --formula should find python"
fi

# Test: info shows formula details
test_section "Info Command"

if $BRU info wget 2>/dev/null | grep -qi "wget"; then
    pass "info shows formula name"
else
    fail "info should show formula name"
fi

if $BRU info wget 2>/dev/null | grep -q "https://"; then
    pass "info shows homepage"
else
    fail "info should show homepage"
fi

# Test: info --json returns valid JSON
if $BRU info --json wget 2>/dev/null | python3 -m json.tool > /dev/null 2>&1; then
    pass "info --json returns valid JSON"
else
    fail "info --json should return valid JSON"
fi

# Test: deps shows dependencies
test_section "Deps Command"

if $BRU deps wget 2>/dev/null | grep -qi "Dependencies\|dependencies"; then
    pass "deps command runs"
else
    fail "deps command should run"
fi

# Test: list command
test_section "List Command"

if $BRU list 2>/dev/null | grep -qi "Installed packages\|No packages"; then
    pass "list command runs"
else
    fail "list command should run"
fi

# Test: list --json returns valid JSON
if $BRU list --json 2>/dev/null | python3 -m json.tool > /dev/null 2>&1; then
    pass "list --json returns valid JSON"
else
    fail "list --json should return valid JSON"
fi

# Test: commands command
test_section "Utility Commands"

if $BRU commands 2>/dev/null | grep -qi "install\|search\|info"; then
    pass "commands lists available commands"
else
    fail "commands should list available commands"
fi

# Test: config command
if $BRU config 2>/dev/null | grep -qi "Homebrew\|Configuration"; then
    pass "config shows system info"
else
    fail "config should show system info"
fi

# Test: tap command (list taps)
if $BRU tap 2>/dev/null | grep -qi "homebrew/core\|No custom taps"; then
    pass "tap lists taps"
else
    fail "tap should list taps"
fi

# Test: cache command
if $BRU cache 2>/dev/null | grep -qi "Cache\|cache"; then
    pass "cache shows cache info"
else
    fail "cache should show cache info"
fi

# Test: doctor command
if $BRU doctor 2>/dev/null | grep -qi "System\|Ready\|issue"; then
    pass "doctor runs system checks"
else
    fail "doctor should run system checks"
fi

# Test: desc command
test_section "Description Command"

if $BRU desc wget curl 2>/dev/null | grep -qi "wget\|curl"; then
    pass "desc shows descriptions for multiple packages"
else
    fail "desc should show descriptions"
fi

# Test: uses command
test_section "Dependency Analysis"

# Pick a common dependency
if $BRU uses openssl 2>/dev/null | grep -qi "depend\|used\|Formulae"; then
    pass "uses shows reverse dependencies"
else
    fail "uses should show reverse dependencies"
fi

# Summary
test_section "Test Summary"
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
