# WebSocket API Reference

The Call-Core WebSocket API provides real-time access to the Callchain network. It supports subscriptions for ledgers, transactions, validations, and more, enabling efficient event-driven applications.

## Endpoint

```
ws://localhost:6006/
```

## Connection

Connect using any WebSocket client:

```javascript
const ws = new WebSocket('ws://localhost:6006/');

ws.onopen = () => {
  console.log('Connected to Call-Core');
};

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Received:', data);
};

ws.onerror = (error) => {
  console.error('WebSocket error:', error);
};

ws.onclose = () => {
  console.log('Disconnected');
};
```

## Message Format

All messages are JSON-encoded.

### Request Format

```json
{
  "id": "unique-request-id",
  "command": "command_name",
  "param1": "value1",
  "param2": "value2"
}
```

### Response Format

```json
{
  "id": "unique-request-id",
  "result": {},
  "status": "success"
}
```

### Error Format

```json
{
  "id": "unique-request-id",
  "error": "error_code",
  "error_message": "Human readable description",
  "status": "error"
}
```

---

## Commands

### ping

Tests connectivity.

**Request:**
```json
{
  "id": "1",
  "command": "ping"
}
```

**Response:**
```json
{
  "id": "1",
  "result": {},
  "status": "success"
}
```

---

### subscribe

Subscribe to streams of data.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| streams | array | No | Stream types to subscribe |
| accounts | array | No | Account addresses to watch |
| accounts_proposed | array | No | Watch proposed transactions |
| books | array | No | Order books to watch |

**Stream Types:**
- `ledger` - New ledgers
- `transactions` - Confirmed transactions
- `transactions_proposed` - Proposed transactions
- `validations` - Validator validations
- `manifests` - Validator manifests
- `peer_status` - Peer status changes
- `server` - Server status changes
- `book_changes` - Order book changes

**Request - Subscribe to Ledger Stream:**
```json
{
  "id": "2",
  "command": "subscribe",
  "streams": ["ledger"]
}
```

**Request - Subscribe to Account Transactions:**
```json
{
  "id": "3",
  "command": "subscribe",
  "accounts": ["cLSKzJZg4w2dgLfwf"]
}
```

**Request - Subscribe to Order Book:**
```json
{
  "id": "4",
  "command": "subscribe",
  "books": [
    {
      "taker_pays": {
        "currency": "CALL"
      },
      "taker_gets": {
        "currency": "USD",
        "issuer": "cG6vVq8oTo1R3mYRg"
      }
    }
  ]
}
```

**Response:**
```json
{
  "id": "2",
  "result": {
    "fee_base": 10,
    "fee_ref": 10,
    "ledger_hash": "E08D6E9754025BA2534A787A0DA63A3A8F08E3A98D2E",
    "ledger_index": 12345,
    "ledger_time": 680000100,
    "reserve_base": 10000000,
    "reserve_inc": 2000000,
    "validated_ledgers": "1-12345"
  },
  "status": "success"
}
```

---

### unsubscribe

Unsubscribe from streams.

**Request:**
```json
{
  "id": "5",
  "command": "unsubscribe",
  "streams": ["ledger"]
}
```

Or unsubscribe from all:

```json
{
  "id": "6",
  "command": "unsubscribe"
}
```

**Response:**
```json
{
  "id": "5",
  "result": {},
  "status": "success"
}
```

---

### account_info

Get account information (WebSocket version).

**Request:**
```json
{
  "id": "7",
  "command": "account_info",
  "account": "cLSKzJZg4w2dgLfwf"
}
```

**Response:**
```json
{
  "id": "7",
  "result": {
    "account_data": {
      "Account": "cLSKzJZg4w2dgLfwf",
      "Balance": "10000000",
      "Flags": 0,
      "LedgerEntryType": "AccountRoot",
      "OwnerCount": 0,
      "Sequence": 1,
      "index": "B4A5E8D1C2F3A6B9..."
    },
    "ledger_current_index": 12346,
    "validated": false
  },
  "status": "success"
}
```

---

### account_lines

Get trust lines.

**Request:**
```json
{
  "id": "8",
  "command": "account_lines",
  "account": "cLSKzJZg4w2dgLfwf"
}
```

