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

# ========================
# Native (Local Binary) Functions - 3 Nodes
# ========================

NATIVE_PID_DIR="$DEVNET_DIR/.native-pids"
NATIVE_LOG_DIR="$DEVNET_DIR/.native-logs"

# Check if local binary exists
check_native_binary() {
    if [ ! -f "$PROJECT_ROOT/target/release/calld" ]; then
        log_error "Local binary not found: $PROJECT_ROOT/target/release/calld"
        log_info "Please build the release binary first:"
        log_info "  cargo build --release --bin calld"
        exit 1
    fi
}

# Create native mode directories and configs
setup_native() {
    log_info "Setting up native mode directories..."

    mkdir -p "$NATIVE_PID_DIR"
    mkdir -p "$NATIVE_LOG_DIR"

    # Create genesis.json for native mode (matching default_devnet() in genesis.rs)
    # These addresses match what stress_test.py expects
    cat > "$DEVNET_DIR/genesis.json" << 'GENEOF'
{
  "config": {
    "chainId": 1337,
    "networkName": "callchain-devnet",
    "genesisTime": "2025-01-01T00:00:00Z",
    "consensusParams": {
      "ledgerMinCloseTime": 2,
      "ledgerMaxCloseTime": 20,
      "ledgerMinConsensus": 1,
      "ledgerMaxConsensus": 50,
      "validationQuorum": 66,
      "minProposeTime": 3,
      "maxProposeTime": 30
    },
    "feeSettings": {
      "baseFee": 10,
      "reserveBase": 10000000,
      "reserveIncrement": 2000000
    }
  },
  "alloc": {
    "cGmJBrEfFssWuas4kCoHTX5r6aMEf6QHhy": {
      "balance": "100000000000",
      "sequence": 1,
      "flags": 0
    },
    "c3K3xXhvsWBnP3TitQfeg2ihAuaYybvtc7": {
      "balance": "50000000000",
      "sequence": 1,
      "flags": 0
    },
    "cHSFoKcGXFZdbB7EKmWQMTUJbr66dwfMR1": {
      "balance": "25000000000",
      "sequence": 1,
      "flags": 0
    },
    "cKKeufyrSZymFeGmtF1Vhi11eCSf2i6MhR": {
      "balance": "10000000000",
      "sequence": 1,
      "flags": 0
    },
    "cUUsn5u9qPq7MiMiEDwdjMPsHHKyaesHPH": {
      "balance": "5000000000",
      "sequence": 1,
      "flags": 0
    }
  },
  "validators": [],
  "coinbase": "cGmJBrEfFssWuas4kCoHTX5r6aMEf6QHhy"
}
GENEOF

    for i in 1 2 3; do
        mkdir -p "$DEVNET_DIR/node$i/data"

        # Create config for native mode (localhost instead of Docker network)
        # Set validation seed based on node number
        if [ $i -eq 1 ]; then
            validation_seed="sEdTLQ75P2X9VBbNqGihzrNWGtE7d6NHPmgSG8q5d8fM7YjHcXK"
        elif [ $i -eq 2 ]; then
            validation_seed="sEdTvJH6xms2zZ4sDqpadBP4FVsK2KXGv4PBg4Pd8QhNbmXvSRL"
        else
            validation_seed="sEdTvyZ9d9ZFz1LzhAuvP7CK3v5qPQzAcXKPm2fCBLkH2xKo1KQ"
        fi

        # Calculate peer ports (each node connects to the other two)
        if [ $i -eq 1 ]; then
            peer1="127.0.0.1:51236"
            peer2="127.0.0.1:51237"
        elif [ $i -eq 2 ]; then
            peer1="127.0.0.1:51235"
            peer2="127.0.0.1:51237"
        else
            peer1="127.0.0.1:51235"
            peer2="127.0.0.1:51236"
        fi

        cat > "$DEVNET_DIR/node$i/config-native.toml" << EOF
# Call-core Node $i Configuration (Native Mode)
node_name = "call-dev-node-$i"

# P2P network settings
listen_address = "127.0.0.1:$((51234 + i))"

# Bootstrap peers - connect to other nodes on localhost
peers = [
    "$peer1",
    "$peer2",
]

# Data directory for blockchain storage
data_dir = "$DEVNET_DIR/node$i/data"

# Genesis configuration (must match stress test expectations)
genesis_file = "$DEVNET_DIR/genesis.json"

# Validator configuration
validation_seed = "$validation_seed"

# Peer connection limits
max_peers = 50
target_peers = 10

# RPC server configuration
rpc_enabled = true
rpc_bind_address = "127.0.0.1"
rpc_port = $((5004 + i))

# Enable admin RPC methods
rpc_admin_enabled = true

# Logging (must be before table sections)
log_level = "debug"

# WebSocket server configuration
[websocket]
enabled = true
bind_address = "127.0.0.1"
port = $((6004 + i))
max_connections = 1000

# Transaction pool configuration - increased for large stress tests
[transaction_pool]
pre_seq_cache_enabled = true
pre_seq_cache_rounds = 100
pre_seq_cache_max_size = 200000
pre_seq_per_account_limit = 50000
pre_seq_max_sequence_gap = 10000
max_tx_per_account_per_ledger = 5000

# Consensus configuration
[consensus]
# Validation quorum percentage (80% for BFT consensus)
validation_quorum = 66
# Minimum consensus percentage for ledger close
ledger_min_consensus_pct = 66
# Minimum validators required for consensus
ledger_min_consensus = 1
EOF
    done

    log_success "Native mode configs created"
}

