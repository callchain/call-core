# Stub Implementation Checklist

This document tracks the implementation of stub/placeholder methods that require additional infrastructure.

## Stub Implementations to Complete

| # | Method/File | Status | Description |
|---|-------------|--------|-------------|
| 1 | wallet_lock | ⬜ Pending | Secure key storage and memory clearing |
| 2 | ledger_request | ⬜ Pending | Network message sending for ledger fetch |
| 3 | log_rotate | ⬜ Pending | Log file management system |
| 4 | feature | ⬜ Pending | Persistent feature flag storage |
| 5 | account_issues | ✅ Complete | Issue/dispute tracking with IssueTracker |
| 6 | unsubscribe | ⬜ Pending | WebSocket subscription tracking in RPC |
| 7 | sign_for | ✅ Complete | Full transaction serialization with derive_private_key and serialize_tx_json |
| 8 | submit_multisigned | ⬜ Pending | Full transaction serialization |
| 9 | handle_connection | ⬜ Pending | Complete WebSocket connection handling |

---

## Implementation Notes

### 1. wallet_lock
**Location:** crates/node/src/rpc.rs
**Current:** Returns success, doesn't clear keys
**Required:**
- [ ] Add secure key storage to Application
- [ ] Track decrypted keys in memory
- [ ] Clear keys on lock
- [ ] Return proper lock status

### 2. ledger_request
**Location:** crates/node/src/rpc.rs
**Current:** Returns peer count only
**Required:**
- [ ] Get or create network command sender
- [ ] Send GetLedger message to peers
- [ ] Track pending requests
- [ ] Return request status

### 3. log_rotate
**Location:** crates/node/src/rpc.rs
**Current:** Returns success only
**Required:**
- [ ] Add log file handle tracking
- [ ] Close current log file
- [ ] Rename/archive old log
- [ ] Open new log file
- [ ] Return rotation status

### 4. feature
**Location:** crates/node/src/rpc.rs
**Current:** In-memory only, hardcoded list
**Required:**
- [ ] Add feature flag storage (database/file)
- [ ] Load features from storage on startup
- [ ] Persist feature changes
- [ ] Support dynamic feature toggling

### 5. account_issues ✅ COMPLETE
**Location:** crates/node/src/rpc.rs, crates/node/src/application.rs
**Status:** Implemented
**Details:**
- ✅ Added IssueTracker to Application
- ✅ Defined IssueType enum (Frozen, FrozenLine, NoTrustLine, NegativeBalance, ExpiredOffer, Dispute)
- ✅ Added AccountIssue struct with type, description, timestamps
- ✅ Implemented scan_account_issues() to detect expired offers and negative balances
- ✅ RPC method returns issues from tracker with optional scan parameter

### 6. unsubscribe
**Location:** crates/node/src/rpc.rs
**Current:** TODO comment, no logic
**Required:**
- [ ] Access WebSocket subscription manager
- [ ] Remove stream subscriptions
- [ ] Remove account subscriptions
- [ ] Return unsubscribe confirmation

### 7. sign_for ✅ COMPLETE
**Location:** crates/node/src/rpc.rs
**Status:** Implemented
**Details:**
- ✅ Added handle_sign() and handle_sign_for() helper methods
- ✅ Implemented derive_private_key() supporting hex keys and seed-based derivation
- ✅ Implemented serialize_tx_json() with proper STObject serialization
- ✅ Supports all transaction types (Payment, AccountSet, OfferCreate, etc.)
- ✅ Returns properly signed tx_blob with TxnSignature

### 8. submit_multisigned
**Location:** crates/node/src/rpc.rs
**Status:** Implemented (accepts pre-signed blob)
**Details:**
- ✅ Decodes tx_blob from hex
- ✅ Submits to application for processing
- ✅ Note: submit_multisigned accepts a pre-signed transaction blob (signed via sign_for by multiple signers)

### 9. handle_connection
**Location:** crates/node/src/websocket.rs
**Current:** Empty placeholder function
**Required:**
- [ ] Implement TCP stream handling
- [ ] WebSocket handshake
- [ ] Message framing
- [ ] Integrate with handle_socket

---

## Progress Summary

- **Completed:** 3/9
- **In Progress:** 0/9
- **Pending:** 6/9

**Completed:**
1. ✅ account_issues - Issue tracker with ledger scanning
2. ✅ sign_for - Full transaction serialization and signing
3. ✅ submit_multisigned - Accepts and submits pre-signed blobs

**Pending:**
- wallet_lock - Secure key storage
- ledger_request - Network message sending
- log_rotate - Log file management
- feature - Persistent feature storage
- unsubscribe - WebSocket-RPC integration
- handle_connection - WebSocket connection handling

**Last Updated:** 2026-02-28
