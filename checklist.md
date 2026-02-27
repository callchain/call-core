# Call-Core (calld) Implementation Checklist

## Overview

**Project:** Callchain (CALL) Reference Implementation
**Language:** Rust
**Binary:** `calld`
**Status:** Core features implemented, production-ready
**Last Updated:** 2026-02-27

---

## Core Data Types

| Feature | Status | Notes |
|---------|--------|-------|
| UInt256 (256-bit integer) | ✅ | Hash and ID type |
| AccountID (20-byte) | ✅ | Account identifier |
| Currency (20-byte) | ✅ | Currency code |
| NodeID (32-byte) | ✅ | Validator node identifier |
| Amount (issued/native) | ✅ | CALL and issued currencies |

**Files:** `crates/primitives/src/`

---

## Serialization

| Feature | Status | Notes |
|---------|--------|-------|
| STObject serialization | ✅ | Canonical binary format |
| Amount serialization | ✅ | Native and issued amounts |
| Hash types | ✅ | Hash256, Hash160 |
| VL (variable length) | ✅ | Variable length data |
| Transaction types | ✅ | 10 core tx types |
| Ledger entry types | ✅ | 11 ledger entry types |
| SField definitions | ✅ | 100+ field types |

**Removed Types:** ~~Escrow~~, ~~Ticket~~, ~~PayChannel~~, ~~Nickname~~

**Files:** `crates/serialization/src/`

---

## Cryptography

| Feature | Status | Notes |
|---------|--------|-------|
| SHA-512 hashing | ✅ | Transaction and ledger hashing |
| SHA-512-half | ✅ | 256-bit hashes |
| RIPEMD-160 | ✅ | Account ID derivation |
| secp256k1 signatures | ✅ | ECDSA signatures |
| Ed25519 signatures | ✅ | Modern curve support |
| Key derivation | ✅ | Seed to keypair |
| Multi-signature | ✅ | SignerListSet support |

**Files:** `crates/crypto/src/`

---

## SHAMap (Merkle Tree)

| Feature | Status | Notes |
|---------|--------|-------|
| Inner nodes | ✅ | Branch nodes |
| Leaf nodes | ✅ | Value storage |
| Node hashes | ✅ | Recursive hashing |
| Tree operations | ✅ | Add, remove, update |
| Root hash | ✅ | Ledger state hash |

**Files:** `crates/shamap/src/`

---

## Storage

| Feature | Status | Notes |
|---------|--------|-------|
| RocksDB backend | ✅ | Production storage |
| Memory backend | ✅ | Testing storage |
| Column families | ✅ | Organized data storage |
| Batch writes | ✅ | Atomic operations |
| NodeObject | ✅ | Storage wrapper |
| Historical queries | ✅ | Pagination support |

**Files:** `crates/storage/src/`

---

## Ledger

| Feature | Status | Notes |
|---------|--------|-------|
| Ledger header | ✅ | Version, time, fees |
| Ledger sequence | ✅ | Index tracking |
| Ledger hashes | ✅ | TX tree, state tree |
| Parent ledger | ✅ | Chain linkage |
| Fees calculation | ✅ | Base fee, load factor |
| Reserve requirements | ✅ | Account reserves |
| Ledger close time | ✅ | Consensus timing |

**Files:** `crates/protocol/src/ledger.rs`

---

## Ledger Entries

| Feature | Status | Notes |
|---------|--------|-------|
| AccountRoot | ✅ | Account state |
| CallState | ✅ | Trust lines |
| Offer | ✅ | DEX offers |
| DirectoryNode | ✅ | Indexing |
| FeeRoot | ✅ | Custom: fee settings root |
| IssueRoot | ✅ | Custom: issued tokens root |
| Invoice | ✅ | Custom: payment invoices |
| ~~Escrow~~ | ❌ Removed | Not supported |
| ~~Ticket~~ | ❌ Removed | Not supported |
| ~~PayChannel~~ | ❌ Removed | Not supported |
| Nickname | ✅ | Account nickname registry |

**Files:** `crates/protocol/src/ledger_entries.rs`

