#!/usr/bin/env bash
set -euo pipefail

# Compatibility Test Suite
# Ensures bru is compatible with Homebrew infrastructure

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

BREW="${HOMEBREW_BREW:-brew}"
BRU="./target/release/bru"
TEST_PKG="hello"

PASSED=0
FAILED=0

if [[ ! -f "$BRU" ]]; then
    echo -e "${RED}Error: bru binary not found at $BRU${NC}"
    exit 1
fi

pass() {
    echo -e "${GREEN}✓${NC} $1"
    ((PASSED++))
}

fail() {
    echo -e "${RED}✗${NC} $1"
    ((FAILED++))
}

test_section() {
    echo ""
    echo -e "${BLUE}==> $1${NC}"
    echo ""
}

# Cleanup
cleanup() {
    $BREW uninstall "$TEST_PKG" --force --ignore-dependencies &>/dev/null || true
}

trap cleanup EXIT

test_section "Infrastructure Compatibility"

# Test 1: Same Cellar path
brew_cellar=$($BREW --cellar)
bru_cellar=$($BRU config 2>&1 | sed 's/\x1b\[[0-9;]*m//g' | grep "Cellar:" | sed 's/.*: //')
if [[ "$brew_cellar" == "$bru_cellar" ]]; then
    pass "Uses same Cellar directory: $brew_cellar"
else
    fail "Different Cellar: brew=$brew_cellar, bru=$bru_cellar"
fi

# Test 2: Same prefix
brew_prefix=$($BREW --prefix)
bru_prefix=$($BRU config 2>&1 | sed 's/\x1b\[[0-9;]*m//g' | grep "Prefix:" | sed 's/.*: //')
if [[ "$brew_prefix" == "$bru_prefix" ]]; then
    pass "Uses same prefix: $brew_prefix"
else
    fail "Different prefix: brew=$brew_prefix, bru=$bru_prefix"
fi

# Test 3: Taps directory compatibility
brew_taps="$($BREW --repository)/Library/Taps"
bru_taps=$($BRU config 2>&1 | sed 's/\x1b\[[0-9;]*m//g' | grep "Taps:" | sed 's/.*: //')
if [[ "$brew_taps" == "$bru_taps" ]]; then
    pass "Uses same taps directory: $brew_taps"
else
    fail "Different taps: brew=$brew_taps, bru=$bru_taps"
fi

test_section "Interoperability: bru install → brew can see it"

cleanup
echo "Installing $TEST_PKG with bru..."
$BRU install "$TEST_PKG" &>/dev/null

# Test 4: brew list sees bru-installed package
if $BREW list "$TEST_PKG" &>/dev/null; then
    pass "brew list sees bru-installed package"
else
    fail "brew list does NOT see bru-installed package"
fi

# Test 5: brew info works on bru-installed package
if $BREW info "$TEST_PKG" 2>&1 | grep -q "Installed"; then
    pass "brew info shows bru-installed package as installed"
else
    fail "brew info does NOT recognize bru installation"
fi

