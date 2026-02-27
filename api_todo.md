# Call-core RPC and WebSocket API TODO

This document tracks missing RPC and WebSocket APIs that exist in the original `calld` (C++) implementation but are not yet implemented in `call-core` (Rust).

---

## RPC API Comparison

### Implemented RPC Methods in call-core

| Method | Status | Notes |
|--------|--------|-------|
| `server_info` | ✅ | Returns server status |
| `server_state` | ✅ | Machine-readable server state |
| `ping` | ✅ | Simple ping/pong |
| `ledger_current` | ✅ | Get current ledger |
| `ledger_closed` | ✅ | Get closed ledger |
| `ledger` | ✅ | Get ledger by index |
| `ledger_data` | ✅ | Get entries in a ledger |
| `ledger_entry` | ✅ | Get specific ledger entry |
| `ledger_header` | ✅ | Get ledger header |
| `account_info` | ✅ | Get account information |
| `account_tx` | ✅ | Get account transactions (stub) |
| `account_lines` | ✅ | Get trust lines (stub) |
| `account_objects` | ✅ | Get account objects (stub) |
| `account_offers` | ✅ | Get account offers (stub) |
| `account_channels` | ✅ | Get payment channels (stub) |
| `account_currencies` | ✅ | Get currencies for account (stub) |
| `gateway_balances` | ✅ | Get gateway balances (stub) |
| `owner_info` | ✅ | Get owner info (stub) |
| `submit` | ✅ | Submit transaction |
| `submit_multisigned` | ✅ | Submit multisigned transaction |
| `tx` | ✅ | Get transaction by hash |
| `tx_history` | ✅ | Get transaction history (stub) |
| `transaction_entry` | ✅ | Get transaction entry details |
| `sign` | ✅ | Sign transaction locally (stub) |
| `sign_for` | ✅ | Sign for another account (stub) |
| `book_offers` | ✅ | Get order book (stub) |
| `path_find` | ✅ | Find payment paths (stub) |
| `call_path_find` | ✅ | Callchain path finding (stub) |
| `validation_create` | ✅ | Generate validation key |
| `validation_seed` | ✅ | Get validation from seed (stub) |
| `wallet_propose` | ✅ | Generate wallet |
| `wallet_seed` | ✅ | Get wallet from seed (stub) |
| `wallet_lock` | ✅ | Lock wallet |
| `wallet_unlock` | ✅ | Unlock wallet (stub) |
| `wallet_verify` | ✅ | Verify wallet signature (stub) |
| `peers` | ✅ | Get peer count and list |
| `connect` | ✅ | Connect to peer (stub) |
| `consensus_info` | ✅ | Get consensus information |
| `fee` | ✅ | Get current fee info |
| `unl_list` | ✅ | Get UNL list |
| `validators` | ✅ | Get validators info |
| `validator_list_sites` | ✅ | Get validator list sites |
| `blacklist` | ✅ | Get blacklist (stub) |
| `stop` | ✅ | Stop server |
| `ledger_accept` | ✅ | Force ledger close |
| `ledger_cleaner` | ✅ | Ledger cleanup (stub) |
| `ledger_request` | ✅ | Request ledger from peers (stub) |
| `log_level` | ✅ | Set log level (stub) |
| `log_rotate` | ✅ | Rotate log files |
| `get_counts` | ✅ | Get various counts (stub) |
| `fetch_info` | ✅ | Get fetch info |
| `feature` | ✅ | Query/set features (stub) |
| `random` | ✅ | Generate random number |
| `print` | ✅ | Print debug info (stub) |
| `no_call_check` | ✅ | Disable CALL check |
| `can_delete` | ✅ | Check if ledger can be deleted (stub) |
| `session_open` | ✅ | Open session |
| `session_close` | ✅ | Close session |
| `nick_search` | ✅ | Search nicknames (stub) |
| `account_issues` | ✅ | Get issues for account (stub) |
| `account_invoices` | ✅ | Get invoices for account (stub) |
| `version` | ✅ | Get version info |
| `channel_authorize` | ✅ | Create channel auth (stub) |
| `channel_verify` | ✅ | Verify channel auth (stub) |
| `paychan_claim` | ✅ | Claim payment channel (stub) |
| `unsubscribe` | ✅ | Unsubscribe from streams |

---

## Missing RPC Methods

All RPC method stubs are now implemented. The following need full implementation:

### Methods with Stub Implementation (Need Full Implementation)