# Start native mode (3 nodes)
start_native() {
    log_info "Starting 3-node native devnet..."

    check_native_binary
    setup_native

    # Stop any existing native nodes
    stop_native_silent

    # Start each node
    for i in 1 2 3; do
        log_info "Starting node $i on RPC port $((5004 + i)), P2P port $((51234 + i))..."

        RUST_LOG=debug "$PROJECT_ROOT/target/release/calld" \
            --config "$DEVNET_DIR/node$i/config-native.toml" \
            start > "$NATIVE_LOG_DIR/node$i.log" 2>&1 &

        echo $! > "$NATIVE_PID_DIR/node$i.pid"
        log_success "Node $i started (PID: $!)"
    done

    echo ""
    log_info "Waiting for nodes to initialize..."
    sleep 5

    echo ""
    log_success "Native devnet started!"
    log_info "RPC Endpoints:"
    echo "  Node 1: http://localhost:5005"
    echo "  Node 2: http://localhost:5006"
    echo "  Node 3: http://localhost:5007"
    echo ""
    log_info "WebSocket Endpoints:"
    echo "  Node 1: ws://localhost:6005"
    echo "  Node 2: ws://localhost:6006"
    echo "  Node 3: ws://localhost:6007"
    echo ""
    log_info "Logs: $NATIVE_LOG_DIR/"
    log_info "Use '$0 native stop' to stop the devnet"
}

# Stop native mode (silent - for internal use)
stop_native_silent() {
    for i in 1 2 3; do
        if [ -f "$NATIVE_PID_DIR/node$i.pid" ]; then
            pid=$(cat "$NATIVE_PID_DIR/node$i.pid")
            if kill -0 "$pid" 2>/dev/null; then
                kill "$pid" 2>/dev/null || true
            fi
            rm -f "$NATIVE_PID_DIR/node$i.pid"
        fi
        # Also kill any lingering calld processes with our config
        pkill -f "calld.*config-native.toml" 2>/dev/null || true
    done
}

# Stop native mode
stop_native() {
    log_info "Stopping native devnet..."

    stop_native_silent

    log_success "Native devnet stopped"
}

# Show native mode status
status_native() {
    echo "Native Devnet Status:"
    echo "====================="
    for i in 1 2 3; do
        if [ -f "$NATIVE_PID_DIR/node$i.pid" ]; then
            pid=$(cat "$NATIVE_PID_DIR/node$i.pid")
            if kill -0 "$pid" 2>/dev/null; then
                echo -e "Node $i: ${GREEN}Running${NC} (PID: $pid)"
                # Test RPC
                response=$(curl -s -m 2 -X POST "http://localhost:$((5004 + i))/" \
                    -H "Content-Type: application/json" \
                    -d '{"jsonrpc":"2.0","method":"ping","id":1}' 2>/dev/null)
                if echo "$response" | grep -q "success"; then
                    echo "  RPC: OK (port $((5004 + i)))"
                else
                    echo "  RPC: Not responding"
                fi
            else
                echo -e "Node $i: ${RED}Not running${NC} (stale PID file)"
                rm -f "$NATIVE_PID_DIR/node$i.pid"
            fi
        else
            echo -e "Node $i: ${RED}Not running${NC}"
        fi
    done
}

