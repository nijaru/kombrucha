#!/usr/bin/env bash
# Integration test: Full install → upgrade → uninstall workflow
set -eu

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "Building bru..."
cargo build --release --quiet

BRU="./target/release/bru"
TEST_PACKAGE="tree"  # Simple package with no dependencies

echo -e "\n${YELLOW}=== Integration Test: Install → Upgrade → Uninstall ===${NC}\n"

# Cleanup any existing installation
echo "Cleaning up any existing installation..."
$BRU uninstall --force $TEST_PACKAGE 2>/dev/null || true

# Test 1: Install
echo -e "\n${YELLOW}Test 1: Install $TEST_PACKAGE${NC}"
if $BRU install $TEST_PACKAGE; then
    echo -e "${GREEN}✓${NC} Install succeeded"
else
    echo -e "${RED}✗${NC} Install failed"
    exit 1
fi

# Verify installation
if $BRU list | sed 's/\x1b\[[0-9;]*m//g' | grep -q "^$TEST_PACKAGE "; then
    echo -e "${GREEN}✓${NC} Package appears in list"
else
    echo -e "${RED}✗${NC} Package not found in list"
    exit 1
fi

# Test 2: Reinstall (should work)
echo -e "\n${YELLOW}Test 2: Reinstall $TEST_PACKAGE${NC}"
if $BRU reinstall $TEST_PACKAGE; then
    echo -e "${GREEN}✓${NC} Reinstall succeeded"
else
    echo -e "${RED}✗${NC} Reinstall failed"
    exit 1
fi

# Test 3: Info on installed package
echo -e "\n${YELLOW}Test 3: Info for installed package${NC}"
if $BRU info $TEST_PACKAGE | grep -q $TEST_PACKAGE; then
    echo -e "${GREEN}✓${NC} Info command works"
else
    echo -e "${RED}✗${NC} Info command failed"
    exit 1
fi

# Test 4: Uninstall
echo -e "\n${YELLOW}Test 4: Uninstall $TEST_PACKAGE${NC}"
if $BRU uninstall $TEST_PACKAGE; then
    echo -e "${GREEN}✓${NC} Uninstall succeeded"
else
    echo -e "${RED}✗${NC} Uninstall failed"
    exit 1
fi

# Verify uninstallation
if $BRU list | sed 's/\x1b\[[0-9;]*m//g' | grep -q "^$TEST_PACKAGE "; then
    echo -e "${RED}✗${NC} Package still appears in list after uninstall"
    exit 1
else
    echo -e "${GREEN}✓${NC} Package removed from list"
fi

echo -e "\n${GREEN}=== All integration tests passed! ===${NC}\n"
