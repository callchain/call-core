# Call-core Dev Testnet

A 3-node validator testnet for development and testing of the Call-core blockchain.

## Quick Start

### 1. Build the Docker Image

```bash
# From the call-core root directory
./scripts/docker-build.sh
```

### 2. Start the Dev Testnet

```bash
# From the devnet directory
./devnet-up.sh start
```

### 3. Check Status

```bash
./devnet-up.sh status
./devnet-up.sh test
```

### 4. View Logs

```bash
# All nodes
./devnet-up.sh logs

# Specific node
./devnet-up.sh logs call-dev-node-1
```

### 5. Stop the Devnet

```bash
./devnet-up.sh stop
```

## Architecture

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  call-dev-node-1│◄───►│  call-dev-node-2│◄───►│  call-dev-node-3│
│                 │     │                 │     │                 │
│ RPC: 5005       │     │ RPC: 5006       │     │ RPC: 5007       │
│ P2P: 51235      │     │ P2P: 51236      │     │ P2P: 51237      │
│                 │     │                 │     │                 │
│ Validator: Yes  │     │ Validator: Yes  │     │ Validator: Yes  │
└─────────────────┘     └─────────────────┘     └─────────────────┘
```

## Node Configuration

| Node | RPC Port | P2P Port | Data Directory |
|------|----------|----------|----------------|
| Node 1 | 5005 | 51235 | ./node1/data |
| Node 2 | 5006 | 51236 | ./node2/data |
| Node 3 | 5007 | 51237 | ./node3/data |

## RPC Endpoints

- Node 1: http://localhost:5005
- Node 2: http://localhost:5006
- Node 3: http://localhost:5007

## Example RPC Commands

### Get Server Info

```bash
curl -X POST http://localhost:5005/ \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"server_info","id":1}'
```

### Get Current Ledger

```bash
curl -X POST http://localhost:5005/ \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"ledger_current","id":1}'
```

### Generate a Wallet

```bash
curl -X POST http://localhost:5005/ \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"wallet_propose","id":1}'
```

### Submit a Transaction

```bash
curl -X POST http://localhost:5005/ \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"submit","params":[{"tx_blob":"<hex>"}],"id":1}'
```

## Commands

| Command | Description |
|---------|-------------|
| `./devnet-up.sh start` | Start the devnet |
| `./devnet-up.sh stop` | Stop the devnet |
| `./devnet-up.sh restart` | Restart the devnet |
| `./devnet-up.sh status` | Show node status |
| `./devnet-up.sh logs [node]` | View logs |
| `./devnet-up.sh test` | Test connectivity |
| `./devnet-up.sh clean` | Remove all data |

## Cleanup

To completely remove the devnet and all data:

```bash
./devnet-up.sh clean
```

This will:
- Stop all containers
- Remove Docker volumes
- Delete data directories

## Troubleshooting

### Port Already in Use

If you get port binding errors, either:
1. Stop other processes using those ports
2. Modify the ports in `docker-compose.yml`

### Nodes Not Connecting

Check logs for connection errors:
```bash
./devnet-up.sh logs
```

Ensure all nodes are on the same Docker network:
```bash
docker network inspect call-devnet
```

### Reset a Single Node

```bash
docker-compose rm -f call-dev-node-1
docker-compose up -d call-dev-node-1
```

## Validator Seeds

The devnet uses pre-configured validator seeds for reproducibility:

- Node 1: `sEdTLQ75P2X9VBbNqGihzrNWGtE7d6NHPmgSG8q5d8fM7YjHcXK`
- Node 2: `sEdVhK8P3Y8VBbmNrHjhsOXHuF8e7OIQnRhTF9r6e9gN8ZkIdYL`
- Node 3: `sEdWmL9Q4Z9WCcnOsIkltPYIvG9f8PJRoShUG0s7f0hO9AlJeZM`

**WARNING**: These seeds are for testing only. Never use them for production!

To generate new seeds:

```bash
docker run --rm callchain/call-core:latest generate-seed
```