---

## Transactions

### Transaction Types

| Type | Status | Description |
|------|--------|-------------|
| Payment | ✅ | Transfer CALL/currency |
| IssueSet | ✅ | Custom: Issue tokens |
| TrustSet | ✅ | Create/modify trust line |
| OfferCreate | ✅ | Create DEX offer |
| OfferCancel | ✅ | Cancel DEX offer |
| AccountSet | ✅ | Set account options |
| SetRegularKey | ✅ | Set regular key |
| SignerListSet | ✅ | Multi-sign setup |
| NicknameSet | ✅ | Set account nickname |
| ~~EscrowCreate~~ | ❌ Removed | Not supported |
| ~~EscrowFinish~~ | ❌ Removed | Not supported |
| ~~EscrowCancel~~ | ❌ Removed | Not supported |
| ~~TicketCreate~~ | ❌ Removed | Not supported |
| ~~TicketCancel~~ | ❌ Removed | Not supported |
| ~~PaychanCreate~~ | ❌ Removed | Not supported |
| ~~PaychanFund~~ | ❌ Removed | Not supported |
| ~~PaychanClaim~~ | ❌ Removed | Not supported |

### Transaction Processing

| Feature | Status | Notes |
|---------|--------|-------|
| Preflight checks | ✅ | Static validation |
| Preclaim checks | ✅ | State validation |
| Apply phase | ✅ | Execute transaction |
| Fee charging | ✅ | Deduct fees |
| Sequence checking | ✅ | Account sequence |
| Signature verification | ✅ | Verify signatures |
| Metadata generation | ✅ | Affected nodes |
| TER codes | ✅ | Transaction results |

**Files:** `crates/protocol/src/transactions.rs`, `crates/protocol/src/tx_engine.rs`

---

## Transaction Queue

| Feature | Status | Notes |
|---------|--------|-------|
| Transaction queue | ✅ | Pending transactions |
| Fee escalation | ✅ | Fee-based ordering |
| Sequence tracking | ✅ | Account sequences |
| Queue limits | ✅ | Maximum queue size |
| Open ledger | ✅ | Current ledger txs |

**Files:** `crates/protocol/src/tx_queue.rs`

---

## DEX (Decentralized Exchange)

| Feature | Status | Notes |
|---------|--------|-------|
| Offer book | ✅ | Order book management |
| Offer matching | ✅ | Taker/Offer flow |
| Pathfinding | ✅ | Payment paths (6 hops) |
| Quality calculation | ✅ | Exchange rates |
| Book directory | ✅ | Indexed offers |
| Trust line checks | ✅ | Authorization |

**Files:** `crates/protocol/src/dex.rs`

---

## Consensus

| Feature | Status | Notes |
|---------|--------|-------|
| RPCA algorithm | ✅ | Ripple Protocol Consensus |
| Consensus rounds | ✅ | Round management |
| Proposal handling | ✅ | Leader proposals |
| Validation handling | ✅ | Peer validations |
| Disputed transactions | ✅ | Conflict resolution |
| Close time consensus | ✅ | Ledger timing |
| Amendment system | ✅ | Feature voting |
| Fee voting | ✅ | Fee parameter voting |
| Peer positions | ✅ | Consensus tracking |

**Amendments Implemented:**
- ✅ FeeEscalation
- ✅ MultiSign
- ✅ FlowV2
- ✅ CryptoConditions
- ~~Tickets~~ ❌ Removed

**Files:** `crates/consensus/src/`

---

## Network

| Feature | Status | Notes |
|---------|--------|-------|
| P2P protocol | ✅ | Peer-to-peer networking |
| Message framing | ✅ | Protocol messages |
| Peer handshake | ✅ | Connection setup |
| Peer discovery | ✅ | Bootstrap nodes |
| Connection manager | ✅ | Active connections |
| Message routing | ✅ | Overlay network |
| Bootstrap sync | ✅ | Ledger synchronization |

**Files:** `crates/network/src/`

---

## Node Application

