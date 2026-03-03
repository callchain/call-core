# Transaction Types

Call-Core supports a comprehensive set of transaction types for managing accounts, transfers, trading, and account settings.

## Overview

| Transaction Type | Description | Purpose |
|-----------------|-------------|---------|
| `Payment` | Transfer CALL or issued assets | Basic value transfer |
| `TrustSet` | Create or modify trust lines | Enable asset holding |
| `OfferCreate` | Create a DEX limit order | Trade assets |
| `OfferCancel` | Cancel an existing offer | Remove order from book |
| `AccountSet` | Modify account settings | Configure account |
| `SetRegularKey` | Set secondary signing key | Key management |
| `SignerListSet` | Configure multi-signature | Multi-sign setup |
| `DepositPreauth` | Pre-authorize depositors | Deposit control |
| `NicknameSet` | Set account nickname | Human-readable names |

---

## Payment

Transfers CALL tokens or issued assets between accounts.

### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `Account` | string | Yes | Sender address |
| `Destination` | string | Yes | Recipient address |
| `Amount` | Amount | Yes | Amount to send (CALL or issued asset) |
| `DestinationTag` | uint32 | No | Recipient tag (for exchanges) |
| `InvoiceID` | string | No | 256-bit hash for invoice tracking |
| `Paths` | array | No | Payment paths for cross-currency |
| `SendMax` | Amount | No | Maximum amount to send (path payments) |
| `DeliverMin` | Amount | No | Minimum to deliver (partial payments) |

### Example: Simple CALL Payment

```json
{
  "TransactionType": "Payment",
  "Account": "cLSKzJZg4w2dgLfwf",
  "Destination": "cN5E7s8x9y2z3w4v5u6t",
  "Amount": "1000000",
  "Fee": "10",
  "Sequence": 1
}
```

### Example: Issued Asset Payment

```json
{
  "TransactionType": "Payment",
  "Account": "cLSKzJZg4w2dgLfwf",
  "Destination": "cN5E7s8x9y2z3w4v5u6t",
  "Amount": {
    "currency": "USD",
    "issuer": "cG6vVq8oTo1R3mYRg",
    "value": "100"
  },
  "Fee": "10",
  "Sequence": 1
}
```

### Example: Cross-Currency Payment with Paths

```json
{
  "TransactionType": "Payment",
  "Account": "cLSKzJZg4w2dgLfwf",
  "Destination": "cN5E7s8x9y2z3w4v5u6t",
  "Amount": {
    "currency": "EUR",
    "issuer": "cBank1234567890",
    "value": "50"
  },
  "SendMax": "100000000",
  "Paths": [
    [
      {
        "currency": "USD",
        "issuer": "cG6vVq8oTo1R3mYRg"
      },
      {
        "currency": "EUR",
        "issuer": "cBank1234567890"
      }
    ]
  ],
  "Fee": "10",
  "Sequence": 1
}
```

### Result Codes

| Code | Description |
|------|-------------|
| `tesSUCCESS` | Payment completed successfully |
| `tecPATH_PARTIAL` | Could not deliver full amount |
| `tecNO_DST` | Destination account doesn't exist |
| `tecNO_DST_INSUF_CALL` | Destination needs CALL for reserve |
| `tecPATH_DRY` | No path found between currencies |
| `tecINSUF_RESERVE_LINE` | Need reserve for new trust line |

---

## TrustSet

Creates or modifies a trust line for holding issued assets.

### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `Account` | string | Yes | Account address |
| `LimitAmount` | Amount | Yes | Trust limit with currency/issuer |
| `QualityIn` | uint32 | No | Quality for incoming payments (0-1 billion) |
| `QualityOut` | uint32 | No | Quality for outgoing payments (0-1 billion) |

### Example

```json
{
  "TransactionType": "TrustSet",
  "Account": "cLSKzJZg4w2dgLfwf",
  "LimitAmount": {
    "currency": "USD",
    "issuer": "cG6vVq8oTo1R3mYRg",
    "value": "10000"
  },
  "QualityIn": 1000000000,
  "QualityOut": 1000000000,
  "Fee": "10",
  "Sequence": 1
}
```

### Setting Trust Limit to Zero

To remove a trust line, set limit to 0 (only works if balance is 0):

```json
{
  "TransactionType": "TrustSet",
  "Account": "cLSKzJZg4w2dgLfwf",
  "LimitAmount": {
    "currency": "USD",
    "issuer": "cG6vVq8oTo1R3mYRg",
    "value": "0"
  },
  "Fee": "10",
  "Sequence": 2
}
```

### Quality Settings

- **QualityIn**: Discount when receiving payments (1000000000 = 1.0, no discount)
- **QualityOut**: Premium when sending payments (1000000000 = 1.0, no premium)
- Value range: 0 to 1000000000 (1 billion)

