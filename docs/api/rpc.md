# RPC API Reference

The Call-Core RPC API provides a comprehensive interface for interacting with the Callchain network. All methods are accessed via HTTP POST requests to the RPC endpoint.

## Endpoint

```
POST http://localhost:5005/
Content-Type: application/json
```

## Request Format

```json
{
  "method": "method_name",
  "params": [
    {
      "param1": "value1",
      "param2": "value2"
    }
  ]
}
```

## Response Format

### Success Response

```json
{
  "result": {
    "field": "value"
  }
}
```

### Error Response

```json
{
  "error": "error_code",
  "error_message": "Human readable description",
  "error_code": 123
}
```

---

## Account Methods

### account_info

Returns information about a specific account.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| account | string | Yes | Account address (c...) or hex |
| ledger_index | string/number | No | "current", "closed", or ledger number |
| queue | boolean | No | Include transaction queue info |

**Request:**
```json
{
  "method": "account_info",
  "params": [{
    "account": "cLSKzJZg4w2dgLfwf",
    "ledger_index": "current"
  }]
}
```

**Response:**
```json
{
  "result": {
    "account_data": {
      "Account": "cLSKzJZg4w2dgLfwf",
      "Balance": "10000000",
      "Sequence": 1,
      "OwnerCount": 0,
      "Flags": 0
    },
    "ledger_index": 12345,
    "validated": true
  }
}
```

### account_lines

Returns trust lines for an account.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| account | string | Yes | Account address |
| peer | string | No | Filter by counterparty |
| ledger_index | string/number | No | Ledger to query |

**Request:**
```json
{
  "method": "account_lines",
  "params": [{
    "account": "cLSKzJZg4w2dgLfwf"
  }]
}
```

**Response:**
```json
{
  "result": {
    "account": "cLSKzJZg4w2dgLfwf",
    "lines": [
      {
        "account": "cG6vVq8oTo1R3mYRg",
        "balance": "1000",
        "currency": "USD",
        "limit": "10000",
        "limit_peer": "0",
        "quality_in": 0,
        "quality_out": 0
      }
    ]
  }
}
```

### account_tx

Returns transaction history for an account.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| account | string | Yes | Account address |
| ledger_index_min | number | No | Start ledger |
| ledger_index_max | number | No | End ledger |
| limit | number | No | Max transactions (default: 10) |
| forward | boolean | No | Sort order |

### account_objects

Returns ledger objects owned by an account.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| account | string | Yes | Account address |
| type | string | No | Filter by object type |
| ledger_index | string/number | No | Ledger to query |
| limit | number | No | Max objects |

**Object Types:**
- `check` - Check objects (not implemented)
- `escrow` - Escrow objects (excluded)
- `offer` - Offer objects
- `payment_channel` - Payment channels (excluded)
- `signer_list` - Signer lists
- `state` - Trust lines

### account_offers

Returns open offers for an account.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| account | string | Yes | Account address |
| ledger_index | string/number | No | Ledger to query |

---

## Transaction Methods

### submit

Submits a signed transaction to the network.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tx_blob | string | Yes | Signed transaction blob (hex) |
| fail_hard | boolean | No | Fail on validation error |

**Request:**
```json
{
  "method": "submit",
  "params": [{
    "tx_blob": "1200002280000000240000000161400000000000000068400000000000000A7321..."
  }]
}
```

**Response:**
```json
{
  "result": {
    "engine_result": "tesSUCCESS",
    "engine_result_code": 0,
    "engine_result_message": "The transaction was applied.",
    "tx_blob": "120000...",
    "tx_json": {
      "Account": "cLSKzJZg4w2dgLfwf",
      "TransactionType": "Payment",
      "Amount": "1000000",
      "Destination": "cN5E7s8x9y2z3w4v5u6t",
      "Fee": "10",
      "Sequence": 1
    }
  }
}
```

### submit_multisigned

Submits a multi-signed transaction.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tx_json | object | Yes | Transaction with Signers array |
| fail_hard | boolean | No | Fail on validation error |

### tx

Retrieves information about a specific transaction.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| transaction | string | Yes | Transaction hash |
| binary | boolean | No | Return binary format |

**Request:**
```json
{
  "method": "tx",
  "params": [{
    "transaction": "E08D6E9754025BA2534A787A0DA63A3A8F08E3A98D2E"
  }]
}
```

