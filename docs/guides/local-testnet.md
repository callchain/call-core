# Local Testnet Deployment Guide

This guide explains how to set up and run a local Callchain testnet for development and testing.

## Overview

A local testnet allows you to:
- Test transactions without spending real CALL
- Develop and debug applications locally
- Experiment with validator consensus
- Control network parameters (ledger close time, fees, etc.)

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Local Testnet                             │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │   Node 1     │  │   Node 2     │  │   Node 3     │       │
│  │  (Validator) │  │  (Validator) │  │  (Validator) │       │
│  │   :51235     │  │   :51236     │  │   :51237     │       │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘       │
│         │                 │                 │                │
│         └─────────────────┼─────────────────┘                │
│                           │                                  │
│                    ┌──────┴──────┐                          │
│                    │  Genesis     │                          │
│                    │  Ledger      │                          │
│                    └─────────────┘                          │
└─────────────────────────────────────────────────────────────┘
```

## Quick Start

### Prerequisites

- Rust 1.75+ installed
- Call-Core built from source
- jq (for JSON processing)
- curl (for RPC calls)

```bash
# Verify installation
calld --version
```

### Automated Setup

Use the provided setup script:

```bash
# Create testnet with 3 validators
./scripts/testnet-setup.sh --nodes 3 --dir ./testnet

# Start the testnet
./scripts/testnet-start.sh ./testnet

# Check status
./scripts/testnet-status.sh ./testnet

# Stop the testnet
./scripts/testnet-stop.sh ./testnet
```

## Manual Setup

### Step 1: Create Directory Structure

```bash
mkdir -p testnet/{node1,node2,node3}/{data,logs}
```

### Step 2: Generate Validator Keys

Generate unique validation keys for each node:

```bash
# Node 1
calld validation-create > testnet/node1/keys.txt
# Extract: validation_seed and validation_public_key

# Node 2
calld validation-create > testnet/node2/keys.txt

# Node 3
calld validation-create > testnet/node3/keys.txt
```

### Step 3: Create Genesis Ledger

```bash
# Create genesis configuration
cat > testnet/genesis.json << 'EOF'
{
  "ledger_index": 1,
  "close_time": 0,
  "base_fee": 10,
  "reserve_base": 10000000,
  "reserve_inc": 2000000,
  "accounts": [
    {
      "address": "cMASTERACCOUNT111111111111111",
      "balance": "100000000000000000"
    }
  ]
}
EOF

# Generate genesis ledger
calld genesis --create --config testnet/genesis.json --output testnet/genesis.ledger
```

### Step 4: Create Node Configurations

**Node 1 (Primary Validator):**

```toml
# testnet/node1/call-core.toml
[node]
validation_seed = "snNode1ValidationSeedHere"
validation_public_key = "nNode1PublicKeyHere"
database_path = "./testnet/node1/data"
log_file = "./testnet/node1/logs/node.log"
log_level = "info"
network_id = 1  # Testnet

[network]
listen_address = "127.0.0.1:51235"
bootstrap_peers = []
allow_inbound = true
max_peers = 10

[consensus]
validation_quorum = 66  # 2 of 3 validators
ledger_close_time = 3   # 3 seconds for faster testing

[rpc]
enabled = true
address = "127.0.0.1:5005"
admin_only = false

[websocket]
enabled = true
address = "127.0.0.1:6005"

[genesis]
ledger_file = "./testnet/genesis.ledger"
```

**Node 2:**

```toml
# testnet/node2/call-core.toml
[node]
validation_seed = "snNode2ValidationSeedHere"
validation_public_key = "nNode2PublicKeyHere"
database_path = "./testnet/node2/data"
log_file = "./testnet/node2/logs/node.log"
network_id = 1

[network]
listen_address = "127.0.0.1:51236"
bootstrap_peers = ["127.0.0.1:51235"]
allow_inbound = true

[consensus]
validation_quorum = 66

[rpc]
enabled = true
address = "127.0.0.1:5006"

[websocket]
enabled = true
address = "127.0.0.1:6006"
```

**Node 3:**

```toml
# testnet/node3/call-core.toml
[node]
validation_seed = "snNode3ValidationSeedHere"
validation_public_key = "nNode3PublicKeyHere"
database_path = "./testnet/node3/data"
log_file = "./testnet/node3/logs/node.log"
network_id = 1

