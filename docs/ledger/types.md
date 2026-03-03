# Ledger Entry Types

Call-Core maintains ledger state using various ledger entry types. Each entry represents a specific piece of state in the blockchain.

## Overview

| Entry Type | Description | Key Fields |
|------------|-------------|------------|
| `AccountRoot` | Account state | Balance, Sequence, Flags |
| `RippleState` | Trust line between two accounts | Balance, Limit, Quality |
| `Offer` | Open DEX order | TakerGets, TakerPays, Price |
| `DirectoryNode` | Index for iterating entries | Indexes, Owner |
| `Nickname` | Account nickname mapping | Name, Account |
| `DepositPreauth` | Pre-authorized depositor | Authorize, Unauthorize |
| `SignerList` | Multi-signature configuration | Signers, Quorum |

---

## AccountRoot

Represents the state of a single account.

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `Account` | string | Account address (c...) |
| `Balance` | string | CALL balance in drops |
| `Sequence` | uint32 | Next transaction sequence number |
| `OwnerCount` | uint32 | Number of objects owned (offers, trust lines) |
| `Flags` | uint32 | Account flags (see below) |
| `RegularKey` | string | Optional regular key address |
| `Domain` | string | Optional domain (hex-encoded) |
| `EmailHash` | string | Optional MD5 hash of email |
| `MessageKey` | string | Optional public key for messages |
| `TransferRate` | uint32 | Transfer fee (0 or 1000000000+) |
| `WalletLocator` | string | Optional 256-bit wallet locator |
| `WalletSize` | uint32 | Deprecated |

### Example

```json
{
  "LedgerEntryType": "AccountRoot",
  "Account": "cLSKzJZg4w2dgLfwf",
  "Balance": "10000000",
  "Sequence": 5,
  "OwnerCount": 2,
  "Flags": 0,
  "index": "B4A5E8D1C2F3A6B9..."
}
```

### Account Flags

| Flag | Hex | Name | Description |
|------|-----|------|-------------|
| 0x00010000 | 65536 | lsfDefaultCall | Default to CALL |
| 0x00020000 | 131072 | lsfDepositAuth | Deposit authorization required |
| 0x00040000 | 262144 | lsfDisableMaster | Master key disabled |
| 0x00100000 | 1048576 | lsfNoFreeze | No global freeze |
| 0x00200000 | 2097152 | lsfGlobalFreeze | All assets frozen |
| 0x00400000 | 4194304 | lsfRequireDestTag | Require destination tag |
| 0x00800000 | 8388608 | lsfRequireAuth | Require trust line auth |
| 0x01000000 | 16777216 | lsfDisallowCALL | Disallow incoming CALL |

### Owner Count Calculation

OwnerCount tracks objects that consume reserve:

- Trust lines (RippleState) - 1 each
- Offers - 1 each
- SignerList - 1
- DepositPreauth entries - 1 each (for authorizer)

---

## RippleState

Represents a trust line between two accounts for an issued asset.

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `Balance` | Amount | Current balance (negative means first account owes) |
| `LowLimit` | Amount | Trust limit from low account |
| `HighLimit` | Amount | Trust limit from high account |
| `LowNode` | string | Directory node for low account (internal) |
| `HighNode` | string | Directory node for high account (internal) |
| `LowQualityIn` | uint32 | Quality for payments to low account |
| `HighQualityIn` | uint32 | Quality for payments to high account |
| `LowQualityOut` | uint32 | Quality for payments from low account |
| `HighQualityOut` | uint32 | Quality for payments from high account |
| `Flags` | uint32 | Trust line flags |

### Example

```json
{
  "LedgerEntryType": "RippleState",
  "Balance": {
    "currency": "USD",
    "issuer": "cG6vVq8oTo1R3mYRg",
    "value": "500"
  },
  "LowLimit": {
    "currency": "USD",
    "issuer": "cG6vVq8oTo1R3mYRg",
    "value": "10000"
  },
  "HighLimit": {
    "currency": "USD",
    "issuer": "cG6vVq8oTo1R3mYRg",
    "value": "0"
  },
  "Flags": 0,
  "index": "C3D4E5F6A7B8C9D0..."
}
```

### RippleState Naming

- **Low Account**: Account with numerically smaller address
- **High Account**: Account with numerically larger address
- Balance is from low account's perspective (negative = low account owes)

### Trust Line Flags

| Flag | Value | Name | Description |
|------|-------|------|-------------|
| 0x00010000 | 65536 | lsfLowReserve | Low account has reserve obligation |
| 0x00020000 | 131072 | lsfHighReserve | High account has reserve obligation |
| 0x00040000 | 262144 | lsfLowAuth | Low account authorized |
| 0x00080000 | 524288 | lsfHighAuth | High account authorized |
| 0x00100000 | 1048576 | lsfLowNoCall | Low account disallows CALL |
| 0x00200000 | 2097152 | lsfHighNoCall | High account disallows CALL |
| 0x00400000 | 4194304 | lsfLowFreeze | Low account frozen |
| 0x00800000 | 8388608 | lsfHighFreeze | High account frozen |

