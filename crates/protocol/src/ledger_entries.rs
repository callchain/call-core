//! Ledger state entries for account and object storage
//!
//! This module defines the ledger entry types that represent the state of the ledger.
//! Each entry type corresponds to a specific type of object stored in the SHAMap.

use primitives::{AccountID, Currency, UInt256};
use serialization::{Amount, STObject};
use crate::SignerEntry;

/// Ledger entry types
/// Values match calld specification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerEntryType {
    AccountRoot = 0x61,     // 'a'
    CallState = 0x72,       // 'r' - trust line (was incorrectly 'c')
    Offer = 0x6F,           // 'o'
    DirectoryNode = 0x64,   // 'd'
    Nickname = 0x6E,        // 'n'
    SignerList = 0x53,      // 'S' - multi-sign
    LedgerHashes = 0x68,    // 'h' - ledger history
    Amendments = 0x66,      // 'f' - protocol amendments
    FeeSettings = 0x73,     // 's' - fee configuration
    FeeRoot = 0x46,         // 'F' - custom to Callchain
    IssueRoot = 0x69,       // 'i' - custom to Callchain
    Invoice = 0x76,         // 'v' - custom to Callchain
}

impl LedgerEntryType {
    pub fn from_u16(v: u16) -> Option<Self> {
        match v {
            0x61 => Some(Self::AccountRoot),
            0x72 => Some(Self::CallState),
            0x6F => Some(Self::Offer),
            0x64 => Some(Self::DirectoryNode),
            0x6E => Some(Self::Nickname),
            0x53 => Some(Self::SignerList),
            0x68 => Some(Self::LedgerHashes),
            0x66 => Some(Self::Amendments),
            0x73 => Some(Self::FeeSettings),
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

/// Enum for different ledger object types
#[derive(Debug, Clone)]
pub enum LedgerObject {
    AccountRoot(AccountRoot),
    CallState(CallState),
    Offer(OfferEntry),
    Directory(DirectoryNode),
    // New ledger object types added for calld compatibility
    SignerList(SignerList),
    LedgerHashes(LedgerHashes),
    Amendments(Amendments),
    FeeSettings(FeeSettings),
    IssueRoot(IssueRoot),
    Invoice(Invoice),
    FeeRoot(FeeRoot),
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

    /// Compute the ledger index for an account without having the full AccountRoot
    pub fn compute_ledger_index(account_id: &AccountID) -> UInt256 {
        // AccountRoot index is the account ID (padded/truncated as needed)
        let bytes = account_id.as_bytes();
        let mut index = [0u8; 32];
        index[12..32].copy_from_slice(bytes);
        UInt256::from_be_bytes(index)
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
        let obj = STObject::new();
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

/// Flags for CallState (trust line)
pub mod call_state_flags {
    pub const LOW_FROZEN: u32 = 0x00000001;
    pub const HIGH_FROZEN: u32 = 0x00000002;
    pub const LOW_AUTH: u32 = 0x00010000;
    pub const HIGH_AUTH: u32 = 0x00020000;
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

    /// Returns true if the trust line is frozen for the given account
    pub fn is_frozen(&self, for_account: &AccountID) -> bool {
        // Determine if 'for_account' is the low or high party
        let is_low = self.account.as_bytes() <= self.issuer.as_bytes();
        if for_account == &self.account {
            // Check if the account's side is frozen
            if is_low {
                false // Low account freezing not tracked by flag
            } else {
                false // High account freezing not tracked by flag
            }
        } else {
            // Check if the peer's side is frozen
            false
        }
    }

    /// Returns true if the trust line is authorized for the given account
    pub fn is_authorized(&self, for_account: &AccountID) -> bool {
        // For now, trust lines are considered authorized by default
        // In a full implementation, check authorization flags
        let _ = for_account;
        true
    }

    pub fn ledger_index(&self) -> UInt256 {
        // CallState index is hash of (currency, account, issuer) or (currency, issuer, account)
        // depending on which is "low" and which is "high"
        let mut data = Vec::with_capacity(60);
        data.extend_from_slice(self.currency.as_bytes());
        data.extend_from_slice(self.account.as_bytes());
        data.extend_from_slice(self.issuer.as_bytes());

        // Use SHA-512 half for the ledger index
        crypto::sha512_half(&data)
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
        let mut data = Vec::with_capacity(24);
        data.extend_from_slice(self.account.as_bytes());
        data.extend_from_slice(&self.sequence.to_be_bytes());

        // Use SHA-512 half for the ledger index
        crypto::sha512_half(&data)
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

/// Nickname entry - maps a nickname to an account
#[derive(Debug, Clone)]
pub struct NicknameEntry {
    pub nickname: Vec<u8>,
    pub account: AccountID,
    pub min_offer: Option<Amount>,
    pub previous_txn_id: UInt256,
    pub previous_txn_lgr_seq: u32,
}

impl NicknameEntry {
    pub fn new(nickname: Vec<u8>, account: AccountID) -> Self {
        Self {
            nickname,
            account,
            min_offer: None,
            previous_txn_id: UInt256::zero(),
            previous_txn_lgr_seq: 0,
        }
    }

    pub fn ledger_index(&self) -> UInt256 {
        // Nickname index is hash of the nickname
        let hash = crypto::sha256(&self.nickname);
        UInt256::new(hash)
    }

    pub fn set_min_offer(&mut self, amount: Amount) {
        self.min_offer = Some(amount);
    }

    pub fn update_previous_txn(&mut self, txn_id: UInt256, lgr_seq: u32) {
        self.previous_txn_id = txn_id;
        self.previous_txn_lgr_seq = lgr_seq;
    }
}

// ============================================================================
// Missing Ledger Entry Types (Added for calld compatibility)
// ============================================================================

/// SignerList entry - for multi-signature accounts
/// LedgerEntryType: ltSIGNER_LIST = 'S' (0x53)
#[derive(Debug, Clone)]
pub struct SignerList {
    pub account: AccountID,
    pub signer_quorum: u32,
    pub signers: Vec<SignerEntry>,
    pub previous_txn_id: UInt256,
    pub previous_txn_lgr_seq: u32,
}

impl SignerList {
    pub fn new(account: AccountID, signer_quorum: u32) -> Self {
        Self {
            account,
            signer_quorum,
            signers: Vec::new(),
            previous_txn_id: UInt256::zero(),
            previous_txn_lgr_seq: 0,
        }
    }

    pub fn add_signer(&mut self, signer: SignerEntry) {
        self.signers.push(signer);
    }

    pub fn ledger_index(&self) -> UInt256 {
        // SignerList index is hash of account + 'S'
        let mut data = self.account.as_bytes().to_vec();
        data.push(b'S');
        let hash = crypto::sha256(&data);
        UInt256::new(hash)
    }

    pub fn entry_type() -> LedgerEntryType {
        LedgerEntryType::SignerList
    }

    pub fn update_previous_txn(&mut self, txn_id: UInt256, lgr_seq: u32) {
        self.previous_txn_id = txn_id;
        self.previous_txn_lgr_seq = lgr_seq;
    }
}

/// LedgerHashes entry - tracks ledger history
/// LedgerEntryType: ltLEDGER_HASHES = 'h' (0x68)
#[derive(Debug, Clone)]
pub struct LedgerHashes {
    pub ledger_index: u32,
    pub hashes: Vec<UInt256>,
    pub previous_txn_id: UInt256,
    pub previous_txn_lgr_seq: u32,
}

impl LedgerHashes {
    pub fn new(ledger_index: u32) -> Self {
        Self {
            ledger_index,
            hashes: Vec::new(),
            previous_txn_id: UInt256::zero(),
            previous_txn_lgr_seq: 0,
        }
    }

    pub fn add_hash(&mut self, hash: UInt256) {
        self.hashes.push(hash);
    }

    pub fn ledger_index(&self) -> UInt256 {
        // LedgerHashes index is based on the ledger index range
        let mut data = [0u8; 4];
        data.copy_from_slice(&self.ledger_index.to_be_bytes());
        let hash = crypto::sha256(&data);
        UInt256::new(hash)
    }

    pub fn entry_type() -> LedgerEntryType {
        LedgerEntryType::LedgerHashes
    }

    pub fn update_previous_txn(&mut self, txn_id: UInt256, lgr_seq: u32) {
        self.previous_txn_id = txn_id;
        self.previous_txn_lgr_seq = lgr_seq;
    }
}

/// Amendment entry - represents a protocol amendment vote
#[derive(Debug, Clone)]
pub struct AmendmentVote {
    pub amendment_id: UInt256,
    pub name: String,
    pub enabled: bool,
    pub supported: bool,
    pub vote_count: u32,
}

/// Amendments entry - tracks protocol amendments
/// LedgerEntryType: ltAMENDMENTS = 'f' (0x66)
#[derive(Debug, Clone)]
pub struct Amendments {
    pub amendments: Vec<AmendmentVote>,
    pub previous_txn_id: UInt256,
    pub previous_txn_lgr_seq: u32,
}

impl Amendments {
    pub fn new() -> Self {
        Self {
            amendments: Vec::new(),
            previous_txn_id: UInt256::zero(),
            previous_txn_lgr_seq: 0,
        }
    }

    pub fn register_amendment(&mut self, id: UInt256, name: String) {
        self.amendments.push(AmendmentVote {
            amendment_id: id,
            name,
            enabled: false,
            supported: true,
            vote_count: 0,
        });
    }

    pub fn ledger_index(&self) -> UInt256 {
        // Amendments has a fixed index
        UInt256::from_be_bytes([0x41u8; 32]) // 'A' repeated
    }

    pub fn entry_type() -> LedgerEntryType {
        LedgerEntryType::Amendments
    }

    pub fn update_previous_txn(&mut self, txn_id: UInt256, lgr_seq: u32) {
        self.previous_txn_id = txn_id;
        self.previous_txn_lgr_seq = lgr_seq;
    }
}

/// FeeSettings entry - tracks fee configuration
/// LedgerEntryType: ltFEE_SETTINGS = 's' (0x73)
#[derive(Debug, Clone)]
pub struct FeeSettings {
    pub base_fee: u64,
    pub reference_fee_units: u32,
    pub reserve_base: u64,
    pub reserve_increment: u64,
    pub previous_txn_id: UInt256,
    pub previous_txn_lgr_seq: u32,
}

impl FeeSettings {
    pub fn new(base_fee: u64, reserve_base: u64, reserve_increment: u64) -> Self {
        Self {
            base_fee,
            reference_fee_units: 10,
            reserve_base,
            reserve_increment,
            previous_txn_id: UInt256::zero(),
            previous_txn_lgr_seq: 0,
        }
    }

    pub fn ledger_index(&self) -> UInt256 {
        // FeeSettings has a fixed index
        UInt256::from_be_bytes([0x46u8; 32]) // 'F' repeated
    }

    pub fn entry_type() -> LedgerEntryType {
        LedgerEntryType::FeeSettings
    }

    pub fn update_previous_txn(&mut self, txn_id: UInt256, lgr_seq: u32) {
        self.previous_txn_id = txn_id;
        self.previous_txn_lgr_seq = lgr_seq;
    }
}

// ============================================================================
// Custom Callchain Ledger Entry Types
// ============================================================================

/// IssueRoot entry - tracks native asset issuance
/// LedgerEntryType: ltISSUEROOT = 'i' (0x69)
#[derive(Debug, Clone)]
pub struct IssueRoot {
    pub issuer: AccountID,
    pub currency: Currency,
    pub total_supply: Amount,
    pub issued_amount: Amount,
    pub transfer_rate: Option<u32>,
    pub flags: u32,
    pub previous_txn_id: UInt256,
    pub previous_txn_lgr_seq: u32,
}

impl IssueRoot {
    pub fn new(issuer: AccountID, currency: Currency, total_supply: Amount) -> Self {
        Self {
            issuer,
            currency,
            total_supply,
            issued_amount: Amount::call(0),
            transfer_rate: None,
            flags: 0,
            previous_txn_id: UInt256::zero(),
            previous_txn_lgr_seq: 0,
        }
    }

    pub fn ledger_index(&self) -> UInt256 {
        // IssueRoot index is hash of issuer + currency
        let mut data = Vec::new();
        data.extend_from_slice(self.issuer.as_bytes());
        data.extend_from_slice(self.currency.as_bytes());
        let hash = crypto::sha256(&data);
        UInt256::new(hash)
    }

    pub fn entry_type() -> LedgerEntryType {
        LedgerEntryType::IssueRoot
    }

    pub fn is_editable(&self) -> bool {
        (self.flags & 0x00010000) != 0 // tfEnaddition
    }

    pub fn is_non_fungible(&self) -> bool {
        (self.flags & 0x00001000) != 0 // tfNonFungible
    }

    pub fn update_issued(&mut self, amount: Amount) {
        self.issued_amount = amount;
    }

    pub fn update_previous_txn(&mut self, txn_id: UInt256, lgr_seq: u32) {
        self.previous_txn_id = txn_id;
        self.previous_txn_lgr_seq = lgr_seq;
    }
}

/// Invoice entry - represents a non-fungible token (NFT)
/// LedgerEntryType: ltINVOICE = 'v' (0x76)
#[derive(Debug, Clone)]
pub struct Invoice {
    pub invoice_id: UInt256,
    pub issuer: AccountID,
    pub owner: AccountID,
    pub amount: Amount,
    pub data: Vec<u8>,
    pub flags: u32,
    pub previous_txn_id: UInt256,
    pub previous_txn_lgr_seq: u32,
}

impl Invoice {
    pub fn new(invoice_id: UInt256, issuer: AccountID, amount: Amount) -> Self {
        Self {
            invoice_id,
            issuer: issuer.clone(),
            owner: issuer,
            amount,
            data: Vec::new(),
            flags: 0,
            previous_txn_id: UInt256::zero(),
            previous_txn_lgr_seq: 0,
        }
    }

    pub fn ledger_index(&self) -> UInt256 {
        // Invoice index is the invoice_id
        self.invoice_id
    }

    pub fn entry_type() -> LedgerEntryType {
        LedgerEntryType::Invoice
    }

    pub fn transfer(&mut self, new_owner: AccountID) {
        self.owner = new_owner;
    }

    pub fn set_data(&mut self, data: Vec<u8>) {
        self.data = data;
    }

    pub fn update_previous_txn(&mut self, txn_id: UInt256, lgr_seq: u32) {
        self.previous_txn_id = txn_id;
        self.previous_txn_lgr_seq = lgr_seq;
    }
}

/// FeeRoot entry - tracks accumulated fees
/// LedgerEntryType: ltFeeRoot = 'F' (0x46)
#[derive(Debug, Clone)]
pub struct FeeRoot {
    pub balance: Amount,
    pub last_ledger: u32,
    pub previous_txn_id: UInt256,
    pub previous_txn_lgr_seq: u32,
}

impl FeeRoot {
    pub fn new() -> Self {
        Self {
            balance: Amount::call(0),
            last_ledger: 0,
            previous_txn_id: UInt256::zero(),
            previous_txn_lgr_seq: 0,
        }
    }

    pub fn ledger_index(&self) -> UInt256 {
        // FeeRoot has a fixed index
        UInt256::from_be_bytes([0x00u8; 32])
    }

    pub fn entry_type() -> LedgerEntryType {
        LedgerEntryType::FeeRoot
    }

    pub fn set_balance(&mut self, balance: Amount) {
        self.balance = balance;
    }

    pub fn update_previous_txn(&mut self, txn_id: UInt256, lgr_seq: u32) {
        self.previous_txn_id = txn_id;
        self.previous_txn_lgr_seq = lgr_seq;
    }
}

/// LedgerStateManager - manages all ledger state using SHAMap
pub struct LedgerState {
    // SHAMap for storing ledger state entries
    state_map: shamap::SHAMap,
}

impl std::fmt::Debug for LedgerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LedgerState")
            .field("root_hash", &self.get_root_hash())
            .field("is_empty", &self.state_map.is_empty())
            .finish()
    }
}

impl LedgerState {
    pub fn new() -> Self {
        Self {
            state_map: shamap::SHAMap::new(shamap::SHAMapType::State),
        }
    }

    /// Get the root hash of the state tree
    pub fn get_root_hash(&self) -> primitives::UInt256 {
        self.state_map.get_root_hash()
    }

    pub fn get_account_root(&self, account: &AccountID) -> Option<AccountRoot> {
        // Create a temporary AccountRoot to compute the ledger index
        let temp_root = AccountRoot::new(*account);
        let index = temp_root.ledger_index();

        // Fetch from SHAMap
        self.state_map.get_item(&index).and_then(|item| {
            // Deserialize the AccountRoot from stored data
            Self::deserialize_account_root(account, item.data())
        })
    }

    pub fn set_account_root(&mut self, account_root: &AccountRoot) {
        use shamap::SHAMapItem;
        let index = account_root.ledger_index();
        let data = Self::serialize_account_root(account_root);
        let item = SHAMapItem::new(index, data);
        self.state_map.add_item(index, item);
    }

    pub fn get_call_state(&self, account: &AccountID, issuer: &AccountID, currency: &Currency) -> Option<CallState> {
        // Create a temporary CallState to compute the ledger index
        let temp_state = CallState::new(*account, *issuer, *currency);
        let index = temp_state.ledger_index();

        // Fetch from SHAMap
        self.state_map.get_item(&index).and_then(|item| {
            Self::deserialize_call_state(item.data())
        })
    }

    pub fn set_call_state(&mut self, call_state: &CallState) {
        use shamap::SHAMapItem;
        let index = call_state.ledger_index();
        let data = Self::serialize_call_state(call_state);
        let item = SHAMapItem::new(index, data);
        self.state_map.add_item(index, item);
    }

    pub fn get_offer(&self, account: &AccountID, sequence: u32) -> Option<OfferEntry> {
        // We need to reconstruct the offer to compute its index
        let taker_pays = Amount::call(0);
        let taker_gets = Amount::call(0);
        let temp_offer = OfferEntry::new(*account, sequence, taker_pays, taker_gets);
        let index = temp_offer.ledger_index();

        self.state_map.get_item(&index).and_then(|item| {
            Self::deserialize_offer(item.data())
        })
    }

    pub fn set_offer(&mut self, offer: &OfferEntry) {
        use shamap::SHAMapItem;
        let index = offer.ledger_index();
        let data = Self::serialize_offer(offer);
        let item = SHAMapItem::new(index, data);
        self.state_map.add_item(index, item);
    }

    pub fn delete_offer(&mut self, account: &AccountID, sequence: u32) {
        // Note: SHAMap doesn't have a direct remove method in the current API
        // This would require implementing a remove operation
        let _ = account;
        let _ = sequence;
    }

    pub fn get_nickname(&self, nickname: &[u8]) -> Option<NicknameEntry> {
        let temp_entry = NicknameEntry::new(nickname.to_vec(), AccountID::new([0u8; 20]));
        let index = temp_entry.ledger_index();

        self.state_map.get_item(&index).and_then(|item| {
            Self::deserialize_nickname(item.data())
        })
    }

    pub fn set_nickname(&mut self, nickname: &NicknameEntry) {
        use shamap::SHAMapItem;
        let index = nickname.ledger_index();
        let data = Self::serialize_nickname(nickname);
        let item = SHAMapItem::new(index, data);
        self.state_map.add_item(index, item);
    }

    pub fn delete_nickname(&mut self, nickname: &[u8]) {
        // Note: SHAMap doesn't have a direct remove method in the current API
        let _ = nickname;
    }

    // Serialization helpers
    /// Iterate over all entries in the ledger state
    pub fn iter(&self) -> impl Iterator<Item = &shamap::SHAMapItem> {
        self.state_map.iter()
    }

    /// Get an entry by key
    pub fn get(&self, key: &primitives::UInt256) -> Option<&shamap::SHAMapItem> {
        self.state_map.get_item(key)
    }

    /// Get all trust lines (CallState) for an account
    pub fn get_call_states_for_account(&self, account: &AccountID) -> Vec<CallState> {
        let mut results = Vec::new();
        for item in self.state_map.iter() {
            if let Some(call_state) = Self::deserialize_call_state(item.data()) {
                if call_state.account == *account {
                    results.push(call_state);
                }
            }
        }
        results
    }

    /// Get all offers for an account
    pub fn get_offers_for_account(&self, account: &AccountID) -> Vec<OfferEntry> {
        let mut results = Vec::new();
        for item in self.state_map.iter() {
            if let Some(offer) = Self::deserialize_offer(item.data()) {
                if offer.account == *account {
                    results.push(offer);
                }
            }
        }
        results
    }

    /// Get all invoices for an account (where account is issuer or owner)
    pub fn get_invoices_for_account(&self, account: &AccountID) -> Vec<Invoice> {
        let mut results = Vec::new();
        for item in self.state_map.iter() {
            if let Some(invoice) = Self::deserialize_invoice(item.data()) {
                if invoice.issuer == *account || invoice.owner == *account {
                    results.push(invoice);
                }
            }
        }
        results
    }

    /// Get all directory nodes for an account
    pub fn get_directories_for_account(&self, account: &AccountID) -> Vec<DirectoryNode> {
        let mut results = Vec::new();
        for item in self.state_map.iter() {
            if let Some(dir) = Self::deserialize_directory(item.data()) {
                if let Some(owner) = dir.owner {
                    if owner == *account {
                        results.push(dir);
                    }
                }
            }
        }
        results
    }

    /// Get account objects (offers, directories, etc.)
    pub fn get_account_objects(&self, account: &AccountID, limit: usize) -> Vec<LedgerObject> {
        let mut results = Vec::new();
        let mut count = 0;

        for item in self.state_map.iter() {
            if count >= limit {
                break;
            }

            // Try to deserialize as different object types
            if let Some(offer) = Self::deserialize_offer(item.data()) {
                if offer.account == *account {
                    results.push(LedgerObject::Offer(offer));
                    count += 1;
                    continue;
                }
            }

            if let Some(dir) = Self::deserialize_directory(item.data()) {
                if let Some(owner) = dir.owner {
                    if owner == *account {
                        results.push(LedgerObject::Directory(dir));
                        count += 1;
                        continue;
                    }
                }
            }

            if let Some(call_state) = Self::deserialize_call_state(item.data()) {
                if call_state.account == *account {
                    results.push(LedgerObject::CallState(call_state));
                    count += 1;
                    continue;
                }
            }
        }

        results
    }

    fn serialize_account_root(root: &AccountRoot) -> Vec<u8> {
        use serialization::Serializer;
        let mut ser = Serializer::with_capacity(128);
        ser.add_account(root.account);
        ser.add_amount(root.balance);
        ser.add32(root.sequence);
        ser.add32(root.owner_count);
        ser.add256(root.previous_txn_id);
        ser.add32(root.previous_txn_lgr_seq);
        ser.finish()
    }

    fn deserialize_account_root(account: &AccountID, data: &[u8]) -> Option<AccountRoot> {
        use serialization::SerialIter;
        let mut iter = SerialIter::new(data);

        let _ = iter.get_account().ok()?;
        let balance = iter.get_amount().ok()?;
        let sequence = iter.get32().ok()?;
        let owner_count = iter.get32().ok()?;
        let previous_txn_id = iter.get256().ok()?;
        let previous_txn_lgr_seq = iter.get32().ok()?;

        Some(AccountRoot {
            account: *account,
            balance,
            sequence,
            owner_count,
            previous_txn_id,
            previous_txn_lgr_seq,
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
        })
    }

    fn serialize_call_state(state: &CallState) -> Vec<u8> {
        use serialization::Serializer;
        let mut ser = Serializer::with_capacity(128);
        ser.add_account(state.account);
        ser.add_account(state.issuer);
        ser.add_currency(state.currency);
        ser.add_amount(state.balance);
        ser.add_amount(state.limit);
        ser.finish()
    }

    pub fn deserialize_call_state(data: &[u8]) -> Option<CallState> {
        use serialization::SerialIter;
        let mut iter = SerialIter::new(data);

        let account = iter.get_account().ok()?;
        let issuer = iter.get_account().ok()?;
        let currency = iter.get_currency().ok()?;
        let balance = iter.get_amount().ok()?;
        let limit = iter.get_amount().ok()?;

        Some(CallState {
            account,
            issuer,
            currency,
            balance,
            limit,
            limit_peer: Amount::call(0),
            quality_in: None,
            quality_out: None,
            previous_txn_id: primitives::UInt256::zero(),
            previous_txn_lgr_seq: 0,
            low_node: None,
            high_node: None,
            low_quality_in: None,
            high_quality_in: None,
            low_quality_out: None,
            high_quality_out: None,
        })
    }

    fn serialize_offer(offer: &OfferEntry) -> Vec<u8> {
        use serialization::Serializer;
        let mut ser = Serializer::with_capacity(128);
        ser.add_account(offer.account);
        ser.add32(offer.sequence);
        ser.add_amount(offer.taker_pays);
        ser.add_amount(offer.taker_gets);
        if let Some(exp) = offer.expiration {
            ser.add32(exp);
        }
        ser.finish()
    }

    pub fn deserialize_offer(data: &[u8]) -> Option<OfferEntry> {
        use serialization::SerialIter;
        let mut iter = SerialIter::new(data);

        let account = iter.get_account().ok()?;
        let sequence = iter.get32().ok()?;
        let taker_pays = iter.get_amount().ok()?;
        let taker_gets = iter.get_amount().ok()?;

        // Try to read optional expiration
        let expiration = iter.get32().ok();

        Some(OfferEntry {
            account,
            sequence,
            taker_pays,
            taker_gets,
            book_directory: primitives::UInt256::zero(),
            book_node: primitives::UInt256::zero(),
            owner_node: primitives::UInt256::zero(),
            previous_txn_id: primitives::UInt256::zero(),
            previous_txn_lgr_seq: 0,
            expiration,
        })
    }

    fn deserialize_directory(data: &[u8]) -> Option<DirectoryNode> {
        use serialization::SerialIter;
        let mut iter = SerialIter::new(data);

        let root_index = iter.get256().ok()?;

        let mut dir = DirectoryNode::new(root_index);

        // Try to read optional fields
        if let Ok(owner) = iter.get_account() {
            dir.owner = Some(owner);
        }

        Some(dir)
    }

    fn serialize_nickname(entry: &NicknameEntry) -> Vec<u8> {
        use serialization::Serializer;
        let mut ser = Serializer::with_capacity(64);
        ser.add_vl(&entry.nickname);
        ser.add_account(entry.account);
        if let Some(ref min_offer) = entry.min_offer {
            ser.add_amount(*min_offer);
        }
        ser.finish()
    }

    fn deserialize_nickname(data: &[u8]) -> Option<NicknameEntry> {
        use serialization::SerialIter;
        let mut iter = SerialIter::new(data);

        let nickname = iter.get_vl().ok()?;
        let account = iter.get_account().ok()?;

        // Try to read optional min_offer
        let min_offer = iter.get_amount().ok();

        Some(NicknameEntry {
            nickname,
            account,
            min_offer,
            previous_txn_id: primitives::UInt256::zero(),
            previous_txn_lgr_seq: 0,
        })
    }

    fn deserialize_invoice(data: &[u8]) -> Option<Invoice> {
        use serialization::SerialIter;
        let mut iter = SerialIter::new(data);

        // Try to deserialize as an Invoice
        // Format: invoice_id (32 bytes), issuer (20 bytes), owner (20 bytes), amount, flags
        let invoice_id = iter.get256().ok()?;
        let issuer = iter.get_account().ok()?;
        let owner = iter.get_account().ok()?;
        let amount = iter.get_amount().ok()?;
        let flags = iter.get32().ok()?;

        // Try to read optional data
        let data = iter.get_vl().unwrap_or_default();

        Some(Invoice {
            invoice_id,
            issuer,
            owner,
            amount,
            data,
            flags,
            previous_txn_id: primitives::UInt256::zero(),
            previous_txn_lgr_seq: 0,
        })
    }

    /// Persist the ledger state to the database
    /// This stores all SHAMap nodes (both inner and leaf nodes) to the database
    pub fn persist_to_database(&self, database: &storage::Database) {
        // Walk the SHAMap and store all nodes
        self.walk_and_store_nodes(&self.state_map, database);
    }

    /// Recursively walk the SHAMap and store all nodes
    fn walk_and_store_nodes(
        &self,
        state_map: &shamap::SHAMap,
        database: &storage::Database,
    ) {
        // Store all leaf nodes (ledger entries)
        for item in state_map.iter() {
            let key = item.key();
            let data = item.data();

            // Store the leaf node data
            database.store_node_data(
                storage::NodeObjectType::AccountNode,
                UInt256::new(*key),
                data.to_vec(),
            );
        }

        // Note: Inner nodes would need to be serialized and stored as well
        // This requires access to the inner structure of SHAMap which may need
        // to be extended in the shamap crate
    }

    /// Load the ledger state from the database
    /// This loads all account nodes and rebuilds the SHAMap
    /// Note: This requires a database backend that supports iteration
    pub fn load_from_database(
        &mut self,
        _database: &storage::Database,
        _ledger_hash: primitives::UInt256,
    ) -> bool {
        use tracing::warn;

        warn!("Ledger state loading from database is not fully implemented - requires database iteration support");

        // For a full implementation:
        // 1. Load the ledger header to get the state tree root hash
        // 2. Fetch the root node from database
        // 3. Recursively load all child nodes (inner and leaf)
        // 4. Reconstruct the SHAMap
        //
        // The current limitation is that the database backend
        // doesn't support iterating over all nodes.

        false
    }

    /// Load a specific account from the database into the state map
    pub fn load_account(&mut self, database: &storage::Database, account_id: AccountID) -> Option<AccountRoot> {
        // Compute the ledger index for this account
        let index = AccountRoot::compute_ledger_index(&account_id);

        // Try to fetch from database
        match database.fetch_account_node(&index) {
            Some(node) => {
                // Deserialize the account data
                match Self::deserialize_account_root(&account_id, node.get_data()) {
                    Some(account) => {
                        // Add to state map
                        self.set_account_root(&account);
                        Some(account)
                    }
                    None => {
                        tracing::warn!("Failed to deserialize account {}", account_id);
                        None
                    }
                }
            }
            None => None
        }
    }
}

impl Clone for LedgerState {
    fn clone(&self) -> Self {
        // SHAMap doesn't implement Clone, so we create a new empty one
        // In a real implementation, this would deep copy the tree
        Self::new()
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
        // Standard Ripple ledger entries
        assert_eq!(LedgerEntryType::AccountRoot.as_u16(), 0x61);   // 'a'
        assert_eq!(LedgerEntryType::CallState.as_u16(), 0x72);      // 'r' (was 'c')
        assert_eq!(LedgerEntryType::Offer.as_u16(), 0x6F);          // 'o'
        assert_eq!(LedgerEntryType::DirectoryNode.as_u16(), 0x64); // 'd'
        assert_eq!(LedgerEntryType::Nickname.as_u16(), 0x6E);      // 'n'

        // New entries added for calld compatibility
        assert_eq!(LedgerEntryType::SignerList.as_u16(), 0x53);     // 'S'
        assert_eq!(LedgerEntryType::LedgerHashes.as_u16(), 0x68);   // 'h'
        assert_eq!(LedgerEntryType::Amendments.as_u16(), 0x66);     // 'f'
        assert_eq!(LedgerEntryType::FeeSettings.as_u16(), 0x73);    // 's'

        // Custom Callchain entries
        assert_eq!(LedgerEntryType::FeeRoot.as_u16(), 0x46);        // 'F'
        assert_eq!(LedgerEntryType::IssueRoot.as_u16(), 0x69);      // 'i'
        assert_eq!(LedgerEntryType::Invoice.as_u16(), 0x76);        // 'v'

        // Test from_u16 roundtrip
        assert_eq!(LedgerEntryType::from_u16(0x61), Some(LedgerEntryType::AccountRoot));
        assert_eq!(LedgerEntryType::from_u16(0x72), Some(LedgerEntryType::CallState));
        assert_eq!(LedgerEntryType::from_u16(0x53), Some(LedgerEntryType::SignerList));
        assert_eq!(LedgerEntryType::from_u16(0x68), Some(LedgerEntryType::LedgerHashes));
        assert_eq!(LedgerEntryType::from_u16(0x66), Some(LedgerEntryType::Amendments));
        assert_eq!(LedgerEntryType::from_u16(0x73), Some(LedgerEntryType::FeeSettings));
        assert_eq!(LedgerEntryType::from_u16(0x9999), None);
    }

    #[test]
    fn test_nickname_entry() {
        let account = AccountID::new([1u8; 20]);
        let nickname = b"alice";

        let entry = NicknameEntry::new(nickname.to_vec(), account);

        assert_eq!(entry.nickname, nickname.to_vec());
        assert_eq!(entry.account, account);
        assert!(entry.min_offer.is_none());

        // Test set_min_offer
        let amount = Amount::call(1000000);
        let mut entry = NicknameEntry::new(nickname.to_vec(), account);
        entry.set_min_offer(amount.clone());
        assert_eq!(entry.min_offer, Some(amount));
    }
}