---

### account_tx

Get account transaction history.

**Request:**
```json
{
  "id": "9",
  "command": "account_tx",
  "account": "cLSKzJZg4w2dgLfwf",
  "limit": 10
}
```

---

### submit

Submit a signed transaction.

**Request:**
```json
{
  "id": "10",
  "command": "submit",
  "tx_blob": "1200002280000000240000000161400000000000000068400000000000000A7321..."
}
```

**Response:**
```json
{
  "id": "10",
  "result": {
    "engine_result": "tesSUCCESS",
    "engine_result_code": 0,
    "engine_result_message": "The transaction was applied.",
    "tx_blob": "120000...",
    "tx_json": {
      "Account": "cLSKzJZg4w2dgLfwf",
      "Amount": "1000000",
      "Destination": "cN5E7s8x9y2z3w4v5u6t",
      "Fee": "10",
      "Sequence": 1,
      "TransactionType": "Payment"
    }
  },
  "status": "success"
}
```

---

### ledger

Get ledger information.

**Request:**
```json
{
  "id": "11",
  "command": "ledger",
  "ledger_index": "validated"
}
```

---

### book_offers

Get order book offers.

**Request:**
```json
{
  "id": "12",
  "command": "book_offers",
  "taker_pays": {
    "currency": "CALL"
  },
  "taker_gets": {
    "currency": "USD",
    "issuer": "cG6vVq8oTo1R3mYRg"
  }
}
```

---

### path_find

Find payment paths.

**Create Path Find:**
```json
{
  "id": "13",
  "command": "path_find",
  "subcommand": "create",
  "source_account": "cLSKzJZg4w2dgLfwf",
  "destination_account": "cN5E7s8x9y2z3w4v5u6t",
  "destination_amount": {
    "currency": "USD",
    "issuer": "cG6vVq8oTo1R3mYRg",
    "value": "100"
  }
}
```

**Close Path Find:**
```json
{
  "id": "14",
  "command": "path_find",
  "subcommand": "close"
}
```

**Status Check:**
```json
{
  "id": "15",
  "command": "path_find",
  "subcommand": "status"
}
```

---

### server_info

Get server information.

**Request:**
```json
{
  "id": "16",
  "command": "server_info"
}
```

---

### fee

Get current fee information.

**Request:**
```json
{
  "id": "17",
  "command": "fee"
}
```

---

## Stream Messages

Once subscribed, the server pushes messages without request IDs.

### Ledger Stream

Sent when a new ledger is validated.

```json
{
  "type": "ledgerClosed",
  "fee_base": 10,
  "fee_ref": 10,
  "ledger_hash": "E08D6E9754025BA2534A787A0DA63A3A8F08E3A98D2E",
  "ledger_index": "12346",
  "ledger_time": 680000105,
  "reserve_base": 10000000,
  "reserve_inc": 2000000,
  "txn_count": 25,
  "validated_ledgers": "1-12346"
}
```

### Transaction Stream

Sent when a subscribed account has a confirmed transaction.

```json
{
  "type": "transaction",
  "engine_result": "tesSUCCESS",
  "engine_result_code": 0,
  "engine_result_message": "The transaction was applied.",
  "ledger_hash": "E08D6E9754025BA2534A787A0DA63A3A8F08E3A98D2E",
  "ledger_index": 12346,
  "meta": {
    "AffectedNodes": [
      {
        "ModifiedNode": {
          "FinalFields": {
            "Account": "cLSKzJZg4w2dgLfwf",
            "Balance": "9000000",
            "Sequence": 2
          },
          "LedgerEntryType": "AccountRoot",
          "PreviousFields": {
            "Balance": "10000000",
            "Sequence": 1
          }
        }
      }
    ],
    "TransactionIndex": 0,
    "TransactionResult": "tesSUCCESS"
  },
  "status": "closed",
  "transaction": {
    "Account": "cLSKzJZg4w2dgLfwf",
    "Amount": "1000000",
    "Destination": "cN5E7s8x9y2z3w4v5u6t",
    "Fee": "10",
    "Sequence": 1,
    "SigningPubKey": "0330E7FC9D56BB25D6893BA3F317AE5BCF33B3291BD63DB32654A313222F7FD020",
    "TransactionType": "Payment",
    "TxnSignature": "3045022100...",
    "date": 680000105,
    "hash": "E08D6E9754025BA2534A787A0DA63A3A8F08E3A98D2E"
  },
  "validated": true
}
```

