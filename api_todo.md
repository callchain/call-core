# Call-core RPC and WebSocket API TODO

This document tracks missing RPC and WebSocket APIs that exist in the original `calld` (C++) implementation but are not yet implemented in `call-core` (Rust).

---

## RPC API Comparison

### Implemented RPC Methods in call-core

| Method | Status | Notes |
|--------|--------|-------|
| `server_info` | ✅ | Returns server status |
| `ping` | ✅ | Simple ping/pong |
| `ledger_current` | ✅ | Get current ledger |
| `ledger_closed` | ✅ | Get closed ledger |
| `ledger` | ⚠️ | Partial - only current ledger |
| `account_info` | ✅ | Get account information |
| `account_tx` | ⚠️ | Returns empty array |
| `submit` | ✅ | Submit transaction |
| `tx` | ✅ | Get transaction by hash |
| `book_offers` | ⚠️ | Returns empty array |
| `validation_create` | ✅ | Generate validation key |
| `wallet_propose` | ✅ | Generate wallet |
| `peers` | ⚠️ | Returns peer count only |
| `stop` | ✅ | Stop server |
| `ledger_accept` | ✅ | Force ledger close |

---

## Missing RPC Methods (High Priority)

### Ledger Related

| Method | Description | Priority |
|--------|-------------|----------|
| `ledger_accept` | Implemented | - |
| `ledger_cleaner` | Ledger cleanup admin method | Low |
| `ledger_closed` | Implemented | - |
| `ledger_current` | Implemented | - |
| `ledger_data` | Get entries in a ledger | High |
| `ledger_entry` | Get specific ledger entry | High |
| `ledger_header` | Get ledger header | Medium |
| `ledger_request` | Request ledger from peers | Medium |

### Account Related

| Method | Description | Priority |
|--------|-------------|----------|
| `account_channels` | Get payment channels for account | High |
| `account_currencies` | Get currencies for account | Medium |
| `account_info` | Implemented | - |
| `account_lines` | Get trust lines for account | High |
| `account_objects` | Get objects owned by account | High |
| `account_offers` | Get offers for account | High |
| `account_tx` | Partial - empty array | High |
| `account_issues` | Get issues for account | Medium |
| `account_invoices` | Get invoices for account | Medium |
| `owner_info` | Get owner info | Medium |
| `gateway_balances` | Get gateway balances | High |

### Transaction Related

| Method | Description | Priority |
|--------|-------------|----------|
| `submit` | Implemented | - |
| `submit_multisigned` | Submit multisigned transaction | High |
| `tx` | Implemented | - |
| `tx_history` | Get transaction history | Medium |
| `transaction_entry` | Get transaction entry details | Medium |
| `sign` | Sign transaction locally | High |
| `sign_for` | Sign transaction for another account | Medium |
| `paychan_claim` | Claim payment channel | Low |

### DEX / Order Book Related

| Method | Description | Priority |
|--------|-------------|----------|
| `book_offers` | Partial - empty array | High |
| `path_find` | Find payment paths | High |
| `call_path_find` | Callchain path finding | High |
| `random` | Generate random number | Low |

### Consensus / Network Related

| Method | Description | Priority |
|--------|-------------|----------|
| `blacklist` | Manage blacklist | Medium |
| `can_delete` | Check if ledger can be deleted | Low |
| `channel_authorize` | Create channel authorization | Medium |
| `channel_verify` | Verify channel authorization | Medium |
| `connect` | Connect to peer | Medium |
| `consensus_info` | Get consensus information | High |
| `feature` | Query/set features | Medium |
| `fee` | Get current fee info | High |
| `fetch_info` | Get fetch info | Low |
| `log_level` | Set log level | Medium |
| `log_rotate` | Rotate log files | Low |
| `no_call_check` | Disable call check | Low |
| `peers` | Partial - count only | Medium |
| `session_close` | Close session | Low |
| `session_open` | Open session | Low |
| `stop` | Implemented | - |
| `unl_list` | Get UNL list | High |
| `validators` | Get validators info | High |
| `validator_list_sites` | Get validator list sites | Medium |
| `validation_create` | Implemented | - |
| `validation_seed` | Get validation seed | Medium |

### Wallet / Key Management

| Method | Description | Priority |
|--------|-------------|----------|
| `wallet_lock` | Lock wallet | Medium |
| `wallet_propose` | Implemented | - |
| `wallet_seed` | Get wallet seed | Medium |
| `wallet_unlock` | Unlock wallet | Medium |
| `wallet_verify` | Verify wallet signature | Medium |

### System / Debug Related

| Method | Description | Priority |
|--------|-------------|----------|
| `get_counts` | Get various counts | Low |
| `nick_search` | Search nicknames | Low |
| `print` | Print debug info | Low |
| `server_info` | Implemented | - |
| `server_state` | Get server state (machine format) | Medium |
| `version` | Get version info | Low |

---

## WebSocket API Comparison

### Implemented WebSocket Features in call-core

| Feature | Status | Notes |
|---------|--------|-------|
| WebSocket server | ⚠️ | Basic structure, incomplete handler |
| `subscribe` command | ⚠️ | Basic streams only |
| `unsubscribe` command | ⚠️ | Basic streams only |
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

### Recommended Implementation Order

#### Phase 1 - Core APIs (High Priority)
1. `account_tx` - Full implementation
2. `account_lines` - Trust lines
3. `account_objects` - Account objects
4. `account_offers` - Account offers
5. `book_offers` - Full order book
6. `ledger_entry` - Ledger entry lookup
7. `ledger_data` - Ledger data iteration
8. `fee` - Fee information
9. `consensus_info` - Consensus status
10. `unl_list` - UNL management
11. WebSocket subscribe/unsubscribe - Full streams

#### Phase 2 - Transaction APIs (High Priority)
1. `submit_multisigned` - Multisig support
2. `sign` - Transaction signing
3. `path_find` - Payment path finding
4. `call_path_find` - Callchain path finding
5. `gateway_balances` - Gateway balances
6. `account_channels` - Payment channels
7. `account_currencies` - Currency info

#### Phase 3 - Network & Admin (Medium Priority)
1. `peers` - Full peer information
2. `server_state` - Machine-readable state
3. `connect` - Peer connection
4. `validation_seed` - Validation management
5. `validators` - Validator info
6. `log_level` - Runtime logging control
7. WebSocket account subscriptions

#### Phase 4 - Additional Features (Medium/Low Priority)
1. `tx_history` - Transaction history
2. `transaction_entry` - Transaction details
3. `ledger_header` - Ledger headers
4. `ledger_request` - Peer ledger requests
5. `sign_for` - Sign for another account
6. Remaining admin and debug methods

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

Total Missing RPC Methods: ~50
Total Missing WebSocket Features: ~15

### Progress

- [ ] Phase 1: Core APIs
- [ ] Phase 2: Transaction APIs
- [ ] Phase 3: Network & Admin
- [ ] Phase 4: Additional Features