[network]
listen_address = "127.0.0.1:51237"
bootstrap_peers = ["127.0.0.1:51235", "127.0.0.1:51236"]

[consensus]
validation_quorum = 66

[rpc]
enabled = true
address = "127.0.0.1:5007"

[websocket]
enabled = true
address = "127.0.0.1:6007"
```

### Step 5: Start the Testnet

**Terminal 1 - Node 1:**
```bash
calld --config testnet/node1/call-core.toml
```

**Terminal 2 - Node 2:**
```bash
calld --config testnet/node2/call-core.toml
```

**Terminal 3 - Node 3:**
```bash
calld --config testnet/node3/call-core.toml
```

### Step 6: Verify Network

```bash
# Check Node 1 status
curl -s http://127.0.0.1:5005 \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{"method": "server_info"}' | jq

# Check peer connections
curl -s http://127.0.0.1:5005 \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{"method": "peers"}' | jq

# Check consensus status
curl -s http://127.0.0.1:5005 \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{"method": "consensus_info"}' | jq
```

## Testnet Operations

### Creating Test Accounts

```bash
# Generate a test wallet
calld wallet-generate

# Output:
# Account ID: cTestAccount123...
# Master Seed: snTestSeed...
```

### Funding Test Accounts

Since you control the genesis account, you can fund any address:

```bash
# Create and sign a payment from genesis account
calld sign $GENESIS_SEED '{
  "TransactionType": "Payment",
  "Account": "cMASTERACCOUNT111111111111111",
  "Destination": "cTestAccount123...",
  "Amount": "10000000000",
  "Fee": "10",
  "Sequence": 1
}' > signed_tx.json

# Submit to network
curl -s http://127.0.0.1:5005 \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{
    "method": "submit",
    "params": [{"tx_blob": "'$(jq -r .tx_blob signed_tx.json)'"}]
  }' | jq
```

### Submitting Transactions

```bash
# Create a trust line
calld sign $TEST_SEED '{
  "TransactionType": "TrustSet",
  "Account": "cTestAccount123...",
  "LimitAmount": {
    "currency": "USD",
    "issuer": "cGatewayAccount...",
    "value": "1000000"
  },
  "Fee": "10",
  "Sequence": 1
}' | jq -r '.tx_blob' | xargs -I {} \
curl -s http://127.0.0.1:5005 \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{"method": "submit", "params": [{"tx_blob": "{}"}]}' | jq
```

### Checking Balances

```bash
# Check account balance
curl -s http://127.0.0.1:5005 \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{
    "method": "account_info",
    "params": [{"account": "cTestAccount123..."}]
  }' | jq '.result.account_data.Balance'
```

### WebSocket Subscriptions

```bash
# Subscribe to ledger closes
wscat -c ws://127.0.0.1:6005
> {"id": 1, "command": "subscribe", "streams": ["ledger"]}

# Subscribe to account transactions
> {"id": 2, "command": "subscribe", "accounts": ["cTestAccount123..."]}
```

## Advanced Configuration

### Single-Node Testnet

For simple testing, run a single validator:

```toml
[node]
validation_seed = "snTestSeed..."
standalone = true  # Enable standalone mode

[consensus]
validation_quorum = 0  # No consensus needed
```

In standalone mode, ledgers close immediately when transactions are submitted.

### Custom Ledger Close Time

```toml
[consensus]
ledger_close_time = 1  # 1 second for rapid testing
```

### Resetting the Testnet

To start fresh:

```bash
# Stop all nodes
pkill -f calld

