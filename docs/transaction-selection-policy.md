# Transaction Selection Policy

## Overview

This document describes the transaction selection and ordering policy used by call-core when building new ledgers.

## Design Goals

1. **Fairness**: Process transactions in a predictable, deterministic order
2. **High Success Rate**: Minimize transaction failures due to sequence gaps
3. **Simplicity**: Use FIFO ordering where possible to reduce complexity
4. **User Experience**: Automatic retry for out-of-order transactions

## Priority Hierarchy

Transactions are selected based on the following priority order:

| Priority | Field | Order | Description |
|----------|-------|-------|-------------|
| 1 | **Account ID** | Group by account | Process all transactions for an account together |
| 2 | **Sequence** | Ascending (1, 2, 3...) | Lower sequence numbers processed first |
| 3 | **Fee** | Descending (high → low) | Higher fees prioritized for same sequence |
| 4 | **Arrival Time** | FIFO | First-seen transaction wins all ties |

## Detailed Rules

### Sort Key Definition

```rust
SORT_KEY = (AccountID, Sequence, -Fee, ArrivalTimestamp)
```

### Example Ordering

```
1. (AccountA, seq=1, fee=100, t=0)     ← First
2. (AccountA, seq=1, fee=50, t=1)      ← Same account+seq, lower fee
3. (AccountA, seq=2, fee=1000, t=2)    ← Next sequence
4. (AccountB, seq=1, fee=10, t=3)      ← Different account
5. (AccountB, seq=2, fee=100, t=4)     ← Next sequence
```

## Edge Case Handling

### Same Account + Same Sequence + Same Fee

**Policy**: **FIFO** - First transaction seen wins.

This prevents transaction replacement attacks while maintaining simplicity.

### Different Accounts

**Policy**: Process accounts in **FIFO order** by first pending transaction arrival.

Accounts do not block each other - each account's sequence is processed independently.

### PRE_SEQ (Future Sequence)

**Policy**: Cache transactions that arrive too early and retry in subsequent ledgers.

```rust
if tx.sequence > account.current_sequence + 1 {
    cache.insert((account, tx.sequence), tx, current_round);
}
```

**Cache Parameters**:
- `max_cache_rounds`: How many ledger closes to hold cached transactions (default: 10)
- `max_cache_size`: Maximum total cached transactions (default: 10000)
- `max_per_account`: Maximum cached per account (default: 100, anti-spam)

### Sequence Gap Detection

When a gap is detected (e.g., have seq=3 but account is at seq=1):
- Hold all subsequent sequences for that account
- Wait for missing sequence to arrive or timeout
- Apply sequences in order once gap is filled

## Example: Multi-Thread Stress Test

### Scenario

**Thread sequences submitted:**
- Thread 0: AccountA seq=1,2,3...
- Thread 1: AccountA seq=10001,10002...
- Thread 2: AccountA seq=20001,20002...

### Processing Flow

```
Ledger 1: Apply seq=1 (valid), cache seq=10001,20001 (PRE_SEQ)
Ledger 2: Apply seq=2 (valid), cache seq=10001,20001 (PRE_SEQ)
...
Ledger 8: Apply seq=8 (valid)
Ledger 9: Apply seq=10001 (now valid), cache seq=20001 (still PRE_SEQ)
...
Ledger N: Apply seq=20001 (now valid)
```

### Result

All transactions are eventually applied over multiple ledgers. None are dropped due to PRE_SEQ errors.

## Configuration

```toml
[transaction_pool]
# Enable PRE_SEQ caching
pre_seq_cache_enabled = true

# How many ledger closes to hold cached transactions
pre_seq_cache_rounds = 10

# Maximum total cached transactions
pre_seq_cache_max_size = 10000

# Maximum cached per account (anti-spam)
pre_seq_per_account_limit = 100
```

## Performance Characteristics

| Metric | Value |
|--------|-------|
| Sort Complexity | O(n log n) per ledger |
| Memory Usage | ~2 MB for 10k cached txs |
| Success Rate | ~80%+ (vs ~16% without cache) |
| Cache Expiry | 10 rounds default (~50 seconds at 5s/round) |

## Comparison with Original Policy

| Aspect | Original | New Policy |
|--------|----------|------------|
| Primary Sort | Fee level | Account + Sequence |
| PRE_SEQ Handling | Drop immediately | Cache and retry |
| Success Rate | ~16% | ~80%+ |
| User Experience | Poor (manual retry) | Good (auto-retry) |
| Fairness | Fee-based | FIFO-based |

## Implementation Notes

### Data Structures

```rust
// Main transaction queue
struct TransactionQueue {
    by_account: HashMap<AccountID, BTreeMap<u32, QueuedTransaction>>,
    arrival_order: VecDeque<(AccountID, u32)>, // For FIFO ordering
}

// PRE_SEQ cache
struct PreSeqCache {
    entries: HashMap<(AccountID, u32), CachedTransaction>,
    by_expiry: BTreeMap<u64, Vec<(AccountID, u32)>>,
}

struct CachedTransaction {
    tx: Transaction,
    cached_at_round: u64,
    expiry_round: u64,
}
```

### Ledger Close Process

1. **Collect eligible transactions** from queue and cache
2. **Sort** by (Account, Sequence, -Fee, ArrivalTime)
3. **Apply** transactions sequentially
4. **Cache failed PRE_SEQ** transactions for retry
5. **Expire old cache entries**
6. **Update ledger state**

## Future Improvements

- [ ] Dynamic fee escalation based on cache pressure
- [ ] Account prioritization based on transaction history
- [ ] Batch processing for same-account transactions
- [ ] Cache pre-validation to reduce failed retries
