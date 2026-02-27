# Call-Core Implementation Checklist

This document tracks the implementation status of call-core features compared to the original calld project (as documented in research.md).

**Excluded from this checklist (as per requirements):**
- Escrow transactions (ttESCROW_CREATE, ttESCROW_FINISH, ttESCROW_CANCEL)
- Payment channel transactions (ttPAYCHAN_CREATE, ttPAYCHAN_FUND, ttPAYCHAN_CLAIM)
- Ticket transactions (ttTICKET_CREATE, ttTICKET_CANCEL)

---

## 1. Transaction Types

| Type ID | Name | Status | Notes |
|---------|------|--------|-------|
| 0 | ttPAYMENT | ✅ Implemented | `TxType::Payment = 0` - matches calld |
| 16 | ttISSUE_SET | ⚠️ Wrong Value | Implemented as `TxType::IssueSet = 1`, should be **16** |
| 20 | ttTRUST_SET | ⚠️ Wrong Value | Implemented as `TxType::TrustSet = 2`, should be **20** |
| 7 | ttOFFER_CREATE | ⚠️ Wrong Value | Implemented as `TxType::OfferCreate = 3`, should be **7** |
| 8 | ttOFFER_CANCEL | ⚠️ Wrong Value | Implemented as `TxType::OfferCancel = 4`, should be **8** |
| 3 | ttACCOUNT_SET | ⚠️ Wrong Value | Implemented as `TxType::AccountSet = 5`, should be **3** |
| 5 | ttREGULAR_KEY_SET | ⚠️ Wrong Value | Implemented as `TxType::SetRegularKey = 6`, should be **5** |
| 12 | ttSIGNER_LIST_SET | ⚠️ Wrong Value | Implemented as `TxType::SignerListSet = 7`, should be **12** |

### Critical Issue: Transaction Type Values

The current implementation uses sequential values (0-7) but must match calld's specific values for compatibility:

```rust
// Current (WRONG)
pub enum TxType {
    Payment = 0,        // OK
    IssueSet = 1,       // WRONG - should be 16
    TrustSet = 2,       // WRONG - should be 20
    OfferCreate = 3,    // WRONG - should be 7
    OfferCancel = 4,    // WRONG - should be 8
    AccountSet = 5,     // WRONG - should be 3
    SetRegularKey = 6,  // WRONG - should be 5
    SignerListSet = 7,  // WRONG - should be 12
}
```

**Action Required:** Update all TxType values to match calld specifications.

---

## 2. Ledger Entry Types

| Type Code | Name | Status | Notes |
|-----------|------|--------|-------|
| 'a' (0x61) | ltACCOUNT_ROOT | ✅ Implemented | `LedgerEntryType::AccountRoot = 0x61` |
| 'r' (0x72) | ltCALL_STATE | ❌ Wrong Code | Using `'c' (0x63)` instead of `'r' (0x72)` |
| 'o' (0x6F) | ltOFFER | ✅ Implemented | `LedgerEntryType::Offer = 0x6F` |
| 'd' (0x64) | ltDIR_NODE | ✅ Implemented | `LedgerEntryType::DirectoryNode = 0x64` |
| 'S' (0x53) | ltSIGNER_LIST | ❌ Missing | Not implemented |
| 'h' (0x68) | ltLEDGER_HASHES | ❌ Missing | Not implemented |
| 'f' (0x66) | ltAMENDMENTS | ❌ Missing | Not implemented |
| 's' (0x73) | ltFEE_SETTINGS | ❌ Missing | Not implemented |
| 'i' (0x69) | ltISSUEROOT | ✅ Implemented | Custom Callchain feature |
| 'F' (0x46) | ltFeeRoot | ✅ Implemented | Custom Callchain feature |
| 'v' (0x76) | ltINVOICE | ⚠️ Partial | Type defined, struct missing |

### Critical Issue: CallState Type Code

The implementation uses `0x63 ('c')` but calld uses `0x72 ('r')` for trust lines.

### Missing Ledger Entry Structs

- [ ] **SignerList** - For multi-sign functionality
- [ ] **LedgerHashes** - For ledger history tracking
- [ ] **Amendments** - For protocol amendments
- [ ] **FeeSettings** - For fee configuration

---

## 3. Custom Callchain Features

### IssueSet Transaction (Native Asset Issuance)

