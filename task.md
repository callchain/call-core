# Call-Core Implementation Tasks

## Overview
Tracking stub code, placeholder implementations, and incomplete features in call-core.

## Critical Gaps (Priority 1)

### 1. Network Bootstrap System [CRITICAL] ✓
**File:** `crates/protocol/src/bootstrap.rs`
**Status:** ✅ COMPLETED

**Implementation Completed:**
- [x] Implement real ledger fetching from peers via GetLedger messages
- [x] Remove placeholder hash system
- [x] Add ledger verification against expected hashes
- [x] Implement disk loading with fallback to network

**Changes Made:**
- Added `PeerNetwork` trait for broadcasting GetLedger requests to peers
- Added `LedgerStorage` trait for persistent ledger storage/retrieval
- Implemented disk loading with fallback to network in `GenesisLoader`
- Updated `LedgerSynchronizer` to try local storage before network requests
- Added `save_to_storage()` to persist validated ledgers
- `request_ledger()` and `request_ledger_by_seq()` now broadcast to peers
- Added `NullPeerNetwork` and `NullLedgerStorage` for testing
- Updated `BootstrapManager::initialize()` to return `Result<Ledger, String>`

### 2. Transaction Field Parsing [CRITICAL] ✓
**File:** `crates/node/src/application.rs:1523-1660`
**Status:** ✅ COMPLETED

**Implementation Completed:**
- [x] Flags (type=2/UInt32, field=22)
- [x] SourceTag (type=2/UInt32, field=3)
- [x] Amount (type=6/Amount, field=1)
- [x] Destination (type=8/Account, field=3)
- [x] SigningPubKey (type=7/VL, field=3)
- [x] TxnSignature (type=7/VL, field=4)
- [x] TakerPays, TakerGets, LimitAmount

### 3. Pre-Authorized Depositors [HIGH] ✓
**File:** `crates/protocol/src/tx_engine.rs:348`
**Status:** ✅ COMPLETED

**Implementation Completed:**
- [x] Added `DepositPreauth` ledger entry type (0x70)
- [x] Created `DepositPreauth` struct with account, authorize, flags fields
- [x] Added deposit preauth methods to `LedgerState`:
  * `get_deposit_preauth()` - retrieve a preauthorization
  * `set_deposit_preauth()` - create/update a preauthorization
  * `delete_deposit_preauth()` - remove a preauthorization
  * `is_authorized_to_send()` - check if sender can deposit to recipient
  * `get_account_deposit_preauths()` - list all preauths for an account
  * `get_authorized_deposit_preauths()` - list all preauths where account is authorized
- [x] Added `DepositPreauth` transaction type (tx type 19)
- [x] Implemented transaction processing for DepositPreauth:
  * `preflight_deposit_preauth()` - validation
  * `preclaim_deposit_preauth()` - state checks
  * `apply_deposit_preauth()` - create preauthorization
- [x] Updated payment processing to check `is_authorized_to_send()`
- [x] Updated `LedgerView` trait with deposit preauth methods
- [x] Added serialization/deserialization for DepositPreauth entries

**Files Modified:**
- `crates/protocol/src/ledger_entries.rs`
- `crates/protocol/src/tx_engine.rs`
- `crates/protocol/src/transactions.rs`
- `crates/protocol/src/views.rs`
- `crates/protocol/src/tx_queue.rs`

## Stub Functions (Priority 2)

### 4. Mock LedgerView Functions [HIGH]
**Files:**
- `crates/protocol/src/tx_engine.rs:1063-1087`
- `crates/protocol/src/tx_queue.rs:533-571`
- `crates/protocol/src/views.rs:80-118`

**Stub Functions to Implement:**
- [ ] `set_call_state()` - empty body
- [ ] `set_offer()` - empty body
- [ ] `delete_offer()` - empty body
- [ ] `set_signer_list()` - empty body
- [ ] `set_nickname_entry()` - empty body

### 5. Ledger State Cloning [MEDIUM] ✓
**File:** `crates/protocol/src/ledger_entries.rs:2592`
**Status:** ✅ COMPLETED

**Implementation Completed:**
- [x] Added `#[derive(Clone)]` to `SHAMap` struct
- [x] Updated `Clone` implementation for `LedgerState` to deep copy:
  * `state_map: self.state_map.clone()` - deep copy of SHAMap tree
  * `nickname_index: self.nickname_index.clone()` - copy of HashMap