Example: QualityIn of 500000000 means you value incoming USD at 50%, so you'd receive 2 USD for every 1 USD worth of CALL.

---

## OfferCreate

Creates a limit order on the decentralized exchange (DEX).

### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `Account` | string | Yes | Account address |
| `TakerGets` | Amount | Yes | Asset to sell |
| `TakerPays` | Amount | Yes | Asset to receive |
| `Expiration` | uint32 | No | Offer expiration time (seconds since epoch) |
| `OfferSequence` | uint32 | No | Sequence of offer to cancel (rate update) |

### Example: Sell CALL for USD

```json
{
  "TransactionType": "OfferCreate",
  "Account": "cLSKzJZg4w2dgLfwf",
  "TakerGets": "10000000",
  "TakerPays": {
    "currency": "USD",
    "issuer": "cG6vVq8oTo1R3mYRg",
    "value": "100"
  },
  "Fee": "10",
  "Sequence": 1
}
```

### Example: Sell USD for CALL

```json
{
  "TransactionType": "OfferCreate",
  "Account": "cLSKzJZg4w2dgLfwf",
  "TakerGets": {
    "currency": "USD",
    "issuer": "cG6vVq8oTo1R3mYRg",
    "value": "100"
  },
  "TakerPays": "10000000",
  "Fee": "10",
  "Sequence": 1
}
```

### Example: Trade Between Issued Assets

```json
{
  "TransactionType": "OfferCreate",
  "Account": "cLSKzJZg4w2dgLfwf",
  "TakerGets": {
    "currency": "USD",
    "issuer": "cG6vVq8oTo1R3mYRg",
    "value": "100"
  },
  "TakerPays": {
    "currency": "EUR",
    "issuer": "cBank1234567890",
    "value": "85"
  },
  "Expiration": 680000000,
  "Fee": "10",
  "Sequence": 1
}
```

### Rate Update Pattern

To update an offer's rate, cancel the old and create new:

```json
{
  "TransactionType": "OfferCreate",
  "Account": "cLSKzJZg4w2dgLfwf",
  "TakerGets": "10000000",
  "TakerPays": {
    "currency": "USD",
    "issuer": "cG6vVq8oTo1R3mYRg",
    "value": "110"
  },
  "OfferSequence": 5,
  "Fee": "10",
  "Sequence": 10
}
```

### Result Codes

| Code | Description |
|------|-------------|
| `tesSUCCESS` | Offer created and/or partially/fully filled |
| `tecUNFUNDED_OFFER` | Insufficient balance for offer |
| `tecINSUF_RESERVE_OFFER` | Need reserve for offer object |
| `tesSUCCESS` (code `tecKILLED`) | Offer would cross itself, not created |

---

## OfferCancel

Cancels an existing offer.

### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `Account` | string | Yes | Account address |
| `OfferSequence` | uint32 | Yes | Sequence number of offer to cancel |

### Example

```json
{
  "TransactionType": "OfferCancel",
  "Account": "cLSKzJZg4w2dgLfwf",
  "OfferSequence": 5,
  "Fee": "10",
  "Sequence": 10
}
```

### Notes

- Funds locked in the offer are returned to the owner
- Canceling a non-existent or already-closed offer succeeds (no-op)
- Partially filled offers return remaining funds

---

## AccountSet

Modifies account settings and metadata.

### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `Account` | string | Yes | Account address |
| `Domain` | string | No | Domain name (hex-encoded) |
| `EmailHash` | string | No | MD5 hash of email (for Gravatar) |
| `MessageKey` | string | No | Public key for encrypted messages |
| `SetFlag` | uint32 | No | Account flag to enable |
| `ClearFlag` | uint32 | No | Account flag to disable |

### Account Flags

| Flag | Value | Name | Description |
|------|-------|------|-------------|
| 1 | `lsfRequireDestTag` | Require Destination Tag | Require DestinationTag on incoming payments |
| 2 | `lsfRequireAuth` | Require Authorization | Require trust line authorization |
| 3 | `lsfDisallowCALL` | Disallow CALL | Reject incoming CALL payments |
| 4 | `lsfDisableMaster` | Disable Master Key | Prevent master key from signing |
| 5 | `lsfNoFreeze` | No Freeze | Permanently disable freezing trust lines |
| 6 | `lsfGlobalFreeze` | Global Freeze | Freeze all issued assets |
| 7 | `lsfDefaultCall` | Default CALL | Treat empty amount as CALL |
| 8 | `lsfDepositAuth` | Deposit Authorization | Require pre-authorization for deposits |

### Example: Set Domain

```json
{
  "TransactionType": "AccountSet",
  "Account": "cLSKzJZg4w2dgLfwf",
  "Domain": "6578616D706C652E636F6D",
  "Fee": "10",
  "Sequence": 1
}
```