| Component | Status | Notes |
|-----------|--------|-------|
| Transaction Type | ⚠️ Wrong Value | Value 1, should be 16 |
| Preflight Check | ✅ Implemented | `preflight_issue_set()` in tx_engine.rs |
| Preclaim Check | ✅ Implemented | `preclaim_issue_set()` in tx_engine.rs |
| Apply Logic | ✅ Implemented | `apply_issue_set()` in tx_engine.rs |
| IssueRoot Entry | ⚠️ Partial | Type defined, full struct implementation needed |
| Transfer Rate | ✅ Implemented | Validation logic present |
| Editable Supply | ✅ Implemented | `tfEnaddition` flag support |
| NFT Support | ⚠️ Partial | `tfNonFungible` flag defined, InvoiceRoot for NFTs needs work |

### Invoice System (NFT Support)

| Component | Status | Notes |
|-----------|--------|-------|
| Ledger Entry Type | ✅ Implemented | `LedgerEntryType::Invoice = 0x76` |
| Invoice Struct | ❌ Missing | Need struct with InvoiceID, Amount, Invoice data |
| InvoiceRoot | ❌ Missing | Directory linking for invoices |
| Payment Logic | ❌ Missing | Integration with payment transaction |

### FeeRoot (Accumulated Fee Tracking)

| Component | Status | Notes |
|-----------|--------|-------|
| Ledger Entry Type | ✅ Implemented | `LedgerEntryType::FeeRoot = 0x46` |
| FeeRoot Struct | ❌ Missing | Need struct with Balance field |
| Fee Accumulation | ❌ Missing | Logic to accumulate fees |
| Fee Distribution | ❌ Missing | Logic to distribute accumulated fees |

---

## 4. RPC Interface

### Server Info Methods

| Method | Status | Notes |
|--------|--------|-------|
| server_info | ✅ Implemented | Full implementation |
| server_state | ✅ Implemented | Full implementation |
| ping | ✅ Implemented | Full implementation |

### Ledger Methods

| Method | Status | Notes |
|--------|--------|-------|
| ledger | ✅ Implemented | Full implementation |
| ledger_closed | ✅ Implemented | Full implementation |
| ledger_current | ✅ Implemented | Full implementation |
| ledger_data | ✅ Implemented | Full implementation with SHAMap iteration |
| ledger_entry | ✅ Implemented | Full implementation |
| ledger_header | ✅ Implemented | Full implementation |
| ledger_accept | ✅ Implemented | Admin method for forcing ledger close |

### Account Methods

| Method | Status | Notes |
|--------|--------|-------|
| account_info | ✅ Implemented | Full implementation with ledger lookup |
| account_currencies | ✅ Implemented | Returns trust line currencies |
| account_lines | ✅ Implemented | Full trust line query |
| account_objects | ✅ Implemented | Returns all account objects |
| account_offers | ✅ Implemented | Returns DEX offers |
| account_channels | ✅ Implemented | Returns payment channels |
| account_tx | ⚠️ Stub | Returns empty array, needs tx indexing |
| gateway_balances | ⚠️ Stub | Returns placeholder data |
| owner_info | ⚠️ Stub | Returns placeholder data |

### Transaction Methods

| Method | Status | Notes |
|--------|--------|-------|
| submit | ✅ Implemented | Full transaction submission |
| submit_multisigned | ✅ Implemented | Multi-signature submission |
| tx | ✅ Implemented | Transaction lookup by hash |
| transaction_entry | ✅ Implemented | Transaction with metadata |
| tx_history | ⚠️ Stub | Returns empty array, needs history |
| sign | ✅ Implemented | Local transaction signing |
| sign_for | ✅ Implemented | Sign for multi-sign |

### DEX/Order Book Methods

| Method | Status | Notes |
|--------|--------|-------|
| book_offers | ✅ Implemented | Order book query |
| path_find | ✅ Implemented | Payment path finding (stub) |
| call_path_find | ✅ Implemented | Custom pathfinding |

### Consensus/Network Methods

| Method | Status | Notes |
|--------|--------|-------|
| consensus_info | ✅ Implemented | Returns consensus state |
| validators | ⚠️ Stub | Returns empty array |
| peers | ✅ Implemented | Returns peer information |
| connect | ⚠️ Stub | Returns success message |
| unl_list | ⚠️ Stub | Returns empty array |
| validator_list_sites | ⚠️ Stub | Returns empty array |
| blacklist | ⚠️ Stub | Returns empty array |