### Transaction Proposed Stream

Sent when a transaction is proposed (before validation).

```json
{
  "type": "transaction",
  "status": "proposed",
  "transaction": {
    "Account": "cLSKzJZg4w2dgLfwf",
    "Amount": "1000000",
    "Destination": "cN5E7s8x9y2z3w4v5u6t",
    "Fee": "10",
    "Sequence": 1,
    "TransactionType": "Payment",
    "hash": "E08D6E9754025BA2534A787A0DA63A3A8F08E3A98D2E"
  }
}
```

### Validation Stream

Sent when a validator publishes a validation.

```json
{
  "type": "validationReceived",
  "validation_public_key": "n9JZF7Q5K7U7ZQZ...",
  "ledger_hash": "E08D6E9754025BA2534A787A0DA63A3A8F08E3A98D2E",
  "ledger_index": "12346",
  "signature": "3045022100...",
  "full": true,
  "flags": 0,
  "signing_time": 680000105,
  "data": "..."
}
```

### Peer Status Stream

Sent when peer status changes.

```json
{
  "type": "peerStatusChange",
  "action": "CONNECT",
  "date": 680000105,
  "ledger_hash": "E08D6E9754025BA2534A787A0DA63A3A8F08E3A98D2E",
  "ledger_index": "12345",
  "ledger_index_max": 12345
}
```

### Server Stream

Sent when server status changes.

```json
{
  "type": "serverStatus",
  "server_status": "full",
  "load_base": 256,
  "load_factor": 256
}
```

### Book Changes Stream

Sent when subscribed order book changes.

```json
{
  "type": "bookChanges",
  "ledger_index": 12346,
  "ledger_hash": "E08D6E9754025BA2534A787A0DA63A3A8F08E3A98D2E",
  "validated": true,
  "changes": [
    {
      "side": "buy",
      "type": "offerCreated",
      "account": "cSellerAccount1234",
      "taker_gets": {
        "currency": "USD",
        "issuer": "cG6vVq8oTo1R3mYRg",
        "value": "100"
      },
      "taker_pays": "100000000",
      "sequence": 1
    }
  ]
}
```

---

## Complete Examples

### Example 1: Monitor Account Transactions

```javascript
const ws = new WebSocket('ws://localhost:6006/');

ws.onopen = () => {
  // Subscribe to account transactions
  ws.send(JSON.stringify({
    id: 1,
    command: 'subscribe',
    accounts: ['cLSKzJZg4w2dgLfwf']
  }));
};

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);

  if (data.type === 'transaction' && data.validated) {
    console.log('New transaction for account:');
    console.log('  Hash:', data.transaction.hash);
    console.log('  Type:', data.transaction.TransactionType);
    console.log('  Result:', data.engine_result);
    console.log('  Ledger:', data.ledger_index);
  }
};
```

### Example 2: Monitor New Ledgers

```javascript
const ws = new WebSocket('ws://localhost:6006/');

ws.onopen = () => {
  ws.send(JSON.stringify({
    id: 1,
    command: 'subscribe',
    streams: ['ledger']
  }));
};

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);

  if (data.type === 'ledgerClosed') {
    console.log('New ledger validated:');
    console.log('  Index:', data.ledger_index);
    console.log('  Hash:', data.ledger_hash);
    console.log('  Transactions:', data.txn_count);
    console.log('  Time:', new Date(data.ledger_time * 1000));
  }
};
```

### Example 3: Monitor Order Book

```javascript
const ws = new WebSocket('ws://localhost:6006/');

ws.onopen = () => {
  ws.send(JSON.stringify({
    id: 1,
    command: 'subscribe',
    books: [
      {
        taker_pays: { currency: 'CALL' },
        taker_gets: {
          currency: 'USD',
          issuer: 'cG6vVq8oTo1R3mYRg'
        }
      }
    ]
  }));
};

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);

  if (data.type === 'bookChanges') {
    console.log('Order book updated:');
    data.changes.forEach(change => {
      console.log('  Side:', change.side);
      console.log('  Type:', change.type);
      console.log('  Account:', change.account);
      console.log('  Price:', change.taker_pays / change.taker_gets.value);
    });
  }
};
```