Domain is hex-encoded ASCII: `6578616D706C652E636F6D` = "example.com"

### Example: Enable Deposit Authorization

```json
{
  "TransactionType": "AccountSet",
  "Account": "cLSKzJZg4w2dgLfwf",
  "SetFlag": 8,
  "Fee": "10",
  "Sequence": 1
}
```

### Example: Disable Master Key

**Warning**: Only disable master key if you have a regular key or multi-sign configured, or you will lose access to the account.

```json
{
  "TransactionType": "AccountSet",
  "Account": "cLSKzJZg4w2dgLfwf",
  "SetFlag": 4,
  "Fee": "10",
  "Sequence": 1
}
```

### Example: Require Destination Tag

```json
{
  "TransactionType": "AccountSet",
  "Account": "cLSKzJZg4w2dgLfwf",
  "SetFlag": 1,
  "Fee": "10",
  "Sequence": 1
}
```

### Clearing Flags

```json
{
  "TransactionType": "AccountSet",
  "Account": "cLSKzJZg4w2dgLfwf",
  "ClearFlag": 1,
  "Fee": "10",
  "Sequence": 2
}
```

---

## SetRegularKey

Sets or removes a regular key pair for an account.

### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `Account` | string | Yes | Account address |
| `RegularKey` | string | No | Public key for regular key (omit to remove) |

### Example: Set Regular Key

```json
{
  "TransactionType": "SetRegularKey",
  "Account": "cLSKzJZg4w2dgLfwf",
  "RegularKey": "cN5E7s8x9y2z3w4v5u6t",
  "Fee": "10",
  "Sequence": 1
}
```

### Example: Remove Regular Key

```json
{
  "TransactionType": "SetRegularKey",
  "Account": "cLSKzJZg4w2dgLfwf",
  "Fee": "10",
  "Sequence": 2
}
```

### Regular Key Usage

The regular key can sign transactions in place of the master key:

