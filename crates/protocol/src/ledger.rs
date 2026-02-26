use primitives::{AccountID, UInt256};
use serialization::STObject;

/// LedgerIndex is the sequence number of a ledger
pub type LedgerIndex = u32;

/// Fees structure for transaction costs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fees {
    /// Base fee for a transaction (drops)
    pub base: u64,
    /// Fee units for reference transaction
    pub units: u32,
    /// Reserve base (account minimum balance)
    pub reserve: u64,
    /// Reserve increment per owned object
    pub increment: u64,
    /// Commission rate (custom to Callchain)
    pub commission: u32,
}

impl Default for Fees {
    fn default() -> Self {
        Self {
            base: 10,
            units: 10,
            reserve: 10_000_000, // 10 CALL
            increment: 2_000_000, // 2 CALL
            commission: 0,
        }
    }
}

impl Fees {
    pub fn new(base: u64, reserve: u64, increment: u64) -> Self {
        Self {
            base,
            units: 10,
            reserve,
            increment,
            commission: 0,
        }
    }

    /// Calculate the transaction fee based on fee units
    pub fn calculate_fee(&self, fee_units: u32) -> u64 {
        (self.base as u128 * fee_units as u128 / self.units as u128) as u64
    }
}

/// LedgerInfo contains the header information for a ledger
#[derive(Debug, Clone)]
pub struct LedgerInfo {
    /// Ledger sequence number
    pub seq: LedgerIndex,
    /// Hash of this ledger
    pub hash: UInt256,
    /// Hash of the parent ledger
    pub parent_hash: UInt256,
    /// Hash of the transaction tree
    pub tx_hash: UInt256,
    /// Hash of the account state tree
    pub account_hash: UInt256,
    /// Close time (seconds since Callchain epoch)
    pub close_time: u32,
    /// Parent ledger close time
    pub parent_close_time: u32,
    /// Resolution of close time (seconds)
    pub close_time_resolution: u8,
    /// Total CALL drops in existence
    pub drops: u64,
    /// Close flags
    pub close_flags: u8,
}

impl Default for LedgerInfo {
    fn default() -> Self {
        Self {
            seq: 0,
            hash: UInt256::zero(),
            parent_hash: UInt256::zero(),
            tx_hash: UInt256::zero(),
            account_hash: UInt256::zero(),
            close_time: 0,
            parent_close_time: 0,
            close_time_resolution: 10,
            drops: 100_000_000_000_000_000, // 100 billion CALL
            close_flags: 0,
        }
    }
}

impl LedgerInfo {
    /// Create new ledger info for the genesis ledger
    pub fn genesis() -> Self {
        Self {
            seq: 1,
            hash: UInt256::zero(), // TODO: compute actual genesis hash
            parent_hash: UInt256::zero(),
            tx_hash: UInt256::zero(),
            account_hash: UInt256::zero(),
            close_time: 0,
            parent_close_time: 0,
            close_time_resolution: 10,
            drops: 100_000_000_000_000_000,
            close_flags: 0,
        }
    }

    /// Create a child ledger from this parent
    pub fn create_child(&self, close_time: u32) -> Self {
        Self {
            seq: self.seq + 1,
            hash: UInt256::zero(), // Will be computed
            parent_hash: self.hash,
            tx_hash: UInt256::zero(),
            account_hash: UInt256::zero(),
            close_time,
            parent_close_time: self.close_time,
            close_time_resolution: self.close_time_resolution,
            drops: self.drops,
            close_flags: 0,
        }
    }
}

/// Ledger represents a complete ledger with state and transactions
#[derive(Debug, Clone)]
pub struct Ledger {
    pub info: LedgerInfo,
    pub transactions: Vec<UInt256>,
}

impl Ledger {
    pub fn new(info: LedgerInfo) -> Self {
        Self {
            info,
            transactions: Vec::new(),
        }
    }

    pub fn genesis() -> Self {
        Self::new(LedgerInfo::genesis())
    }

    pub fn create_child(&self, close_time: u32) -> Self {
        Self::new(self.info.create_child(close_time))
    }

    pub fn get_hash(&self) -> UInt256 {
        self.info.hash
    }

