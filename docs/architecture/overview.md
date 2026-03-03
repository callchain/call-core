# Call-Core Architecture Overview

This document provides a high-level overview of the Call-Core architecture, explaining how the various components work together to form a complete blockchain node.

## System Architecture

Call-Core follows a modular, layered architecture:

```
┌──────────────────────────────────────────────────────────────┐
│                    Application Layer                          │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐     │
│  │ RPC API  │  │ WebSocket│  │   CLI    │  │  Admin   │     │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘     │
├──────────────────────────────────────────────────────────────┤
│                   Transaction Layer                           │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              Transaction Engine                        │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────────────┐   │   │
│  │  │ Preflight│→ │ Preclaim │→ │      Apply       │   │   │
│  │  │ (Static) │  │ (State)  │  │ (Execute)        │   │   │
│  │  └──────────┘  └──────────┘  └──────────────────┘   │   │
│  └──────────────────────────────────────────────────────┘   │
├──────────────────────────────────────────────────────────────┤
│                    Consensus Layer                            │
│  ┌──────────┐  ┌──────────┐  ┌──────────────────────────┐   │
│  │ Proposal │  │ Validate │  │  Ledger Close (80%)      │   │
│  └──────────┘  └──────────┘  └──────────────────────────┘   │
├──────────────────────────────────────────────────────────────┤
│                     Ledger Layer                              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐     │
│  │  SHAMap  │  │ Accounts │  │  Offers  │  │  Trust   │     │
│  │ (Merkle) │  │          │  │  (DEX)   │  │  Lines   │     │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘     │
├──────────────────────────────────────────────────────────────┤
│                    Network Layer                              │
│  ┌──────────┐  ┌──────────┐  ┌──────────────────────────┐   │
│  │   Peer   │  │  Overlay │  │   Message Processing     │   │
│  │  Manager │  │  Network │  │   (Tx/Ledger/Val)        │   │
│  └──────────┘  └──────────┘  └──────────────────────────┘   │
├──────────────────────────────────────────────────────────────┤
│                   Storage Layer                               │
│  ┌──────────┐  ┌──────────┐  ┌──────────────────────────┐   │
│  │ Ledger   │  │  State   │  │   Transaction History    │   │
│  │ Database │  │   Tree   │  │   (Historical Data)      │   │
│  └──────────┘  └──────────┘  └──────────────────────────┘   │
└──────────────────────────────────────────────────────────────┘
```

## Crate Organization

Call-Core is organized into the following crates:

### Core Crates

| Crate | Description | Key Components |
|-------|-------------|----------------|
| `protocol` | Core protocol implementation | Ledger, transactions, consensus interface |
| `consensus` | Consensus algorithm | BFT consensus, validator tracking |
| `network` | P2P networking | Peer management, message propagation |
| `node` | Full node implementation | RPC, WebSocket, CLI, application logic |

### Support Crates

| Crate | Description | Key Components |
|-------|-------------|----------------|
| `primitives` | Core data types | AccountID, UInt256, Currency, Amount |
| `serialization` | STObject serialization | Binary format, field encoding |
| `crypto` | Cryptographic functions | Keys, signatures, hashing, mnemonics |
| `shamap` | Merkle tree implementation | SHAMap data structure |

## Component Interactions

### Transaction Flow

```
1. Submission
   Client → RPC/WebSocket → Transaction Queue

2. Validation (Three-Phase)
   Preflight: Static validation (format, signature syntax)
   Preclaim:  State validation (balance, sequence, auth)
   Apply:     Execute and update ledger state

3. Propagation
   Valid Transaction → Network → Peer mempool

4. Consensus
   Proposed → Validated → Ledger Close (80% agreement)

5. Finality
   Ledger Accepted → State Updated → Notifications Sent
```

### Ledger Close Process

```
┌─────────┐    ┌──────────┐    ┌──────────┐    ┌─────────┐
│  Open   │───→│ Establish│───→│ Processing│───→│ Accepted│
│  Phase  │    │  Phase   │    │  Phase    │    │  Phase  │
└─────────┘    └──────────┘    └──────────┘    └─────────┘
     │               │               │               │
     ▼               ▼               ▼               ▼
 Collect Tx    Build Proposal   Apply Tx Set    Ledger
 From Queue    Share w/ Peers   Update State    Closed
```

### State Management

The ledger state is maintained using a **SHAMap** (Sparse Hash Array Mapped Prefix Trie):

