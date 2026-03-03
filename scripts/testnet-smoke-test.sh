#!/bin/bash
#
# testnet-smoke-test.sh - Run smoke tests against local testnet
#
# Usage: ./testnet-smoke-test.sh [OPTIONS]
#
# Options:
#   -r, --rpc URL         RPC endpoint (default: http://127.0.0.1:5005)
#   -v, --verbose         Show detailed output
#   -h, --help            Show this help message
#
# Example:
#   ./testnet-smoke-test.sh --rpc http://127.0.0.1:5006

set -e

# Configuration
RPC_URL="http://127.0.0.1:5005"
VERBOSE=0
TESTS_PASSED=0
TESTS_FAILED=0

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

usage() {
    grep '^#' "$0" | tail -n +2 | sed 's/^# //'
    exit 1
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -r|--rpc)
            RPC_URL="$2"
            shift 2
            ;;
        -v|--verbose)
            VERBOSE=1
            shift
            ;;
        -h|--help)
            usage
            ;;
        *)
            echo "Unknown option: $1"
            usage
            ;;
    esac
done

# Test functions
test_rpc_connection() {
    echo -n "Test: RPC connection... "

    local response
    response=$(curl -s "$RPC_URL" -X POST \
        -H "Content-Type: application/json" \
        -d '{"method":"server_info","id":1}' 2>/dev/null) || {
        echo -e "${RED}FAILED${NC}"
        echo "  Error: Cannot connect to RPC at $RPC_URL"
        return 1
    }

    if [[ "$response" == *""* ]]; then
        echo -e "${GREEN}PASSED${NC}"
        [[ $VERBOSE -eq 1 ]] && echo "  Response: $response"
        return 0
    else
        echo -e "${RED}FAILED${NC}"
        echo "  Error: Invalid response"
        return 1
    fi
}

test_server_state() {
    echo -n "Test: Server state... "

    local response
    local state

    response=$(curl -s "$RPC_URL" -X POST \
        -H "Content-Type: application/json" \
        -d '{"method":"server_info"}' 2>/dev/null)

    state=$(echo "$response" | grep -o '"server_state":"[^"]*"' | cut -d'"' -f4 || echo "")

    if [[ -n "$state" ]]; then
        if [[ "$state" == "full" ]] || [[ "$state" == "proposing" ]] || [[ "$state" == "syncing" ]]; then
            echo -e "${GREEN}PASSED${NC} (state: $state)"
            [[ $VERBOSE -eq 1 ]] && echo "  State: $state"
            return 0
        else
            echo -e "${YELLOW}WARNING${NC} (state: $state)"
            return 0
        fi
    else
        echo -e "${RED}FAILED${NC}"
        echo "  Error: Could not parse server state"
        return 1
    fi
}

test_ledger_current() {
    echo -n "Test: Current ledger... "

    local response
    local ledger

    response=$(curl -s "$RPC_URL" -X POST \
        -H "Content-Type: application/json" \
        -d '{"method":"ledger_current"}' 2>/dev/null)

    ledger=$(echo "$response" | grep -o '"ledger_current_index":[0-9]*' | cut -d':' -f2 || echo "")

    if [[ -n "$ledger" ]] && [[ "$ledger" -gt 0 ]]; then
        echo -e "${GREEN}PASSED${NC} (ledger: $ledger)"
        [[ $VERBOSE -eq 1 ]] && echo "  Ledger: $ledger"
        return 0
    else
        echo -e "${RED}FAILED${NC}"
        echo "  Error: Invalid ledger index"
        return 1
    fi
}

test_ping() {
    echo -n "Test: Ping... "

    local response

    response=$(curl -s "$RPC_URL" -X POST \
        -H "Content-Type: application/json" \
        -d '{"method":"ping"}' 2>/dev/null)

    if [[ "$response" == *""* ]]; then
        echo -e "${GREEN}PASSED${NC}"
        return 0
    else
        echo -e "${RED}FAILED${NC}"
        return 1
    fi
}

test_fee() {
    echo -n "Test: Fee endpoint... "

    local response
    local base_fee

    response=$(curl -s "$RPC_URL" -X POST \
        -H "Content-Type: application/json" \
        -d '{"method":"fee"}' 2>/dev/null)

    base_fee=$(echo "$response" | grep -o '"fee_base":[0-9]*' | cut -d':' -f2 || echo "")

    if [[ -n "$base_fee" ]]; then
        echo -e "${GREEN}PASSED${NC} (base fee: $base_fee)"
        return 0
    else
        echo -e "${RED}FAILED${NC}"
        return 1
    fi
}

test_consensus_info() {
    echo -n "Test: Consensus info... "

    local response

    response=$(curl -s "$RPC_URL" -X POST \
        -H "Content-Type: application/json" \
        -d '{"method":"consensus_info"}' 2>/dev/null)

    if [[ "$response" == *"consensus"* ]]; then
        echo -e "${GREEN}PASSED${NC}"
        [[ $VERBOSE -eq 1 ]] && echo "  Response: $response"
        return 0
    else
        echo -e "${RED}FAILED${NC}"
        return 1
    fi
}

test_peers() {
    echo -n "Test: Peers endpoint... "

    local response

    response=$(curl -s "$RPC_URL" -X POST \
        -H "Content-Type: application/json" \
        -d '{"method":"peers"}' 2>/dev/null)

    if [[ "$response" == *"peers"* ]]; then
        local peer_count
        peer_count=$(echo "$response" | grep -o '"peers":[0-9]*' | cut -d':' -f2 || echo "0")
        echo -e "${GREEN}PASSED${NC} (peers: $peer_count)"
        return 0
    else
        echo -e "${RED}FAILED${NC}"
        return 1
    fi
}

# Main
echo "=========================================="
echo "  Call-Core Testnet Smoke Test"
echo "=========================================="
echo ""
echo "RPC Endpoint: $RPC_URL"
echo ""

# Run tests
test_rpc_connection && ((TESTS_PASSED++)) || ((TESTS_FAILED++))
test_ping && ((TESTS_PASSED++)) || ((TESTS_FAILED++))
test_server_state && ((TESTS_PASSED++)) || ((TESTS_FAILED++))
test_ledger_current && ((TESTS_PASSED++)) || ((TESTS_FAILED++))
test_fee && ((TESTS_PASSED++)) || ((TESTS_FAILED++))
test_consensus_info && ((TESTS_PASSED++)) || ((TESTS_FAILED++))
test_peers && ((TESTS_PASSED++)) || ((TESTS_FAILED++))

echo ""
echo "=========================================="
echo "  Results: $TESTS_PASSED passed, $TESTS_FAILED failed"
echo "=========================================="

if [[ $TESTS_FAILED -eq 0 ]]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed!${NC}"
    exit 1
fi
