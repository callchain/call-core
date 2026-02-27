//! Transaction Queue and Open Ledger Management
//!
//! This module manages the queue of pending transactions waiting to be included
//! in the next ledger. It handles:
//!
//! - Transaction queueing with fee escalation
//! - Open ledger management for building candidate ledgers
//! - Transaction replacement (same account/sequence with higher fee)
//! - Queue ordering by fee level

use crate::ledger::Fees;
use crate::transactions::{TER, Transaction};
use crate::tx_engine::{ApplyContext, ApplyRules, TransactionEngine, TxResult};
use crate::views::LedgerView;
use primitives::{AccountID, UInt256};
use std::collections::{BTreeMap, HashMap, VecDeque};

/// Transaction with metadata for queue management
#[derive(Debug, Clone)]
pub struct QueuedTransaction {
    pub transaction: Transaction,
    pub received_time: u64,
    pub fee_level: u64,
    pub priority: u32,
}

impl QueuedTransaction {
    pub fn new(transaction: Transaction, received_time: u64) -> Self {
        let fee_level = Self::calculate_fee_level(&transaction);
        Self {
            transaction,
            received_time,
            fee_level,
            priority: 0,
        }
    }

    /// Calculate fee level for queue ordering
    fn calculate_fee_level(tx: &Transaction) -> u64 {
        // Base fee level is the fee paid
        // In a real implementation, this would consider:
        // - Base fee
        // - Fee units consumed
        // - Current network load
        tx.fee
    }

    pub fn sequence(&self) -> u32 {
        self.transaction.sequence
    }

    pub fn account(&self) -> AccountID {
        self.transaction.account
    }
}

/// Transaction queue with ordering by fee
#[derive(Debug, Clone)]
pub struct TransactionQueue {
    /// Transactions by account
    by_account: HashMap<AccountID, BTreeMap<u32, QueuedTransaction>>,
    /// Global queue ordered by fee level (descending)
    by_fee: BTreeMap<u64, VecDeque<UInt256>>,
    /// Transaction hashes for quick lookup
    tx_hashes: HashMap<UInt256, (AccountID, u32)>,
    /// Maximum queue size
    max_size: usize,
    /// Current size
    size: usize,
}

impl TransactionQueue {
    pub fn new(max_size: usize) -> Self {
        Self {
            by_account: HashMap::new(),
            by_fee: BTreeMap::new(),
            tx_hashes: HashMap::new(),
            max_size,
            size: 0,
        }
    }

    /// Add a transaction to the queue
    pub fn insert(&mut self, tx: Transaction) -> Result<(), TER> {
        if self.size >= self.max_size {
            return Err(TER::terINSUFF_FEE);
        }

        let account = tx.account;
        let sequence = tx.sequence;
        let hash = tx.get_hash();
        let fee = tx.fee;

        // Check if we already have this transaction
        if self.tx_hashes.contains_key(&hash) {
            return Err(TER::terDUPLICATE);
        }

        // Check for replacement (same sequence, higher fee)
        if let Some(existing_seq) = self.by_account.get(&account).and_then(|q| q.get(&sequence)) {
            if fee <= existing_seq.transaction.fee {
                return Err(TER::terINSUFF_FEE); // Need higher fee to replace
            }
            // Remove old transaction first
            let old_hash = existing_seq.transaction.get_hash();
            self.remove_by_hash(&old_hash);
        }

        // Check if sequence is already in ledger (would need proper ledger view)
        // For now, we allow any future sequence

        let now = self.current_time();
        let queued = QueuedTransaction::new(tx, now);

        // Add to account queue
        self.by_account
            .entry(account)
            .or_default()
            .insert(sequence, queued.clone());

        // Add to fee queue
        self.by_fee
            .entry(queued.fee_level)
            .or_default()
            .push_back(hash);

        // Add to hash lookup
        self.tx_hashes.insert(hash, (account, sequence));

        self.size += 1;
        Ok(())
    }

    /// Remove a transaction by hash
    pub fn remove_by_hash(&mut self, hash: &UInt256) -> Option<QueuedTransaction> {
        let (account, sequence) = self.tx_hashes.remove(hash)?;

        // Remove from account queue
        let account_queue = self.by_account.get_mut(&account)?;
        let tx = account_queue.remove(&sequence)?;

        // Remove from fee queue
        if let Some(fee_queue) = self.by_fee.get_mut(&tx.fee_level) {
            fee_queue.retain(|h| h != hash);
            if fee_queue.is_empty() {
                self.by_fee.remove(&tx.fee_level);
            }
        }

        self.size -= 1;
        Some(tx)
    }