```
┌─────────────────────────────────────┐
│           Ledger Header             │
│  - Sequence Number                  │
│  - Previous Hash                    │
│  - Transaction Hash (Merkle Root)   │
│  - State Hash (SHAMap Root)         │
│  - Close Time                       │
└─────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────┐
│           SHAMap Root               │
│       (256-bit Hash)                │
└─────────────────────────────────────┘
                  │
      ┌───────────┼───────────┐
      ▼           ▼           ▼
┌─────────┐ ┌─────────┐ ┌─────────┐
│ Branch  │ │ Branch  │ │ Branch  │
│  Node   │ │  Node   │ │  Node   │
└─────────┘ └─────────┘ └─────────┘
      │           │           │
      ▼           ▼           ▼
┌─────────┐ ┌─────────┐ ┌─────────┐
│  Leaf   │ │  Leaf   │ │  Leaf   │
│Account  │ │  Offer  │ │TrustLine│
│  Root   │ │         │ │         │
└─────────┘ └─────────┘ └─────────┘
```

## Data Flow

### Incoming Transaction

```rust
// 1. Receive via RPC/WebSocket
let tx_blob: Vec<u8> = receive_from_client();

// 2. Deserialize
let tx: Transaction = deserialize(&tx_blob)?;

// 3. Queue for validation
queue.insert(tx)?;

// 4. Three-phase validation
let result = engine.process(&mut ctx, &tx)?;

// 5. If valid, propagate to peers
if result.ter == TER::tesSUCCESS {
    network.broadcast_transaction(tx);
}

// 6. Include in next ledger (via consensus)
consensus.propose_transaction(tx);
```

### Ledger Synchronization

```rust
// 1. Discover peers
let peers = network.discover_peers();

// 2. Find ledger chain
let ledger_chain = find_common_ledger(peers)?;

// 3. Download missing ledgers
for ledger_hash in missing_ledgers {
    let ledger = network.fetch_ledger(ledger_hash).await?;
    storage.save_ledger(&ledger)?;
}

// 4. Verify state
verify_ledger_chain()?;
```

## Security Architecture

### Transaction Security

1. **Cryptographic Signatures**: All transactions require Ed25519 or Secp256k1 signatures
2. **Replay Protection**: Sequence numbers prevent transaction replay
3. **Authorization**: Multi-signature and deposit authorization enforce access control

### Network Security

1. **Proof of Work**: Overlay network uses PoW for spam prevention
2. **Peer Authentication**: Validators authenticate via public keys
3. **Rate Limiting**: Transaction and connection rate limiting

### Consensus Security

1. **Byzantine Fault Tolerance**: Tolerates up to 20% faulty validators
2. **Weighted Consensus**: Validators have configurable weights
3. **Conflict Detection**: Detects and handles conflicting proposals

## Performance Considerations

### Throughput

- **Target TPS**: 1,500+ transactions per second
- **Ledger Close Time**: 5 seconds (configurable)
- **Max Transactions/Ledger**: 5,000

### Optimizations

1. **SHAMap**: O(log n) state updates with Merkle proofs
2. **Transaction Queue**: Priority queue with fee escalation
3. **Parallel Validation**: Independent transaction pre-validation
4. **Efficient Serialization**: Compact binary format (STObject)

### Resource Management

1. **Memory**: Bounded transaction queue and cache sizes
2. **Disk**: Rotating ledger history with configurable retention
3. **Network**: Rate-limited message propagation

## Deployment Patterns

### Validator Node

```
┌─────────────────────────────────┐
│         Validator Node          │
│  ┌───────────────────────────┐  │
│  │   Full Ledger History     │  │
│  │   Validation Seed         │  │
│  │   UNL Management          │  │
│  │   Proposal Generation     │  │
│  └───────────────────────────┘  │
└─────────────────────────────────┘
```

### Stock Node

```
┌─────────────────────────────────┐
│          Stock Node             │
│  ┌───────────────────────────┐  │
│  │   Recent Ledger History   │  │
│  │   Transaction Relay       │  │
│  │   API Serving             │  │
│  └───────────────────────────┘  │
└─────────────────────────────────┘
```

### API Node

```
┌─────────────────────────────────┐
│           API Node              │
│  ┌───────────────────────────┐  │
│  │   Load Balanced RPC       │  │
│  │   WebSocket Clusters      │  │
│  │   Cache Layer             │  │
│  └───────────────────────────┘  │
└─────────────────────────────────┘
```

## See Also

- [Consensus Algorithm](consensus.md) - Detailed consensus documentation
- [Transaction Processing](../transactions/overview.md) - How transactions work
- [RPC API Reference](../api/rpc.md) - Complete API documentation
- [Configuration Guide](../guides/configuration.md) - Node configuration