# Show native mode logs
logs_native() {
    if [ -n "$1" ]; then
        node_num="${1#node}"
        if [ -f "$NATIVE_LOG_DIR/node$node_num.log" ]; then
            tail -f "$NATIVE_LOG_DIR/node$node_num.log"
        else
            log_error "Log file not found for node $node_num"
        fi
    else
        log_info "Showing logs for all nodes (Ctrl+C to exit)..."
        if command -v multitail &> /dev/null; then
            multitail "$NATIVE_LOG_DIR/node1.log" "$NATIVE_LOG_DIR/node2.log" "$NATIVE_LOG_DIR/node3.log" 2>/dev/null || \
                tail -f "$NATIVE_LOG_DIR/node1.log" "$NATIVE_LOG_DIR/node2.log" "$NATIVE_LOG_DIR/node3.log"
        else
            tail -f "$NATIVE_LOG_DIR/node1.log" "$NATIVE_LOG_DIR/node2.log" "$NATIVE_LOG_DIR/node3.log"
        fi
    fi
}

# Clean native mode
clean_native() {
    log_warn "This will stop native devnet and remove all data"
    read -p "Are you sure? (y/N) " confirm

    if [[ ! $confirm =~ ^[Yy]$ ]]; then
        log_info "Cancelled"
        exit 0
    fi

    log_info "Cleaning up native devnet..."

    stop_native_silent

    # Remove data directories
    for i in 1 2 3; do
        rm -rf "$DEVNET_DIR/node$i/data"
        rm -f "$DEVNET_DIR/node$i/config-native.toml"
    done

    rm -rf "$NATIVE_PID_DIR"
    rm -rf "$NATIVE_LOG_DIR"

    log_success "Native devnet cleaned up"
}

# Test native mode connectivity
test_native_connectivity() {
    log_info "Testing native devnet connectivity..."

    # Test RPC endpoints
    for port in 5005 5006 5007; do
        echo -n "  Testing Node on port $port... "

        # Retry logic for node check
        local retries=3
        local retry_count=0
        local node_ok=false

        while [ $retry_count -lt $retries ]; do
            response=$(curl -s -m 3 -X POST "http://localhost:$port/" \
                -H "Content-Type: application/json" \
                -d '{"jsonrpc":"2.0","method":"ping","id":1}' 2>/dev/null)

            if echo "$response" | grep -q "success"; then
                node_ok=true
                break
            fi

            retry_count=$((retry_count + 1))
            if [ $retry_count -lt $retries ]; then
                sleep 1
            fi
        done

        if [ "$node_ok" = true ]; then
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

# Stress test the native devnet
stress_native() {
    local stress_type="${1:-full}"
    local iterations="${2:-100}"
    local threads="${3:-4}"

    log_info "Running native devnet stress test (type: $stress_type)..."

    # Check if Python is available
    if ! command -v python3 &> /dev/null; then
        log_error "Python3 is required for stress testing"
        exit 1
    fi

    # Check if native devnet is running (with retry)
    local node1_ok=false
    local retries=3
    local retry_count=0

    while [ $retry_count -lt $retries ]; do
        local response=$(curl -s -m 5 -X POST "http://localhost:5005/" \
            -H "Content-Type: application/json" \
            -d '{"jsonrpc":"2.0","method":"ping","id":1}' 2>/dev/null)
        if echo "$response" | grep -q "success"; then
            node1_ok=true
            break
        fi
        retry_count=$((retry_count + 1))
        log_warn "Node not responding, retrying... ($retry_count/$retries)"
        sleep 1
    done

    if [ "$node1_ok" = false ]; then
        log_error "Native devnet is not running. Start it first with: $0 native start"
        exit 1
    fi

    local script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

    case "$stress_type" in
        simple)
            log_info "Running simple stress test ($iterations iterations)..."
            python3 "$script_dir/simple_stress_test.py" \
                --url http://localhost:5005 \
                --iterations "$iterations"
            ;;
        full|comprehensive)
            log_info "Running comprehensive stress test ($iterations iterations, $threads threads)..."
            python3 "$script_dir/stress_test.py" \
                --url http://localhost:5005 \
                --count "$iterations" \
                --threads "$threads"
            ;;
        all)
            log_info "Running all stress tests..."
            echo ""
            log_info "=== Simple Stress Test ==="
            python3 "$script_dir/simple_stress_test.py" \
                --url http://localhost:5005 \
                --iterations "$iterations"
            echo ""
            log_info "=== Comprehensive Stress Test ==="
            python3 "$script_dir/stress_test.py" \
                --url http://localhost:5005 \
                --count "$iterations" \
                --threads "$threads"
            ;;
        *)
            log_error "Unknown stress test type: $stress_type"
            echo "Valid types: simple, full, all"
            exit 1
            ;;
    esac

    local exit_code=$?
    if [ $exit_code -eq 0 ]; then
        log_success "Stress test completed successfully"
    else
        log_error "Stress test failed with exit code $exit_code"
    fi
    return $exit_code
}