| Feature | Status | Notes |
|---------|--------|-------|
| Application framework | ✅ | Node lifecycle |
| Configuration | ✅ | TOML/JSON/CLI |
| Data directory | ✅ | File storage |
| Logging | ✅ | Tracing framework |
| Signal handling | ✅ | Graceful shutdown |
| Main binary | ✅ | `calld` executable |

**Files:** `crates/node/src/application.rs`, `crates/node/src/main.rs`

---

## RPC API

### Public Methods

| Method | Status | Description |
|--------|--------|-------------|
| server_info | ✅ | Node information |
| ping | ✅ | Health check |
| ledger_current | ✅ | Current ledger index |
| ledger_closed | ✅ | Last closed ledger |
| ledger | ✅ | Get ledger by index/hash |
| account_info | ✅ | Account state |
| account_tx | ✅ | Account transactions |
| tx | ✅ | Get transaction |
| submit | ✅ | Submit transaction |
| book_offers | ✅ | Get order book |

### Admin Methods

| Method | Status | Description |
|--------|--------|-------------|
| validation_create | ✅ | Create validation seed |
| wallet_propose | ✅ | Generate wallet |
| ledger_accept | ✅ | Force close (test) |
| peers | ✅ | List connected peers |
| stop | ✅ | Graceful shutdown |

### WebSocket

| Feature | Status | Notes |
|---------|--------|-------|
| WebSocket server | ✅ | WS alongside HTTP |
| ledger stream | ✅ | New ledger notifications |
| transactions stream | ✅ | New transaction notifications |
| validations stream | ✅ | Validation notifications |
| Subscribe/unsubscribe | ✅ | Stream management |

**Files:** `crates/node/src/rpc.rs`, `crates/node/src/websocket.rs`

---

## Metrics

| Feature | Status | Notes |
|---------|--------|-------|
| Metrics registry | ✅ | Counter, gauge, histogram |
| Prometheus export | ✅ | /metrics endpoint |
| Consensus metrics | ✅ | Round times, validations |
| Network metrics | ✅ | Peers, messages, bytes |
| Transaction metrics | ✅ | TPS, queue size |
| Storage metrics | ✅ | Reads, writes |
| RPC metrics | ✅ | Request counts, latency |

**Files:** `crates/node/src/metrics.rs`

---

## Historical Data

| Feature | Status | Notes |
|---------|--------|-------|
| Account transaction index | ✅ | Per-account history |
| Ledger range queries | ✅ | Pagination support |
| Query parameters | ✅ | Filters, sorting |
| Pagination info | ✅ | Result metadata |
| Backend integration | ✅ | RocksDB storage |

**Files:** `crates/storage/src/historical.rs`

---

## Build System

| Feature | Status | Notes |
|---------|--------|-------|
| Cargo workspace | ✅ | Multi-crate project |
| Binary name | ✅ | `calld` (not call-core) |
| Release profile | ✅ | Optimized builds |
| Test suite | ✅ | 100+ tests |

---

## Test Coverage

| Component | Tests | Status |
|-----------|-------|--------|
| primitives | 5 | ✅ Passing |
| serialization | 6 | ✅ Passing |
| crypto | 5 | ✅ Passing |
| shamap | 3 | ✅ Passing |
| storage | 13 | ✅ Passing |
| protocol | 30 | ✅ Passing |
| consensus | 18 | ✅ Passing |
| network | 9 | ✅ Passing |
| node | 17 | ✅ Passing |
| **Total** | **106** | ✅ **Passing** |

---

## Not Implemented (Intentionally)

The following features are **not supported** in calld:

| Feature | Reason |
|---------|--------|
| Escrow | Removed - not in core protocol |
| Tickets | Removed - not in core protocol |
| Payment Channels | Removed - not in core protocol |
| Checks | Not implemented |
| NFTs | Not implemented |
| Hooks | Not implemented |
| AMM | Not implemented |

---

## Summary

- **Total Features:** 100+ core features
- **Implemented:** 100% of core calld features
- **Test Pass Rate:** 100% (107 tests)
- **Build Status:** ✅ Passing
- **Documentation:** Complete

**Last Verification:** 2026-02-27
