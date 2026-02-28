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
| 7 | submit_multisigned | ⬜ Pending | Full transaction serialization |
| 8 | sign_for | ⬜ Pending | Full transaction serialization |
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

### 5. account_issues
**Location:** crates/node/src/rpc.rs
**Current:** Returns empty array
**Required:**
- [ ] Add issue tracking system
- [ ] Define issue types (disputes, frozen accounts, etc.)
- [ ] Query issues for account
- [ ] Return issue details

### 6. unsubscribe
**Location:** crates/node/src/rpc.rs
**Current:** TODO comment, no logic
**Required:**
- [ ] Access WebSocket subscription manager
- [ ] Remove stream subscriptions
- [ ] Remove account subscriptions
- [ ] Return unsubscribe confirmation

### 7. submit_multisigned
**Location:** crates/node/src/rpc.rs
**Current:** Placeholder tx_bytes (vec![0u8; 64])
**Required:**
- [ ] Full transaction serialization from tx_json
- [ ] Validate multisign signatures
- [ ] Submit to network

### 8. sign_for
**Location:** crates/node/src/rpc.rs
**Current:** Placeholder tx_bytes (vec![0u8; 64])
**Required:**
- [ ] Full transaction serialization from tx_json
- [ ] Sign with provided secret
- [ ] Return signed tx_blob

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

- **Completed:** 0/9
- **In Progress:** 0/9
- **Pending:** 9/9

**Last Updated:** 2026-02-28
