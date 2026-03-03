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

### 4. Mock LedgerView Functions [HIGH] ✓
**Status:** ✅ COMPLETED

**Files:**
- `crates/protocol/src/views.rs`

**Implementation Completed:**
- [x] `set_call_state()` - now stores CallState in HashMap
- [x] `set_offer()` - now stores OfferEntry in HashMap
- [x] `delete_offer()` - now removes offers from HashMap
- [x] `set_signer_list()` - now stores SignerList in HashMap
- [x] `set_nickname_entry()` - now stores NicknameEntry in HashMap with indexing
- [x] `set_deposit_preauth()` - now stores DepositPreauth in HashMap
- [x] `delete_deposit_preauth()` - now removes preauths from HashMap
- [x] `is_authorized_to_send()` - now properly checks deposit authorization

**Changes Made:**
- Enhanced `BasicLedgerView` with full state storage:
  * Added `accounts: HashMap<AccountID, AccountRoot>`
  * Added `call_states: HashMap<(AccountID, AccountID, Currency), CallState>`
  * Added `offers: HashMap<(AccountID, u32), OfferEntry>`
  * Added `signer_lists: HashMap<AccountID, SignerList>`
  * Added `nicknames: HashMap<UInt256, NicknameEntry>` with account index
  * Added `deposit_preauths: HashMap<(AccountID, AccountID), DepositPreauth>`

- Implemented all getter/setter methods to use storage
- Added `new_with_funded_account()` helper for testing
- Added helper methods: `account_count()`, `offer_count()`, `clear()`
- Updated `is_authorized_to_send()` to properly check LSF_DEPOSIT_AUTH flag and preauthorizations

- Added comprehensive tests (7 new tests):
  * `test_basic_ledger_view_account_operations`
  * `test_basic_ledger_view_new_with_funded_account`
  * `test_basic_ledger_view_offer_operations`
  * `test_basic_ledger_view_call_state_operations`
  * `test_basic_ledger_view_signer_list_operations`
  * `test_basic_ledger_view_deposit_preauth`
  * `test_basic_ledger_view_is_authorized_to_send`

**Files Modified:**
- `crates/protocol/src/views.rs`

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

### 6. Consensus Voting Logic [HIGH] ✓
**File:** `crates/consensus/src/consensus.rs`
**Status:** ✅ COMPLETED

**Implementation Completed:**
- [x] Added `weight` field to `ValidatorInfo` for weighted consensus
- [x] Implemented `weighted_consensus_pct()` using validator weights
- [x] Added Byzantine fault detection structures (`ByzantineFault`, `FaultType`, `FaultEvidence`)
- [x] Implemented `detect_conflicting_proposals()` to detect double-signing
- [x] Added `is_validator_faulty()` and `get_trusted_weight()` for BFT
- [x] Updated `have_consensus()` to require 80% of trusted validator weight
- [x] Updated `close_time_consensus()` to use weighted voting
- [x] Fixed `vote_on_disputes()` to use weighted vote counting with tx_inclusion map
- [x] Added comprehensive tests for BFT consensus

**Changes Made:**
- `ConsensusState` now tracks validator weights and detects Byzantine faults
- `Consensus::add_validator_with_weight()` allows setting validator weights
- Consensus now uses trusted weight (excluding faulty validators) as denominator
- All consensus calculations use weighted voting instead of simple majority
- Added 6 new tests for BFT consensus scenarios

**Files Modified:**
- `crates/consensus/src/consensus.rs`

### 7. Ledger Items Iterator [MEDIUM] ✓
**File:** `crates/protocol/src/ledger.rs`
**Status:** ✅ COMPLETED

**Implementation Completed:**
- [x] Added `Ledger::items()` method returning `impl Iterator<Item = (UInt256, STObject)>`
- [x] Implemented `ReadView` trait for `Ledger`
- [x] Fixed `OpenView::items()` to properly combine base items with changes
- [x] Items from base are filtered to exclude changed keys
- [x] Inserted/updated items from changes are included
- [x] Deleted items (None in changes) are excluded
- [x] Added 3 comprehensive tests:
  * `test_ledger_items` - validates ledger item iteration
  * `test_read_view_ledger` - validates ReadView implementation for Ledger
  * `test_open_view_items` - validates OpenView item merging with changes

**Files Modified:**
- `crates/protocol/src/ledger.rs`

### 8. Genesis Validation [MEDIUM] ✓
**File:** `crates/protocol/src/genesis.rs`
**Status:** ✅ COMPLETED

**Implementation Completed:**
- [x] Validate chain ID (cannot be 0)
- [x] Validate fee settings (base_fee, reserve settings)
- [x] Validate consensus parameters (min/max close times)
- [x] Validate network name (not empty)
- [x] Validate all allocation addresses (parseable)
- [x] Validate allocation balances (valid u64, not zero)
- [x] Validate allocation sequences (> 0)
- [x] Validate validator configurations (node_id, public_key not empty)
- [x] Comprehensive validation with descriptive error messages