---

## Offer

Represents an open offer on the decentralized exchange.

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `Account` | string | Owner's account address |
| `Sequence` | uint32 | Sequence number of OfferCreate transaction |
| `TakerGets` | Amount | Asset being sold |
| `TakerPays` | Amount | Asset being bought |
| `BookDirectory` | string | Index for order book (rate-based) |
| `BookNode` | string | Directory node in book (internal) |
| `OwnerNode` | string | Directory node for owner (internal) |
| `Expiration` | uint32 | Optional expiration time |
| `PreviousTxnID` | string | Hash of previous transaction affecting this offer |
| `PreviousTxnLgrSeq` | uint32 | Ledger sequence of previous transaction |
| `Flags` | uint32 | Offer flags |

### Example

```json
{
  "LedgerEntryType": "Offer",
  "Account": "cLSKzJZg4w2dgLfwf",
  "Sequence": 5,
  "TakerGets": {
    "currency": "USD",
    "issuer": "cG6vVq8oTo1R3mYRg",
    "value": "100"
  },
  "TakerPays": "100000000",
  "BookDirectory": "D4A5B6C7E8F9A0B1...",
  "BookNode": "0000000000000000",
  "OwnerNode": "0000000000000000",
  "Flags": 0,
  "index": "E5F6A7B8C9D0E1F2..."
}
```

### Offer Quality

Quality is the exchange rate: `TakerPays / TakerGets`

- Higher quality = better deal for taker
- Offers are matched from best quality to worst
- `BookDirectory` encodes quality for efficient ordering

### Offer Flags