# Clear data directories
rm -rf testnet/*/data/*

# Restart nodes
calld --config testnet/node1/call-core.toml
```

## Troubleshooting

### Nodes Not Connecting

**Symptom:** `peers` RPC returns 0 peers

**Solutions:**
1. Check firewall settings (allow localhost)
2. Verify bootstrap_peers configuration
3. Check node logs for connection errors

```bash
# Check Node 1 is listening
netstat -an | grep 51235

# Check Node 2 can reach Node 1
telnet 127.0.0.1 51235
```

### Consensus Not Reaching

**Symptom:** Ledgers not closing

**Solutions:**
1. Ensure validation_quorum is set correctly (2 of 3 = 66%)
2. Verify all validators have unique keys
3. Check network connectivity between validators

### Port Conflicts

**Symptom:** "Address already in use" error

**Solution:** Ensure each node uses unique ports:
- Node 1: P2P 51235, RPC 5005, WS 6005
- Node 2: P2P 51236, RPC 5006, WS 6006
- Node 3: P2P 51237, RPC 5007, WS 6007

### Genesis Ledger Issues

**Symptom:** Nodes fail to start with genesis errors

**Solution:** Regenerate genesis with proper format:

```bash
calld genesis --create --output testnet/genesis.ledger
```

## Network Scripts

### Automated Testing Script

```bash
#!/bin/bash
# testnet-smoke-test.sh

RPC_URL="http://127.0.0.1:5005"

# Test 1: Server info
echo "Test 1: Server info"
curl -s $RPC_URL -X POST \
  -H "Content-Type: application/json" \
  -d '{"method": "server_info"}' | jq '.result.info.server_state'

# Test 2: Ledger current
echo "Test 2: Current ledger"
curl -s $RPC_URL -X POST \
  -H "Content-Type: application/json" \
  -d '{"method": "ledger_current"}' | jq '.result.ledger_current_index'

# Test 3: Submit transaction
echo "Test 3: Submit transaction"
TX_BLOB="12000022000000002400000001..."
curl -s $RPC_URL -X POST \
  -H "Content-Type: application/json" \
  -d "{\"method\": \"submit\", \"params\": [{\"tx_blob\": \"$TX_BLOB\"}]}" | jq '.result.engine_result'

echo "Smoke test complete!"
```

### Monitor Script

```bash
#!/bin/bash
# testnet-monitor.sh

while true; do
  clear
  echo "=== Local Testnet Status ==="
  echo ""

  for port in 5005 5006 5007; do
    echo "Node (RPC:$port):"
    curl -s http://127.0.0.1:$port \
      -X POST \
      -H "Content-Type: application/json" \
      -d '{"method": "server_info"}' 2>/dev/null | \
      jq -r '[.result.info.server_state, .result.info.validated_ledger.seq] | @tsv' || \
      echo "  UNREACHABLE"
    echo ""
  done

  sleep 5
done
```

## Integration with Applications

### Connecting from JavaScript

```javascript
const { CallClient } = require('callchain.js');

const client = new CallClient('ws://127.0.0.1:6005');

async function main() {
  await client.connect();

  // Get server info
  const info = await client.request({
    command: 'server_info'
  });
  console.log('Server state:', info.info.server_state);

  // Fund a test account
  const wallet = CallClient.generateFaucetWallet();
  console.log('Test account:', wallet.address);

  await client.disconnect();
}

main();
```

### Python Integration

```python
import requests

RPC_URL = "http://127.0.0.1:5005"

def submit_transaction(tx_blob):
    response = requests.post(RPC_URL, json={
        "method": "submit",
        "params": [{"tx_blob": tx_blob}]
    })
    return response.json()

def get_account_info(account):
    response = requests.post(RPC_URL, json={
        "method": "account_info",
        "params": [{"account": account}]
    })
    return response.json()

# Usage
info = get_account_info("cTestAccount123...")
print(f"Balance: {info['result']['account_data']['Balance']}")
```

## Best Practices

1. **Use Fresh Data Directories:** Always clear data when restarting testnet
2. **Version Control Configs:** Store testnet configs in git
3. **Log Rotation:** Enable log rotation to prevent disk fill
4. **Monitoring:** Use the monitor script for real-time status
5. **Backup Genesis:** Save the genesis ledger for consistent testing
6. **Document Ports:** Keep a record of which ports each node uses

## Next Steps

- [Validator Setup](validator-setup.md) - Run a permanent validator
- [Configuration Guide](configuration.md) - Advanced configuration options
- [CLI Reference](cli-reference.md) - All available commands
- [RPC API Reference](../api/rpc.md) - Complete API documentation

## See Also

- [Architecture Overview](../architecture/overview.md) - System design
- [Consensus Algorithm](../consensus/algorithm.md) - How consensus works
- [Contributing Guide](contributing.md) - How to contribute
