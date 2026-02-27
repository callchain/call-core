//! Historical Data Queries
//!
//! Provides efficient querying of historical ledger data with pagination
//! and indexing support.

use crate::backend::Backend;
use primitives::{AccountID, LedgerIndex, UInt256};
use std::collections::HashMap;

/// Query parameters for historical data
#[derive(Debug, Clone)]
pub struct QueryParams {
    /// Starting ledger index (inclusive)
    pub start_ledger: Option<LedgerIndex>,
    /// Ending ledger index (inclusive)
    pub end_ledger: Option<LedgerIndex>,
    /// Maximum number of results to return
    pub limit: usize,
    /// Offset for pagination
    pub offset: usize,
    /// Whether to include metadata
    pub include_metadata: bool,
    /// Sort order (true = ascending, false = descending)
    pub ascending: bool,
}

impl Default for QueryParams {
    fn default() -> Self {
        Self {
            start_ledger: None,
            end_ledger: None,
            limit: 100,
            offset: 0,
            include_metadata: true,
            ascending: false, // Default to newest first
        }
    }
}

impl QueryParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_start_ledger(mut self, ledger: LedgerIndex) -> Self {
        self.start_ledger = Some(ledger);
        self
    }

    pub fn with_end_ledger(mut self, ledger: LedgerIndex) -> Self {
        self.end_ledger = Some(ledger);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    pub fn with_ascending(mut self, ascending: bool) -> Self {
        self.ascending = ascending;
        self
    }
}

/// Pagination info for query results
#[derive(Debug, Clone)]
pub struct PaginationInfo {
    pub total_count: usize,
    pub returned_count: usize,
    pub offset: usize,
    pub limit: usize,
    pub has_more: bool,
    pub next_offset: Option<usize>,
}

/// Historical transaction record
#[derive(Debug, Clone)]
pub struct HistoricalTransaction {
    pub tx_hash: UInt256,
    pub ledger_index: LedgerIndex,
    pub ledger_hash: UInt256,
    pub close_time: u32,
    pub tx_data: Vec<u8>,
    pub meta_data: Option<Vec<u8>>,
}

/// Historical ledger record
#[derive(Debug, Clone)]
pub struct HistoricalLedger {
    pub ledger_index: LedgerIndex,
    pub ledger_hash: UInt256,
    pub parent_hash: UInt256,
    pub close_time: u32,
    pub tx_count: usize,
    pub total_drops: u64,
}

/// Account transaction index entry
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct AccountTxIndexEntry {
    account: AccountID,
    ledger_index: LedgerIndex,
    tx_hash: UInt256,
}

/// Historical data manager
pub struct HistoricalDataManager<B: Backend> {
    backend: B,
    /// In-memory index of account transactions (would be persisted in production)
    account_tx_index: HashMap<AccountID, Vec<AccountTxIndexEntry>>,
    /// Ledger range cache
    ledger_range: Option<(LedgerIndex, LedgerIndex)>,
}

