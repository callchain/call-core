//! Ledger state entries for account and object storage
//!
//! This module defines the ledger entry types that represent the state of the ledger.
//! Each entry type corresponds to a specific type of object stored in the SHAMap.

use primitives::{AccountID, Currency, UInt256};
use serialization::{Amount, STObject};

/// Ledger entry types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerEntryType {
    AccountRoot = 0x61, // 'a'
    CallState = 0x63,   // 'c' - trust line
    Offer = 0x6F,       // 'o'
    DirectoryNode = 0x64, // 'd'
    FeeRoot = 0x46,     // 'F' - custom to Callchain
    IssueRoot = 0x69,   // 'i' - custom to Callchain
    Invoice = 0x76,     // 'v' - custom to Callchain
}

impl LedgerEntryType {
    pub fn from_u16(v: u16) -> Option<Self> {
        match v {
            0x61 => Some(Self::AccountRoot),
            0x63 => Some(Self::CallState),
            0x6F => Some(Self::Offer),
            0x64 => Some(Self::DirectoryNode),
            0x46 => Some(Self::FeeRoot),
            0x69 => Some(Self::IssueRoot),
            0x76 => Some(Self::Invoice),
            _ => None,
        }
    }

    pub fn as_u16(&self) -> u16 {
        *self as u16
    }
}

/// Base trait for all ledger entries
pub trait LedgerEntry: Clone {
    fn entry_type() -> LedgerEntryType
    where
        Self: Sized;

    fn ledger_index(&self) -> UInt256;
    fn to_stobject(&self) -> STObject;
    fn from_stobject(obj: &STObject) -> Option<Self>
    where
        Self: Sized;
}

/// AccountRoot entry - represents an account in the ledger
#[derive(Debug, Clone)]
pub struct AccountRoot {
    pub account: AccountID,
    pub balance: Amount,
    pub sequence: u32,
    pub owner_count: u32,
    pub previous_txn_id: UInt256,
    pub previous_txn_lgr_seq: u32,
    pub account_txn_id: Option<UInt256>,
    pub wallet_locator: Option<UInt256>,
    pub wallet_size: Option<u32>,
    pub message_key: Option<Vec<u8>>,
    pub domain: Option<Vec<u8>>,
    pub transfer_rate: Option<u32>,
    pub code_garage: Option<u32>,
    pub email_hash: Option<UInt256>,
    pub regular_key: Option<AccountID>,
    pub tick_size: Option<u8>,
}

impl AccountRoot {
    pub fn new(account: AccountID) -> Self {
        Self {
            account,
            balance: Amount::call(0),
            sequence: 1,
            owner_count: 0,
            previous_txn_id: UInt256::zero(),
            previous_txn_lgr_seq: 0,
            account_txn_id: None,
            wallet_locator: None,
            wallet_size: None,
            message_key: None,
            domain: None,
            transfer_rate: None,
            code_garage: None,
            email_hash: None,
            regular_key: None,
            tick_size: None,
        }
    }

    pub fn with_balance(mut self, drops: u64) -> Self {
        self.balance = Amount::call(drops);
        self
    }

    pub fn increment_sequence(&mut self) {
        self.sequence += 1;
    }

    pub fn add_owner_count(&mut self, delta: u32) {
        self.owner_count += delta;
    }

    pub fn subtract_owner_count(&mut self, delta: u32) {
        self.owner_count = self.owner_count.saturating_sub(delta);
    }

    pub fn update_previous_txn(&mut self, txn_id: UInt256, lgr_seq: u32) {
        self.previous_txn_id = txn_id;
        self.previous_txn_lgr_seq = lgr_seq;
    }
}

impl LedgerEntry for AccountRoot {
    fn entry_type() -> LedgerEntryType {
        LedgerEntryType::AccountRoot
    }

    fn ledger_index(&self) -> UInt256 {
        // AccountRoot index is the account ID (padded/truncated as needed)
        let bytes = self.account.as_bytes();
        let mut index = [0u8; 32];
        index[12..32].copy_from_slice(bytes);
        UInt256::from_be_bytes(index)
    }

    fn to_stobject(&self) -> STObject {
        let mut obj = STObject::new();
        // In a real implementation, serialize all fields
        obj
    }

    fn from_stobject(_obj: &STObject) -> Option<Self> {
        // In a real implementation, deserialize all fields
        None
    }
}