### Example 4: Submit and Monitor Transaction

```javascript
const ws = new WebSocket('ws://localhost:6006/');

ws.onopen = () => {
  // First subscribe to our account
  ws.send(JSON.stringify({
    id: 1,
    command: 'subscribe',
    accounts: ['cLSKzJZg4w2dgLfwf']
  }));

  // Then submit a transaction (already signed)
  ws.send(JSON.stringify({
    id: 2,
    command: 'submit',
    tx_blob: '12000022800000002400000001...' // Signed transaction blob
  }));
};

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);

  if (data.id === 2) {
    // Submission response
    if (data.status === 'success') {
      console.log('Transaction submitted:', data.result.engine_result);
      console.log('Hash:', data.result.tx_json.hash);
    } else {
      console.error('Submission failed:', data.error);
    }
  }

  if (data.type === 'transaction' && data.validated) {
    console.log('Transaction validated!');
    console.log('Result:', data.engine_result);
    console.log('Ledger:', data.ledger_index);
  }
};
```

---

## Error Handling

### Connection Errors

```javascript
ws.onerror = (error) => {
  console.error('WebSocket error:', error);
};

ws.onclose = (event) => {
  console.log('Connection closed:', event.code, event.reason);

  // Reconnect logic
  if (event.code !== 1000) {
    setTimeout(() => {
      console.log('Reconnecting...');
      connect();
    }, 5000);
  }
};
```

### Request Errors

```json
{
  "id": "1",
  "error": "actNotFound",
  "error_message": "Account not found.",
  "status": "error"
}
```

Common errors:
- `actNotFound` - Account doesn't exist
- `lgrNotFound` - Ledger not found
- `txnNotFound` - Transaction not found
- `invalidParams` - Invalid parameters
- `noPermission` - Permission denied
- `tooBusy` - Server too busy

---

## Best Practices

### 1. Connection Management

```javascript
class CallWebSocket {
  constructor(url) {
    this.url = url;
    this.ws = null;
    this.reconnectInterval = 5000;
    this.subscriptions = [];
  }

  connect() {
    this.ws = new WebSocket(this.url);

    this.ws.onopen = () => {
      console.log('Connected');
      this.resubscribe();
    };

    this.ws.onclose = () => {
      console.log('Disconnected, reconnecting...');
      setTimeout(() => this.connect(), this.reconnectInterval);
    };
  }

  resubscribe() {
    this.subscriptions.forEach(sub => {
      this.send(sub);
    });
  }

  send(message) {
    if (this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(message));
    }
  }
}
```

### 2. Message Deduplication

```javascript
const processedTransactions = new Set();

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);

  if (data.type === 'transaction') {
    const hash = data.transaction.hash;

    if (processedTransactions.has(hash)) {
      return; // Skip duplicate
    }

    processedTransactions.add(hash);
    processTransaction(data);
  }
};
```

### 3. Heartbeat

```javascript
// Send ping every 30 seconds
setInterval(() => {
  ws.send(JSON.stringify({
    id: 'ping',
    command: 'ping'
  }));
}, 30000);
```

---

## Rate Limits

| Operation | Limit |
|-----------|-------|
| Connection attempts | 10/minute |
| Messages per connection | 1000/minute |
| Subscriptions per connection | 100 |
| Concurrent path finds | 10 |

---

## Comparison with RPC API

| Feature | RPC | WebSocket |
|---------|-----|-----------|
| Real-time updates | ❌ Polling | ✅ Push |
| Connection overhead | Per request | Single connection |
| Subscription support | ❌ No | ✅ Yes |
| Bi-directional | ❌ Request/response | ✅ Full duplex |
| Best for | Queries, one-off | Monitoring, real-time |

---

## See Also

- [RPC API Reference](rpc.md) - HTTP API documentation
- [Architecture Overview](../architecture/overview.md) - System architecture
- [JavaScript Client](https://github.com/callchain/calljs) - Official WebSocket client
