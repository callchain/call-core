# Call Core

Callchain (CALL) reference implementation in Rust - a high-performance blockchain node compatible with the Callchain protocol.

## Overview

Call Core is a Rust implementation of the Callchain blockchain protocol, featuring:

- **High Performance**: Built with Rust for safety and speed
- **Full Node**: Complete validation and consensus participation
- **RPC API**: JSON-RPC 2.0 compatible API
- **P2P Networking**: Peer-to-peer network protocol
- **Storage**: RocksDB-based persistent storage
- **DEX Support**: Built-in decentralized exchange
- **Custom Transactions**: IssueSet for token issuance

## Architecture

The project is organized into modular crates:

```
crates/
├── primitives/    # Core types (UInt256, AccountID, Currency, NodeID)
├── serialization/ # Protocol serialization (STObject, Amount, Serializer)
├── crypto/        # Cryptographic primitives (SHA-512, secp256k1, ed25519)
├── shamap/        # Merkle Patricia Tree implementation
├── storage/       # NodeStore database layer
├── protocol/      # Ledger, transactions, DEX, consensus types
├── consensus/     # RPCA consensus algorithm
├── network/       # P2P networking overlay
└── node/          # Application and RPC server
```

## Building

### Requirements

- Rust 1.75+ (install via [rustup](https://rustup.rs/))
- RocksDB (installed automatically via crate)

### Build

```bash
# Clone the repository
git clone https://github.com/callchain/call-core.git
cd call-core

# Build in release mode
cargo build --release

# The binary will be at:
# target/release/calld
```

## Running

### Start a Node

```bash
# Start with default configuration
./target/release/calld

# Start with custom data directory
./target/release/calld --data-dir /path/to/data

# Start as validator
./target/release/calld --validation-seed sn3nxiW7v8KXzPzAqzyHXbSSKNuN

# Start with bootstrap peers
./target/release/calld --peers 127.0.0.1:51235,192.168.1.100:51235
```

### CLI Commands

```bash
# Generate a new validation seed
./target/release/calld generate-seed

# Show ledger information
./target/release/calld ledger-info current

# Show account information
./target/release/calld account-info rN7n7otQDd6FczFgLdlqtyMVrn3HMfHgFj

# Submit a transaction (hex blob)
./target/release/calld submit 12000022000000002400000001...
```

### Options

- `--config <FILE>` - Configuration file path
- `--data-dir <DIR>` - Data directory for storage
- `--rpc-port <PORT>` - RPC server port (default: 5005)
- `--peer-port <PORT>` - P2P networking port (default: 51235)
- `--peers <PEERS>` - Bootstrap peers (comma-separated)
- `--validation-seed <SEED>` - Validator seed
- `--log-level <LEVEL>` - Logging level (default: info)

## Testing

```bash
# Run all tests
cargo test --all

# Run specific crate tests
cargo test -p primitives
cargo test -p serialization
cargo test -p crypto
cargo test -p protocol
cargo test -p consensus
cargo test -p network
cargo test -p storage

# Run integration tests
cargo test --test integration_tests
cargo test --test genesis_ledger

# Run with output
cargo test --all -- --nocapture

# Run specific test
cargo test test_full_ledger_close -- --nocapture

# Run with logging
RUST_LOG=debug cargo test test_name -- --nocapture

# Run single threaded
cargo test --all -- --test-threads=1
```

## RPC Methods

The node exposes a JSON-RPC 2.0 API:

- `server_info` - Node status and information
- `ping` - Health check
- `ledger_current` - Current ledger index
- `account_info` - Account information
- `submit` - Submit a transaction
- `tx` - Get transaction information

Example:
```bash
curl -X POST http://localhost:5005 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"server_info","id":1}'
```

## Configuration

Configuration can be provided via:
1. Command-line arguments
2. Configuration file (TOML/JSON)
3. Environment variables

Example configuration file:
```toml
node_name = "my-call-node"
data_dir = "/var/lib/calld"
listen_address = "0.0.0.0:51235"
validation_seed = "sn3nxiW7v8KXzPzAqzyHXbSSKNuN"

[rpc]
enabled = true
bind_address = "127.0.0.1"
port = 5005

[network]
max_peers = 50
target_peers = 10
peers = ["127.0.0.1:51235", "192.168.1.100:51235"]
```

## Transaction Types

Call Core supports the following transaction types:

- `Payment` (0) - Transfer CALL or issued currencies
- `IssueSet` (1) - Issue new tokens (Callchain-specific)
- `TrustSet` (2) - Create or modify trust lines
- `OfferCreate` (3) - Create DEX offer
- `OfferCancel` (4) - Cancel DEX offer
- `AccountSet` (5) - Set account options
- `SetRegularKey` (6) - Set regular key
- `SignerListSet` (7) - Set multi-signing list

## Development Status

| Phase | Status | Description |
|-------|--------|-------------|
| Foundation | ✅ Complete | Workspace and crate structure |
| Serialization | ✅ Complete | Protocol serialization layer |
| Cryptography | ✅ Complete | SHA-512, secp256k1, ed25519 |
| SHAMap | ✅ Complete | Merkle Patricia Tree |
| Storage | ✅ Complete | RocksDB backend |
| Ledger | ✅ Complete | Ledger management |
| Transactions | ✅ Complete | Transaction processing |
| DEX | ✅ Complete | Pathfinding and offers |
| Consensus | ✅ Complete | RPCA algorithm |
| Networking | ✅ Complete | P2P overlay |
| RPC | ✅ Complete | JSON-RPC server |
| Application | ✅ Complete | Node framework |
| Testing | ✅ Complete | Integration tests |
| Tools | ✅ Complete | CLI interface |
| Documentation | 🔄 In Progress | Documentation |

## Contributing

Contributions are welcome! Please read our [Contributing Guide](CONTRIBUTING.md) for details.

## License

Call Core is licensed under the MIT License. See [LICENSE](LICENSE) for details.

## Acknowledgments

- Callchain protocol design inspired by Ripple
- Built with Rust and the tokio ecosystem
- Uses RocksDB for storage

## Resources

- [Callchain Website](https://callchain.org)
- [Documentation](https://docs.callchain.org)
- [GitHub Repository](https://github.com/callchain/call-core)