**Files Modified:**
- `crates/protocol/src/genesis.rs`

### 9. Network Send Confirmation [MEDIUM] ✓
**File:** `crates/network/src/connection.rs`
**Status:** ✅ COMPLETED

**Implementation Completed:**
- [x] Added `MessageTracker` struct for tracking pending message confirmations
- [x] Added `TrackedMessage` struct with sequence_id, status, retry_count, sent_at
- [x] Added `DeliveryStatus` enum (Pending, Delivered, Failed)
- [x] Implemented `send_with_retry()` with configurable max attempts and delay
- [x] Implemented `send_tracked()` returning sequence ID for status checking
- [x] Implemented `send_with_confirmation()` waiting for peer response
- [x] Added `get_delivery_status()` to check message delivery state
- [x] Added `confirm_delivery()` to mark messages as delivered
- [x] Added `pending_message_count()` for monitoring
- [x] Added constants: SEND_RETRY_MAX_ATTEMPTS (3), SEND_RETRY_DELAY (100ms), SEND_CONFIRMATION_TIMEOUT (5s)
- [x] Added 3 tests for message tracking functionality

**Files Modified:**
- `crates/network/src/connection.rs`

## Mock Implementations (Priority 4)

### 10. Test Mock Cleanup [LOW] ✓
**Files:**
- `tests/rpc_api_tests.rs:73-74`
- `tests/websocket_api_tests.rs:97-98`
- `crates/node/src/rpc.rs:1743`

**Status:** ✅ COMPLETED

**Implementation Completed:**
- [x] Document mock handlers properly
- [x] Ensure mocks don't leak into production code

**Changes Made:**
- Added comprehensive documentation to `MockRpcHandler` in `tests/rpc_api_tests.rs`:
  * Purpose, limitations, usage examples
  * When to use / when NOT to use guidelines
  * Suggestion to use `BasicLedgerView` for more realistic state

- Added comprehensive documentation to `MockWsHandler` in `tests/websocket_api_tests.rs`:
  * Purpose, limitations, usage examples
  * Notes about subscription limitations
  * When to use / when NOT to use guidelines

- Fixed mock response in `crates/node/src/rpc.rs:1743`:
  * Changed from misleading "mock success" to proper error response
  * Added warning log when network manager is unavailable
  * Added error code 5020 for "Network manager not available"

- Added documentation to duplicate `MockLedgerView` implementations:
  * Documented simple version in `tx_queue.rs` with consolidation TODO
  * Documented functional version in `tx_engine.rs` with usage guidelines

**Files Modified:**
- `tests/rpc_api_tests.rs`
- `tests/websocket_api_tests.rs`
- `crates/node/src/rpc.rs`
- `crates/protocol/src/tx_queue.rs`
- `crates/protocol/src/tx_engine.rs`

## Other Placeholders

### 11. Time Implementation [MEDIUM] ✓
**File:** `crates/protocol/src/tx_queue.rs:322-327`
**Status:** ✅ COMPLETED

**Implementation Completed:**
- [x] Verified time handling is correct (already using `SystemTime::now()`)
- [x] Removed misleading placeholder comment

**Changes Made:**
- Removed outdated comment "In a real implementation, use actual time"
- The implementation was already correct - using `SystemTime::now()` for Unix timestamp

**Files Modified:**
- `crates/protocol/src/tx_queue.rs`

### 12. Progress Calculation [LOW] ✓
**File:** `crates/protocol/src/bootstrap.rs`
**Status:** ✅ COMPLETED

**Implementation Completed:**
- [x] Enhanced with weighted progress (bytes vs ledgers)
- [x] Added ETA estimation
- [x] Improved accuracy with multiple calculation methods

**Changes Made:**
- Extended `SyncStats` struct with new fields:
  * `transactions_to_fetch` - for transaction-weighted progress
  * `bytes_fetched` and `bytes_to_fetch` - for byte-weighted progress

- Added comprehensive progress calculation methods to `SyncStats`:
  * `progress_percent()` - simple ledger-based calculation
  * `weighted_progress_percent()` - uses bytes, transactions, or ledgers (in priority order)
  * `estimated_time_remaining()` - calculates ETA based on current rate
  * `progress_summary()` - human-readable summary with percentage and ETA
  * `sync_rate_ledgers_per_sec()` - current sync speed
  * `sync_rate_txs_per_sec()` - transaction processing speed

- Added corresponding methods to `LedgerSynchronizer`:
  * `weighted_progress_percent()` - get weighted progress
  * `estimated_time_remaining()` - get ETA in seconds
  * `progress_summary()` - get formatted progress string

- Added tests for new functionality:
  * `test_weighted_progress_calculation` - verifies all three progress methods
  * `test_progress_summary` - verifies human-readable output
  * `test_sync_rate_calculation` - verifies rate calculations