- [x] Added test `test_shamap_clone()` to verify:
  * Clone has same data as original
  * Clone has same root hash as original
  * Modifying original doesn't affect clone
  * Root hashes differ after modification

**Files Modified:**
- `crates/shamap/src/map.rs`
- `crates/protocol/src/ledger_entries.rs`

## Simplified Implementations (Priority 3)

### 6. Consensus Voting Logic [HIGH]
**File:** `crates/consensus/src/consensus.rs:430`
**Issue:** Hardcoded 60% acceptance
- [ ] Implement proper consensus voting algorithm
- [ ] Add validator vote tracking
- [ ] Implement Byzantine fault tolerance

### 7. Ledger Items Iterator [MEDIUM]
**File:** `crates/protocol/src/ledger.rs:395`
**Issue:** Returns `std::iter::empty()`
- [ ] Implement actual ledger item iteration
- [ ] Combine base items with changes properly

### 8. Genesis Validation [MEDIUM]
**File:** `crates/protocol/src/genesis.rs:242`
**Issue:** Returns `Ok(())` without validation
- [ ] Implement comprehensive genesis validation
- [ ] Validate chain ID, network name
- [ ] Verify allocation amounts
- [ ] Check validator configurations

### 9. Network Send Confirmation [MEDIUM]
**File:** `crates/network/src/connection.rs:168`
**Issue:** Returns success without confirmation
- [ ] Implement message delivery confirmation
- [ ] Add retry logic for failed sends
- [ ] Track message delivery status

## Mock Implementations (Priority 4)

### 10. Test Mock Cleanup [LOW]
**Files:**
- `tests/rpc_api_tests.rs:73-74`
- `tests/websocket_api_tests.rs:97-98`
- `crates/node/src/rpc.rs:1743`

**Tasks:**
- [ ] Document mock handlers properly
- [ ] Ensure mocks don't leak into production code
- [ ] Add integration tests with real components

## Other Placeholders

### 11. Time Implementation [MEDIUM]
**File:** `crates/protocol/src/tx_queue.rs:322-327`
**Issue:** Comment says "use actual time" but already using SystemTime
- [ ] Verify time handling is correct
- [ ] Remove placeholder comment if implementation is complete

### 12. Progress Calculation [LOW]
**File:** `crates/protocol/src/bootstrap.rs:402`
**Issue:** Basic percentage calculation
- [ ] Enhance with weighted progress (bytes vs ledgers)
- [ ] Add ETA estimation
- [ ] Improve accuracy

## Implementation Order

1. **Phase 1 - Critical:** Items 1-3 (Bootstrap, Transaction Fields, Deposit Auth)
2. **Phase 2 - Core:** Items 4-6 (Stub Functions, Ledger Cloning, Consensus)
3. **Phase 3 - Polish:** Items 7-9 (Iterator, Genesis Validation, Network)
4. **Phase 4 - Cleanup:** Items 10-12 (Mocks, Time, Progress)

## Progress Tracking

| Phase | Items | Completed | Status |
|-------|-------|-----------|--------|
| Phase 1 | 3 | 3 | ✅ COMPLETE |
| Phase 2 | 3 | 2 | In Progress |
| Phase 3 | 3 | 1 | In Progress |
| Phase 4 | 3 | 0 | Not Started |

**Total Tasks:** 12
**Completed:** 6
**In Progress:** 0
**Pending:** 6

## Completed Tasks

### Task 1: Transaction Field Parsing ✓
- Added `flags` and `source_tag` fields to Transaction struct
- Updated deserialize_transaction to store all parsed values:
  - Flags (type=2/UInt32, field=22)
  - SourceTag (type=2/UInt32, field=3)
  - Amount (type=6/Amount, field=1)
  - Destination (type=8/Account, field=3)
  - SigningPubKey (type=7/VL, field=3)
  - TxnSignature (type=7/VL, field=4)
  - TakerPays (type=6/Amount, field=5)
  - TakerGets (type=6/Amount, field=6)
  - LimitAmount (type=6/Amount, field=17)
- Files modified:
  - `crates/protocol/src/transactions.rs`
  - `crates/node/src/application.rs`

