#!/bin/bash
#
# Test script for calld network and validator commands
# Tests net and validator subcommands against running devnet nodes
#

set -e

CALLD="./target/release/calld"
DATA_DIR="./devnet/node1/data"
CONFIG="./devnet/node1/config-native.toml"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

test_command() {
    local cmd="$1"
    local description="$2"
    echo ""
    info "Testing: $description"
    info "Command: $cmd"
    if eval "$cmd"; then
        info "✓ PASSED: $description"
        return 0
    else
        error "✗ FAILED: $description"
        return 1
    fi
}

# Check if calld binary exists
if [ ! -f "$CALLD" ]; then
    error "calld binary not found at $CALLD"
    error "Please build with: cargo build --release --bin calld"
    exit 1
fi

echo "=========================================="
echo "Call-Core Network & Validator Tests"
echo "=========================================="
echo "Binary: $CALLD"
echo "Data Dir: $DATA_DIR"
echo "Config: $CONFIG"
echo ""

# Check if data directory exists
if [ ! -d "$DATA_DIR" ]; then
    error "Data directory not found: $DATA_DIR"
    error "Please start the devnet first: ./devnet/devnet-up.sh native start"
    exit 1
fi

# Use config if available
CMD_BASE="$CALLD"
if [ -f "$CONFIG" ]; then
    CMD_BASE="$CALLD --config $CONFIG"
fi

PASS=0
FAIL=0

# ============================================
# Network Commands
# ============================================
echo ""
echo "=========================================="
echo "Network Commands (net)"
echo "=========================================="

# Test net peers
if test_command "$CMD_BASE net peers" "List connected peers"; then
    ((PASS++))
else
    ((FAIL++))
fi

# Test net crawler
if test_command "$CMD_BASE net crawler" "Show crawler information"; then
    ((PASS++))
else
    ((FAIL++))
fi

# Test net status
if test_command "$CMD_BASE net status" "Show network status"; then
    ((PASS++))
else
    ((FAIL++))
fi

# ============================================
# Validator Commands
# ============================================
echo ""
echo "=========================================="
echo "Validator Commands"
echo "=========================================="

# Test validator info
if test_command "$CMD_BASE validator info" "Show validator info"; then
    ((PASS++))
else
    ((FAIL++))
fi

# Test validator list
if test_command "$CMD_BASE validator list" "List known validators"; then
    ((PASS++))
else
    ((FAIL++))
fi

# Test validator quorum
if test_command "$CMD_BASE validator quorum" "Show quorum information"; then
    ((PASS++))
else
    ((FAIL++))
fi

# ============================================
# Summary
# ============================================
echo ""
echo "=========================================="
echo "Test Summary"
echo "=========================================="
info "Passed: $PASS"
if [ $FAIL -gt 0 ]; then
    error "Failed: $FAIL"
    exit 1
else
    info "All tests passed!"
    exit 0
fi