| Flag | Value | Name | Description |
|------|-------|------|-------------|
| 0x00010000 | 65536 | lsfPassive | Passive offer (doesn't cross existing) |
| 0x00020000 | 131072 | lsfSell | Sell offer (fill or kill) |

### Partial Fills

Offers can be partially filled:
- Remaining amount stays on the books
- TakerGets and TakerPay are reduced proportionally
- Owner can cancel unfilled portion

---

## DirectoryNode

Indexes for efficiently iterating ledger entries.

### Types

1. **Owner Directories**: Track all objects owned by an account
2. **Book Directories**: Track offers in an order book at specific rates

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `Owner` | string | Account that owns this directory (owner dirs only) |
| `Indexes` | array | List of ledger entry indexes |
| `IndexNext` | uint64 | Next node in linked list |
| `IndexPrevious` | uint64 | Previous node in linked list |
| `ExchangeRate` | string | Rate for book directory (book dirs only) |
| `TakerPaysCurrency` | string | Book currency (book dirs only) |
| `TakerPaysIssuer` | string | Book issuer (book dirs only) |
| `TakerGetsCurrency` | string | Book currency (book dirs only) |
| `TakerGetsIssuer` | string | Book issuer (book dirs only) |

### Example: Owner Directory

```json
{
  "LedgerEntryType": "DirectoryNode",
  "Owner": "cLSKzJZg4w2dgLfwf",
  "Indexes": [
    "A1B2C3D4E5F6...",
    "B2C3D4E5F6A7...",
    "C3D4E5F6A7B8..."
  ],
  "Flags": 0,
  "index": "F6A7B8C9D0E1..."
}
```

### Purpose

- **Efficient Lookup**: Find all objects of a type without scanning entire ledger
- **Pagination**: Linked list structure allows iterating large sets
- **Book Ordering**: Book directories organize offers by quality

---

## Nickname

Maps a human-readable nickname to an account address.

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `Account` | string | Account address |
| `Nickname` | string | Human-readable nickname |
| `PreviousTxnID` | string | Hash of NicknameSet transaction |
| `PreviousTxnLgrSeq` | uint32 | Ledger sequence |

### Example

```json
{
  "LedgerEntryType": "Nickname",
  "Account": "cLSKzJZg4w2dgLfwf",
  "Nickname": "Alice",
  "PreviousTxnID": "A1B2C3D4E5F6...",
  "PreviousTxnLgrSeq": 12345,
  "index": "G7H8I9J0K1L2..."
}
```

### Nickname Constraints

- Globally unique across the ledger
- 3-32 characters
- Alphanumeric, hyphens, underscores only
- Case-sensitive

---

## DepositPreauth

Records pre-authorization for deposit authorization.

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `Account` | string | Account with deposit authorization |
| `Authorize` | string | Account pre-authorized to send payments |
| `PreviousTxnID` | string | Hash of DepositPreauth transaction |
| `PreviousTxnLgrSeq` | uint32 | Ledger sequence |

### Example

```json
{
  "LedgerEntryType": "DepositPreauth",
  "Account": "cLSKzJZg4w2dgLfwf",
  "Authorize": "cTrustedSender1234567890",
  "PreviousTxnID": "B2C3D4E5F6A7...",
  "PreviousTxnLgrSeq": 12345,
  "index": "H8I9J0K1L2M3..."
}
```

### Index Calculation

The entry index is derived from both Account and Authorize addresses, making lookups efficient for:
- Listing all authorized senders for an account
- Checking if a specific sender is authorized

---

## SignerList

Multi-signature configuration for an account.

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `Account` | string | Account address |
| `SignerQuorum` | uint32 | Required signature weight threshold |
| `SignerEntries` | array | List of signers |
| `SignerListID` | uint32 | Always 0 (reserved for future) |
| `PreviousTxnID` | string | Hash of SignerListSet transaction |
| `PreviousTxnLgrSeq` | uint32 | Ledger sequence |

### SignerEntry Fields

| Field | Type | Description |
|-------|------|-------------|
| `Account` | string | Signer's account address |
| `SignerWeight` | uint32 | Weight of this signer's signature |

### Example

```json
{
  "LedgerEntryType": "SignerList",
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
    }
  ],
  "SignerListID": 0,
  "PreviousTxnID": "C3D4E5F6A7B8...",
  "PreviousTxnLgrSeq": 12345,
  "Flags": 0,
  "index": "I9J0K1L2M3N4..."
}
```

### SignerList Flags

| Flag | Value | Name | Description |
|------|-------|------|-------------|
| 0x00010000 | 65536 | lsfOneOwnerCount | Counts as 1 owner object regardless of size |

---

## Ledger Entry Index

Each ledger entry has a unique 256-bit index derived from its content.

### Index Calculation

```
index = SHA-256(index_data)
```

Where `index_data` varies by entry type:

| Type | Index Data |
|------|------------|
| AccountRoot | Account address |
| RippleState | LowAccount + HighAccount + Currency (sorted) |
| Offer | Account + Sequence |
| DirectoryNode | Owner + NodeID (owner) or BookKey (book) |
| Nickname | Nickname string |
| DepositPreauth | Account + Authorize |
| SignerList | Account + SignerListID |

### Looking Up Entries

Use `ledger_entry` RPC:

```json
{
  "method": "ledger_entry",
  "params": [{
    "index": "B4A5E8D1C2F3A6B9...",
    "ledger_index": "validated"
  }]
}
```

---

## Ledger Entry Relationships

```
┌─────────────────────────────────────────────────────────────────┐
│                        AccountRoot                               │
│                     (cLSKzJZg4w2dgLfwf)                          │
└─────────────────────────────────────────────────────────────────┘
         │                    │                    │
         ▼                    ▼                    ▼
┌─────────────────┐   ┌─────────────────┐   ┌─────────────────┐
│  RippleState    │   │     Offer       │   │   SignerList    │
│ (USD Trust Line)│   │   (Sell USD)    │   │  (Multi-sig)    │
└─────────────────┘   └─────────────────┘   └─────────────────┘
         │                    │
         ▼                    ▼
┌─────────────────┐   ┌─────────────────┐
│ DirectoryNode   │   │ DirectoryNode   │
│ (Owner Directory)│  │ (Book Directory)│
└─────────────────┘   └─────────────────┘
```

---

## Entry Creation and Deletion

| Entry Type | Created By | Deleted By |
|------------|------------|------------|
| AccountRoot | Payment to new address | Cannot be deleted |
| RippleState | TrustSet | TrustSet with limit 0 (if balance 0) |
| Offer | OfferCreate | OfferCancel, OfferCreate (rate update), or fully filled |
| DirectoryNode | Automatic | Automatic when empty |
| Nickname | NicknameSet | Cannot be deleted (can be changed) |
| DepositPreauth | DepositPreauth (Authorize) | DepositPreauth (Unauthorize) |
| SignerList | SignerListSet | SignerListSet with quorum 0 |

---

## Reserve Requirements

Each ledger entry owned by an account increases its reserve requirement:

| Entry Type | Reserve Impact |
|------------|----------------|
| RippleState | 1 owner count |
| Offer | 1 owner count |
| SignerList | 1 owner count |
| DepositPreauth | 1 owner count (for authorizer) |

### Current Reserve

- **Base Reserve**: 10 CALL (minimum account balance)
- **Owner Reserve**: 2 CALL per owned object

Example: An account with 3 trust lines and 2 offers:
```
Required Reserve = 10 + (5 × 2) = 20 CALL
```

---

## See Also

- [Transaction Types](../transactions/types.md) - Transactions that create/modify entries
- [RPC API Reference](../api/rpc.md) - Query ledger entries
- [Architecture Overview](../architecture/overview.md) - Ledger state management
