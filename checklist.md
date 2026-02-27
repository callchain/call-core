# Call-Core Placeholder Implementation Checklist

This document tracks placeholder code that needs to be fully implemented.

## Critical Placeholders

### 1. SHAMap Inner Node Hashing
- [x] **File**: `crates/shamap/src/lib.rs:69-75`
- **Issue**: Inner and InnerV2 nodes return `UInt256::zero()` for hash
- **Impact**: Breaks ledger state integrity - Merkle tree incomplete
- **Implementation**: Compute inner node hash from child hashes
- **Status**: ✅ Implemented - Added `hash` field to SHAMapInnerNode, updated compute_hash() to store hash, hash() now returns cached value

### 2. Network Message Serialization
- [x] **File**: `crates/network/src/message.rs:57-82`
- **Issue**: `validation()`, `propose()`, `transaction()` methods create empty serializers
- **Impact**: P2P messages serialize to empty payloads
- **Implementation**: Properly serialize Validation, Proposal, Transaction to bytes
- **Status**: ✅ Implemented - All three methods now properly serialize their respective data structures using the serialization crate

### 3. Genesis Ledger Hash
- [x] **File**: `crates/protocol/src/ledger.rs:150`
- **Issue**: Genesis ledger uses `UInt256::zero()` instead of computed hash
- **Impact**: Genesis ledger not properly identifiable
- **Implementation**: Compute genesis hash from ledger contents
- **Status**: ✅ Implemented - Modified `Ledger::genesis()` to call `update_hashes()` which computes proper ledger hash from contents

### 4. RPC Endpoint: show_ledger_info
- [x] **File**: `crates/node/src/main.rs:239-244`
- **Issue**: Prints "not yet implemented"
- **Impact**: Cannot query ledger information via RPC
- **Implementation**: Query ledger from storage and return JSON
- **Status**: ✅ Implemented - CLI command now makes JSON-RPC call to `ledger` or `ledger_current` endpoint

### 5. RPC Endpoint: submit_transaction
- [x] **File**: `crates/node/src/main.rs:246-251`
- **Issue**: Prints "not yet implemented"
- **Impact**: Cannot submit transactions via RPC
- **Implementation**: Deserialize tx blob, validate, submit to consensus
- **Status**: ✅ Implemented - CLI command now makes JSON-RPC call to `submit` endpoint with transaction blob

### 6. RPC Endpoint: show_account_info
- [x] **File**: `crates/node/src/main.rs:253-258`
- **Issue**: Prints "not yet implemented"
- **Impact**: Cannot query account information via RPC
- **Implementation**: Query account root from ledger
- **Status**: ✅ Implemented - CLI command now makes JSON-RPC call to `account_info` endpoint

### 7. Application Bootstrap
- [x] **File**: `crates/node/src/application.rs:109-114`
- **Issue**: TODO comments for bootstrap, genesis, consensus startup
- **Impact**: Node cannot start properly
- **Implementation**: Implement full startup sequence
- **Status**: ✅ Implemented - Added `initialize_peers()`, `initialize_ledger()`, and `initialize_consensus()` methods

### 8. Transaction Processing Pipeline
- [x] **File**: `crates/node/src/application.rs:257-265`
- **Issue**: TODO comments for transaction handling
- **Impact**: Transactions not processed
- **Implementation**: Deserialize, validate, add to ledger, broadcast
- **Status**: ✅ Implemented - Added `deserialize_transaction()`, `validate_transaction()`, `add_to_open_ledger()`, and `broadcast_transaction()` methods

### 9. SHAMap Inner Node compute_hash
- [x] **File**: `crates/shamap/src/inner_node.rs:65-78`
- **Issue**: Computes hash but doesn't store it
- **Impact**: Inner node hash not cached
- **Implementation**: Store computed hash in node
- **Status**: ✅ Implemented - Completed as part of item #1

### 10. Node Shutdown Persistence
- [x] **File**: `crates/node/src/application.rs:310`
- **Issue**: TODO: Persist ledger state, peer info
- **Impact**: Data loss on shutdown
- **Implementation**: Save state to database
- **Status**: ✅ Implemented - Added `persist_state()` method that saves peer info and node state to database

## Progress

- **Total**: 10 items
- **Completed**: 10
- **Remaining**: 0