**Response:**
```json
{
  "result": {
    "Account": "cLSKzJZg4w2dgLfwf",
    "Amount": "1000000",
    "Destination": "cN5E7s8x9y2z3w4v5u6t",
    "Fee": "10",
    "Sequence": 1,
    "SigningPubKey": "0330E7FC9D56BB25D6893BA3F317AE5BCF33B3291BD63DB32654A313222F7FD020",
    "TransactionType": "Payment",
    "TxnSignature": "3045022100...",
    "date": 680000001,
    "hash": "E08D6E9754025BA2534A787A0DA63A3A8F08E3A98D2E",
    "inLedger": 12345,
    "ledger_index": 12345,
    "meta": {
      "TransactionIndex": 0,
      "TransactionResult": "tesSUCCESS",
      "AffectedNodes": [...]
    },
    "validated": true
  }
}
```

### transaction_entry

Returns transaction metadata from a specific ledger.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tx_hash | string | Yes | Transaction hash |
| ledger_index | number | Yes | Ledger sequence |

### tx_history

Returns recent transaction history.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| start | number | No | Start index (default: 0) |

---

## Ledger Methods

### ledger

Retrieves information about a specific ledger.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| ledger_index | string/number | No | "current", "closed", "validated", or number |
| ledger_hash | string | No | Ledger hash |
| full | boolean | No | Include full ledger |
| accounts | boolean | No | Include account state |
| transactions | boolean | No | Include transactions |
| expand | boolean | No | Expand transaction JSON |
| binary | boolean | No | Return in binary format |

**Request:**
```json
{
  "method": "ledger",
  "params": [{
    "ledger_index": "validated",
    "transactions": true,
    "expand": true
  }]
}
```

**Response:**
```json
{
  "result": {
    "ledger": {
      "accepted": true,
      "account_hash": "B4A5E8D1C2F3A6B9...",
      "close_flags": 0,
      "close_time": 680000100,
      "close_time_resolution": 10,
      "closed": true,
      "hash": "E08D6E9754025BA2...",
      "ledger_index": "12345",
      "parent_close_time": 680000095,
      "parent_hash": "A1B2C3D4E5F6A7B8...",
      "seqNum": "12345",
      "totalCoins": "100000000000000000",
      "transaction_hash": "C3D4E5F6A7B8C9D0...",
      "transactions": [
        {
          "Account": "cLSKzJZg4w2dgLfwf",
          "Amount": "1000000",
          "Destination": "cN5E7s8x9y2z3w4v5u6t",
          "Fee": "10",
          "Sequence": 1,
          "TransactionType": "Payment",
          "hash": "E08D6E9754025BA2..."
        }
      ]
    },
    "ledger_hash": "E08D6E9754025BA2...",
    "ledger_index": 12345,
    "validated": true
  }
}
```

### ledger_current

Returns the current in-progress ledger.

**Request:**
```json
{
  "method": "ledger_current"
}
```

### ledger_closed

Returns the most recently closed ledger.

**Request:**
```json
{
  "method": "ledger_closed"
}
```

### ledger_entry

Returns a specific ledger entry.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| index | string | No | Ledger entry index |
| account_root | string | No | Account ID |
| directory | string | No | Directory node |
| offer | object | No | Offer index |
| ripple_state | object | No | Trust line |
| ledger_index | string/number | No | Ledger to query |

### ledger_data

Returns raw ledger data for debugging.

---

## DEX Methods

### book_offers

Returns offers from the order book.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| taker_pays | object | Yes | Asset to pay |
| taker_gets | object | Yes | Asset to receive |
| ledger_index | string/number | No | Ledger to query |
| limit | number | No | Max offers (default: 100) |
| taker | string | No | Taker account |

**Request:**
```json
{
  "method": "book_offers",
  "params": [{
    "taker_pays": {
      "currency": "CALL"
    },
    "taker_gets": {
      "currency": "USD",
      "issuer": "cG6vVq8oTo1R3mYRg"
    },
    "limit": 10
  }]
}
```