/// CallState entry - represents a trust line between accounts
#[derive(Debug, Clone)]
pub struct CallState {
    pub account: AccountID,
    pub issuer: AccountID,
    pub currency: Currency,
    pub balance: Amount,
    pub limit: Amount,
    pub limit_peer: Amount,
    pub quality_in: Option<u32>,
    pub quality_out: Option<u32>,
    pub previous_txn_id: UInt256,
    pub previous_txn_lgr_seq: u32,
    pub low_node: Option<UInt256>,
    pub high_node: Option<UInt256>,
    pub low_quality_in: Option<u32>,
    pub high_quality_in: Option<u32>,
    pub low_quality_out: Option<u32>,
    pub high_quality_out: Option<u32>,
}

impl CallState {
    pub fn new(account: AccountID, issuer: AccountID, currency: Currency) -> Self {
        Self {
            account,
            issuer,
            currency,
            balance: Amount::issued(0, 0, currency, issuer).unwrap_or_else(|| Amount::call(0)),
            limit: Amount::issued(0, 0, currency, issuer).unwrap_or_else(|| Amount::call(0)),
            limit_peer: Amount::issued(0, 0, currency, issuer).unwrap_or_else(|| Amount::call(0)),
            quality_in: None,
            quality_out: None,
            previous_txn_id: UInt256::zero(),
            previous_txn_lgr_seq: 0,
            low_node: None,
            high_node: None,
            low_quality_in: None,
            high_quality_in: None,
            low_quality_out: None,
            high_quality_out: None,
        }
    }

    pub fn ledger_index(&self) -> UInt256 {
        // CallState index is hash of (currency, account, issuer) or (currency, issuer, account)
        // depending on which is "low" and which is "high"
        let mut data = Vec::new();
        data.extend_from_slice(self.currency.as_bytes());
        data.extend_from_slice(self.account.as_bytes());
        data.extend_from_slice(self.issuer.as_bytes());

        // In a real implementation, use a proper hash function
        // For now, return a placeholder
        UInt256::zero()
    }

    pub fn is_frozen(&self) -> bool {
        // Check if the trust line is frozen
        false
    }

    pub fn is_authorized(&self) -> bool {
        // Check if the trust line is authorized
        true
    }
}

/// Offer entry - represents a DEX offer in the ledger
#[derive(Debug, Clone)]
pub struct OfferEntry {
    pub account: AccountID,
    pub sequence: u32,
    pub taker_pays: Amount,
    pub taker_gets: Amount,
    pub book_directory: UInt256,
    pub book_node: UInt256,
    pub owner_node: UInt256,
    pub previous_txn_id: UInt256,
    pub previous_txn_lgr_seq: u32,
    pub expiration: Option<u32>,
}

impl OfferEntry {
    pub fn new(account: AccountID, sequence: u32, taker_pays: Amount, taker_gets: Amount) -> Self {
        Self {
            account,
            sequence,
            taker_pays,
            taker_gets,
            book_directory: UInt256::zero(),
            book_node: UInt256::zero(),
            owner_node: UInt256::zero(),
            previous_txn_id: UInt256::zero(),
            previous_txn_lgr_seq: 0,
            expiration: None,
        }
    }

    pub fn ledger_index(&self) -> UInt256 {
        // Offer index is hash of (account, sequence)
        let mut data = Vec::new();
        data.extend_from_slice(self.account.as_bytes());
        data.extend_from_slice(&self.sequence.to_be_bytes());

        // In a real implementation, use a proper hash function
        UInt256::zero()
    }

    pub fn quality(&self) -> f64 {
        // Calculate offer quality (exchange rate)
        if self.taker_gets.is_zero() {
            return 0.0;
        }

        let pays_val = self.taker_pays.mantissa as f64 * 10f64.powi(self.taker_pays.exponent);
        let gets_val = self.taker_gets.mantissa as f64 * 10f64.powi(self.taker_gets.exponent);

        pays_val / gets_val
    }

    pub fn is_expired(&self, current_time: u32) -> bool {
        match self.expiration {
            Some(exp) => current_time >= exp,
            None => false,
        }
    }
}

/// DirectoryNode entry - for indexing offers and other entries
#[derive(Debug, Clone)]
pub struct DirectoryNode {
    pub owner: Option<AccountID>,
    pub taker_pays_currency: Option<Currency>,
    pub taker_pays_issuer: Option<AccountID>,
    pub taker_gets_currency: Option<Currency>,
    pub taker_gets_issuer: Option<AccountID>,
    pub indexes: Vec<UInt256>,
    pub root_index: UInt256,
    pub index_next: Option<u64>,
    pub index_previous: Option<u64>,
}