1. Generate a new key pair (doesn't need to be funded)
2. Set it as the account's regular key
3. Use the regular key's secret to sign transactions
4. Keep the master key secret offline for security

---

## SignerListSet

Configures multi-signature for an account.

### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `Account` | string | Yes | Account address |
| `SignerQuorum` | uint32 | Yes | Required signature weight |
| `SignerEntries` | array | Yes | List of signers (max 32) |

### SignerEntry Structure

| Field | Type | Description |
|-------|------|-------------|
| `Account` | string | Signer's address |
| `SignerWeight` | uint32 | Weight for this signer's signature |

### Example: 2-of-3 Multi-Signature

```json
{
  "TransactionType": "SignerListSet",
  "Account": "cLSKzJZg4w2dgLfwf",
  "SignerQuorum": 2,
  "SignerEntries": [
    {
      "SignerEntry": {
        "Account": "cSigner1xxxxxxxxxxxxxxxx",
        "SignerWeight": 1
      }
    },
    {
      "SignerEntry": {
        "Account": "cSigner2xxxxxxxxxxxxxxxx",
        "SignerWeight": 1
      }
    },
    {
      "SignerEntry": {
        "Account": "cSigner3xxxxxxxxxxxxxxxx",
        "SignerWeight": 1
      }
    }
  ],
  "Fee": "10",
  "Sequence": 1
}
```

### Example: Weighted Multi-Signature

```json
{
  "TransactionType": "SignerListSet",
  "Account": "cLSKzJZg4w2dgLfwf",
  "SignerQuorum": 3,
  "SignerEntries": [
    {
      "SignerEntry": {
        "Account": "cCEOxxxxxxxxxxxxxxxxxxxx",
        "SignerWeight": 3
      }
    },
    {
      "SignerEntry": {
        "Account": "cCFOxxxxxxxxxxxxxxxxxxxx",
        "SignerWeight": 2
      }
    },
    {
      "SignerEntry": {
        "Account": "cCTOxxxxxxxxxxxxxxxxxxxx",
        "SignerWeight": 2
      }
    }
  ],
  "Fee": "10",
  "Sequence": 1
}
```

In this example, CEO can sign alone (weight 3), or any two officers together.

### Example: Remove Signer List

```json
{
  "TransactionType": "SignerListSet",
  "Account": "cLSKzJZg4w2dgLfwf",
  "SignerQuorum": 0,
  "Fee": "10",
  "Sequence": 2
}
```

### Signing with Multi-Signature

Multi-signed transactions use the `Signers` field:

```json
{
  "TransactionType": "Payment",
  "Account": "cLSKzJZg4w2dgLfwf",
  "Destination": "cN5E7s8x9y2z3w4v5u6t",
  "Amount": "1000000",
  "Signers": [
    {
      "Signer": {
        "Account": "cSigner1xxxxxxxxxxxxxxxx",
        "TxnSignature": "3045022100...",
        "SigningPubKey": "0330E7FC9D..."
      }
    },
    {
      "Signer": {
        "Account": "cSigner2xxxxxxxxxxxxxxxx",
        "TxnSignature": "3045022100...",
        "SigningPubKey": "0330E7FC9D..."
      }
    }
  ],
  "Fee": "10",
  "Sequence": 1
}
```

---

## DepositPreauth

Pre-authorizes accounts to send payments to an account with Deposit Authorization enabled.

### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `Account` | string | Yes | Account with deposit auth enabled |
| `Authorize` | string | No | Account to pre-authorize (mutually exclusive with Unauthorize) |
| `Unauthorize` | string | No | Account to remove pre-authorization |

### Example: Authorize Depositor

```json
{
  "TransactionType": "DepositPreauth",
  "Account": "cLSKzJZg4w2dgLfwf",
  "Authorize": "cTrustedSender1234567890",
  "Fee": "10",
  "Sequence": 1
}
```

### Example: Unauthorize Depositor

```json
{
  "TransactionType": "DepositPreauth",
  "Account": "cLSKzJZg4w2dgLfwf",
  "Unauthorize": "cTrustedSender1234567890",
  "Fee": "10",
  "Sequence": 2
}
```

### Deposit Authorization Flow

1. Enable DepositAuth on your account:
   ```json
   { "TransactionType": "AccountSet", "SetFlag": 8 }
   ```

2. Pre-authorize specific senders:
   ```json
   { "TransactionType": "DepositPreauth", "Authorize": "cSender..." }
   ```

3. Only pre-authorized accounts can send you payments

4. You can still send payments to anyone

---

## NicknameSet

Sets or updates a human-readable nickname for an account.

### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `Account` | string | Yes | Account address |
| `Nickname` | string | Yes | Nickname string (3-32 characters) |

### Example

```json
{
  "TransactionType": "NicknameSet",
  "Account": "cLSKzJZg4w2dgLfwf",
  "Nickname": "Alice",
  "Fee": "10",
  "Sequence": 1
}
```

### Nickname Requirements

- Length: 3 to 32 characters
- Characters: Alphanumeric, hyphen, underscore
- Uniqueness: Nicknames are globally unique
- Cost: Small fee to prevent squatting

### Resolving Nicknames

Nicknames can be resolved to addresses via the `nickname_info` RPC method:

```json
{
  "method": "nickname_info",
  "params": [{ "nickname": "Alice" }]
}
```

---

## Transaction Common Fields

All transactions include these common fields:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `TransactionType` | string | Yes | Type of transaction |
| `Account` | string | Yes | Sender's account address |
| `Fee` | string | Yes | Transaction fee in drops |
| `Sequence` | uint32 | Yes | Account sequence number |
| `AccountTxnID` | string | No | Hash of previous transaction (for idempotency) |
| `Flags` | uint32 | No | Transaction-specific flags |
| `SourceTag` | uint32 | No | Sender tag (for exchanges) |
| `SigningPubKey` | string | Yes* | Public key for signature verification |
| `TxnSignature` | string | Yes* | Transaction signature |

*Required for single-signed transactions. Not used for multi-signed transactions.

### Transaction Flags

Global flags applicable to all transactions:

| Flag | Value | Description |
|------|-------|-------------|
| `tfFullyCanonicalSig` | 0x80000000 | Require fully canonical signatures |

Payment-specific flags:

| Flag | Value | Description |
|------|-------|-------------|
| `tfNoDirectCall` | 0x00000001 | Do not use direct CALL transfer |
| `tfPartialPayment` | 0x00020000 | Allow partial payment |
| `tfLimitQuality` | 0x00040000 | Only take offers at requested quality |
| `tfNoCallDirect` | 0x00080000 | Do not use CALL in paths |

---

## Transaction Fees

| Component | Cost (drops) |
|-----------|--------------|
| Base fee | 10 |
| Reference transaction | 10 |
| Payment | 10 |
| TrustSet | 10 |
| OfferCreate | 10 |
| OfferCancel | 10 |
| AccountSet | 10 |
| SetRegularKey | 10 |
| SignerListSet | 10 + (num_signers × 10) |
| DepositPreauth | 10 |
| NicknameSet | 1000 |

### Fee Scaling

Under network load, fees scale dynamically:

```
adjusted_fee = base_fee × load_factor
```

Check current fee with the `fee` RPC method.

---

## See Also

- [RPC API Reference](../api/rpc.md) - Submitting transactions via RPC
- [WebSocket API](../api/websocket.md) - Real-time transaction updates
- [Architecture Overview](../architecture/overview.md) - Transaction processing flow