**Response:**
```json
{
  "result": {
    "ledger_index": 12345,
    "offers": [
      {
        "Account": "cSellerAccount1234",
        "BookDirectory": "C73C9D8E3B3...",
        "BookNode": "0000000000000000",
        "Flags": 0,
        "LedgerEntryType": "Offer",
        "OwnerNode": "0000000000000000",
        "Sequence": 1,
        "TakerGets": {
          "currency": "USD",
          "issuer": "cG6vVq8oTo1R3mYRg",
          "value": "100"
        },
        "TakerPays": "100000000",
        "index": "A984EFA...",
        "owner_funds": "50000000",
        "quality": "1000000"
      }
    ],
    "validated": true
  }
}
```

### path_find

Finds paths for a payment.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| subcommand | string | Yes | "create", "close", or "status" |
| source_account | string | Yes (create) | Source account |
| destination_account | string | Yes (create) | Destination account |
| destination_amount | object | Yes (create) | Amount to deliver |
| send_max | object | No | Maximum to send |
| paths | array | No | Known paths |

**Create Request:**
```json
{
  "method": "path_find",
  "params": [{
    "subcommand": "create",
    "source_account": "cLSKzJZg4w2dgLfwf",
    "destination_account": "cN5E7s8x9y2z3w4v5u6t",
    "destination_amount": {
      "currency": "USD",
      "issuer": "cG6vVq8oTo1R3mYRg",
      "value": "100"
    }
  }]
}
```

### call_path_find

Callchain-specific enhanced path finding.

### ripple_path_find

Legacy path finding (compatibility).

---

## Server Methods

### server_info

Returns server status and information.

**Request:**
```json
{
  "method": "server_info"
}
```

**Response:**
```json
{
  "result": {
    "info": {
      "build_version": "1.0.0",
      "complete_ledgers": "1-12345",
      "hostid": "node1",
      "io_latency_ms": 1,
      "last_close": {
        "converge_time_s": 2.0,
        "proposers": 5
      },
      "load_factor": 1,
      "peers": 10,
      "pubkey_node": "n9JZF7Q5K...",
      "server_state": "full",
      "server_state_duration_us": 3600000000,
      "time": "2024-Jan-01 12:00:00",
      "uptime": 86400,
      "validated_ledger": {
        "age": 4,
        "base_fee": 10,
        "hash": "E08D6E9754025BA2...",
        "reserve_base": 10000000,
        "reserve_inc": 2000000,
        "seq": 12345
      },
      "validation_quorum": 4
    }
  }
}
```

### server_state

Returns detailed server state.

### ping

Simple connectivity test.

**Request:**
```json
{
  "method": "ping"
}
```

**Response:**
```json
{
  "result": {}
}
```

---

## Signing Methods

### sign

Signs a transaction locally (requires trusted connection).

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tx_json | object | Yes | Transaction to sign |
| secret | string | Yes | Seed or private key |
| offline | boolean | No | Skip validation |
| fee_mult_max | number | No | Max fee multiplier |

**Request:**
```json
{
  "method": "sign",
  "params": [{
    "tx_json": {
      "TransactionType": "Payment",
      "Account": "cLSKzJZg4w2dgLfwf",
      "Destination": "cN5E7s8x9y2z3w4v5u6t",
      "Amount": "1000000"
    },
    "secret": "sn3nxiW7v8KXzPzA..."
  }]
}
```

### sign_for

Multi-signs a transaction for another account.

---

## Wallet Methods

### wallet_propose

Generates a new wallet.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| passphrase | string | No | Optional passphrase |
| seed | string | No | Use specific seed |
| key_type | string | No | "secp256k1" or "ed25519" |

**Request:**
```json
{
  "method": "wallet_propose"
}
```

**Response:**
```json
{
  "result": {
    "account_id": "cLSKzJZg4w2dgLfwf",
    "account_sequence": 0,
    "key_type": "secp256k1",
    "master_key": "A B C D E F G H I J K L",
    "master_seed": "sn3nxiW7v8KXzPzA...",
    "public_key": "n9JZF7Q5K...",
    "public_key_hex": "0330E7FC9D56BB25...",
    "status": "success"
  }
}
```

### validation_create

Creates validator keys.

### validation_seed

Derives validation keys from seed.

---

## Consensus/Network Methods

### consensus_info

Returns consensus state information.

### fee

Returns current fee information.

**Response:**
```json
{
  "result": {
    "current_ledger_size": "14",
    "current_queue_size": "0",
    "drops": {
      "base_fee": "10",
      "median_fee": "10",
      "minimum_fee": "10",
      "open_ledger_fee": "10"
    },
    "expected_ledger_size": "55",
    "ledger_current_index": 12345,
    "levels": {
      "median_level": "256000",
      "minimum_level": "256",
      "open_ledger_level": "256",
      "reference_level": "256"
    },
    "max_queue_size": "2000"
  }
}
```

