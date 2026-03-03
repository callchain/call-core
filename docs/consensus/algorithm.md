# Call-Core Consensus Algorithm

Call-Core implements the **Callchain Consensus Protocol (CCP)**, a Byzantine Fault Tolerant (BFT) consensus algorithm designed for high-throughput, low-latency blockchain networks.

## Overview

The consensus algorithm enables distributed validators to agree on the contents of each ledger without requiring a central authority. Key properties:

- **Byzantine Fault Tolerant**: Tolerates up to 20% malicious validators
- **Weighted Voting**: Validators have configurable voting weights
- **Fast Finality**: 5-second ledger close times with single-block finality
- **Safety over Liveness**: Prefers to halt rather than produce inconsistent ledgers

## Consensus Phases

The consensus algorithm operates in four distinct phases:

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│    Open      │────→│  Establish   │────→│  Processing  │────→│   Accepted   │
│    Phase     │     │    Phase     │     │    Phase     │     │    Phase     │
└──────────────┘     └──────────────┘     └──────────────┘     └──────────────┘
     │                     │                     │                     │
     ▼                     ▼                     ▼                     ▼
 Collect Tx          Build Proposal         Apply Tx Set        Ledger Closed
 From Network        Share w/ Peers         Update State        Generate Validations
```

### Phase 1: Open

**Duration**: Until sufficient transactions or timeout (max 10 seconds)

**Activities**:
- Collect transactions from the queue
- Monitor peer positions
- Track validation messages
- Wait for consensus trigger

**Transition to Establish**: Triggered by:
- Transaction queue fills above threshold
- Timeout reached (configurable, default 10s)
- External consensus signal

### Phase 2: Establish

**Duration**: ~2-4 seconds

**Activities**:
- Build transaction set proposal
- Calculate initial close time
- Share proposal with peers
- Collect peer proposals
- Identify disputes (conflicting transactions)

**Key Mechanism**:
```rust
struct Proposal {
    ledger_seq: u32,
    previous_ledger: UInt256,
    transaction_set: Vec<Transaction>,
    close_time: u32,
    consensus_round: u32,
}
```

**Dispute Resolution**:
When validators propose different transaction sets:
1. Identify disputed transactions (in some proposals, not others)
2. Collect votes from all validators
3. Include transaction if >50% of weighted validators include it
4. Exclude if <50% support

### Phase 3: Processing

**Duration**: ~1 second

**Activities**:
- Apply agreed transaction set
- Calculate new ledger state
- Update SHAMap root hash
- Generate ledger header

**Transaction Application**:
```rust
for tx in agreed_transactions {
    match apply_transaction(&mut ledger, &tx) {
        Ok(result) => {
            record_transaction_result(&mut metadata, tx.hash(), result);
        }
        Err(e) => {
            // Transaction failed, record error
            record_failure(&mut metadata, tx.hash(), e);
        }
    }
}
```

### Phase 4: Accepted

**Duration**: Instantaneous (completion)

**Activities**:
- Finalize ledger
- Generate validation message
- Broadcast validation to network
- Notify subscribers
- Begin next consensus round

## Byzantine Fault Tolerance

### Fault Detection

The algorithm detects and handles Byzantine (malicious) validators:

```rust
struct ByzantineFault {
    validator: AccountID,
    fault_type: FaultType,
    evidence: FaultEvidence,
    timestamp: Instant,
}

enum FaultType {
    ConflictingProposals,  // Double-signing different proposals
    InvalidValidation,     // Signature or format invalid
    TimeoutViolation,      // Not responding in time
}
```

### Double-Signing Detection

Validators are monitored for conflicting proposals:

```rust
fn detect_conflicting_proposals(
    &mut self,
    validator: &AccountID,
    proposal: &Proposal
) -> Option<ByzantineFault> {
    if let Some(existing) = self.proposals.get(validator) {
        if existing.hash() != proposal.hash() {
            // Validator signed two different proposals!
            return Some(ByzantineFault::conflicting(
                validator,
                existing,
                proposal
            ));
        }
    }
    None
}
```

### Consensus Calculation

**Weighted Consensus**:
```rust
fn have_consensus(&self, proposal: &Proposal) -> bool {
    let total_weight = self.get_total_validator_weight();
    let trusted_weight = self.get_trusted_weight();

    let supporting_weight = self.validators
        .iter()
        .filter(|v| v.supports(proposal))
        .map(|v| v.weight)
        .sum();

    // Require 80% of trusted (non-faulty) validator weight
    (supporting_weight as f64 / trusted_weight as f64) >= 0.80
}
```

## Validator Weight System

### Weight Assignment

Validators can be assigned different voting weights:

```rust
struct ValidatorInfo {
    account_id: AccountID,
    public_key: PublicKey,
    weight: u32,  // Default: 1
    trusted: bool,
}
```

### Use Cases

1. **Geographic Distribution**: Higher weights for underrepresented regions
2. **Stake-Based**: Weight proportional to staked tokens
3. **Reputation**: Weight based on historical reliability

### Trusted Weight Calculation

```rust
fn get_trusted_weight(&self) -> u32 {
    self.validators
        .iter()
        .filter(|v| !self.is_faulty(&v.account_id))
        .map(|v| v.weight)
        .sum()
}
```

## Close Time Consensus

### Median-Based Close Time

The ledger close time is determined by the median of validator proposals:

```rust
fn calculate_close_time(&self, proposals: &[Proposal]) -> u32 {
    let mut times: Vec<u32> = proposals
        .iter()
        .map(|p| p.close_time)
        .collect();

    times.sort();

    // Take median
    let mid = times.len() / 2;
    if times.len() % 2 == 0 {
        (times[mid - 1] + times[mid]) / 2
    } else {
        times[mid]
    }
}
```

### Close Time Resolution

Close times are rounded to maintain consensus:

```rust
fn round_close_time(
    proposed: u32,
    parent_close: u32,
    resolution: u8
) -> u32 {
    let close_time = proposed.max(parent_close + 1);
    let resolution_secs = resolution as u32;

    // Round to nearest resolution boundary
    ((close_time + resolution_secs / 2) / resolution_secs)
        * resolution_secs
}
```

## Amendment System

The consensus protocol supports protocol upgrades through amendments.

### Amendment Lifecycle

```
Proposed → Supported → LockedIn → Active
```

### Amendment Voting

```rust
struct Amendment {
    name: String,
    hash: UInt256,
    status: AmendmentStatus,
    support_percentage: f64,
}