# ========================
# Single Node Functions
# ========================

# Start single node for transaction type testing
start_single() {
    log_info "Starting Call-core single node..."

    check_docker

    # Setup single node directory
    mkdir -p "$DEVNET_DIR/node-single/data"

    # Check if image exists, if not build it
    if ! docker images callchain/call-core:latest --format "{{.Repository}}" | grep -q "callchain/call-core"; then
        log_warn "Docker image not found. Building..."
        "$PROJECT_ROOT/scripts/docker-build.sh"
    fi

    local compose_cmd=$(get_compose_cmd)

    cd "$DEVNET_DIR"
    $compose_cmd -f docker-compose.single.yml up -d

    log_success "Single node started!"
    echo ""
    log_info "Node status:"
    $compose_cmd -f docker-compose.single.yml ps

    echo ""
    log_info "RPC Endpoint: http://localhost:5005"
    log_info "Use '$0 single stop' to stop the node"
    log_info "Use '$0 single test' to run transaction type tests"
    log_info "Use '$0 single logs' to view logs"
}

# Stop single node
stop_single() {
    log_info "Stopping Call-core single node..."

    check_docker

    local compose_cmd=$(get_compose_cmd)
    cd "$DEVNET_DIR"
    $compose_cmd -f docker-compose.single.yml down

    log_success "Single node stopped"
}

# Clean single node
clean_single() {
    log_warn "This will remove single node data directory"
    read -p "Are you sure? (y/N) " confirm

    if [[ ! $confirm =~ ^[Yy]$ ]]; then
        log_info "Cancelled"
        exit 0
    fi

    log_info "Cleaning up single node..."

    check_docker

    local compose_cmd=$(get_compose_cmd)
    cd "$DEVNET_DIR"
    $compose_cmd -f docker-compose.single.yml down -v

    # Remove data directory
    rm -rf "$DEVNET_DIR/node-single/data"

    log_success "Single node cleaned up"
}

# Show single node status
status_single() {
    check_docker

    local compose_cmd=$(get_compose_cmd)
    cd "$DEVNET_DIR"
    $compose_cmd -f docker-compose.single.yml ps
}

# Show single node logs
logs_single() {
    check_docker

    local compose_cmd=$(get_compose_cmd)
    cd "$DEVNET_DIR"
    $compose_cmd -f docker-compose.single.yml logs -f
}