impl DirectoryNode {
    pub fn new(root_index: UInt256) -> Self {
        Self {
            owner: None,
            taker_pays_currency: None,
            taker_pays_issuer: None,
            taker_gets_currency: None,
            taker_gets_issuer: None,
            indexes: Vec::new(),
            root_index,
            index_next: None,
            index_previous: None,
        }
    }

    pub fn add_entry(&mut self, index: UInt256) {
        if !self.indexes.contains(&index) {
            self.indexes.push(index);
        }
    }

    pub fn remove_entry(&mut self, index: &UInt256) -> bool {
        if let Some(pos) = self.indexes.iter().position(|i| i == index) {
            self.indexes.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn is_empty(&self) -> bool {
        self.indexes.is_empty()
    }
}

/// LedgerStateManager - manages all ledger state
#[derive(Debug, Clone)]
pub struct LedgerState {
    // Maps ledger entry indices to their serialized data
    // In a real implementation, this would interface with the SHAMap
}

impl LedgerState {
    pub fn new() -> Self {
        Self {}
    }

    pub fn get_account_root(&self, _account: &AccountID) -> Option<AccountRoot> {
        // In a real implementation, fetch from SHAMap
        None
    }

    pub fn set_account_root(&mut self, _account_root: &AccountRoot) {
        // In a real implementation, store in SHAMap
    }

    pub fn get_call_state(&self, _account: &AccountID, _issuer: &AccountID, _currency: &Currency) -> Option<CallState> {
        // In a real implementation, fetch from SHAMap
        None
    }

    pub fn set_call_state(&mut self, _call_state: &CallState) {
        // In a real implementation, store in SHAMap
    }

    pub fn get_offer(&self, _account: &AccountID, _sequence: u32) -> Option<OfferEntry> {
        // In a real implementation, fetch from SHAMap
        None
    }

    pub fn set_offer(&mut self, _offer: &OfferEntry) {
        // In a real implementation, store in SHAMap
    }

    pub fn delete_offer(&mut self, _account: &AccountID, _sequence: u32) {
        // In a real implementation, remove from SHAMap
    }
}

impl Default for LedgerState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_root_creation() {
        let account = AccountID::new([1u8; 20]);
        let account_root = AccountRoot::new(account)
            .with_balance(1_000_000); // 1 CALL

        assert_eq!(account_root.account, account);
        assert_eq!(account_root.sequence, 1);
        assert_eq!(account_root.balance.mantissa, 1_000_000);
    }

    #[test]
    fn test_account_root_sequence() {
        let account = AccountID::new([1u8; 20]);
        let mut account_root = AccountRoot::new(account);

        account_root.increment_sequence();
        assert_eq!(account_root.sequence, 2);

        account_root.increment_sequence();
        assert_eq!(account_root.sequence, 3);
    }

    #[test]
    fn test_call_state_creation() {
        let account = AccountID::new([1u8; 20]);
        let issuer = AccountID::new([2u8; 20]);
        let currency = Currency::new([3u8; 20]);

        let call_state = CallState::new(account, issuer, currency);

        assert_eq!(call_state.account, account);
        assert_eq!(call_state.issuer, issuer);
        assert_eq!(call_state.currency, currency);
    }

    #[test]
    fn test_offer_entry_quality() {
        let account = AccountID::new([1u8; 20]);
        let taker_pays = Amount::call(2000000); // 2 CALL
        let taker_gets = Amount::call(1000000); // 1 CALL

        let offer = OfferEntry::new(account, 1, taker_pays, taker_gets);

        assert_eq!(offer.quality(), 2.0); // 2:1 ratio
    }

    #[test]
    fn test_directory_node() {
        let root_index = UInt256::from_be_bytes([1u8; 32]);
        let mut dir = DirectoryNode::new(root_index);

        assert!(dir.is_empty());

        let entry1 = UInt256::from_be_bytes([2u8; 32]);
        let entry2 = UInt256::from_be_bytes([3u8; 32]);

        dir.add_entry(entry1);
        dir.add_entry(entry2);

        assert_eq!(dir.indexes.len(), 2);

        dir.remove_entry(&entry1);
        assert_eq!(dir.indexes.len(), 1);

        dir.remove_entry(&entry2);
        assert!(dir.is_empty());
    }

    #[test]
    fn test_ledger_entry_types() {
        assert_eq!(LedgerEntryType::AccountRoot.as_u16(), 0x61);
        assert_eq!(LedgerEntryType::CallState.as_u16(), 0x63);
        assert_eq!(LedgerEntryType::Offer.as_u16(), 0x6F);
        assert_eq!(LedgerEntryType::DirectoryNode.as_u16(), 0x64);
    }
}