### Custom Callchain Commands

| Method | Status | Notes |
|--------|--------|-------|
| call_path_find | ✅ Implemented | Custom pathfinding |
| nick_search | ⚠️ Stub | Returns empty results |
| account_issues | ⚠️ Stub | Returns empty array |
| account_invoices | ⚠️ Stub | Returns empty array |

### Admin/System Methods

| Method | Status | Notes |
|--------|--------|-------|
| stop | ✅ Implemented | Server shutdown |
| ledger_cleaner | ⚠️ Stub | Returns success |
| ledger_request | ⚠️ Stub | Returns success |
| log_level | ✅ Implemented | Sets log level |
| log_rotate | ✅ Implemented | Rotates logs |
| get_counts | ✅ Implemented | Returns ledger object counts |
| fetch_info | ✅ Implemented | Returns sync status |
| feature | ✅ Implemented | Feature flags query/set |
| print | ✅ Implemented | Debug info printing |
| no_call_check | ✅ Implemented | Disables CALL check |
| can_delete | ✅ Implemented | Ledger deletion check |
| session_open | ✅ Implemented | Session management |
| session_close | ✅ Implemented | Session management |
| version | ✅ Implemented | Version info |

### Wallet/Key Methods

| Method | Status | Notes |
|--------|--------|-------|
| wallet_propose | ✅ Implemented | Generates new wallet |
| wallet_seed | ⚠️ Stub | Returns placeholder |
| wallet_lock | ✅ Implemented | Lock wallet |
| wallet_unlock | ⚠️ Stub | Returns placeholder |
| wallet_verify | ⚠️ Stub | Returns placeholder |
| validation_create | ✅ Implemented | Creates validation key |
| validation_seed | ⚠️ Stub | Returns placeholder |

---

## 5. Network Protocol

| Component | Status | Notes |
|-----------|--------|-------|
| Message Framing | ✅ Implemented | 6-byte header (size + type) |
| TMHello | ✅ Implemented | Handshake message |
| TMPing | ✅ Implemented | Keepalive |
| TMProposeSet | ✅ Implemented | Consensus proposals |
| TMValidation | ✅ Implemented | Ledger validations |
| TMTransaction | ✅ Implemented | Transaction propagation |
| TMGetLedger | ✅ Implemented | Ledger data requests |
| TMLedgerData | ✅ Implemented | Ledger data responses |
| TMStatusChange | ✅ Implemented | Node status changes |
| TMHaveTransactionSet | ✅ Implemented | Tx set announcements |
| TMGetObjects | ✅ Implemented | Object retrieval |
| Peer Management | ✅ Implemented | Peer connection handling |
| Overlay Network | ✅ Implemented | P2P network layer |

---

## 6. Consensus

| Component | Status | Notes |
|-----------|--------|-------|
| RPCA Algorithm | ✅ Implemented | Ripple Consensus Algorithm |
| 80% Threshold | ✅ Implemented | Byzantine fault tolerance |
| Open Phase | ✅ Implemented | Transaction collection |
| Establish Phase | ✅ Implemented | Proposal exchange |
| Accepted Phase | ✅ Implemented | Ledger finalization |
| Proposal Signing | ✅ Implemented | Validator signatures |
| Validation Signing | ✅ Implemented | Ledger validation |
| UNL Support | ✅ Implemented | Unique Node List |
| Consensus Parameters | ✅ Implemented | Timing and thresholds |

---

## 7. Storage Layer

| Component | Status | Notes |
|-----------|--------|-------|
| NodeObject Format | ✅ Implemented | Compatible with calld |
| RocksDB Backend | ✅ Implemented | Database storage |
| Memory Backend | ✅ Implemented | Testing/development |
| SHAMap Inner Node V1 | ✅ Implemented | `MIN\0` prefix |
| SHAMap Inner Node V2 | ✅ Implemented | `INR\0` prefix |
| SHAMap Leaf Node | ✅ Implemented | `MLN\0` prefix |
| Transaction Node NM | ✅ Implemented | `TXN\0` prefix |
| Transaction Node MD | ✅ Implemented | `SND\0` prefix |
| Hash Prefixes | ✅ Implemented | All prefixes match calld |
| SHA-512/256 Hashing | ✅ Implemented | `sha512_half()` function |

---

## 8. Serialization

