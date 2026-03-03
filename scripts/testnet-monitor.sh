#!/bin/bash
#
# testnet-monitor.sh - Monitor local testnet status
#
# Usage: ./testnet-monitor.sh [OPTIONS]
#
# Options:
#   -n, --nodes NUM       Number of nodes to monitor (default: auto-detect)
#   -i, --interval SEC    Refresh interval in seconds (default: 5)
#   -r, --rpc-base PORT   Base RPC port (default: 5005)
#   -h, --help            Show this help message
#
# Example:
#   ./testnet-monitor.sh --nodes 3 --interval 3

set -e

# Configuration
NUM_NODES=0
INTERVAL=5
RPC_BASE_PORT=5005

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

usage() {
    grep '^#' "$0" | tail -n +2 | sed 's/^# //'
    exit 1
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -n|--nodes)
            NUM_NODES="$2"
            shift 2
            ;;
        -i|--interval)
            INTERVAL="$2"
            shift 2
            ;;
        -r|--rpc-base)
            RPC_BASE_PORT="$2"
            shift 2
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

# Auto-detect nodes if not specified
if [[ $NUM_NODES -eq 0 ]]; then
    # Try to detect from process list or common directories
    if [[ -d "./testnet" ]]; then
        NUM_NODES=$(ls -d ./testnet/node* 2>/dev/null | wc -l)
    fi

    # Default to 3 if still not detected
    if [[ $NUM_NODES -eq 0 ]]; then
        NUM_NODES=3
    fi
fi

# Function to query node status
query_node() {
    local port=$1
    local response
    local state
    local ledger
    local peers
    local uptime

    response=$(curl -s "http://127.0.0.1:$port" \
        -X POST \
        -H "Content-Type: application/json" \
        -d '{"method":"server_info"}' 2>/dev/null) || {
        echo "unreachable"
        return
    }

    state=$(echo "$response" | grep -o '"server_state":"[^"]*"' | cut -d'"' -f4 || echo "unknown")
    ledger=$(echo "$response" | grep -o '"seq":[0-9]*' | head -1 | cut -d':' -f2 || echo "-")
    peers=$(echo "$response" | grep -o '"peers":[0-9]*' | cut -d':' -f2 || echo "-")
    uptime=$(echo "$response" | grep -o '"uptime":[0-9]*' | cut -d':' -f2 || echo "-")

    echo "$state|$ledger|$peers|$uptime"
}

# Function to get color for state
state_color() {
    case "$1" in
        full|proposing)
            echo -e "${GREEN}$1${NC}"
            ;;
        syncing|tracking)
            echo -e "${YELLOW}$1${NC}"
            ;;
        connected|disconnected)
            echo -e "${RED}$1${NC}"
            ;;
        *)
            echo "$1"
            ;;
    esac
}

# Main loop
clear_screen() {
    # Clear screen and move cursor to top
    printf '\033[2J\033[H'
}

while true; do
    clear_screen

    echo "=========================================="
    echo "  Call-Core Local Testnet Monitor"
    echo "=========================================="
    echo ""
    echo -e "${BLUE}Node Status:${NC}"
    echo ""

    printf "%-8s %-15s %-10s %-10s %-8s %-10s\n" \
        "Node" "RPC Port" "State" "Ledger" "Peers" "Uptime"
    printf "%0.s-" {1..70}
    echo ""

    for i in $(seq 1 $NUM_NODES); do
        port=$((RPC_BASE_PORT + i - 1))
        status=$(query_node $port)

        if [[ "$status" == "unreachable" ]]; then
            printf "%-8s %-15s %-15s\n" \
                "Node $i" "$port" "${RED}UNREACHABLE${NC}"
        else
            IFS='|' read -r state ledger peers uptime <<< "$status"
            colored_state=$(state_color "$state")

            # Format uptime
            if [[ "$uptime" != "-" ]] && [[ -n "$uptime" ]]; then
                uptime_str="${uptime}s"
            else
                uptime_str="-"
            fi

            printf "%-8s %-15s %-15s %-10s %-8s %-10s\n" \
                "Node $i" "$port" "$colored_state" "$ledger" "$peers" "$uptime_str"
        fi
    done

    echo ""
    printf "%0.s-" {1..70}
    echo ""

    # Check consensus across nodes
    echo ""
    echo -e "${BLUE}Consensus Check:${NC}"

    ledgers=""
    consensus_ok=true

    for i in $(seq 1 $NUM_NODES); do
        port=$((RPC_BASE_PORT + i - 1))
        response=$(curl -s "http://127.0.0.1:$port" \
            -X POST \
            -H "Content-Type: application/json" \
            -d '{"method":"ledger_current"}' 2>/dev/null) || continue

        ledger=$(echo "$response" | grep -o '"ledger_current_index":[0-9]*' | cut -d':' -f2 || echo "")

        if [[ -n "$ledger" ]]; then
            if [[ -z "$ledgers" ]]; then
                ledgers="$ledger"
                first_ledger="$ledger"
            elif [[ "$ledger" != "$first_ledger" ]]; then
                consensus_ok=false
            fi
        fi
    done

    if $consensus_ok && [[ -n "$first_ledger" ]]; then
        echo -e "  ${GREEN}✓${NC} All nodes at ledger $first_ledger"
    else
        echo -e "  ${YELLOW}!${NC} Nodes at different ledgers"
    fi

    echo ""
    echo "Press Ctrl+C to exit"
    echo "Refreshing every ${INTERVAL} seconds..."

    sleep $INTERVAL
done