### Task 4: SHAMap Remove and delete_offer ✓
- Implemented `remove_item()` method for SHAMap
- Implemented `take_child()` method for SHAMapInnerNode and SHAMapInnerNodeV2
- Implemented `is_empty()` and `is_branch_set()` for SHAMapInnerNodeV2
- Updated `delete_offer()` in LedgerState to properly remove offers
- Files modified:
  - `crates/shamap/src/map.rs`
  - `crates/shamap/src/inner_node.rs`
  - `crates/protocol/src/ledger_entries.rs`

### Task 7: Genesis Validation ✓
- Add network name validation (must not be empty)
- Add reserve settings validation (base and increment must not be 0)
- Add per-allocation validation:
  * Validate address format for each allocation
  * Validate balance is a valid number
  * Validate balance is not 0
  * Validate sequence > 0
- Add validator configuration validation:
  * Check node_id is not empty
  * Check public_key is not empty
- Files modified:
  - `crates/protocol/src/genesis.rs`

### Task 2: Network Bootstrap System ✓
- Added `PeerNetwork` trait for real peer communication via GetLedger messages
- Added `LedgerStorage` trait for persistent ledger storage/retrieval
- Implemented `try_load_from_storage()` to load ledgers locally before network requests
- Implemented `save_to_storage()` to persist validated ledgers
- Updated `request_ledger()` to broadcast GetLedger requests to connected peers
- Updated `request_ledger_by_seq()` to request from network with placeholder hash tracking
- Enhanced `GenesisLoader::load_or_create()` with disk loading and hash verification
- Added `load_or_create_with_fallback()` for network fallback support
- Updated `BootstrapManager` with `set_network()` and `set_storage()` methods
- Added `NullPeerNetwork` and `NullLedgerStorage` implementations for testing
- Files modified:
  - `crates/protocol/src/bootstrap.rs`

### Task 3: Pre-Authorized Depositors List ✓
- Added `DepositPreauth` ledger entry type (0x70 / 'p')
- Created `DepositPreauth` struct with fields: account, authorize, flags, previous_txn_id, previous_txn_lgr_seq
- Added deposit preauth flags module with `LSF_ACTIVE` flag
- Implemented `LedgerEntry` trait for `DepositPreauth` with serialization
- Added deposit preauth methods to `LedgerState`:
  * `get_deposit_preauth()` - retrieve by account + authorize
  * `set_deposit_preauth()` - store preauthorization
  * `delete_deposit_preauth()` - remove preauthorization
  * `is_authorized_to_send()` - check if payment is allowed (handles deposit auth logic)
  * `get_account_deposit_preauths()` - list by owner account
  * `get_authorized_deposit_preauths()` - list by authorized sender
- Added `DepositPreauth` transaction type (tx type 19)
- Implemented transaction processing: preflight, preclaim, and apply phases
- Added `unauthorize` field to Transaction struct for cancellation support
- Updated payment logic to use `is_authorized_to_send()` instead of blanket rejection
- Updated `LedgerView` trait with deposit preauth methods
- Updated all mock implementations (BasicLedgerView, MutableLedgerView, MockLedgerView)
- Added `tecNO_ENTRY` error code for missing preauthorization during cancellation
- Files modified:
  - `crates/protocol/src/ledger_entries.rs`
  - `crates/protocol/src/tx_engine.rs`
  - `crates/protocol/src/transactions.rs`
  - `crates/protocol/src/views.rs`
  - `crates/protocol/src/tx_queue.rs`

### Task 5: SHAMap Clone / Ledger State Cloning ✓
- Added `#[derive(Clone)]` to `SHAMap` struct in `crates/shamap/src/map.rs`
- Updated `Clone` implementation for `LedgerState` to perform deep copy:
  * `state_map: self.state_map.clone()` - deep copy of SHAMap tree structure
  * `nickname_index: self.nickname_index.clone()` - copy of nickname HashMap
- All component types already derived Clone (SHAMapAbstractNode, SHAMapInnerNode, SHAMapInnerNodeV2, SHAMapTreeNode, SHAMapItem)
- Added comprehensive test `test_shamap_clone()` verifying:
  * Clone has identical data to original
  * Clone has identical root hash to original
  * Modifying original after clone doesn't affect clone
  * Root hashes differ after modification (proving independent copies)
- Files modified:
  - `crates/shamap/src/map.rs`
  - `crates/protocol/src/ledger_entries.rs`
