use primitives::{AccountID, UInt256};
use serialization::STObject;
use shamap::{SHAMap, SHAMapItem, SHAMapType};

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
    /// Current load factor (1000 = 1.0x, 2000 = 2.0x)
    pub load_factor: u32,
    /// Target ledger close time in seconds
    pub target_ledger_close_time: u32,
}

impl Default for Fees {
    fn default() -> Self {
        Self {
            base: 10,
            units: 10,
            reserve: 10_000_000, // 10 CALL
            increment: 2_000_000, // 2 CALL
            commission: 0,
            load_factor: 1000, // 1.0x base
            target_ledger_close_time: 5, // 5 seconds
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
            load_factor: 1000,
            target_ledger_close_time: 5,
        }
    }

    /// Calculate the transaction fee based on fee units
    pub fn calculate_fee(&self, fee_units: u32) -> u64 {
        let base_fee = (self.base as u128 * fee_units as u128 / self.units as u128) as u64;
        // Apply load factor
        (base_fee as u128 * self.load_factor as u128 / 1000) as u64
    }

    /// Calculate load factor based on transaction queue pressure
    /// Returns load factor in thousandths (1000 = 1.0x)
    pub fn calculate_load_factor(
        tx_count: usize,
        tx_capacity: usize,
        last_ledger_time: u32,
        target_time: u32,
    ) -> u32 {
        if tx_capacity == 0 {
            return 1000;
        }

        // Capacity factor: how full is the queue (0-1000)
        let capacity_factor = ((tx_count as u64 * 1000) / tx_capacity as u64).min(1000) as u32;

        // Time factor: how fast are ledgers closing relative to target
        let time_factor = if last_ledger_time >= target_time {
            // Ledgers closing slower than target, reduce fees
            800
        } else {
            // Ledgers closing faster, increase fees
            1200.min((target_time * 1000) / last_ledger_time.max(1))
        };

        // Combine factors: capacity has 70% weight, time has 30% weight
        let load = (capacity_factor * 700 + time_factor * 300) / 1000;

        // Clamp between 1.0x and 10.0x
        load.max(1000).min(10000)
    }

    /// Update load factor based on network conditions
    pub fn update_load_factor(&mut self, tx_count: usize, tx_capacity: usize, last_ledger_time: u32) {
        self.load_factor = Self::calculate_load_factor(
            tx_count,
            tx_capacity,
            last_ledger_time,
            self.target_ledger_close_time,
        );
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
            hash: UInt256::zero(), // Note: Ledger::genesis() computes the actual hash via update_hashes()
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
pub struct Ledger {
    pub info: LedgerInfo,
    pub transactions: Vec<UInt256>,
    /// State tree containing all ledger entries (AccountRoot, CallState, etc.)
    pub state_tree: SHAMap,
}

impl std::fmt::Debug for Ledger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ledger")
            .field("info", &self.info)
            .field("transactions", &self.transactions)
            .field("state_tree", &"<SHAMap>")
            .finish()
    }
}

impl Ledger {
    pub fn new(info: LedgerInfo) -> Self {
        Self {
            info,
            transactions: Vec::new(),
            state_tree: SHAMap::new(SHAMapType::State),
        }
    }

    pub fn genesis() -> Self {
        let mut ledger = Self::new(LedgerInfo::genesis());
        // Compute the genesis ledger hash
        // The genesis hash is computed from its contents (empty state tree, no transactions)
        ledger.update_hashes();
        ledger
    }

    pub fn create_child(&self, close_time: u32) -> Self {
        let mut child = Self::new(self.info.create_child(close_time));
        // Copy state tree from parent
        for item in self.state_tree.iter() {
            let key = item.key();
            let cloned_item = SHAMapItem::new(key, item.data().to_vec());
            let _ = child.state_tree.add_item(key, cloned_item);
        }
        child
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

    /// Add a state entry to the ledger
    pub fn add_state_entry(&mut self, key: UInt256, data: Vec<u8>) -> bool {
        let item = SHAMapItem::new(key, data);
        self.state_tree.add_item(key, item)
    }

    /// Get a state entry by key
    pub fn get_state_entry(&self, key: &UInt256) -> Option<&SHAMapItem> {
        self.state_tree.get_item(key)
    }

    /// Compute the account hash (state tree root hash)
    pub fn compute_account_hash(&self) -> UInt256 {
        self.state_tree.get_root_hash()
    }

    /// Update the ledger hashes (account_hash and tx_hash)
    /// This should be called before finalizing the ledger
    pub fn update_hashes(&mut self) {
        // Update account hash from state tree
        self.info.account_hash = self.compute_account_hash();

        // Compute transaction tree hash
        self.info.tx_hash = self.compute_tx_hash();

        // Compute overall ledger hash
        self.info.hash = self.compute_ledger_hash();
    }

    /// Compute the transaction tree hash
    fn compute_tx_hash(&self) -> UInt256 {
        use crypto::sha512_half;

        if self.transactions.is_empty() {
            return sha512_half(b"");
        }

        // Build a simple Merkle tree of transactions
        let mut hashes: Vec<UInt256> = self.transactions.clone();

        while hashes.len() > 1 {
            let mut next_level = Vec::new();
            for chunk in hashes.chunks(2) {
                let mut data = Vec::new();
                data.extend_from_slice(chunk[0].as_bytes());
                if chunk.len() > 1 {
                    data.extend_from_slice(chunk[1].as_bytes());
                } else {
                    // Duplicate the last hash if odd number
                    data.extend_from_slice(chunk[0].as_bytes());
                }
                next_level.push(sha512_half(&data));
            }
            hashes = next_level;
        }

        hashes[0]
    }

    /// Compute the overall ledger hash
    fn compute_ledger_hash(&self) -> UInt256 {
        use crypto::sha512_half;

        // Ledger hash includes key fields from LedgerInfo
        let mut data = Vec::new();
        data.extend_from_slice(&self.info.seq.to_be_bytes());
        data.extend_from_slice(self.info.parent_hash.as_bytes());
        data.extend_from_slice(self.info.tx_hash.as_bytes());
        data.extend_from_slice(self.info.account_hash.as_bytes());
        data.extend_from_slice(&self.info.close_time.to_be_bytes());
        data.extend_from_slice(&self.info.parent_close_time.to_be_bytes());
        data.extend_from_slice(&self.info.close_time_resolution.to_be_bytes());
        data.extend_from_slice(&self.info.drops.to_be_bytes());
        data.extend_from_slice(&self.info.close_flags.to_be_bytes());

        sha512_half(&data)
    }

    /// Iterate over all state items in the ledger
    pub fn items(&self) -> impl Iterator<Item = (UInt256, STObject)> + '_ {
        self.state_tree.iter().filter_map(|item| {
            let key = item.key();
            // Parse the serialized data into STObject
            let mut iter = serialization::SerialIter::new(item.data());
            match iter.get_object() {
                Ok(obj) => Some((key, obj)),
                Err(_) => None,
            }
        })
    }
}