    pub fn get_seq(&self) -> LedgerIndex {
        self.info.seq
    }

    pub fn add_transaction(&mut self, tx_hash: UInt256) {
        self.transactions.push(tx_hash);
    }

    pub fn transaction_count(&self) -> usize {
        self.transactions.len()
    }
}

/// LedgerView provides read-only access to ledger data
pub trait ReadView {
    /// Get the ledger info
    fn get_ledger_info(&self) -> &LedgerInfo;

    /// Lookup a state item by key
    fn read(&self, key: &UInt256) -> Option<STObject>;

    /// Iterate over all state items
    fn items(&self) -> Box<dyn Iterator<Item = (UInt256, STObject)> + '_>;

    /// Iterate over transactions
    fn transactions(&self) -> Box<dyn Iterator<Item = UInt256> + '_>;

    /// Check if ledger has transaction
    fn has_transaction(&self, tx_hash: &UInt256) -> bool;
}

/// OpenView accumulates changes to be applied to a base ledger
pub struct OpenView<'a> {
    base: &'a dyn ReadView,
    changes: std::collections::BTreeMap<UInt256, Option<STObject>>,
    transactions: Vec<UInt256>,
}

impl<'a> std::fmt::Debug for OpenView<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenView")
            .field("changes", &self.changes)
            .field("transactions", &self.transactions)
            .finish()
    }
}

impl<'a> OpenView<'a> {
    pub fn new(base: &'a dyn ReadView) -> Self {
        Self {
            base,
            changes: std::collections::BTreeMap::new(),
            transactions: Vec::new(),
        }
    }

    pub fn insert(&mut self, key: UInt256, value: STObject) {
        self.changes.insert(key, Some(value));
    }

    pub fn erase(&mut self, key: UInt256) {
        self.changes.insert(key, None);
    }

    pub fn apply_transaction(&mut self, tx_hash: UInt256) {
        self.transactions.push(tx_hash);
    }

    pub fn get_changes(&self) -> &std::collections::BTreeMap<UInt256, Option<STObject>> {
        &self.changes
    }
}

impl<'a> ReadView for OpenView<'a> {
    fn get_ledger_info(&self) -> &LedgerInfo {
        self.base.get_ledger_info()
    }

    fn read(&self, key: &UInt256) -> Option<STObject> {
        match self.changes.get(key) {
            Some(Some(value)) => Some(value.clone()),
            Some(None) => None,
            None => self.base.read(key),
        }
    }

    fn items(&self) -> Box<dyn Iterator<Item = (UInt256, STObject)> + '_> {
        // Combine base items with changes
        Box::new(std::iter::empty()) // Simplified for now
    }

    fn transactions(&self) -> Box<dyn Iterator<Item = UInt256> + '_> {
        Box::new(self.transactions.clone().into_iter())
    }

    fn has_transaction(&self, tx_hash: &UInt256) -> bool {
        self.transactions.contains(tx_hash) || self.base.has_transaction(tx_hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fees_default() {
        let fees = Fees::default();
        assert_eq!(fees.base, 10);
        assert_eq!(fees.reserve, 10_000_000);
        assert_eq!(fees.calculate_fee(10), 10);
    }

    #[test]
    fn test_ledger_info_genesis() {
        let genesis = LedgerInfo::genesis();
        assert_eq!(genesis.seq, 1);
        assert_eq!(genesis.parent_hash, UInt256::zero());
        assert_eq!(genesis.drops, 100_000_000_000_000_000);
    }

    #[test]
    fn test_ledger_create_child() {
        let parent = Ledger::genesis();
        let child = parent.create_child(100);
        assert_eq!(child.get_seq(), 2);
        assert_eq!(child.info.parent_hash, parent.info.hash);
        assert_eq!(child.info.parent_close_time, parent.info.close_time);
    }

    #[test]
    fn test_ledger_transactions() {
        let mut ledger = Ledger::genesis();
        let tx_hash = UInt256::new([1u8; 32]);
        ledger.add_transaction(tx_hash);
        assert_eq!(ledger.transaction_count(), 1);
    }
}
