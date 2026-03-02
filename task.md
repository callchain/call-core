# Call-Core Implementation Tasks

## Overview
Tracking stub code, placeholder implementations, and incomplete features in call-core.

## Critical Gaps (Priority 1)

### 1. Network Bootstrap System [CRITICAL]
**File:** `crates/protocol/src/bootstrap.rs`
**Issues:**
- Uses placeholder hashes instead of real peer communication
- `compute_placeholder_hash()` creates temporary hashes
- `request_ledger_by_seq()` doesn't actually request from peers
- Lines: 162-261, 197, 235, 436

**Implementation Required:**
- [ ] Implement real ledger fetching from peers via GetLedger messages
- [ ] Remove placeholder hash system
- [ ] Add ledger verification against expected hashes
- [ ] Implement disk loading with fallback to network

### 2. Transaction Field Parsing [CRITICAL]
**File:** `crates/node/src/application.rs:1523-1660`
**Issues:** Multiple fields explicitly skipped:
- [ ] Flags (type=2/UInt32, field=22)
- [ ] SourceTag (type=2/UInt32, field=3)
- [ ] Amount (type=6/Amount, field=1)
- [ ] Destination (type=8/Account, field=3)
- [ ] SigningPubKey (type=7/VL, field=3)
- [ ] TxnSignature (type=7/VL, field=4)
- [ ] NetworkID (if applicable)

### 3. Pre-Authorized Depositors [HIGH]
**File:** `crates/protocol/src/tx_engine.rs:348`
**Issue:** TODO comment - deposit authorization incomplete
- [ ] Implement pre-authorized depositors list
- [ ] Fix payment processing for deposit auth accounts

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

### 5. Ledger State Cloning [MEDIUM]
**File:** `crates/protocol/src/ledger_entries.rs:2376`
**Issue:** SHAMap doesn't implement Clone
- [ ] Implement deep copy for ledger state tree
- [ ] Fix `Clone` implementation for SHAMap

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
| Phase 1 | 3 | 1 | In Progress |
| Phase 2 | 3 | 1 | In Progress |
| Phase 3 | 3 | 0 | Not Started |
| Phase 4 | 3 | 0 | Not Started |

**Total Tasks:** 12
**Completed:** 3
**In Progress:** 0
**Pending:** 9

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