impl ReadView for Ledger {
    fn get_ledger_info(&self) -> &LedgerInfo {
        &self.info
    }

    fn read(&self, key: &UInt256) -> Option<STObject> {
        self.get_state_entry(key).and_then(|item| {
            let mut iter = serialization::SerialIter::new(item.data());
            iter.get_object().ok()
        })
    }

    fn items(&self) -> Box<dyn Iterator<Item = (UInt256, STObject)> + '_> {
        Box::new(self.items())
    }

    fn transactions(&self) -> Box<dyn Iterator<Item = UInt256> + '_> {
        Box::new(self.transactions.clone().into_iter())
    }

    fn has_transaction(&self, tx_hash: &UInt256) -> bool {
        self.transactions.contains(tx_hash)
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
        // Collect all items from base, applying changes
        let mut items: Vec<(UInt256, STObject)> = Vec::new();
        let mut changed_keys = std::collections::HashSet::new();

        // Add all items from changes that are Some (inserts/updates)
        for (key, value) in &self.changes {
            changed_keys.insert(*key);
            if let Some(obj) = value {
                items.push((*key, obj.clone()));
            }
        }

        // Add base items that haven't been changed
        for (key, obj) in self.base.items() {
            if !changed_keys.contains(&key) {
                items.push((key, obj));
            }
        }

        Box::new(items.into_iter())
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

    #[test]
    fn test_ledger_items() {
        use serialization::types::sf;
        let mut ledger = Ledger::genesis();

        // Initially empty (genesis has no state)
        let items: Vec<_> = ledger.items().collect();
        assert!(items.is_empty());

        // Add a state entry
        let key = UInt256::new([1u8; 32]);
        let mut obj = STObject::new();
        obj.insert(sf::ACCOUNT, serialization::STValue::Account(AccountID::new([2u8; 20])));

        let mut serializer = serialization::Serializer::new();
        serializer.add_object(&obj).unwrap();
        let data = serializer.finish();

        ledger.add_state_entry(key, data);

        // Now should have one item
        let items: Vec<_> = ledger.items().collect();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].0, key);
    }

    #[test]
    fn test_read_view_ledger() {
        use serialization::types::sf;
        let mut ledger = Ledger::genesis();

        // Add a state entry
        let key = UInt256::new([1u8; 32]);
        let mut obj = STObject::new();
        obj.insert(sf::ACCOUNT, serialization::STValue::Account(AccountID::new([2u8; 20])));

        let mut serializer = serialization::Serializer::new();
        serializer.add_object(&obj).unwrap();
        let data = serializer.finish();

        ledger.add_state_entry(key, data);

        // Test ReadView implementation
        let items: Vec<_> = ledger.items().collect();
        assert_eq!(items.len(), 1);

        // Test read method
        let read_obj = ledger.read(&key);
        assert!(read_obj.is_some());
    }

    #[test]
    fn test_open_view_items() {
        use serialization::types::sf;
        let mut ledger = Ledger::genesis();

        // Add initial state to ledger
        let key1 = UInt256::new([1u8; 32]);
        let mut obj1 = STObject::new();
        obj1.insert(sf::ACCOUNT, serialization::STValue::Account(AccountID::new([1u8; 20])));
        let mut serializer = serialization::Serializer::new();
        serializer.add_object(&obj1).unwrap();
        ledger.add_state_entry(key1, serializer.finish());

        // Create OpenView on top of ledger
        let mut open_view = OpenView::new(&ledger);

        // Initially should have the base item
        let items: Vec<_> = open_view.items().collect();
        assert_eq!(items.len(), 1);

        // Insert a new item
        let key2 = UInt256::new([2u8; 32]);
        let mut obj2 = STObject::new();
        obj2.insert(sf::ACCOUNT, serialization::STValue::Account(AccountID::new([2u8; 20])));
        open_view.insert(key2, obj2.clone());

        // Now should have two items
        let items: Vec<_> = open_view.items().collect();
        assert_eq!(items.len(), 2);

        // Erase the first item
        open_view.erase(key1);

        // Now should have one item (the new one)
        let items: Vec<_> = open_view.items().collect();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].0, key2);
    }
}