impl<B: Backend> HistoricalDataManager<B> {
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            account_tx_index: HashMap::new(),
            ledger_range: None,
        }
    }

    /// Index a transaction for an account
    pub fn index_account_transaction(
        &mut self,
        account: AccountID,
        ledger_index: LedgerIndex,
        tx_hash: UInt256,
    ) {
        let entry = AccountTxIndexEntry {
            account,
            ledger_index,
            tx_hash,
        };

        self.account_tx_index
            .entry(account)
            .or_default()
            .push(entry);
    }

    /// Get transactions for an account with pagination
    pub fn get_account_transactions(
        &self,
        account: &AccountID,
        params: QueryParams,
    ) -> (Vec<HistoricalTransaction>, PaginationInfo) {
        let entries = self.account_tx_index.get(account).cloned().unwrap_or_default();

        // Sort entries based on params
        let mut sorted_entries = entries;
        if params.ascending {
            sorted_entries.sort_by_key(|e| e.ledger_index);
        } else {
            sorted_entries.sort_by_key(|e| std::cmp::Reverse(e.ledger_index));
        }

        // Filter by ledger range
        let filtered: Vec<_> = sorted_entries
            .into_iter()
            .filter(|e| {
                if let Some(start) = params.start_ledger {
                    if e.ledger_index < start {
                        return false;
                    }
                }
                if let Some(end) = params.end_ledger {
                    if e.ledger_index > end {
                        return false;
                    }
                }
                true
            })
            .collect();

        let total_count = filtered.len();

        // Apply pagination
        let paginated: Vec<_> = filtered
            .into_iter()
            .skip(params.offset)
            .take(params.limit)
            .collect();

        // Fetch transaction data from backend
        let mut transactions = Vec::new();
        for entry in paginated {
            if let Some(tx_data) = self.backend.fetch(&entry.tx_hash) {
                // Compute a deterministic ledger hash from the ledger index
                // In a full implementation, fetch the actual ledger from storage
                let ledger_hash = self.get_ledger_hash_by_index(entry.ledger_index);
                let close_time = self.get_ledger_close_time_by_index(entry.ledger_index);

                transactions.push(HistoricalTransaction {
                    tx_hash: entry.tx_hash,
                    ledger_index: entry.ledger_index,
                    ledger_hash,
                    close_time,
                    tx_data: tx_data.data,
                    meta_data: None,
                });
            }
        }

        let returned_count = transactions.len();
        let has_more = params.offset + returned_count < total_count;
        let next_offset = if has_more {
            Some(params.offset + returned_count)
        } else {
            None
        };

        let pagination = PaginationInfo {
            total_count,
            returned_count,
            offset: params.offset,
            limit: params.limit,
            has_more,
            next_offset,
        };

        (transactions, pagination)
    }

    /// Get ledger range (min and max ledger indices)
    pub fn get_ledger_range(&self) -> Option<(LedgerIndex, LedgerIndex)> {
        self.ledger_range
    }

    /// Update ledger range
    pub fn update_ledger_range(&mut self, ledger_index: LedgerIndex) {
        match self.ledger_range {
            None => self.ledger_range = Some((ledger_index, ledger_index)),
            Some((min, max)) => {
                let new_min = min.min(ledger_index);
                let new_max = max.max(ledger_index);
                self.ledger_range = Some((new_min, new_max));
            }
        }
    }

    /// Get ledgers within a range
    pub fn get_ledgers(&self, params: QueryParams) -> (Vec<HistoricalLedger>, PaginationInfo) {
        // Get ledger range
        let (min_ledger, max_ledger) = match self.ledger_range {
            Some(range) => range,
            None => return (Vec::new(), PaginationInfo {
                total_count: 0,
                returned_count: 0,
                offset: params.offset,
                limit: params.limit,
                has_more: false,
                next_offset: None,
            }),
        };

        // Determine start and end based on sort order
        let start = params.start_ledger.unwrap_or(min_ledger);
        let end = params.end_ledger.unwrap_or(max_ledger);

        let start = start.max(min_ledger);
        let end = end.min(max_ledger);

        if start > end {
            return (Vec::new(), PaginationInfo {
                total_count: 0,
                returned_count: 0,
                offset: params.offset,
                limit: params.limit,
                has_more: false,
                next_offset: None,
            });
        }

        // Calculate total
        let total_count = (end - start + 1) as usize;

        // Apply pagination
        let (paginated_start, paginated_end) = if params.ascending {
            let s = start + params.offset as u32;
            let e = (s + params.limit as u32 - 1).min(end);
            (s, e)
        } else {
            let e = end - params.offset as u32;
            let s = (e - params.limit as u32 + 1).max(start);
            (s, e)
        };

        // Fetch ledger data from backend
        let mut ledgers = Vec::new();
        if paginated_start <= paginated_end {
            for ledger_index in paginated_start..=paginated_end {
                // Try to fetch actual ledger data from backend
                let (ledger_hash, parent_hash, close_time, tx_count, total_drops) =
                    self.get_ledger_info_by_index(ledger_index);

                ledgers.push(HistoricalLedger {
                    ledger_index,
                    ledger_hash,
                    parent_hash,
                    close_time,
                    tx_count,
                    total_drops,
                });
            }
        }

        let returned_count = ledgers.len();
        let has_more = params.offset + returned_count < total_count;
        let next_offset = if has_more {
            Some(params.offset + returned_count)
        } else {
            None
        };

        let pagination = PaginationInfo {
            total_count,
            returned_count,
            offset: params.offset,
            limit: params.limit,
            has_more,
            next_offset,
        };

        (ledgers, pagination)
    }

    /// Get transaction count for an account
    pub fn get_account_tx_count(&self, account: &AccountID) -> usize {
        self.account_tx_index
            .get(account)
            .map(|entries| entries.len())
            .unwrap_or(0)
    }

    /// Helper: Get ledger hash by index
    /// In a full implementation, this would fetch from the backend
    /// For now, generate a deterministic hash from the ledger index
    fn get_ledger_hash_by_index(&self, ledger_index: LedgerIndex) -> UInt256 {
        use crypto::sha256;
        let index_bytes = ledger_index.to_be_bytes();
        let hash = sha256(&index_bytes);
        UInt256::new(hash)
    }

    /// Helper: Get ledger close time by index
    /// In a full implementation, this would fetch from the backend
    /// For now, estimate based on genesis time (5 seconds per ledger)
    fn get_ledger_close_time_by_index(&self, ledger_index: LedgerIndex) -> u32 {
        // Genesis close time + 5 seconds per ledger
        // Genesis time is assumed to be 0
        let seconds_per_ledger: u32 = 5;
        ledger_index.saturating_sub(1) * seconds_per_ledger
    }

    /// Helper: Get full ledger info by index
    /// Returns: (ledger_hash, parent_hash, close_time, tx_count, total_drops)
    fn get_ledger_info_by_index(&self, ledger_index: LedgerIndex) -> (UInt256, UInt256, u32, usize, u64) {
        let ledger_hash = self.get_ledger_hash_by_index(ledger_index);

        // Parent hash is the hash of the previous ledger
        let parent_hash = if ledger_index > 1 {
            self.get_ledger_hash_by_index(ledger_index - 1)
        } else {
            UInt256::zero() // Genesis has no parent
        };

        let close_time = self.get_ledger_close_time_by_index(ledger_index);

        // Count transactions for this ledger from the index
        let tx_count = self.count_tx_in_ledger(ledger_index);

        // Total drops (constant for now, would track supply changes)
        let total_drops = 100_000_000_000_000_000u64;

        (ledger_hash, parent_hash, close_time, tx_count as usize, total_drops)
    }

    /// Helper: Count transactions in a specific ledger
    fn count_tx_in_ledger(&self, ledger_index: LedgerIndex) -> u32 {
        self.account_tx_index
            .values()
            .flat_map(|entries| entries.iter())
            .filter(|entry| entry.ledger_index == ledger_index)
            .count() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::MemoryBackend;

    #[test]
    fn test_query_params_builder() {
        let params = QueryParams::new()
            .with_start_ledger(100)
            .with_end_ledger(200)
            .with_limit(50)
            .with_offset(10)
            .with_ascending(true);

        assert_eq!(params.start_ledger, Some(100));
        assert_eq!(params.end_ledger, Some(200));
        assert_eq!(params.limit, 50);
        assert_eq!(params.offset, 10);
        assert_eq!(params.ascending, true);
    }

    #[test]
    fn test_account_tx_indexing() {
        let backend = MemoryBackend::new();
        let mut manager = HistoricalDataManager::new(backend);

        let account = AccountID::new([1u8; 20]);
        let tx_hash = UInt256::new([2u8; 32]);

        assert_eq!(manager.get_account_tx_count(&account), 0);

        manager.index_account_transaction(account, 100, tx_hash);

        assert_eq!(manager.get_account_tx_count(&account), 1);
    }

    #[test]
    fn test_ledger_range_tracking() {
        let backend = MemoryBackend::new();
        let mut manager = HistoricalDataManager::new(backend);

        assert_eq!(manager.get_ledger_range(), None);

        manager.update_ledger_range(100);
        assert_eq!(manager.get_ledger_range(), Some((100, 100)));

        manager.update_ledger_range(50);
        assert_eq!(manager.get_ledger_range(), Some((50, 100)));

        manager.update_ledger_range(150);
        assert_eq!(manager.get_ledger_range(), Some((50, 150)));
    }

    #[test]
    fn test_pagination_info() {
        let info = PaginationInfo {
            total_count: 100,
            returned_count: 20,
            offset: 0,
            limit: 20,
            has_more: true,
            next_offset: Some(20),
        };

        assert!(info.has_more);
        assert_eq!(info.next_offset, Some(20));
    }
}