| Component | Status | Notes |
|-----------|--------|-------|
| STObject | ✅ Implemented | Core serialization object |
| Serializer | ✅ Implemented | Byte encoding |
| Amount Type | ✅ Implemented | Native and issued currencies |
| Field ID Encoding | ✅ Implemented | Variable length encoding |
| Variable Length Encoding | ✅ Implemented | Length prefix rules |
| Big-Endian Integers | ✅ Implemented | Network byte order |
| Transaction Serialization | ✅ Implemented | Full TX support |
| Ledger Entry Serialization | ✅ Implemented | Full support |
| Metadata Serialization | ⚠️ Partial | Basic implementation |

---

## 9. Cryptography

| Component | Status | Notes |
|-----------|--------|-------|
| secp256k1 | ✅ Implemented | ECDSA signatures |
| ed25519 | ✅ Implemented | EdDSA signatures |
| SHA-512/256 | ✅ Implemented | `sha512_half` |
| SHA-256 | ✅ Implemented | General hashing |
| Transaction Signing | ✅ Implemented | Single signature |
| Multi-Signing | ✅ Implemented | `txMultiSign` hash prefix |
| Key Generation | ✅ Implemented | Deterministic keys |
| Signature Verification | ✅ Implemented | Both key types |
| Hash Prefixes | ✅ Implemented | TXN, STX, SMT, etc. |

---

## 10. WebSocket API

| Feature | Status | Notes |
|---------|--------|-------|
| WebSocket Server | ✅ Implemented | Full implementation |
| subscribe Command | ✅ Implemented | All stream types |
| unsubscribe Command | ✅ Implemented | Full implementation |
| ping Command | ✅ Implemented | Pong response |
| server_info Command | ✅ Implemented | Server info |
| ledger Command | ✅ Implemented | Current ledger info |
| account_info Command | ✅ Implemented | Account info |
| Ledger Stream | ✅ Implemented | Ledger close events |
| Transactions Stream | ✅ Implemented | Real-time transactions |
| Validations Stream | ✅ Implemented | Validation events |
| Consensus Stream | ✅ Implemented | Phase change events |
| Peer Stream | ✅ Implemented | Peer status events |
| Account Subscriptions | ✅ Implemented | Account-specific updates |

---

## Priority Summary

### Critical (Data Compatibility Issues)

1. [ ] **Fix TxType values** - Must match calld for database compatibility
2. [ ] **Fix CallState type code** - Change from 'c' (0x63) to 'r' (0x72)
3. [ ] **Implement missing ledger entry types:**
   - [ ] SignerList (ltSIGNER_LIST = 'S')
   - [ ] LedgerHashes (ltLEDGER_HASHES = 'h')
   - [ ] Amendments (ltAMENDMENTS = 'f')
   - [ ] FeeSettings (ltFEE_SETTINGS = 's')

### High Priority (Custom Callchain Features)

4. [ ] **Complete IssueSet implementation:**
   - [ ] Fix TxType::IssueSet value to 16
   - [ ] Implement IssueRoot struct
   - [ ] Test NFT issuance with Invoice system

5. [ ] **Implement Invoice System:**
   - [ ] Invoice struct with InvoiceID, Amount, data
   - [ ] InvoiceRoot for directory linking
   - [ ] Payment integration

6. [ ] **Implement FeeRoot:**
   - [ ] FeeRoot struct with Balance
   - [ ] Fee accumulation logic
   - [ ] Fee distribution mechanism

### Medium Priority (Feature Completeness)

7. [ ] **Complete transaction indexing for account_tx**
8. [ ] **Complete transaction history for tx_history**
9. [ ] **Implement proper wallet_seed derivation**
10. [ ] **Implement validation_seed derivation**

### Low Priority (Nice to Have)

11. [ ] **Complete nick_search implementation**
12. [ ] **Complete account_issues implementation**
13. [ ] **Complete account_invoices implementation**
14. [ ] **Implement validators/UNL discovery**

---

## Notes

- The codebase has excellent architecture with clean separation across crates
- RPC implementation is very comprehensive (70+ methods)
- Network and consensus layers are fully functional
- Main blockers are transaction type values and missing ledger entry types
- Custom Callchain features need struct implementations

## Estimated Effort

- **Critical fixes**: 2-3 days
- **Custom features completion**: 3-5 days
- **Full feature parity**: 1-2 weeks