    /// Get next transaction to process (highest fee first)
    pub fn pop_highest_fee(&mut self) -> Option<QueuedTransaction> {
        // Find highest fee level with transactions
        let highest_fee = *self.by_fee.keys().next_back()?;

        // Get the hash from the fee queue
        let hash = {
            let fee_queue = self.by_fee.get_mut(&highest_fee)?;
            fee_queue.pop_front()?
        };

        // Remove from other data structures
        let result = self.remove_by_hash(&hash);

        // Clean up empty fee queue
        if let Some(fee_queue) = self.by_fee.get(&highest_fee) {
            if fee_queue.is_empty() {
                self.by_fee.remove(&highest_fee);
            }
        }

        result
    }

    /// Get all transactions for an account
    pub fn get_account_txs(&self, account: &AccountID) -> Vec<&QueuedTransaction> {
        self.by_account
            .get(account)
            .map(|queue| queue.values().collect())
            .unwrap_or_default()
    }

    /// Check if transaction exists
    pub fn contains(&self, hash: &UInt256) -> bool {
        self.tx_hashes.contains_key(hash)
    }

    /// Get queue size
    pub fn len(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Clear all transactions
    pub fn clear(&mut self) {
        self.by_account.clear();
        self.by_fee.clear();
        self.tx_hashes.clear();
        self.size = 0;
    }

    fn current_time(&self) -> u64 {
        // In a real implementation, use actual time
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

impl Default for TransactionQueue {
    fn default() -> Self {
        Self::new(10000)
    }
}

/// Open ledger for building the next candidate
pub struct OpenLedger {
    /// Base ledger we're building on
    base_ledger_seq: u32,
    /// Current fee settings
    fees: Fees,
    /// Transaction queue
    queue: TransactionQueue,
    /// Applied transactions in order
    applied: Vec<(Transaction, TxResult)>,
    /// Current ledger view
    view: Box<dyn LedgerView>,
    /// Transaction engine
    engine: TransactionEngine,
}

impl OpenLedger {
    pub fn new(
        base_ledger_seq: u32,
        fees: Fees,
        view: Box<dyn LedgerView>,
    ) -> Self {
        Self {
            base_ledger_seq,
            fees,
            queue: TransactionQueue::new(10000),
            applied: Vec::new(),
            view,
            engine: TransactionEngine::new(),
        }
    }

    /// Queue a transaction for inclusion
    pub fn queue_transaction(&mut self, tx: Transaction) -> Result<(), TER> {
        // Validate transaction can be queued
        self.validate_for_queue(&tx)?;

        // Add to queue
        self.queue.insert(tx)
    }

    /// Apply transactions from queue to build candidate ledger
    pub fn apply_queued_transactions(&mut self, max_count: usize) -> Vec<(Transaction, TxResult)> {
        let mut results = Vec::new();
        let mut applied = 0;

        while applied < max_count {
            let Some(queued) = self.queue.pop_highest_fee() else {
                break;
            };

            let tx = queued.transaction;

            // Create apply context
            let mut ctx = ApplyContext {
                ledger: &mut *self.view,
                rules: ApplyRules::default(),
                ledger_seq: self.base_ledger_seq + 1,
                parent_ledger_hash: UInt256::zero(),
            };

            // Apply transaction
            let result = self.engine.process(&mut ctx, &tx);

            if result.ter.is_success() {
                results.push((tx.clone(), result.clone()));
                self.applied.push((tx, result));
                applied += 1;
            } else {
                // Transaction failed - could re-queue or drop
                // For now, we just drop failed transactions
            }
        }

        results
    }

    /// Get current queue size
    pub fn queue_size(&self) -> usize {
        self.queue.len()
    }

    /// Get applied transaction count
    pub fn applied_count(&self) -> usize {
        self.applied.len()
    }

    /// Get current fee settings
    pub fn fees(&self) -> &Fees {
        &self.fees
    }

    /// Update fee settings
    pub fn set_fees(&mut self, fees: Fees) {
        self.fees = fees;
    }

    /// Get transactions applied so far
    pub fn applied_transactions(&self) -> &[(Transaction, TxResult)] {
        &self.applied
    }

    /// Finalize the open ledger into a candidate
    pub fn finalize(self) -> Vec<(Transaction, TxResult)> {
        self.applied
    }

    fn validate_for_queue(&self, tx: &Transaction) -> Result<(), TER> {
        // Check fee meets minimum
        if tx.fee < self.fees.base {
            return Err(TER::telINSUFFICIENT_FEE);
        }

        // Check sequence is reasonable (not too far in future)
        // In a real implementation, we'd check against current ledger state

        // Check transaction isn't expired
        // In a real implementation, we'd check expiration

        Ok(())
    }
}

/// Fee escalation for transaction queue
pub struct FeeEscalation {
    /// Base fee
    base_fee: u64,
    /// Current escalation multiplier (in tenths of a percent)
    escalation_multiplier: u32,
    /// Maximum escalation
    max_escalation: u32,
    /// Target queue size
    target_size: usize,
}

impl FeeEscalation {
    pub fn new(base_fee: u64, target_size: usize) -> Self {
        Self {
            base_fee,
            escalation_multiplier: 1000, // 100.0%
            max_escalation: 10000,       // 1000%
            target_size,
        }
    }

    /// Calculate required fee for queue insertion
    pub fn required_fee(&self, current_queue_size: usize) -> u64 {
        if current_queue_size < self.target_size {
            return self.base_fee;
        }

        // Escalate fee based on queue pressure
        let pressure = current_queue_size as f64 / self.target_size as f64;
        let multiplier = (self.escalation_multiplier as f64 * pressure)
            .min(self.max_escalation as f64);

        ((self.base_fee as f64 * multiplier) / 1000.0) as u64
    }

    /// Update escalation based on queue state
    pub fn update(&mut self, current_queue_size: usize) {
        if current_queue_size > self.target_size {
            // Increase escalation
            self.escalation_multiplier =
                (self.escalation_multiplier + 100).min(self.max_escalation);
        } else if current_queue_size < self.target_size / 2 {
            // Decrease escalation
            self.escalation_multiplier = (self.escalation_multiplier.saturating_sub(50)).max(1000);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger_entries::AccountRoot;

    #[allow(dead_code)]
    struct MockLedgerView;

    impl LedgerView for MockLedgerView {
        fn get_account_root(&self, _account: &AccountID) -> Option<AccountRoot> {
            // Return a funded account for testing
            let account = AccountID::new([1u8; 20]);
            Some(AccountRoot::new(account).with_balance(1_000_000))
        }

        fn set_account_root(&mut self, _account: &AccountRoot) {}

        fn get_call_state(
            &self,
            _account: &AccountID,
            _issuer: &AccountID,
            _currency: &primitives::Currency,
        ) -> Option<crate::ledger_entries::CallState> {
            None
        }

        fn set_call_state(&mut self, _state: &crate::ledger_entries::CallState) {}

        fn get_offer(&self, _account: &AccountID, _sequence: u32) -> Option<crate::ledger_entries::OfferEntry> {
            None
        }

        fn set_offer(&mut self, _offer: &crate::ledger_entries::OfferEntry) {}

        fn delete_offer(&mut self, _account: &AccountID, _sequence: u32) {}

        fn get_ledger_info(&self) -> crate::ledger::LedgerInfo {
            crate::ledger::LedgerInfo::default()
        }
    }

    #[test]
    fn test_queue_insert_and_retrieve() {
        let mut queue = TransactionQueue::new(100);

        let account = AccountID::new([1u8; 20]);
        let mut tx = Transaction::new_payment(
            account,
            AccountID::new([2u8; 20]),
            serialization::Amount::call(1000),
        );
        tx.set_hash(UInt256::new([4u8; 32]));

        assert!(queue.insert(tx).is_ok());
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn test_queue_duplicate_rejection() {
        let mut queue = TransactionQueue::new(100);

        let account = AccountID::new([1u8; 20]);
        let mut tx = Transaction::new_payment(
            account,
            AccountID::new([2u8; 20]),
            serialization::Amount::call(1000),
        );
        tx.txn_signature = Some(vec![1, 2, 3]);
        tx.set_hash(UInt256::new([3u8; 32]));

        assert!(queue.insert(tx.clone()).is_ok());
        assert_eq!(queue.insert(tx), Err(TER::terDUPLICATE));
    }

    #[test]
    fn test_fee_escalation() {
        let escalation = FeeEscalation::new(10, 100);

        // At target size, should return base fee
        assert_eq!(escalation.required_fee(50), 10);

        // Above target, should escalate
        let fee_2x = escalation.required_fee(200);
        assert!(fee_2x > 10);

        // Much higher should escalate more
        let fee_5x = escalation.required_fee(500);
        assert!(fee_5x > fee_2x);
    }

    #[test]
    fn test_queue_ordering_by_fee() {
        let mut queue = TransactionQueue::new(100);

        let account = AccountID::new([1u8; 20]);

        // Insert low fee transaction
        let mut tx_low = Transaction::new_payment(
            account,
            AccountID::new([2u8; 20]),
            serialization::Amount::call(1000),
        );
        tx_low.fee = 100;
        tx_low.sequence = 1;
        tx_low.txn_signature = Some(vec![1]);
        tx_low.set_hash(UInt256::new([1u8; 32]));
        queue.insert(tx_low).unwrap();

        // Insert high fee transaction
        let mut tx_high = Transaction::new_payment(
            account,
            AccountID::new([2u8; 20]),
            serialization::Amount::call(1000),
        );
        tx_high.fee = 1000;
        tx_high.sequence = 2;
        tx_high.txn_signature = Some(vec![2]);
        tx_high.set_hash(UInt256::new([2u8; 32]));
        queue.insert(tx_high).unwrap();

        // Pop should return highest fee first
        let next = queue.pop_highest_fee().unwrap();
        assert_eq!(next.transaction.fee, 1000);
    }
}