### peers

Returns connected peer information.

### validators

Returns known validator information.

### unl_list

Returns Unique Node List.

### connect

Connects to a specific peer.

---

## Admin Methods

### stop

Shuts down the server (admin only).

### ledger_accept

Forces ledger close (admin/testing only).

### log_level

Gets or sets log levels.

### logrotate

Rotates log files.

### get_counts

Returns object counts for debugging.

### fetch_info

Returns fetch pack information.

### can_delete

Sets ledger deletion range.

### blacklist

Manages peer blacklist.

---

## Error Codes

### Transaction Engine Errors (tem*)

| Code | Description |
|------|-------------|
| temMALFORMED | Transaction format invalid |
| temBAD_LEDGER | Ledger sequence invalid |
| temBAD_SIGNATURE | Signature invalid |
| temBAD_SEQUENCE | Sequence number invalid |
| temBAD_SENDER | Sender account invalid |
| temBAD_AUTH | Authorization invalid |
| temINVALID_FLAG | Transaction flag invalid |
| temREDUNDANT | Redundant transaction |
| temDISABLED | Feature disabled |

### Transaction Errors (tec*)

| Code | Description |
|------|-------------|
| tecCLAIM | Already claimed |
| tecPATH_PARTIAL | Path could not send full amount |
| tecNO_DST | Destination does not exist |
| tecNO_DST_INSUF_CALL | Destination lacks CALL for reserve |
| tecNO_LINE | No trust line |
| tecINSUF_RESERVE_LINE | Insufficient reserve for line |
| tecINSUF_RESERVE_OFFER | Insufficient reserve for offer |
| tecNO_PERMISSION | Permission denied |
| tecNO_ENTRY | Entry not found |

### Server Errors

| Code | Description |
|------|-------------|
| rpcSUCCESS | Success (0) |
| rpcUNKNOWN | Unknown error (1) |
| rpcNOT_IMPL | Not implemented (2) |
| rpcNO_PERMISSION | Permission denied (4) |
| rpcNOT_STANDALONE | Operation not valid while not synced (5) |
| rpcNO_EVENTS | No events (6) |
| rpcSENDMAX_MALFORMED | Send max malformed (7) |
| rpcLOAD_FAILED | Load failed (8) |

---

## Common Transaction Types

### Payment

```json
{
  "TransactionType": "Payment",
  "Account": "cLSKzJZg4w2dgLfwf",
  "Destination": "cN5E7s8x9y2z3w4v5u6t",
  "Amount": "1000000",
  "Fee": "10",
  "Sequence": 1,
  "DestinationTag": 12345
}
```

### TrustSet

```json
{
  "TransactionType": "TrustSet",
  "Account": "cLSKzJZg4w2dgLfwf",
  "LimitAmount": {
    "currency": "USD",
    "issuer": "cG6vVq8oTo1R3mYRg",
    "value": "10000"
  },
  "Fee": "10",
  "Sequence": 1
}
```

### OfferCreate

```json
{
  "TransactionType": "OfferCreate",
  "Account": "cLSKzJZg4w2dgLfwf",
  "TakerGets": {
    "currency": "USD",
    "issuer": "cG6vVq8oTo1R3mYRg",
    "value": "100"
  },
  "TakerPays": "100000000",
  "Fee": "10",
  "Sequence": 1
}
```

### AccountSet

```json
{
  "TransactionType": "AccountSet",
  "Account": "cLSKzJZg4w2dgLfwf",
  "SetFlag": 8,
  "Domain": "6578616D706C652E636F6D",
  "Fee": "10",
  "Sequence": 1
}
```

---

## Rate Limits

| Endpoint | Rate Limit |
|----------|------------|
| Public RPC | 100 requests/minute |
| Admin RPC | 1000 requests/minute |
| WebSocket | 1000 messages/minute |

## See Also

- [WebSocket API](websocket.md) - Real-time subscriptions
- [Transaction Types](../transactions/types.md) - All transaction types
- [Error Codes](errors.md) - Detailed error documentation
- [JavaScript Client](https://github.com/callchain/calljs) - Official JS client