# Test single node connectivity
test_single_connectivity() {
    log_info "Testing single node connectivity..."

    # Test RPC endpoint
    echo -n "  Testing node on port 5005... "

    local retries=5
    local retry_count=0
    local node_ok=false

    while [ $retry_count -lt $retries ]; do
        response=$(curl -s -m 3 -X POST "http://localhost:5005/" \
            -H "Content-Type: application/json" \
            -d '{"jsonrpc":"2.0","method":"ping","id":1}' 2>/dev/null)

        if echo "$response" | grep -q "success"; then
            node_ok=true
            break
        fi

        retry_count=$((retry_count + 1))
        if [ $retry_count -lt $retries ]; then
            sleep 1
        fi
    done

    if [ "$node_ok" = true ]; then
        echo -e "${GREEN}OK${NC}"
    else
        echo -e "${RED}FAILED${NC}"
        return 1
    fi

    # Get server info
    echo ""
    log_info "Server info:"
    curl -s -X POST "http://localhost:5005/" \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","method":"server_info","id":1}' | \
        python3 -m json.tool 2>/dev/null || cat
}

# Run transaction type tests on single node
test_single_transactions() {
    log_info "Running transaction type tests on single node..."

    # Check if Python is available
    if ! command -v python3 &> /dev/null; then
        log_error "Python3 is required for testing"
        exit 1
    fi

    # Check if node is running
    local node_ok=false
    local retries=3
    local retry_count=0

    while [ $retry_count -lt $retries ]; do
        local response=$(curl -s -m 5 -X POST "http://localhost:5005/" \
            -H "Content-Type: application/json" \
            -d '{"jsonrpc":"2.0","method":"ping","id":1}' 2>/dev/null)
        if echo "$response" | grep -q "success"; then
            node_ok=true
            break
        fi
        retry_count=$((retry_count + 1))
        log_warn "Node not responding, retrying... ($retry_count/$retries)"
        sleep 1
    done

    if [ "$node_ok" = false ]; then
        log_error "Single node is not running. Start it first with: $0 single start"
        exit 1
    fi

    # Run the test script
    python3 "$DEVNET_DIR/test_all_types.py" --url http://localhost:5005

    return $?
}

