# Call-core 3-Node Dev Testnet

Quick guide to start a local 3-node validator testnet for development and testing.

## Prerequisites

- Docker and Docker Compose installed
- Git (to clone the repository)

## Quick Start

### Step 1: Build the Docker Image

```bash
# Clone the repository (if not already done)
git clone https://github.com/callchain/call-core.git
cd call-core

# Build the Docker image
./scripts/docker-build.sh
```

This creates a multi-stage optimized image tagged as `callchain/call-core:latest`.

### Step 2: Start the Testnet

```bash
# Start the 3-node dev testnet
./devnet/devnet-up.sh start
```

The script will:
- Create data directories for each node
- Start 3 Docker containers (one per validator node)
- Configure automatic peer discovery

### Step 3: Verify the Network

```bash
# Check node status
./devnet/devnet-up.sh status

# Test RPC connectivity
./devnet/devnet-up.sh test
```

Expected output shows all 3 nodes running and responding.

## Network Architecture

```
                        ┌──────────────────┐
                        │   call-devnet    │
                        │   Docker Network │
                        └────────┬─────────┘
                                 │
        ┌────────────────────────┼────────────────────────┐
        │                        │                        │
        ▼                        ▼                        ▼
┌───────────────┐      ┌───────────────┐      ┌───────────────┐
│ call-dev-     │      │ call-dev-     │      │ call-dev-     │
│ node-1        │      │ node-2        │      │ node-3        │
│               │      │               │      │               │
│ RPC:  5005    │◄────►│ RPC:  5006    │◄────►│ RPC:  5007    │
│ P2P:  51235   │      │ P2P:  51236   │      │ P2P:  51237   │
│               │      │               │      │               │
│ Validator: ✓  │      │ Validator: ✓  │      │ Validator: ✓  │
└───────────────┘      └───────────────┘      └───────────────┘
```

## Node Configuration

| Node | Container Name | RPC Port | P2P Port | Data Directory |
|------|---------------|----------|----------|----------------|
| 1 | call-dev-node-1 | 5005 | 51235 | ./devnet/node1/data |
| 2 | call-dev-node-2 | 5006 | 51236 | ./devnet/node2/data |
| 3 | call-dev-node-3 | 5007 | 51237 | ./devnet/node3/data |

All 3 nodes are configured as validators with pre-generated seeds.

## RPC API Usage

### Get Server Info

```bash
curl -X POST http://localhost:5005/ \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"server_info","id":1}' | jq
```

### Get Current Ledger

```bash
curl -X POST http://localhost:5005/ \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"ledger_current","id":1}' | jq
```

### Generate a New Wallet

```bash
curl -X POST http://localhost:5005/ \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"wallet_propose","id":1}' | jq
```

### Create a Validator Key

```bash
curl -X POST http://localhost:5005/ \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"validation_create","id":1}' | jq
```

### Check Peer Connections

```bash
curl -X POST http://localhost:5005/ \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"peers","id":1}' | jq
```

### Submit a Transaction

```bash
curl -X POST http://localhost:5005/ \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"submit","params":[{"tx_blob":"<hex>"}],"id":1}' | jq
```

## Management Commands

### View Logs

```bash
# All nodes
./devnet/devnet-up.sh logs

# Specific node
./devnet/devnet-up.sh logs call-dev-node-1

# Follow logs with tail
./devnet/devnet-up.sh logs 2>&1 | tail -f
```

### Stop the Testnet

```bash
./devnet/devnet-up.sh stop
```

### Restart the Testnet

```bash
./devnet/devnet-up.sh restart
```

### Clean Up All Data

```bash
# Warning: This removes all blockchain data
./devnet/devnet-up.sh clean
```

## Troubleshooting

### Port Already in Use

If ports 5005-5007 or 51235-51237 are in use:

1. Find and kill the process:
   ```bash
   lsof -i :5005
   kill -9 <PID>
   ```

2. Or modify `devnet/docker-compose.yml` to use different ports.

### Nodes Not Connecting

Check if nodes are on the same Docker network:

```bash
docker network inspect call-devnet
```

View connection logs:

```bash
./devnet/devnet-up.sh logs call-dev-node-1
```

### Container Fails to Start

Check Docker daemon:

```bash
docker ps
docker info
```

Rebuild the image:

```bash
./scripts/docker-build.sh --no-cache
```

### Reset a Single Node

```bash
cd devnet
docker-compose rm -f call-dev-node-1
docker-compose up -d call-dev-node-1
```

## Advanced: Custom Validator Seeds

The testnet uses pre-configured validator seeds for reproducibility. To generate new seeds:

```bash
# Generate new seeds and update configs
./scripts/generate-devnet-seeds.sh
```

Or manually generate a seed:

```bash
docker run --rm callchain/call-core:latest generate-seed
```

Then update the `validation_seed` in each node's `config.toml`.

## Stopping and Cleaning

### Graceful Shutdown

```bash
./devnet/devnet-up.sh stop
```

### Complete Cleanup

```bash
# Removes containers, networks, volumes, and data directories
./devnet/devnet-up.sh clean
```

### Manual Cleanup

```bash
cd devnet
docker-compose down -v
rm -rf node*/data
```

## Testing Consensus

The 3-node testnet runs RPCA (Ripple Protocol Consensus Algorithm). You can test consensus by:

1. Start the testnet: `./devnet/devnet-up.sh start`
2. Wait for nodes to connect (check with `./devnet/devnet-up.sh test`)
3. Submit transactions via any node's RPC endpoint
4. Use admin command to force ledger close:

```bash
curl -X POST http://localhost:5005/ \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"ledger_accept","id":1}' | jq
```

## Performance Tuning

For better performance during development:

1. Increase Docker resource limits in Docker Desktop
2. Use SSD storage for data directories
3. Reduce log verbosity in `config.toml`:
   ```toml
   log_level = "info"  # Instead of "debug"
   ```

## Security Notice

**This testnet is for development only!**

- Pre-configured validator seeds are public
- No encryption on internal network
- Admin RPC methods are enabled
- Do not expose ports to public networks

For production deployments, use proper key management and network security.
