# Call-core RPC and WebSocket API TODO

This document tracks RPC and WebSocket API implementation status in call-core (Rust).

---

## RPC API Implementation Status

### Fully Implemented RPC Methods ✅

| Method | Status | Notes |
|--------|--------|-------|
| `server_info` | ✅ | Returns server status |
| `server_state` | ✅ | Machine-readable server state |
| `ping` | ✅ | Simple ping/pong |
| `ledger_current` | ✅ | Get current ledger |
| `ledger_closed` | ✅ | Get closed ledger |
| `ledger` | ✅ | Get ledger by index |
| `ledger_data` | ✅ | Get entries in a ledger (iterates SHAMap) |
| `ledger_entry` | ✅ | Get specific ledger entry by index |
| `ledger_header` | ✅ | Get ledger header |
| `account_info` | ✅ | Get account information |
| `account_tx` | ✅ | Get account transactions (stub - empty array) |
| `account_lines` | ✅ | Get trust lines (full ledger state lookup) |
| `account_objects` | ✅ | Get account objects (full lookup) |
| `account_offers` | ✅ | Get account offers (full lookup) |
| `account_channels` | ✅ | Get payment channels (directory-based) |
| `account_currencies` | ✅ | Get currencies for account |
| `gateway_balances` | ✅ | Get gateway balances (stub) |
| `owner_info` | ✅ | Get owner info (stub) |
| `submit` | ✅ | Submit transaction |
| `submit_multisigned` | ✅ | Submit multisigned transaction |
| `tx` | ✅ | Get transaction by hash |
| `tx_history` | ✅ | Get transaction history (stub) |
| `transaction_entry` | ✅ | Get transaction entry details |
| `sign` | ✅ | Sign transaction locally (full signing) |
| `sign_for` | ✅ | Sign for another account (full signing) |
| `book_offers` | ✅ | Get order book (full ledger iteration) |
| `path_find` | ✅ | Find payment paths (stub) |
| `call_path_find` | ✅ | Callchain path finding (stub) |
| `validation_create` | ✅ | Generate validation key |
| `validation_seed` | ✅ | Get validation from seed (stub) |
| `wallet_propose` | ✅ | Generate wallet |
| `wallet_seed` | ✅ | Get wallet from seed (stub) |
| `wallet_lock` | ✅ | Lock wallet |
| `wallet_unlock` | ✅ | Unlock wallet (stub) |
| `wallet_verify` | ✅ | Verify wallet signature (stub) |
| `peers` | ✅ | Get peer count and detailed list |
| `connect` | ✅ | Connect to peer (stub) |
| `consensus_info` | ✅ | Get consensus information |
| `fee` | ✅ | Get current fee info |
| `unl_list` | ✅ | Get UNL list (stub) |
| `validators` | ✅ | Get validators info (stub) |
| `validator_list_sites` | ✅ | Get validator list sites (stub) |
| `blacklist` | ✅ | Get blacklist (stub) |
| `stop` | ✅ | Stop server |
| `ledger_accept` | ✅ | Force ledger close |
| `ledger_cleaner` | ✅ | Ledger cleanup (stub) |
| `ledger_request` | ✅ | Request ledger from peers (stub) |
| `log_level` | ✅ | Set log level (stub) |
| `log_rotate` | ✅ | Rotate log files |
| `get_counts` | ✅ | Get various counts (full counting) |
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

## Implementation Summary

### Fully Implemented (with actual logic)

The following methods have complete implementations that interact with the ledger state:

1. **`account_lines`** - Queries all CallState entries from ledger state for an account
2. **`account_objects`** - Queries all ledger objects (offers, directories, trust lines) for an account
3. **`account_offers`** - Queries all OfferEntry entries for an account with quality calculation
4. **`account_channels`** - Queries directory nodes for payment channel information
5. **`book_offers`** - Iterates through all offers in ledger state, filters by limit
6. **`get_counts`** - Counts accounts, offers, and trust lines by iterating ledger state
7. **`peers`** - Gets detailed peer information from overlay including stats, latency, direction
8. **`sign`** - Signs transactions using secp256k1 private key
9. **`sign_for`** - Signs transactions for another account using secp256k1
10. **`ledger_data`** - Iterates SHAMap entries with limit
11. **`ledger_entry`** - Gets specific entry by key from SHAMap

### Stub Implementations

The following methods return proper response structures but need database/ledger integration:

- `account_tx` - Needs transaction history database
- `tx_history` - Needs transaction history database
- `path_find` / `call_path_find` - Needs path finding algorithm
- `gateway_balances` - Needs gateway balance calculation
- `owner_info` - Needs owner count calculation
- `wallet_verify` / `channel_verify` - Need actual signature verification

---

## WebSocket API Status

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
| Response formatting | Proper JSON response format | High |
| Error handling | WebSocket error responses | Medium |
| Connection management | Track and manage connections | Medium |

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

---

## Tracking

Last updated: 2026-02-27

### Summary

- **Total RPC Methods**: 70+
- **Fully Implemented**: 11 methods with real logic ✅
- **Stub Implemented**: ~60 methods with proper response structures ✅
- **WebSocket features remaining**: ~7

### Progress

- [x] Phase 1: Core APIs (method stubs + account_lines, account_objects, account_offers, book_offers, ledger_data, ledger_entry)
- [x] Phase 2: Transaction APIs (sign, sign_for implemented)
- [x] Phase 3: Network & Admin (peers, get_counts implemented)
- [ ] Phase 4: Full implementation of remaining stub methods
- [ ] WebSocket full implementation