enum AmendmentStatus {
    Proposed,
    Supported,
    LockedIn,  // 80% support for 2 weeks
    Active,    // Activated at next ledger
}
```

### Activation Threshold

- **Support Required**: 80% of validators
- **Lock-in Period**: 2 weeks minimum
- **Grace Period**: 2 weeks after lock-in before activation

## Fee Voting

Validators vote on network fees:

```rust
struct FeeVote {
    base_fee: u64,
    reserve_base: u64,
    reserve_increment: u64,
}

fn calculate_consensus_fee(votes: &[FeeVote]) -> FeeVote {
    FeeVote {
        base_fee: median(votes.iter().map(|v| v.base_fee)),
        reserve_base: median(votes.iter().map(|v| v.reserve_base)),
        reserve_increment: median(votes.iter().map(|v| v.reserve_increment)),
    }
}
```

Fee changes take effect every 256 ledgers (~21 minutes).

## Consensus Parameters

| Parameter | Default Value | Description |
|-----------|---------------|-------------|
| `ledger_close_time` | 5 seconds | Target time between ledgers |
| `consensus_threshold` | 80% | Required validator agreement |
| `max_transactions` | 5,000 | Maximum transactions per ledger |
| `max_ledger_size` | 10 MB | Maximum ledger size |
| `min_consensus_peers` | 2 | Minimum validators needed |
| `max_consensus_peers` | 50 | Maximum validators allowed |
| `validation_timeout` | 2 seconds | Time to wait for validations |
| `proposal_timeout` | 5 seconds | Time to wait for proposals |

## Safety and Liveness

### Safety Guarantees

1. **No Double Spending**: Once a transaction is confirmed, it cannot be reversed
2. **Consistency**: All honest nodes agree on the same ledger
3. **Validity**: Only valid transactions are included in ledgers

### Liveness Guarantees

1. **Progress**: If <20% of validators are faulty, consensus progresses
2. **Fairness**: All validators have opportunity to propose transactions
3. **Availability**: Network continues even with some validator failures

### Trade-offs

- **Safety over Liveness**: If consensus cannot be reached, the network halts rather than producing conflicting ledgers
- **Finality over Speed**: Single-block finality requires additional time for validation

## Monitoring Consensus

### Key Metrics

```rust
struct ConsensusMetrics {
    ledger_sequence: u32,
    consensus_round: u32,
    validator_count: usize,
    trusted_validator_count: usize,
    proposing_validator_count: usize,
    avg_consensus_time_ms: u64,
    disputed_transactions: usize,
}
```

### Health Indicators

- **Consensus Time**: Should be <5 seconds
- **Validator Participation**: Should be >90%
- **Dispute Rate**: Should be <1% of transactions
- **Ledger Gap**: Should never exceed 1 ledger

## Troubleshooting

### Common Issues

**Issue**: Consensus failing to reach 80%
- **Cause**: Network partition or validator failures
- **Solution**: Check network connectivity, validator status

**Issue**: High dispute rate
- **Cause**: Network latency or validator clock skew
- **Solution**: Synchronize clocks, check network paths

**Issue**: Slow consensus (>10 seconds)
- **Cause**: High transaction volume or slow validators
- **Solution**: Adjust fee escalation, optimize validator hardware

## See Also

- [Validator Setup](../guides/validator-setup.md) - Running a validator
- [Consensus Configuration](../guides/configuration.md#consensus) - Consensus settings
- [Network Architecture](../architecture/network.md) - P2P network details
- [Byzantine Faults](byzantine-faults.md) - Handling malicious validators