**Files Modified:**
- `crates/protocol/src/bootstrap.rs`

## Implementation Order

1. **Phase 1 - Critical:** Items 1-3 (Bootstrap, Transaction Fields, Deposit Auth)
2. **Phase 2 - Core:** Items 4-6 (Stub Functions, Ledger Cloning, Consensus)
3. **Phase 3 - Polish:** Items 7-9 (Iterator, Genesis Validation, Network)
4. **Phase 4 - Cleanup:** Items 10-12 (Mocks, Time, Progress)

## Progress Tracking

| Phase | Items | Completed | Status |
|-------|-------|-----------|--------|
| Phase 1 | 3 | 3 | ✅ COMPLETE |
| Phase 2 | 3 | 3 | ✅ COMPLETE |
| Phase 3 | 3 | 3 | ✅ COMPLETE |
| Phase 4 | 3 | 3 | ✅ COMPLETE |

**Total Tasks:** 12
**Completed:** 12
**In Progress:** 0
**Pending:** 0

## Overall Status: ✅ ALL TASKS COMPLETE

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

### Task 6: BFT Consensus Voting Logic ✓
- Added `weight` field to `ValidatorInfo` for weighted consensus voting
- Implemented `weighted_consensus_pct()` using trusted validator weights
- Added Byzantine fault detection structures: `ByzantineFault`, `FaultType`, `FaultEvidence`
- Implemented `detect_conflicting_proposals()` to detect double-signing attacks
- Added `is_validator_faulty()` and `get_trusted_weight()` for BFT security
- Updated `have_consensus()` to require 80% of trusted validator weight (not just node count)
- Updated `close_time_consensus()` to use weighted voting instead of simple majority
- Fixed `vote_on_disputes()` to use weighted vote counting with `tx_inclusion` map
- Added `Consensus::add_validator_with_weight()` API for setting validator weights
- `ConsensusState` now tracks validator weights and proposal history for fault detection
- Consensus now uses trusted weight (excluding faulty validators) as denominator
- Added 6 comprehensive tests for BFT consensus scenarios:
  * `test_validator_weights` - validates weight tracking
  * `test_weighted_consensus` - validates percentage calculation
  * `test_byzantine_fault_detection` - validates double-signing detection
  * `test_byzantine_fault_excludes_from_consensus` - validates fault exclusion
  * `test_bft_consensus_requirements` - validates 80% threshold
  * `test_close_time_consensus_weighted` - validates weighted close time consensus
- All 24 consensus tests passing
- Files modified:
  - `crates/consensus/src/consensus.rs`

### Task 7: Ledger Items Iterator ✓
- Added `Ledger::items()` method returning `impl Iterator<Item = (UInt256, STObject)>`
- Implemented `ReadView` trait for `Ledger` with all required methods:
  * `get_ledger_info()` - returns ledger info reference
  * `read()` - reads and deserializes state entry by key
  * `items()` - iterates over all state items
  * `transactions()` - iterates over transaction hashes
  * `has_transaction()` - checks if transaction exists
- Fixed `OpenView::items()` to properly combine base items with changes:
  * Collects all items from changes that are Some (inserts/updates)
  * Tracks changed keys to avoid duplicates
  * Adds base items that haven't been changed
  * Excludes deleted items (None in changes)
- Added 3 comprehensive tests:
  * `test_ledger_items` - validates ledger item iteration from state_tree
  * `test_read_view_ledger` - validates ReadView trait implementation
  * `test_open_view_items` - validates OpenView item merging (insert, erase)
- All 22 ledger tests passing
- Files modified:
  - `crates/protocol/src/ledger.rs`


### Task 9: Network Send Confirmation ✓
- Added `MessageTracker` struct for tracking pending message confirmations
- Added `TrackedMessage` struct with sequence_id, message, sent_at, status, retry_count
- Added `DeliveryStatus` enum (Pending, Delivered, Failed)
- Implemented `send_with_retry()` with configurable max attempts and delay:
  * Retries up to max_attempts (default 3)
  * Waits SEND_RETRY_DELAY (100ms) between attempts
  * Returns last error if all attempts fail
- Implemented `send_tracked()` returning sequence ID for status checking
- Implemented `send_with_confirmation()` waiting for peer response (ping/pong)
- Added helper methods: `get_delivery_status()`, `confirm_delivery()`, `pending_message_count()`
- Added message tracking constants: SEND_RETRY_MAX_ATTEMPTS, SEND_RETRY_DELAY, SEND_CONFIRMATION_TIMEOUT
- Updated `Connection` struct to include `message_tracker` field
- Added 3 tests for message tracking functionality:
  * `test_message_tracker` - tests sequence IDs, tracking, confirmation
  * `test_message_tracker_retry` - tests retry count increment
  * `test_message_tracker_stale` - tests stale message detection
- All 9 network tests passing
- Files modified:
  - `crates/network/src/connection.rs`