# Quick test of the devnet
test_devnet() {
    log_info "Testing devnet connectivity..."

    # Test RPC endpoints
    for port in 5005 5006 5007; do
        echo -n "  Testing Node on port $port... "

        # Retry logic for node check
        local retries=3
        local retry_count=0
        local node_ok=false

        while [ $retry_count -lt $retries ]; do
            response=$(curl -s -m 3 -X POST "http://localhost:$port/" \
                -H "Content-Type: application/json" \
                -d '{"jsonrpc":"2.0","method":"ping","id":1}' 2>/dev/null)

            if echo "$response" | grep -q "success"; then
                node_ok=true
                break
            fi

            retry_count=$((retry_count + 1))
            if [ $retry_count -lt $retries ]; then
                sleep 1
            fi
        done

        if [ "$node_ok" = true ]; then
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

# Stress test the devnet
stress_devnet() {
    local stress_type="${1:-full}"
    local iterations="${2:-100}"
    local threads="${3:-4}"

    log_info "Running stress test (type: $stress_type)..."

    # Check if Python is available
    if ! command -v python3 &> /dev/null; then
        log_error "Python3 is required for stress testing"
        exit 1
    fi

    # Check if devnet is running (with retry)
    local node1_ok=false
    local retries=3
    local retry_count=0

    while [ $retry_count -lt $retries ]; do
        local response=$(curl -s -m 5 -X POST "http://localhost:5005/" \
            -H "Content-Type: application/json" \
            -d '{"jsonrpc":"2.0","method":"ping","id":1}' 2>/dev/null)
        if echo "$response" | grep -q "success"; then
            node1_ok=true
            break
        fi
        retry_count=$((retry_count + 1))
        log_warn "Node not responding, retrying... ($retry_count/$retries)"
        sleep 1
    done

    if [ "$node1_ok" = false ]; then
        log_error "Devnet is not running. Start it first with: $0 start"
        exit 1
    fi

    local script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

    case "$stress_type" in
        simple)
            log_info "Running simple stress test ($iterations iterations)..."
            python3 "$script_dir/simple_stress_test.py" \
                --url http://localhost:5005 \
                --iterations "$iterations"
            ;;
        full|comprehensive)
            log_info "Running comprehensive stress test ($iterations iterations, $threads threads)..."
            python3 "$script_dir/stress_test.py" \
                --url http://localhost:5005 \
                --count "$iterations" \
                --threads "$threads"
            ;;
        all)
            log_info "Running all stress tests..."
            echo ""
            log_info "=== Simple Stress Test ==="
            python3 "$script_dir/simple_stress_test.py" \
                --url http://localhost:5005 \
                --iterations "$iterations"
            echo ""
            log_info "=== Comprehensive Stress Test ==="
            python3 "$script_dir/stress_test.py" \
                --url http://localhost:5005 \
                --count "$iterations" \
                --threads "$threads"
            ;;
        *)
            log_error "Unknown stress test type: $stress_type"
            echo "Valid types: simple, full, all"
            exit 1
            ;;
    esac

    local exit_code=$?
    if [ $exit_code -eq 0 ]; then
        log_success "Stress test completed successfully"
    else
        log_error "Stress test failed with exit code $exit_code"
    fi
    return $exit_code
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
    echo "  stress      Run stress tests [type] [iterations] [threads]"
    echo "  single      Single node commands (start|stop|test|logs|clean)"
    echo "  help        Show this help message"
    echo ""
    echo "Stress Test Types:"
    echo "  full        Comprehensive transaction testing (default)"
    echo "  simple      Basic RPC endpoint testing only"
    echo "  all         Run both simple and full tests"
    echo ""
    echo "Native Mode Commands (3 nodes, local binary):"
    echo "  native start  Start 3-node devnet using local binary"
    echo "  native stop   Stop native devnet"
    echo "  native status Show native devnet status"
    echo "  native logs   View native node logs"
    echo "  native clean  Clean native devnet data"
    echo "  native test   Test native devnet connectivity"
    echo "  native stress Run stress tests on native devnet"
    echo ""
    echo "Single Node Commands:"
    echo "  single start  Start single node for transaction testing"
    echo "  single stop   Stop single node"
    echo "  single test   Test all transaction types (one per type)"
    echo "  single logs   View single node logs"
    echo "  single clean  Clean single node data"
    echo ""
    echo "Examples:"
    echo "  $0 start                      # Start the docker devnet (default)"
    echo "  $0 native start               # Start 3-node native devnet"
    echo "  $0 native stop                # Stop native devnet"
    echo "  $0 native test                # Test native devnet connectivity"
    echo "  $0 native stress              # Run stress test on native devnet"
    echo "  $0 native stress full 100 4   # Run full stress: 100 iters, 4 threads"
    echo "  $0 logs                       # View all logs"
    echo "  $0 logs call-dev-node-1       # View logs for node 1"
    echo "  $0 test                       # Test connectivity"
    echo "  $0 stress                     # Run full stress test (default: 100 iters)"
    echo "  $0 stress full 100 8          # Run full test: 100 iterations, 8 threads"
    echo "  $0 stress simple 500          # Run simple RPC test: 500 iterations"
    echo "  $0 stress all 50              # Run both tests with 50 iterations"
    echo "  $0 single start               # Start single node"
    echo "  $0 single test                # Test all transaction types"
    echo "  $0 single stop                # Stop single node"
    echo "  $0 clean                      # Clean up everything"
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
    stress)
        stress_devnet "$2" "$3" "$4"
        ;;
    native)
        case "${2:-start}" in
            start|up)
                start_native
                ;;
            stop|down)
                stop_native
                ;;
            status|ps)
                status_native
                ;;
            logs)
                logs_native "$3"
                ;;
            clean)
                clean_native
                ;;
            test|ping)
                test_native_connectivity
                ;;
            stress)
                stress_native "$3" "$4" "$5"
                ;;
            *)
                log_error "Unknown native command: $2"
                echo "Valid native commands: start, stop, status, logs, clean, test, stress"
                exit 1
                ;;
        esac
        ;;
    single)
        case "${2:-test}" in
            start|up)
                start_single
                ;;
            stop|down)
                stop_single
                ;;
            status|ps)
                status_single
                ;;
            logs)
                logs_single
                ;;
            clean)
                clean_single
                ;;
            test|txtest)
                test_single_transactions
                ;;
            ping)
                test_single_connectivity
                ;;
            *)
                log_error "Unknown single command: $2"
                echo "Valid single commands: start, stop, test, logs, clean, status, ping"
                exit 1
                ;;
        esac
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