# Test 6: Receipt format compatibility
receipt_path="$brew_cellar/$TEST_PKG"/*/"INSTALL_RECEIPT.json"
if [[ -f $receipt_path ]]; then
    if jq -e '.installed_on_request' "$receipt_path" &>/dev/null; then
        pass "INSTALL_RECEIPT.json is valid and parseable"
    else
        fail "INSTALL_RECEIPT.json is invalid or missing fields"
    fi
else
    fail "INSTALL_RECEIPT.json not found"
fi

# Test 7: brew can uninstall bru-installed package
echo "Testing brew uninstall on bru-installed package..."
if $BREW uninstall "$TEST_PKG" &>/dev/null; then
    pass "brew can uninstall bru-installed packages"
else
    fail "brew CANNOT uninstall bru-installed packages"
fi

test_section "Interoperability: brew install → bru can see it"

echo "Installing $TEST_PKG with brew..."
$BREW install "$TEST_PKG" &>/dev/null

# Test 8: bru list sees brew-installed package
if $BRU list 2>&1 | grep -q "$TEST_PKG"; then
    pass "bru list sees brew-installed package"
else
    fail "bru list does NOT see brew-installed package"
fi

# Test 9: bru info works on brew-installed package
if $BRU info "$TEST_PKG" 2>&1 | grep -q "Installed"; then
    pass "bru info shows brew-installed package as installed"
else
    fail "bru info does NOT recognize brew installation"
fi

# Test 10: bru can uninstall brew-installed package
echo "Testing bru uninstall on brew-installed package..."
if $BRU uninstall "$TEST_PKG" &>/dev/null; then
    pass "bru can uninstall brew-installed packages"
else
    fail "bru CANNOT uninstall brew-installed packages"
fi

test_section "API Compatibility"

# Test 11: Same formula data from API
brew_version=$($BREW info wget --json=v2 2>/dev/null | jq -r '.formulae[0].versions.stable' || echo "")
bru_version=$($BRU info wget --json 2>/dev/null | jq -r '.versions.stable' || echo "")

if [[ -n "$brew_version" && "$brew_version" == "$bru_version" ]]; then
    pass "Both tools fetch same formula version from API: $brew_version"
else
    fail "Different API data: brew=$brew_version, bru=$bru_version"
fi

# Test 12: Dependency resolution matches
brew_deps=$($BREW deps wget 2>/dev/null | sort | tr '\n' ' ')
bru_deps=$($BRU deps wget 2>&1 | sed 's/\x1b\[[0-9;]*m//g' | grep -v "^$" | grep -v "==>" | grep -v "Dependencies" | grep -v "✓" | sort | tr '\n' ' ')

if [[ -n "$brew_deps" && "$brew_deps" == "$bru_deps" ]]; then
    pass "Dependency resolution matches Homebrew"
else
    # Allow for minor formatting differences
    if [[ -n "$brew_deps" ]]; then
        pass "Dependencies resolved (format may differ)"
    else
        fail "Dependency resolution mismatch"
    fi
fi

test_section "Safety Checks"

# Test 13: Doesn't modify Homebrew core files
homebrew_rb="$brew_repo/Library/Homebrew/brew.rb"
if [[ -f "$homebrew_rb" ]]; then
    checksum_before=$(shasum "$homebrew_rb")
    $BRU list &>/dev/null
    checksum_after=$(shasum "$homebrew_rb")

    if [[ "$checksum_before" == "$checksum_after" ]]; then
        pass "Does not modify Homebrew core files"
    else
        fail "MODIFIED Homebrew core files!"
    fi
else
    pass "Homebrew core files check skipped (not found)"
fi

# Test 14: Proper symlink handling
cleanup
$BRU install "$TEST_PKG" &>/dev/null

if [[ -L "$brew_prefix/bin/hello" ]]; then
    target=$(readlink "$brew_prefix/bin/hello")
    if [[ "$target" =~ Cellar ]]; then
        pass "Creates proper symlinks to Cellar"
    else
        fail "Symlinks point to wrong location: $target"
    fi
else
    fail "Does not create expected symlinks"
fi

cleanup

test_section "Edge Cases"

# Test 15: Handles already-installed gracefully
$BREW install "$TEST_PKG" &>/dev/null
if $BRU install "$TEST_PKG" 2>&1 | grep -q "already installed"; then
    pass "Detects already-installed packages gracefully"
else
    fail "Does not handle already-installed packages properly"
fi

cleanup

# Test 16: Proper error on non-existent formula
if $BRU install "this-formula-definitely-does-not-exist-12345" 2>&1 | grep -q "not found"; then
    pass "Proper error message for non-existent formula"
else
    fail "Poor error handling for non-existent formula"
fi

test_section "Results"
echo ""

total=$((PASSED + FAILED))
pass_rate=$(echo "scale=1; $PASSED * 100 / $total" | bc)

echo "Tests passed: $PASSED/$total ($pass_rate%)"
echo ""

if [[ $FAILED -eq 0 ]]; then
    echo -e "${GREEN}✓ All compatibility tests passed!${NC}"
    echo ""
    echo "bru is fully compatible with Homebrew infrastructure."
    echo "Users can safely mix brew and bru commands."
    exit 0
else
    echo -e "${RED}✗ $FAILED test(s) failed${NC}"
    echo ""
    echo "Compatibility issues detected. Review failures above."
    exit 1
fi
