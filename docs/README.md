# Call-Core Documentation

Welcome to the Call-Core documentation. Call-Core is a high-performance blockchain node implementation for the Callchain network, written in Rust.

## Overview

Call-Core implements the Callchain protocol, a distributed ledger technology designed for fast, secure, and scalable digital asset transfers. It provides:

- **Full Node Implementation**: Complete validation, consensus, and ledger management
- **Enterprise-Grade Consensus**: Byzantine Fault Tolerant consensus with weighted validators
- **DEX Support**: Built-in decentralized exchange with order books and path finding
- **Multi-Signature**: Advanced multi-signature account controls
- **WebSocket & RPC APIs**: Real-time subscriptions and comprehensive query interfaces

## Documentation Structure

| Directory | Contents |
|-----------|----------|
| [`architecture/`](architecture/) | System architecture and design documents |
| [`consensus/`](consensus/) | Consensus algorithm and validator documentation |
| [`api/`](api/) | RPC and WebSocket API reference |
| [`transactions/`](transactions/types.md) | Transaction types and processing |
| [`ledger/`](ledger/types.md) | Ledger entry types and state management |
| [`guides/`](guides/) | Developer and operator guides |

## Quick Reference

### API Documentation
- [RPC API](api/rpc.md) - HTTP API for queries and submissions
- [WebSocket API](api/websocket.md) - Real-time subscriptions

### Core Concepts
- [Architecture Overview](architecture/overview.md) - System design
- [Consensus Algorithm](consensus/algorithm.md) - BFT consensus
- [Transaction Types](transactions/types.md) - All transaction types
- [Ledger Types](ledger/types.md) - Ledger entry types

### Operator Guides
- [Configuration](guides/configuration.md) - Node configuration
- [Validator Setup](guides/validator-setup.md) - Run a validator
- [Local Testnet](guides/local-testnet.md) - Deploy a local testnet
- [CLI Reference](guides/cli-reference.md) - Command-line tools

## Quick Start

### Building from Source

```bash
# Clone the repository
git clone https://github.com/callchain/call-core
cd call-core

# Build in release mode
cargo build --release

# Run tests
cargo test --all
```

### Running a Node

```bash
# Start the node with default configuration
./target/release/calld

# Start with custom config
./target/release/calld --config /path/to/config.toml
```

### Creating a Wallet

```bash
# Generate a new wallet
./target/release/calld wallet-generate

# Output includes:
# - Seed (starts with 's')
# - Address (starts with 'c')
# - Public/Private keys
```

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                         Call-Core Node                       │
├─────────────────────────────────────────────────────────────┤
│  RPC API    │  WebSocket    │  CLI Interface                │
│  (HTTP)     │  (Real-time)  │  (Interactive)                │
├─────────────┴───────────────┴───────────────────────────────┤
│                    Transaction Processing                    │
│  ┌──────────┐  ┌──────────┐  ┌──────────────────────────┐  │
│  │ Preflight│→ │ Preclaim │→ │ Apply (Ledger Changes)   │  │
│  │ (Static) │  │ (State)  │  │ (Execute)                │  │
│  └──────────┘  └──────────┘  └──────────────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                      Consensus Engine                        │
│  ┌──────────┐  ┌──────────┐  ┌──────────────────────────┐  │
│  │ Proposal │→ │ Validate │→ │ Close Ledger             │  │
│  │ (Open)   │  │ (Verify) │  │ (80% Agreement)          │  │
│  └──────────┘  └──────────┘  └──────────────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                      Ledger State                            │
│  ┌──────────┐  ┌──────────┐  ┌──────────────────────────┐  │
│  │ SHAMap   │  │ Account  │  │ Offer Book               │  │
│  │ (Merkle) │  │ Roots    │  │ (DEX)                    │  │
│  └──────────┘  └──────────┘  └──────────────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                    Network Layer                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────────────────────┐  │
│  │ Peer     │  │ Overlay  │  │ Message Propagation      │  │
│  │ Discovery│  │ Network  │  │ (Tx/Ledger/Validations)  │  │
│  └──────────┘  └──────────┘  └──────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Key Features

### Core Blockchain
- **Native Asset**: CALL token with drops precision (1 CALL = 1,000,000 drops)
- **Fast Settlement**: 5-second ledger close time
- **Low Fees**: Dynamic fee scaling based on network load
- **Account Model**: Each account has a reserve requirement

### Decentralized Exchange (DEX)
- **Order Books**: Native limit order support
- **Path Finding**: Multi-hop payment routing
- **Trust Lines**: Issued asset support with quality settings
- **Offer Crossing**: Automatic order matching

### Security
- **Multi-Signature**: Up to 32 signers with configurable quorum
- **Deposit Authorization**: Require pre-approval for incoming payments
- **Regular Keys**: Separate signing keys from account control
- **Master Key Disable**: Enhanced account security

### Advanced Features
- **Nickname System**: Human-readable account names
- **Native Asset Issuance**: Issue custom tokens on Callchain
- **Invoice System**: NFT and invoice representation
- **Deposit Preauthorization**: Whitelist senders for deposit auth

## Configuration

See [`guides/configuration.md`](guides/configuration.md) for detailed configuration options.

Basic configuration example:

```toml
[network]
listen_address = "0.0.0.0:5333"
bootstrap_peers = ["seed1.callchain.io:5333", "seed2.callchain.io:5333"]

[node]
validation_seed = "sxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
validation_public_key = "nxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"

[consensus]
validator_list_sites = ["https://vl.callchain.io"]
validation_quorum = 80

[rpc]
enabled = true
address = "127.0.0.1:5005"
admin_only = false

[websocket]
enabled = true
address = "127.0.0.1:6006"
```

## API Documentation

### RPC API
Full RPC documentation: [`api/rpc.md`](api/rpc.md)

Common methods:
- `account_info` - Get account details
- `submit` - Submit a transaction
- `ledger` - Get ledger information
- `tx` - Get transaction details
- `book_offers` - Get order book

### WebSocket API
Full WebSocket documentation: [`api/websocket.md`](api/websocket.md)

Subscribe to real-time updates:
- `ledger` - New ledger closes
- `transactions` - Confirmed transactions
- `transactions_proposed` - Proposed transactions
- `validations` - Validator validations

## Contributing

Please read our [Contributing Guide](guides/contributing.md) for information on:
- Code style and formatting
- Testing requirements
- Pull request process
- Development workflow

## License

Call-Core is licensed under the MIT License. See [LICENSE](../LICENSE) for details.

## Resources

- **Website**: https://callchain.io
- **Documentation**: https://docs.callchain.io
- **GitHub**: https://github.com/callchain/call-core
- **Discord**: https://discord.gg/callchain

## Support

For technical support:
- Open an issue on GitHub
- Join our Discord community
- Email: dev@callchain.io