| Method | Current Status | What's Missing |
|--------|---------------|----------------|
| `account_tx` | Returns empty array | Database query for account transactions |
| `account_lines` | Returns empty array | Trust line lookup in ledger state |
| `account_objects` | Returns empty array | Account objects lookup |
| `account_offers` | Returns empty array | Account offers lookup |
| `account_channels` | Returns empty array | Payment channel lookup |
| `account_currencies` | Returns empty array | Currency lookup |
| `gateway_balances` | Returns empty objects | Gateway balance calculation |
| `owner_info` | Returns zeros | Owner count calculation |
| `tx_history` | Returns empty array | Transaction history query |
| `sign` | Returns placeholder | Actual transaction signing |
| `sign_for` | Returns placeholder | Signing for another account |
| `book_offers` | Returns empty array | Order book query |
| `path_find` | Returns empty paths | Path finding algorithm |
| `call_path_find` | Returns empty paths | Callchain routing |
| `connect` | Returns message only | Actual peer connection |
| `peers` | Returns count only | Detailed peer information |
| `validation_seed` | Returns placeholder | Validation seed handling |
| `wallet_seed` | Returns placeholder | Wallet seed handling |
| `wallet_unlock` | Returns success only | Actual wallet unlock |
| `wallet_verify` | Returns true only | Actual signature verification |
| `blacklist` | Returns empty array | Blacklist management |
| `ledger_request` | Returns success only | Peer ledger request |
| `log_level` | Returns message only | Runtime log level change |
| `get_counts` | Returns zeros | Actual ledger counting |
| `feature` | Returns empty object | Feature flag management |
| `print` | Returns message only | Debug info output |
| `can_delete` | Returns false only | Delete check logic |
| `nick_search` | Returns empty array | Nickname search |
| `account_issues` | Returns empty array | Issues lookup |
| `account_invoices` | Returns empty array | Invoices lookup |
| `channel_authorize` | Returns placeholder | Channel signature |
| `channel_verify` | Returns true only | Actual signature verification |
| `paychan_claim` | Returns null only | Claim creation |

---

## WebSocket API Status

### Implemented WebSocket Features in call-core

| Feature | Status | Notes |
|---------|--------|-------|
| WebSocket server | ⚠️ | Basic structure, incomplete handler |
| `subscribe` command | ⚠️ | Basic streams only |
| `unsubscribe` command | ✅ | Basic implementation |
| `ping` command | ✅ | Pong response |
| Ledger broadcast | ⚠️ | Basic implementation |
| Transaction broadcast | ⚠️ | Stub |
| Validation broadcast | ⚠️ | Stub |

### Missing WebSocket Features

| Feature | Description | Priority |
|---------|-------------|----------|
| Full subscribe streams | `ledger`, `transactions`, `validations` | High |
| Account subscriptions | Subscribe to specific account updates | High |
| Peer messages | Subscribe to peer messages | Medium |
| Consensus streams | Subscribe to consensus phase updates | Medium |
| Admin streams | Subscribe to admin events | Low |
| RT (real-time) subscriptions | Real-time transaction stream | High |
| Manifestations | Manifest updates | Low |
| Blockchain updates | Full blockchain state updates | Medium |
| Response formatting | Proper JSON response format | High |
| Error handling | WebSocket error responses | Medium |
| Connection management | Track and manage connections | Medium |
| Rate limiting | Prevent abuse | Medium |
| Authentication | Optional auth for admin streams | Low |

---

## Implementation Notes

### Priority Legend

- **High**: Core functionality needed for basic node operation
- **Medium**: Important for advanced features and tooling
- **Low**: Nice to have, admin/debug tools

### Dependencies

Many RPC methods depend on:
1. Full ledger state implementation
2. Transaction history database
3. Payment channel support
4. Trust line management
5. Offer book persistence

### Completed Phases

#### ✅ Phase 1 - Core APIs (Method Stubs Complete)
All method signatures implemented with stub bodies:
- `account_tx`, `account_lines`, `account_objects`, `account_offers`
- `book_offers`, `ledger_entry`, `ledger_data`
- `fee`, `consensus_info`, `unl_list`, `validators`

#### ✅ Phase 2 - Transaction APIs (Method Stubs Complete)
- `submit_multisigned`, `sign`, `sign_for`
- `path_find`, `call_path_find`
- `gateway_balances`, `account_channels`, `account_currencies`

#### ✅ Phase 3 - Network & Admin (Method Stubs Complete)
- `peers`, `server_state`, `connect`
- `validation_seed`, `validators`, `log_level`
- All additional admin methods

#### ⏳ Phase 4 - Full Implementation Needed
Now working on replacing stub implementations with actual logic.

---

## API Schema References

### Standard Response Format (JSON-RPC 2.0)

```json
{
  "jsonrpc": "2.0",
  "result": { ... },
  "id": 1
}
```

### Error Response Format

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32000,
    "message": "Error message",
    "data": "Additional data"
  },
  "id": 1
}
```

### WebSocket Subscribe Request

```json
{
  "command": "subscribe",
  "streams": ["ledger", "transactions"],
  "accounts": ["rAddress1", "rAddress2"]
}
```

### WebSocket Subscribe Response

```json
{
  "type": "response",
  "status": "success"
}
```

### WebSocket Ledger Stream

```json
{
  "type": "ledger",
  "ledger": {
    "ledger_index": 12345,
    "ledger_hash": "...",
    "close_time": 1234567890
  }
}
```

---

## Tracking

Last updated: 2026-02-27

### Summary

- **Total RPC Methods**: 70+
- **Implemented (all stubs)**: 70+ ✅
- **Need full implementation**: ~30 methods
- **WebSocket features remaining**: ~13

### Progress

- [x] Phase 1: Core APIs (method stubs)
- [x] Phase 2: Transaction APIs (method stubs)
- [x] Phase 3: Network & Admin (method stubs)
- [ ] Phase 4: Full implementation of stub methods
- [ ] WebSocket full implementation
