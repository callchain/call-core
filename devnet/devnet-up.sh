#!/bin/bash
# Call-core Dev Testnet Management Script
# Usage: ./devnet-up.sh [start|stop|restart|status|clean|logs]
#
# This script manages a 3-node dev testnet for development and testing

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
DEVNET_DIR="$SCRIPT_DIR"
COMPOSE_FILE="$DEVNET_DIR/docker-compose.yml"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if docker is available
check_docker() {
    if ! command -v docker &> /dev/null; then
        log_error "Docker is not installed or not in PATH"
        exit 1
    fi

    if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
        log_error "Docker Compose is not installed"
        exit 1
    fi
}

# Get docker compose command (docker compose vs docker-compose)
get_compose_cmd() {
    if docker compose version &> /dev/null 2>&1; then
        echo "docker compose"
    else
        echo "docker-compose"
    fi
}

# Create node directories
setup_directories() {
    log_info "Setting up node directories..."

    for i in 1 2 3; do
        mkdir -p "$DEVNET_DIR/node$i/data"
    done

    log_success "Node directories created"
}

# Start the devnet
start_devnet() {
    log_info "Starting Call-core dev testnet..."

    check_docker
    setup_directories

    # Check if image exists, if not build it
    if ! docker images callchain/call-core:latest --format "{{.Repository}}" | grep -q "callchain/call-core"; then
        log_warn "Docker image not found. Building..."
        "$PROJECT_ROOT/scripts/docker-build.sh"
    fi

    local compose_cmd=$(get_compose_cmd)

    cd "$DEVNET_DIR"
    $compose_cmd -f docker-compose.yml up -d

    log_success "Dev testnet started!"
    echo ""
    log_info "Node status:"
    show_status

    echo ""
    log_info "RPC Endpoints:"
    echo "  Node 1: http://localhost:5005"
    echo "  Node 2: http://localhost:5006"
    echo "  Node 3: http://localhost:5007"
    echo ""
    log_info "Use '$0 status' to check node status"
    log_info "Use '$0 logs' to view logs"
}

# Stop the devnet
stop_devnet() {
    log_info "Stopping Call-core dev testnet..."

    check_docker

    local compose_cmd=$(get_compose_cmd)
    cd "$DEVNET_DIR"
    $compose_cmd -f docker-compose.yml down

    log_success "Dev testnet stopped"
}

# Restart the devnet
restart_devnet() {
    log_info "Restarting Call-core dev testnet..."
    stop_devnet
    sleep 2
    start_devnet
}

# Show devnet status
show_status() {
    check_docker

    local compose_cmd=$(get_compose_cmd)
    cd "$DEVNET_DIR"
    $compose_cmd -f docker-compose.yml ps
}

# Show logs
show_logs() {
    check_docker

    local compose_cmd=$(get_compose_cmd)
    cd "$DEVNET_DIR"

    if [ -n "$1" ]; then
        # Show logs for specific node
        $compose_cmd -f docker-compose.yml logs -f "$1"
    else
        # Show logs for all nodes
        $compose_cmd -f docker-compose.yml logs -f
    fi
}

# Clean up everything including volumes
clean_devnet() {
    log_warn "This will remove all data directories and stop the devnet"
    read -p "Are you sure? (y/N) " confirm

    if [[ ! $confirm =~ ^[Yy]$ ]]; then
        log_info "Cancelled"
        exit 0
    fi

    log_info "Cleaning up dev testnet..."

    check_docker

    local compose_cmd=$(get_compose_cmd)
    cd "$DEVNET_DIR"
    $compose_cmd -f docker-compose.yml down -v

    # Remove data directories
    for i in 1 2 3; do
        rm -rf "$DEVNET_DIR/node$i/data"
    done

    log_success "Dev testnet cleaned up"
}

# Quick test of the devnet
test_devnet() {
    log_info "Testing devnet connectivity..."

    # Test RPC endpoints
    for port in 5005 5006 5007; do
        echo -n "  Testing Node on port $port... "
        response=$(curl -s -X POST "http://localhost:$port/" \
            -H "Content-Type: application/json" \
            -d '{"jsonrpc":"2.0","method":"ping","id":1}' 2>/dev/null)

        if echo "$response" | grep -q "success"; then
            echo -e "${GREEN}OK${NC}"
        else
            echo -e "${RED}FAILED${NC}"
        fi
    done

    # Get server info from node 1
    echo ""
    log_info "Server info from Node 1:"
    curl -s -X POST "http://localhost:5005/" \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","method":"server_info","id":1}' | \
        python3 -m json.tool 2>/dev/null || cat
}

# Print usage
print_usage() {
    echo "Call-core Dev Testnet Management"
    echo ""
    echo "Usage: $0 <command> [options]"
    echo ""
    echo "Commands:"
    echo "  start       Start the dev testnet (default)"
    echo "  stop        Stop the dev testnet"
    echo "  restart     Restart the dev testnet"
    echo "  status      Show node status"
    echo "  logs        View logs (optionally specify node: node1, node2, node3)"
    echo "  clean       Remove all data and stop the devnet"
    echo "  test        Test the devnet connectivity"
    echo "  help        Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 start              # Start the devnet"
    echo "  $0 logs               # View all logs"
    echo "  $0 logs call-dev-node-1  # View logs for node 1"
    echo "  $0 test               # Test connectivity"
    echo "  $0 clean              # Clean up everything"
}

# Main command handler
case "${1:-start}" in
    start|up)
        start_devnet
        ;;
    stop|down)
        stop_devnet
        ;;
    restart)
        restart_devnet
        ;;
    status|ps)
        show_status
        ;;
    logs)
        show_logs "$2"
        ;;
    clean)
        clean_devnet
        ;;
    test)
        test_devnet
        ;;
    help|-h|--help)
        print_usage
        ;;
    *)
        log_error "Unknown command: $1"
        print_usage
        exit 1
        ;;
esac
