//! Ledger state entries for account and object storage
//!
//! This module defines the ledger entry types that represent the state of the ledger.
//! Each entry type corresponds to a specific type of object stored in the SHAMap.

use primitives::{AccountID, Currency, UInt256};
use serialization::{Amount, STObject, STValue};
use serialization::types::sf;
use crate::SignerEntry;

/// AccountRoot flags (lsf*)
/// These flags are stored in AccountRoot.flags and can be set/cleared via AccountSet
pub mod account_flags {
    pub const LSF_DEFAULT_CALL: u32 = 0x00000001;
    pub const LSF_NO_CALL: u32 = 0x00000002; // Callchain specific
    pub const LSF_REQUIRE_DEST_TAG: u32 = 0x00000004;
    pub const LSF_REQUIRE_AUTH: u32 = 0x00000008;
    pub const LSF_DISALLOW_CALL: u32 = 0x00000010;
    pub const LSF_DISABLE_MASTER: u32 = 0x00000020;
    pub const LSF_NO_FREEZE: u32 = 0x00000040;
    pub const LSF_GLOBAL_FREEZE: u32 = 0x00000080;
    pub const LSF_DEPOSIT_AUTH: u32 = 0x00000100;
}

/// AccountSet transaction flags (asf*)
/// These are used in the SetFlag/ClearFlag fields of AccountSet transactions
pub mod account_set_flags {
    pub const ASF_ACCOUNT_TXN_ID: u32 = 5;
    pub const ASF_NO_CALL: u32 = 6; // Callchain specific
    pub const ASF_REQUIRE_DEST_TAG: u32 = 1;
    pub const ASF_REQUIRE_AUTH: u32 = 2;
    pub const ASF_DISALLOW_CALL: u32 = 3;
    pub const ASF_DISABLE_MASTER: u32 = 4;
    pub const ASF_NO_FREEZE: u32 = 6;
    pub const ASF_GLOBAL_FREEZE: u32 = 7;
    pub const ASF_DEFAULT_CALL: u32 = 8;
    pub const ASF_DEPOSIT_AUTH: u32 = 9;
}

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
    pub flags: u32,
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
        use account_flags::LSF_DEFAULT_CALL;
        Self {
            account,
            balance: Amount::call(0),
            sequence: 1,
            owner_count: 0,
            flags: LSF_DEFAULT_CALL, // Default flag is set
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

    // Flag checking methods
    pub fn has_flag(&self, flag: u32) -> bool {
        (self.flags & flag) == flag
    }

    pub fn set_flag(&mut self, flag: u32) {
        self.flags |= flag;
    }

    pub fn clear_flag(&mut self, flag: u32) {
        self.flags &= !flag;
    }

    pub fn is_default_call(&self) -> bool {
        self.has_flag(account_flags::LSF_DEFAULT_CALL)
    }

    pub fn is_no_call(&self) -> bool {
        self.has_flag(account_flags::LSF_NO_CALL)
    }

    pub fn requires_dest_tag(&self) -> bool {
        self.has_flag(account_flags::LSF_REQUIRE_DEST_TAG)
    }

    pub fn requires_auth(&self) -> bool {
        self.has_flag(account_flags::LSF_REQUIRE_AUTH)
    }

    pub fn is_disallow_call(&self) -> bool {
        self.has_flag(account_flags::LSF_DISALLOW_CALL)
    }

    pub fn is_disable_master(&self) -> bool {
        self.has_flag(account_flags::LSF_DISABLE_MASTER)
    }

    pub fn is_no_freeze(&self) -> bool {
        self.has_flag(account_flags::LSF_NO_FREEZE)
    }

    pub fn is_global_freeze(&self) -> bool {
        self.has_flag(account_flags::LSF_GLOBAL_FREEZE)
    }

    pub fn requires_deposit_auth(&self) -> bool {
        self.has_flag(account_flags::LSF_DEPOSIT_AUTH)
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

        let mut obj = STObject::new();

        // Required fields
        obj.insert(sf::LEDGER_ENTRY_TYPE, STValue::UInt16(LedgerEntryType::AccountRoot.as_u16()));
        obj.insert(sf::ACCOUNT, STValue::Account(self.account));
        obj.insert(sf::BALANCE, STValue::Amount(self.balance));
        obj.insert(sf::SEQUENCE, STValue::UInt32(self.sequence));
        obj.insert(sf::OWNER_COUNT, STValue::UInt32(self.owner_count));
        obj.insert(sf::PREVIOUS_TXN_ID, STValue::Hash256(self.previous_txn_id));
        obj.insert(sf::PREVIOUS_TXN_LGR_SEQ, STValue::UInt32(self.previous_txn_lgr_seq));

        // Flags (always serialize, even if 0, for compatibility)
        obj.insert(sf::FLAGS, STValue::UInt32(self.flags));

        // Optional fields - only include if present
        if let Some(ref account_txn_id) = self.account_txn_id {
            obj.insert(sf::ACCOUNT_TXN_ID, STValue::Hash256(*account_txn_id));
        }
        if let Some(ref wallet_locator) = self.wallet_locator {
            obj.insert(sf::WALLET_LOCATOR, STValue::Hash256(*wallet_locator));
        }
        if let Some(wallet_size) = self.wallet_size {
            obj.insert(sf::WALLET_SIZE, STValue::UInt32(wallet_size));
        }
        if let Some(ref message_key) = self.message_key {
            obj.insert(sf::MESSAGE_KEY, STValue::VL(message_key.clone()));
        }
        if let Some(ref domain) = self.domain {
            obj.insert(sf::DOMAIN, STValue::VL(domain.clone()));
        }
        if let Some(transfer_rate) = self.transfer_rate {
            obj.insert(sf::TRANSFER_RATE, STValue::UInt32(transfer_rate));
        }
        if let Some(ref regular_key) = self.regular_key {
            obj.insert(sf::REGULAR_KEY, STValue::Account(*regular_key));
        }
        if let Some(tick_size) = self.tick_size {
            obj.insert(sf::TICK_SIZE, STValue::UInt8(tick_size));
        }
        if let Some(ref email_hash) = self.email_hash {
            obj.insert(sf::EMAIL_HASH, STValue::Hash256(*email_hash));
        }

        obj
    }

    fn from_stobject(obj: &STObject) -> Option<Self> {

        // Required fields
        let account = obj.get_account(sf::ACCOUNT)?;
        let balance = obj.get_amount(sf::BALANCE)?;
        let sequence = obj.get_uint32(sf::SEQUENCE)?;
        let owner_count = obj.get_uint32(sf::OWNER_COUNT)?;
        let previous_txn_id = obj.get_hash256(sf::PREVIOUS_TXN_ID)?;
        let previous_txn_lgr_seq = obj.get_uint32(sf::PREVIOUS_TXN_LGR_SEQ)?;

        // Flags (default to LSF_DEFAULT_CALL if not present for backward compatibility)
        let flags = obj.get_uint32(sf::FLAGS).unwrap_or(account_flags::LSF_DEFAULT_CALL);

        // Optional fields
        let account_txn_id = obj.get_hash256(sf::ACCOUNT_TXN_ID);
        let wallet_locator = obj.get_hash256(sf::WALLET_LOCATOR);
        let wallet_size = obj.get_uint32(sf::WALLET_SIZE);
        let message_key = obj.get_vl(sf::MESSAGE_KEY).map(|v| v.to_vec());
        let domain = obj.get_vl(sf::DOMAIN).map(|v| v.to_vec());
        let transfer_rate = obj.get_uint32(sf::TRANSFER_RATE);
        let regular_key = obj.get_account(sf::REGULAR_KEY);
        let tick_size = obj.get_uint8(sf::TICK_SIZE);
        let email_hash = obj.get_hash256(sf::EMAIL_HASH);

        Some(Self {
            account,
            balance,
            sequence,
            owner_count,
            flags,
            previous_txn_id,
            previous_txn_lgr_seq,
            account_txn_id,
            wallet_locator: wallet_locator.map(|_| UInt256::zero()),
            wallet_size,
            message_key,
            domain,
            transfer_rate,
            code_garage: None,
            email_hash,
            regular_key,
            tick_size,
        })
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

    /// Get the quality multiplier for incoming IOUs
    /// Returns 1.0 if no quality is set (default behavior)
    /// Quality is stored as a 32-bit unsigned integer where:
    /// - 0 = no quality set (treated as 1.0, i.e., 100%)
    /// - 1000000000 = 1.0 (100% quality, no markup/discount)
    /// - > 1000000000 = premium (pay more to receive)
    /// - < 1000000000 = discount (pay less to receive)
    pub fn get_quality_in(&self) -> f64 {
        self.quality_in
            .map(|q| if q == 0 { 1.0 } else { q as f64 / 1_000_000_000.0 })
            .unwrap_or(1.0)
    }

    /// Get the quality multiplier for outgoing IOUs
    /// Returns 1.0 if no quality is set (default behavior)
    pub fn get_quality_out(&self) -> f64 {
        self.quality_out
            .map(|q| if q == 0 { 1.0 } else { q as f64 / 1_000_000_000.0 })
            .unwrap_or(1.0)
    }

    /// Calculate the effective amount received after applying quality in
    /// When someone sends IOUs to this account, this determines how much
    /// the account values those IOUs relative to face value
    pub fn apply_quality_in(&self, amount: u64) -> u64 {
        let quality = self.get_quality_in();
        ((amount as f64) * quality) as u64
    }

    /// Calculate the effective amount sent after applying quality out
    /// When this account sends IOUs, this determines how much the recipient
    /// actually receives relative to the face value
    pub fn apply_quality_out(&self, amount: u64) -> u64 {
        let quality = self.get_quality_out();
        ((amount as f64) / quality) as u64
    }

    /// Check if this trust line has quality settings configured
    pub fn has_quality_settings(&self) -> bool {
        self.quality_in.is_some() || self.quality_out.is_some()
    }
}

impl LedgerEntry for CallState {
    fn entry_type() -> LedgerEntryType {
        LedgerEntryType::CallState
    }

    fn ledger_index(&self) -> UInt256 {
        self.ledger_index()
    }

    fn to_stobject(&self) -> STObject {

        let mut obj = STObject::new();

        // Required fields
        obj.insert(sf::LEDGER_ENTRY_TYPE, STValue::UInt16(LedgerEntryType::CallState.as_u16()));
        obj.insert(sf::ACCOUNT, STValue::Account(self.account));
        obj.insert(sf::ISSUER, STValue::Account(self.issuer));
        obj.insert(sf::CURRENCY, STValue::VL(self.currency.as_bytes().to_vec()));
        obj.insert(sf::BALANCE, STValue::Amount(self.balance));
        obj.insert(sf::LOW_LIMIT, STValue::Amount(self.limit));
        obj.insert(sf::HIGH_LIMIT, STValue::Amount(self.limit_peer));
        obj.insert(sf::PREVIOUS_TXN_ID, STValue::Hash256(self.previous_txn_id));
        obj.insert(sf::PREVIOUS_TXN_LGR_SEQ, STValue::UInt32(self.previous_txn_lgr_seq));

        // Optional fields
        if let Some(quality_in) = self.quality_in {
            obj.insert(sf::QUALITY_IN, STValue::UInt32(quality_in));
        }
        if let Some(quality_out) = self.quality_out {
            obj.insert(sf::QUALITY_OUT, STValue::UInt32(quality_out));
        }
        if let Some(low_node) = self.low_node {
            obj.insert(sf::LOW_NODE, STValue::Hash256(low_node));
        }
        if let Some(high_node) = self.high_node {
            obj.insert(sf::HIGH_NODE, STValue::Hash256(high_node));
        }
        if let Some(low_quality_in) = self.low_quality_in {
            obj.insert(sf::LOW_QUALITY_IN, STValue::UInt32(low_quality_in));
        }
        if let Some(high_quality_in) = self.high_quality_in {
            obj.insert(sf::HIGH_QUALITY_IN, STValue::UInt32(high_quality_in));
        }
        if let Some(low_quality_out) = self.low_quality_out {
            obj.insert(sf::LOW_QUALITY_OUT, STValue::UInt32(low_quality_out));
        }
        if let Some(high_quality_out) = self.high_quality_out {
            obj.insert(sf::HIGH_QUALITY_OUT, STValue::UInt32(high_quality_out));
        }

        obj
    }

    fn from_stobject(obj: &STObject) -> Option<Self> {

        // Required fields
        let account = obj.get_account(sf::ACCOUNT)?;
        let issuer = obj.get_account(sf::ISSUER)?;
        let currency_bytes = obj.get_vl(sf::CURRENCY)?;
        let mut currency = Currency::CALL;
        if currency_bytes.len() == 20 {
            let mut currency_bytes_arr = [0u8; 20];
            currency_bytes_arr.copy_from_slice(currency_bytes);
            currency = Currency::new(currency_bytes_arr);
        }
        let balance = obj.get_amount(sf::BALANCE)?;
        let limit = obj.get_amount(sf::LOW_LIMIT)?;
        let limit_peer = obj.get_amount(sf::HIGH_LIMIT)?;
        let previous_txn_id = obj.get_hash256(sf::PREVIOUS_TXN_ID)?;
        let previous_txn_lgr_seq = obj.get_uint32(sf::PREVIOUS_TXN_LGR_SEQ)?;

        // Optional fields
        let quality_in = obj.get_uint32(sf::QUALITY_IN);
        let quality_out = obj.get_uint32(sf::QUALITY_OUT);
        let low_node = obj.get_hash256(sf::LOW_NODE);
        let high_node = obj.get_hash256(sf::HIGH_NODE);
        let low_quality_in = obj.get_uint32(sf::LOW_QUALITY_IN);
        let high_quality_in = obj.get_uint32(sf::HIGH_QUALITY_IN);
        let low_quality_out = obj.get_uint32(sf::LOW_QUALITY_OUT);
        let high_quality_out = obj.get_uint32(sf::HIGH_QUALITY_OUT);

        Some(Self {
            account,
            issuer,
            currency,
            balance,
            limit,
            limit_peer,
            quality_in,
            quality_out,
            previous_txn_id,
            previous_txn_lgr_seq,
            low_node,
            high_node,
            low_quality_in,
            high_quality_in,
            low_quality_out,
            high_quality_out,
        })
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

impl LedgerEntry for OfferEntry {
    fn entry_type() -> LedgerEntryType {
        LedgerEntryType::Offer
    }

    fn ledger_index(&self) -> UInt256 {
        self.ledger_index()
    }

    fn to_stobject(&self) -> STObject {

        let mut obj = STObject::new();

        // Required fields
        obj.insert(sf::LEDGER_ENTRY_TYPE, STValue::UInt16(LedgerEntryType::Offer.as_u16()));
        obj.insert(sf::ACCOUNT, STValue::Account(self.account));
        obj.insert(sf::SEQUENCE, STValue::UInt32(self.sequence));
        obj.insert(sf::TAKER_PAYS, STValue::Amount(self.taker_pays));
        obj.insert(sf::TAKER_GETS, STValue::Amount(self.taker_gets));
        obj.insert(sf::BOOK_DIRECTORY, STValue::Hash256(self.book_directory));
        obj.insert(sf::BOOK_NODE, STValue::Hash256(self.book_node));
        obj.insert(sf::OWNER_NODE, STValue::Hash256(self.owner_node));
        obj.insert(sf::PREVIOUS_TXN_ID, STValue::Hash256(self.previous_txn_id));
        obj.insert(sf::PREVIOUS_TXN_LGR_SEQ, STValue::UInt32(self.previous_txn_lgr_seq));

        // Optional fields
        if let Some(expiration) = self.expiration {
            obj.insert(sf::EXPIRATION, STValue::UInt32(expiration));
        }

        obj
    }

    fn from_stobject(obj: &STObject) -> Option<Self> {

        // Required fields
        let account = obj.get_account(sf::ACCOUNT)?;
        let sequence = obj.get_uint32(sf::SEQUENCE)?;
        let taker_pays = obj.get_amount(sf::TAKER_PAYS)?;
        let taker_gets = obj.get_amount(sf::TAKER_GETS)?;
        let book_directory = obj.get_hash256(sf::BOOK_DIRECTORY)?;
        let book_node = obj.get_hash256(sf::BOOK_NODE).unwrap_or(UInt256::zero());
        let owner_node = obj.get_hash256(sf::OWNER_NODE).unwrap_or(UInt256::zero());
        let previous_txn_id = obj.get_hash256(sf::PREVIOUS_TXN_ID)?;
        let previous_txn_lgr_seq = obj.get_uint32(sf::PREVIOUS_TXN_LGR_SEQ)?;

        // Optional fields
        let expiration = obj.get_uint32(sf::EXPIRATION);

        Some(Self {
            account,
            sequence,
            taker_pays,
            taker_gets,
            book_directory,
            book_node,
            owner_node,
            previous_txn_id,
            previous_txn_lgr_seq,
            expiration,
        })
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

    pub fn ledger_index(&self) -> UInt256 {
        // Directory index is the root_index
        self.root_index
    }
}

impl LedgerEntry for DirectoryNode {
    fn entry_type() -> LedgerEntryType {
        LedgerEntryType::DirectoryNode
    }

    fn ledger_index(&self) -> UInt256 {
        self.ledger_index()
    }

    fn to_stobject(&self) -> STObject {

        let mut obj = STObject::new();

        // Required fields
        obj.insert(sf::LEDGER_ENTRY_TYPE, STValue::UInt16(LedgerEntryType::DirectoryNode.as_u16()));
        obj.insert(sf::ROOT_INDEX, STValue::Hash256(self.root_index));
        obj.insert(sf::INDEXES, STValue::Vector256(self.indexes.clone()));

        // Optional fields
        if let Some(ref owner) = self.owner {
            obj.insert(sf::OWNER, STValue::Account(*owner));
        }
        if let Some(ref taker_pays_currency) = self.taker_pays_currency {
            obj.insert(sf::TAKER_PAYS_CURRENCY, STValue::VL(taker_pays_currency.as_bytes().to_vec()));
        }
        if let Some(ref taker_pays_issuer) = self.taker_pays_issuer {
            obj.insert(sf::TAKER_PAYS_ISSUER, STValue::Account(*taker_pays_issuer));
        }
        if let Some(ref taker_gets_currency) = self.taker_gets_currency {
            obj.insert(sf::TAKER_GETS_CURRENCY, STValue::VL(taker_gets_currency.as_bytes().to_vec()));
        }
        if let Some(ref taker_gets_issuer) = self.taker_gets_issuer {
            obj.insert(sf::TAKER_GETS_ISSUER, STValue::Account(*taker_gets_issuer));
        }
        if let Some(index_next) = self.index_next {
            obj.insert(sf::INDEX_NEXT, STValue::UInt64(index_next));
        }
        if let Some(index_previous) = self.index_previous {
            obj.insert(sf::INDEX_PREVIOUS, STValue::UInt64(index_previous));
        }

        obj
    }

    fn from_stobject(obj: &STObject) -> Option<Self> {

        // Required fields
        let root_index = obj.get_hash256(sf::ROOT_INDEX)?;
        let indexes = match obj.get(sf::INDEXES) {
            Some(STValue::Vector256(v)) => v.clone(),
            _ => Vec::new(),
        };

        // Optional fields
        let owner = obj.get_account(sf::OWNER);
        let taker_pays_currency = obj.get_vl(sf::TAKER_PAYS_CURRENCY).map(|v| {
            let mut bytes = [0u8; 20];
            if v.len() == 20 {
                bytes.copy_from_slice(v);
            }
            Currency::new(bytes)
        });
        let taker_pays_issuer = obj.get_account(sf::TAKER_PAYS_ISSUER);
        let taker_gets_currency = obj.get_vl(sf::TAKER_GETS_CURRENCY).map(|v| {
            let mut bytes = [0u8; 20];
            if v.len() == 20 {
                bytes.copy_from_slice(v);
            }
            Currency::new(bytes)
        });
        let taker_gets_issuer = obj.get_account(sf::TAKER_GETS_ISSUER);
        let index_next = obj.get_uint64(sf::INDEX_NEXT);
        let index_previous = obj.get_uint64(sf::INDEX_PREVIOUS);

        Some(Self {
            owner,
            taker_pays_currency,
            taker_pays_issuer,
            taker_gets_currency,
            taker_gets_issuer,
            indexes,
            root_index,
            index_next,
            index_previous,
        })
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

impl LedgerEntry for NicknameEntry {
    fn entry_type() -> LedgerEntryType {
        LedgerEntryType::Nickname
    }

    fn ledger_index(&self) -> UInt256 {
        self.ledger_index()
    }

    fn to_stobject(&self) -> STObject {

        let mut obj = STObject::new();

        // Required fields
        obj.insert(sf::LEDGER_ENTRY_TYPE, STValue::UInt16(LedgerEntryType::Nickname.as_u16()));
        obj.insert(sf::NICKNAME, STValue::VL(self.nickname.clone()));
        obj.insert(sf::ACCOUNT, STValue::Account(self.account));
        obj.insert(sf::PREVIOUS_TXN_ID, STValue::Hash256(self.previous_txn_id));
        obj.insert(sf::PREVIOUS_TXN_LGR_SEQ, STValue::UInt32(self.previous_txn_lgr_seq));

        // Optional fields
        if let Some(ref min_offer) = self.min_offer {
            obj.insert(sf::MINIMUM_OFFER, STValue::Amount(*min_offer));
        }

        obj
    }

    fn from_stobject(obj: &STObject) -> Option<Self> {

        // Required fields
        let nickname = obj.get_vl(sf::NICKNAME)?.to_vec();
        let account = obj.get_account(sf::ACCOUNT)?;
        let previous_txn_id = obj.get_hash256(sf::PREVIOUS_TXN_ID)?;
        let previous_txn_lgr_seq = obj.get_uint32(sf::PREVIOUS_TXN_LGR_SEQ)?;

        // Optional fields
        let min_offer = obj.get_amount(sf::MINIMUM_OFFER);

        Some(Self {
            nickname,
            account,
            min_offer,
            previous_txn_id,
            previous_txn_lgr_seq,
        })
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

impl LedgerEntry for SignerList {
    fn entry_type() -> LedgerEntryType {
        LedgerEntryType::SignerList
    }

    fn ledger_index(&self) -> UInt256 {
        self.ledger_index()
    }

    fn to_stobject(&self) -> STObject {

        let mut obj = STObject::new();

        // Required fields
        obj.insert(sf::LEDGER_ENTRY_TYPE, STValue::UInt16(LedgerEntryType::SignerList.as_u16()));
        obj.insert(sf::ACCOUNT, STValue::Account(self.account));
        obj.insert(sf::SIGNER_QUORUM, STValue::UInt32(self.signer_quorum));
        obj.insert(sf::PREVIOUS_TXN_ID, STValue::Hash256(self.previous_txn_id));
        obj.insert(sf::PREVIOUS_TXN_LGR_SEQ, STValue::UInt32(self.previous_txn_lgr_seq));

        // Serialize signers as array
        let signers_array: Vec<STValue> = self.signers.iter().map(|signer| {
            let mut signer_obj = STObject::new();
            signer_obj.insert(sf::ACCOUNT, STValue::Account(signer.account));
            signer_obj.insert(sf::SIGNER_WEIGHT, STValue::UInt16(signer.weight as u16));
            STValue::Object(signer_obj)
        }).collect();
        obj.insert(sf::SIGNER_ENTRIES, STValue::Array(signers_array));

        obj
    }

    fn from_stobject(obj: &STObject) -> Option<Self> {

        // Required fields
        let account = obj.get_account(sf::ACCOUNT)?;
        let signer_quorum = obj.get_uint32(sf::SIGNER_QUORUM)?;
        let previous_txn_id = obj.get_hash256(sf::PREVIOUS_TXN_ID)?;
        let previous_txn_lgr_seq = obj.get_uint32(sf::PREVIOUS_TXN_LGR_SEQ)?;

        // Deserialize signers from array
        let mut signers = Vec::new();
        if let Some(signers_values) = obj.get_array(sf::SIGNER_ENTRIES) {
            for signer_val in signers_values {
                if let STValue::Object(signer_obj) = signer_val {
                    if let Some(signer_account) = signer_obj.get_account(sf::ACCOUNT) {
                        if let Some(weight) = signer_obj.get_uint16(sf::SIGNER_WEIGHT) {
                            signers.push(SignerEntry {
                                account: signer_account,
                                weight: weight as u8,
                            });
                        }
                    }
                }
            }
        }

        Some(Self {
            account,
            signer_quorum,
            signers,
            previous_txn_id,
            previous_txn_lgr_seq,
        })
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

impl LedgerEntry for LedgerHashes {
    fn entry_type() -> LedgerEntryType {
        LedgerEntryType::LedgerHashes
    }

    fn ledger_index(&self) -> UInt256 {
        self.ledger_index()
    }

    fn to_stobject(&self) -> STObject {

        let mut obj = STObject::new();

        // Required fields
        obj.insert(sf::LEDGER_ENTRY_TYPE, STValue::UInt16(LedgerEntryType::LedgerHashes.as_u16()));
        obj.insert(sf::LEDGER_INDEX, STValue::UInt32(self.ledger_index));
        obj.insert(sf::HASHES, STValue::Vector256(self.hashes.clone()));
        obj.insert(sf::PREVIOUS_TXN_ID, STValue::Hash256(self.previous_txn_id));
        obj.insert(sf::PREVIOUS_TXN_LGR_SEQ, STValue::UInt32(self.previous_txn_lgr_seq));

        obj
    }

    fn from_stobject(obj: &STObject) -> Option<Self> {

        // Required fields
        let ledger_index = obj.get_uint32(sf::LEDGER_INDEX)?;
        let hashes = match obj.get(sf::HASHES) {
            Some(STValue::Vector256(v)) => v.clone(),
            _ => Vec::new(),
        };
        let previous_txn_id = obj.get_hash256(sf::PREVIOUS_TXN_ID)?;
        let previous_txn_lgr_seq = obj.get_uint32(sf::PREVIOUS_TXN_LGR_SEQ)?;

        Some(Self {
            ledger_index,
            hashes,
            previous_txn_id,
            previous_txn_lgr_seq,
        })
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

impl LedgerEntry for Amendments {
    fn entry_type() -> LedgerEntryType {
        LedgerEntryType::Amendments
    }

    fn ledger_index(&self) -> UInt256 {
        self.ledger_index()
    }

    fn to_stobject(&self) -> STObject {

        let mut obj = STObject::new();

        // Required fields
        obj.insert(sf::LEDGER_ENTRY_TYPE, STValue::UInt16(LedgerEntryType::Amendments.as_u16()));
        obj.insert(sf::PREVIOUS_TXN_ID, STValue::Hash256(self.previous_txn_id));
        obj.insert(sf::PREVIOUS_TXN_LGR_SEQ, STValue::UInt32(self.previous_txn_lgr_seq));

        // Serialize amendments as Vector256 (just the amendment IDs)
        let amendment_ids: Vec<UInt256> = self.amendments.iter()
            .filter(|a| a.enabled)
            .map(|a| a.amendment_id)
            .collect();
        obj.insert(sf::AMENDMENTS, STValue::Vector256(amendment_ids));

        obj
    }

    fn from_stobject(obj: &STObject) -> Option<Self> {

        // Required fields
        let previous_txn_id = obj.get_hash256(sf::PREVIOUS_TXN_ID)?;
        let previous_txn_lgr_seq = obj.get_uint32(sf::PREVIOUS_TXN_LGR_SEQ)?;

        // Deserialize amendments
        let mut amendments = Vec::new();
        if let Some(STValue::Vector256(ids)) = obj.get(sf::AMENDMENTS) {
            for id in ids {
                amendments.push(AmendmentVote {
                    amendment_id: *id,
                    name: String::new(),
                    enabled: true,
                    supported: true,
                    vote_count: 0,
                });
            }
        }

        Some(Self {
            amendments,
            previous_txn_id,
            previous_txn_lgr_seq,
        })
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

impl LedgerEntry for FeeSettings {
    fn entry_type() -> LedgerEntryType {
        LedgerEntryType::FeeSettings
    }

    fn ledger_index(&self) -> UInt256 {
        self.ledger_index()
    }

    fn to_stobject(&self) -> STObject {

        let mut obj = STObject::new();

        // Required fields
        obj.insert(sf::LEDGER_ENTRY_TYPE, STValue::UInt16(LedgerEntryType::FeeSettings.as_u16()));
        obj.insert(sf::BASE_FEE, STValue::UInt64(self.base_fee));
        obj.insert(sf::REFERENCE_FEE_UNITS, STValue::UInt32(self.reference_fee_units));
        obj.insert(sf::RESERVE_BASE, STValue::UInt32(self.reserve_base as u32));
        obj.insert(sf::RESERVE_INCREMENT, STValue::UInt32(self.reserve_increment as u32));
        obj.insert(sf::PREVIOUS_TXN_ID, STValue::Hash256(self.previous_txn_id));
        obj.insert(sf::PREVIOUS_TXN_LGR_SEQ, STValue::UInt32(self.previous_txn_lgr_seq));

        obj
    }

    fn from_stobject(obj: &STObject) -> Option<Self> {

        // Required fields
        let base_fee = obj.get_uint64(sf::BASE_FEE)?;
        let reference_fee_units = obj.get_uint32(sf::REFERENCE_FEE_UNITS).unwrap_or(10);
        let reserve_base = obj.get_uint32(sf::RESERVE_BASE).map(|v| v as u64).unwrap_or(0);
        let reserve_increment = obj.get_uint32(sf::RESERVE_INCREMENT).map(|v| v as u64).unwrap_or(0);
        let previous_txn_id = obj.get_hash256(sf::PREVIOUS_TXN_ID)?;
        let previous_txn_lgr_seq = obj.get_uint32(sf::PREVIOUS_TXN_LGR_SEQ)?;

        Some(Self {
            base_fee,
            reference_fee_units,
            reserve_base,
            reserve_increment,
            previous_txn_id,
            previous_txn_lgr_seq,
        })
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

impl LedgerEntry for IssueRoot {
    fn entry_type() -> LedgerEntryType {
        LedgerEntryType::IssueRoot
    }

    fn ledger_index(&self) -> UInt256 {
        self.ledger_index()
    }

    fn to_stobject(&self) -> STObject {

        let mut obj = STObject::new();

        // Required fields
        obj.insert(sf::LEDGER_ENTRY_TYPE, STValue::UInt16(LedgerEntryType::IssueRoot.as_u16()));
        obj.insert(sf::ACCOUNT, STValue::Account(self.issuer));
        obj.insert(sf::CURRENCY, STValue::VL(self.currency.as_bytes().to_vec()));
        obj.insert(sf::TOTAL, STValue::Amount(self.total_supply));
        obj.insert(sf::ISSUED, STValue::Amount(self.issued_amount));
        obj.insert(sf::FLAGS, STValue::UInt32(self.flags));
        obj.insert(sf::PREVIOUS_TXN_ID, STValue::Hash256(self.previous_txn_id));
        obj.insert(sf::PREVIOUS_TXN_LGR_SEQ, STValue::UInt32(self.previous_txn_lgr_seq));

        // Optional fields
        if let Some(transfer_rate) = self.transfer_rate {
            obj.insert(sf::TRANSFER_RATE, STValue::UInt32(transfer_rate));
        }

        obj
    }

    fn from_stobject(obj: &STObject) -> Option<Self> {

        // Required fields
        let issuer = obj.get_account(sf::ACCOUNT)?;
        let currency_bytes = obj.get_vl(sf::CURRENCY)?;
        let mut currency = Currency::CALL;
        if currency_bytes.len() == 20 {
            let mut currency_bytes_arr = [0u8; 20];
            currency_bytes_arr.copy_from_slice(currency_bytes);
            currency = Currency::new(currency_bytes_arr);
        }
        let total_supply = obj.get_amount(sf::TOTAL)?;
        let issued_amount = obj.get_amount(sf::ISSUED).unwrap_or_else(|| Amount::call(0));
        let flags = obj.get_uint32(sf::FLAGS).unwrap_or(0);
        let previous_txn_id = obj.get_hash256(sf::PREVIOUS_TXN_ID)?;
        let previous_txn_lgr_seq = obj.get_uint32(sf::PREVIOUS_TXN_LGR_SEQ)?;

        // Optional fields
        let transfer_rate = obj.get_uint32(sf::TRANSFER_RATE);

        Some(Self {
            issuer,
            currency,
            total_supply,
            issued_amount,
            transfer_rate,
            flags,
            previous_txn_id,
            previous_txn_lgr_seq,
        })
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

impl LedgerEntry for Invoice {
    fn entry_type() -> LedgerEntryType {
        LedgerEntryType::Invoice
    }

    fn ledger_index(&self) -> UInt256 {
        self.ledger_index()
    }

    fn to_stobject(&self) -> STObject {

        let mut obj = STObject::new();

        // Required fields
        obj.insert(sf::LEDGER_ENTRY_TYPE, STValue::UInt16(LedgerEntryType::Invoice.as_u16()));
        obj.insert(sf::INVOICE_ID, STValue::Hash256(self.invoice_id));
        obj.insert(sf::ACCOUNT, STValue::Account(self.issuer));
        obj.insert(sf::BALANCE_OWNER, STValue::Account(self.owner));
        obj.insert(sf::AMOUNT, STValue::Amount(self.amount));
        obj.insert(sf::FLAGS, STValue::UInt32(self.flags));
        obj.insert(sf::PREVIOUS_TXN_ID, STValue::Hash256(self.previous_txn_id));
        obj.insert(sf::PREVIOUS_TXN_LGR_SEQ, STValue::UInt32(self.previous_txn_lgr_seq));

        // Optional fields
        if !self.data.is_empty() {
            obj.insert(sf::INVOICE, STValue::VL(self.data.clone()));
        }

        obj
    }

    fn from_stobject(obj: &STObject) -> Option<Self> {

        // Required fields
        let invoice_id = obj.get_hash256(sf::INVOICE_ID)?;
        let issuer = obj.get_account(sf::ACCOUNT)?;
        let owner = obj.get_account(sf::BALANCE_OWNER)?;
        let amount = obj.get_amount(sf::AMOUNT)?;
        let flags = obj.get_uint32(sf::FLAGS).unwrap_or(0);
        let previous_txn_id = obj.get_hash256(sf::PREVIOUS_TXN_ID)?;
        let previous_txn_lgr_seq = obj.get_uint32(sf::PREVIOUS_TXN_LGR_SEQ)?;

        // Optional fields
        let data = obj.get_vl(sf::INVOICE).map(|v| v.to_vec()).unwrap_or_default();

        Some(Self {
            invoice_id,
            issuer,
            owner,
            amount,
            data,
            flags,
            previous_txn_id,
            previous_txn_lgr_seq,
        })
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

impl LedgerEntry for FeeRoot {
    fn entry_type() -> LedgerEntryType {
        LedgerEntryType::FeeRoot
    }

    fn ledger_index(&self) -> UInt256 {
        self.ledger_index()
    }

    fn to_stobject(&self) -> STObject {

        let mut obj = STObject::new();

        // Required fields
        obj.insert(sf::LEDGER_ENTRY_TYPE, STValue::UInt16(LedgerEntryType::FeeRoot.as_u16()));
        obj.insert(sf::BALANCE, STValue::Amount(self.balance));
        obj.insert(sf::LEDGER_SEQUENCE, STValue::UInt32(self.last_ledger));
        obj.insert(sf::PREVIOUS_TXN_ID, STValue::Hash256(self.previous_txn_id));
        obj.insert(sf::PREVIOUS_TXN_LGR_SEQ, STValue::UInt32(self.previous_txn_lgr_seq));

        obj
    }

    fn from_stobject(obj: &STObject) -> Option<Self> {

        // Required fields
        let balance = obj.get_amount(sf::BALANCE)?;
        let last_ledger = obj.get_uint32(sf::LEDGER_SEQUENCE)?;
        let previous_txn_id = obj.get_hash256(sf::PREVIOUS_TXN_ID)?;
        let previous_txn_lgr_seq = obj.get_uint32(sf::PREVIOUS_TXN_LGR_SEQ)?;

        Some(Self {
            balance,
            last_ledger,
            previous_txn_id,
            previous_txn_lgr_seq,
        })
    }
}

/// LedgerStateManager - manages all ledger state using SHAMap
pub struct LedgerState {
    // SHAMap for storing ledger state entries
    state_map: shamap::SHAMap,
    // Nickname index for efficient search (nickname -> ledger index)
    nickname_index: std::collections::HashMap<String, UInt256>,
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
            nickname_index: std::collections::HashMap::new(),
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
            tracing::debug!("get_account_root: found item, data_len={}, attempting deserialization", item.data().len());
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

        // Add to nickname index for efficient search
        let nick_str = String::from_utf8_lossy(&nickname.nickname).to_lowercase();
        self.nickname_index.insert(nick_str, index);
    }

    pub fn delete_nickname(&mut self, nickname: &[u8]) {
        // Note: SHAMap doesn't have a direct remove method in the current API
        let _ = nickname;
    }

    pub fn get_signer_list(&self, account: &AccountID) -> Option<SignerList> {
        // Create a temporary SignerList to compute the ledger index
        let temp_list = SignerList::new(*account, 0);
        let index = temp_list.ledger_index();

        self.state_map.get_item(&index).and_then(|item| {
            Self::deserialize_signer_list(item.data())
        })
    }

    pub fn set_signer_list(&mut self, signer_list: &SignerList) {
        use shamap::SHAMapItem;
        let index = signer_list.ledger_index();
        let data = Self::serialize_signer_list(signer_list);
        let item = SHAMapItem::new(index, data);
        self.state_map.add_item(index, item);
    }

    /// Get a NicknameEntry by index
    pub fn get_nickname_entry(&self, nickname_index: &UInt256) -> Option<NicknameEntry> {
        self.state_map.get_item(nickname_index).and_then(|item| {
            Self::deserialize_nickname(item.data())
        })
    }

    /// Set a NicknameEntry
    pub fn set_nickname_entry(&mut self, nickname: &NicknameEntry) {
        use shamap::SHAMapItem;
        let index = nickname.ledger_index();
        let data = Self::serialize_nickname(nickname);
        let item = SHAMapItem::new(index, data);
        self.state_map.add_item(index, item);
    }

    /// Get all nicknames owned by an account
    pub fn get_account_nicknames(&self, account: &AccountID) -> Vec<NicknameEntry> {
        let mut results = Vec::new();
        for item in self.state_map.iter() {
            if let Some(nickname) = Self::deserialize_nickname(item.data()) {
                if nickname.account == *account {
                    results.push(nickname);
                }
            }
        }
        results
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

    /// Import all entries from an iterator of SHAMapItems
    /// Used to populate ledger_state from genesis ledger
    pub fn import_from_iter(&mut self, items: impl Iterator<Item = shamap::SHAMapItem>) {
        for item in items {
            let _ = self.state_map.add_item(item.key(), item);
        }
    }

    /// Import all state entries from a Ledger's state_tree
    /// Used to populate ledger_state from genesis ledger
    pub fn import_from_ledger(&mut self, ledger: &crate::Ledger) -> usize {
        let mut count = 0;
        tracing::info!("import_from_ledger: starting import from genesis ledger");
        for item in ledger.state_tree.iter() {
            let key = item.key();
            let key_hex: String = key.as_bytes().iter().map(|b| format!("{:02x}", b)).collect();
            tracing::info!("import_from_ledger: importing item with key={}", key_hex);
            // Clone the item since we need to add it to our state_map
            let cloned_item = shamap::SHAMapItem::new(item.key(), item.data().to_vec());
            let added = self.state_map.add_item(item.key(), cloned_item);
            tracing::info!("import_from_ledger: add_item result: {}", added);

            // Verify immediately after adding
            let verify = self.state_map.get_item(&key);
            tracing::info!("import_from_ledger: immediate verify result: {}", verify.is_some());

            count += 1;
        }
        tracing::info!("import_from_ledger: imported {} items", count);
        count
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

    /// Search nicknames by partial match using the nickname index
    /// Returns up to `limit` matching nickname entries
    pub fn search_nicknames(&self, search_term: &str, limit: usize) -> Vec<NicknameEntry> {
        let search_lower = search_term.to_lowercase();
        let mut results = Vec::new();

        // Use the nickname index for efficient prefix/substring search
        for (nick_str, index) in &self.nickname_index {
            if nick_str.contains(&search_lower) {
                if let Some(item) = self.state_map.get_item(index) {
                    if let Some(nick_entry) = Self::deserialize_nickname(item.data()) {
                        results.push(nick_entry);
                        if results.len() >= limit {
                            break;
                        }
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
        // First try STObject format (used by genesis)
        use serialization::SerialIter;
        use crate::ledger_entries::LedgerEntry;

        let mut iter = SerialIter::new(data);

        // Try to parse as STObject first
        tracing::debug!("deserialize_account_root: trying STObject format, data_len={}", data.len());
        if let Ok(obj) = iter.get_object() {
            tracing::debug!("deserialize_account_root: parsed STObject with {} fields", obj.len());
            // Try STObject deserialization via LedgerEntry trait
            if let Some(root) = AccountRoot::from_stobject(&obj) {
                tracing::debug!("deserialize_account_root: STObject deserialization successful, balance={}", root.balance.mantissa);
                return Some(root);
            }
            tracing::debug!("deserialize_account_root: STObject parsing succeeded but from_stobject failed");
        } else {
            tracing::debug!("deserialize_account_root: STObject parsing failed, falling back to raw format");
        }

        // Fall back to raw format
        // Account is stored as raw 20 bytes (not VL-encoded)
        iter.set_position(0);
        let stored_account_160 = iter.get160().ok()?;
        let _stored_account = AccountID::new(*stored_account_160.as_bytes());
        let balance = iter.get_amount().ok()?;
        let sequence = iter.get32().ok()?;
        let owner_count = iter.get32().ok()?;
        let previous_txn_id = iter.get256().ok()?;
        let previous_txn_lgr_seq = iter.get32().ok()?;

        tracing::debug!("deserialize_account_root: raw format successful, balance={}", balance.mantissa);
        Some(AccountRoot {
            account: *account,
            balance,
            sequence,
            owner_count,
            flags: account_flags::LSF_DEFAULT_CALL,
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

    fn serialize_signer_list(signer_list: &SignerList) -> Vec<u8> {
        use serialization::Serializer;
        let mut ser = Serializer::with_capacity(256);
        ser.add_account(signer_list.account);
        ser.add32(signer_list.signer_quorum);
        // Serialize signers count
        ser.add32(signer_list.signers.len() as u32);
        // Serialize each signer
        for signer in &signer_list.signers {
            ser.add_account(signer.account);
            ser.add8(signer.weight);
        }
        ser.finish()
    }

    pub fn deserialize_signer_list(data: &[u8]) -> Option<SignerList> {
        use serialization::SerialIter;
        let mut iter = SerialIter::new(data);

        let account = iter.get_account().ok()?;
        let signer_quorum = iter.get32().ok()?;
        let signers_count = iter.get32().ok()? as usize;

        let mut signers = Vec::with_capacity(signers_count.min(32)); // Max 32 signers

        for _ in 0..signers_count {
            if let Ok(signer_account) = iter.get_account() {
                if let Ok(weight) = iter.get8() {
                    signers.push(SignerEntry {
                        account: signer_account,
                        weight,
                    });
                }
            }
        }

        Some(SignerList {
            account,
            signer_quorum,
            signers,
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
    pub fn load_from_database(
        &mut self,
        database: &storage::Database,
        ledger_hash: primitives::UInt256,
    ) -> bool {
        use tracing::{debug, info, warn};
        use storage::NodeObjectType;

        // First, verify the ledger exists
        let _ledger_obj = match database.fetch_ledger(&ledger_hash) {
            Some(obj) => obj,
            None => {
                warn!("Ledger {} not found in database", ledger_hash);
                return false;
            }
        };

        info!("Loading ledger state for ledger {}", ledger_hash);

        // Clear current state
        self.state_map.clear();

        // Load all account nodes from the database
        let account_nodes = database.iterate_nodes(NodeObjectType::AccountNode);
        let mut loaded_count = 0;

        for node in account_nodes {
            // Try to deserialize as AccountRoot
            if let Some(account) = Self::deserialize_account_root_from_data(node.get_data()) {
                self.set_account_root(&account);
                loaded_count += 1;
            } else {
                // Try to deserialize as other ledger entry types
                if let Some(call_state) = Self::deserialize_call_state(node.get_data()) {
                    self.set_call_state(&call_state);
                    loaded_count += 1;
                } else if let Some(offer) = Self::deserialize_offer(node.get_data()) {
                    self.set_offer(&offer);
                    loaded_count += 1;
                } else if let Some(signer_list) = Self::deserialize_signer_list(node.get_data()) {
                    self.set_signer_list(&signer_list);
                    loaded_count += 1;
                } else if let Some(nickname) = Self::deserialize_nickname(node.get_data()) {
                    self.set_nickname(&nickname);
                    loaded_count += 1;
                } else {
                    debug!("Unknown ledger entry type for hash: {:?}", node.get_hash());
                }
            }
        }

        info!(
            "Loaded {} ledger entries from database for ledger {}",
            loaded_count, ledger_hash
        );

        loaded_count > 0
    }

    /// Deserialize AccountRoot from raw data (helper for loading)
    fn deserialize_account_root_from_data(data: &[u8]) -> Option<AccountRoot> {
        // First try to extract account from the data
        if data.len() < 20 {
            return None;
        }

        // Convert bytes to AccountID
        let mut account_bytes = [0u8; 20];
        account_bytes.copy_from_slice(&data[0..20]);
        let account_id = AccountID::new(account_bytes);

        // Try full deserialization first
        if let Some(account) = Self::deserialize_account_root(&account_id, data) {
            return Some(account);
        }

        None
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

    #[test]
    fn test_account_root_flags() {
        let account = AccountID::new([1u8; 20]);
        let mut account_root = AccountRoot::new(account);

        // Default flag is LSF_DEFAULT_CALL
        assert!(account_root.is_default_call());
        assert!(!account_root.requires_dest_tag());
        assert!(!account_root.is_disable_master());

        // Test setting flags
        account_root.set_flag(account_flags::LSF_REQUIRE_DEST_TAG);
        assert!(account_root.requires_dest_tag());

        account_root.set_flag(account_flags::LSF_DISABLE_MASTER);
        assert!(account_root.is_disable_master());

        // Test clearing flags
        account_root.clear_flag(account_flags::LSF_REQUIRE_DEST_TAG);
        assert!(!account_root.requires_dest_tag());

        // Test has_flag
        assert!(account_root.has_flag(account_flags::LSF_DEFAULT_CALL));
        assert!(account_root.has_flag(account_flags::LSF_DISABLE_MASTER));
        assert!(!account_root.has_flag(account_flags::LSF_REQUIRE_DEST_TAG));
    }

    #[test]
    fn test_account_root_flag_values() {
        // Verify flag values match calld specification
        assert_eq!(account_flags::LSF_DEFAULT_CALL, 0x00000001);
        assert_eq!(account_flags::LSF_NO_CALL, 0x00000002);
        assert_eq!(account_flags::LSF_REQUIRE_DEST_TAG, 0x00000004);
        assert_eq!(account_flags::LSF_REQUIRE_AUTH, 0x00000008);
        assert_eq!(account_flags::LSF_DISALLOW_CALL, 0x00000010);
        assert_eq!(account_flags::LSF_DISABLE_MASTER, 0x00000020);
        assert_eq!(account_flags::LSF_NO_FREEZE, 0x00000040);
        assert_eq!(account_flags::LSF_GLOBAL_FREEZE, 0x00000080);
        assert_eq!(account_flags::LSF_DEPOSIT_AUTH, 0x00000100);
    }

    #[test]
    fn test_account_set_flag_values() {
        // Verify AccountSet flag values match calld specification
        assert_eq!(account_set_flags::ASF_REQUIRE_DEST_TAG, 1);
        assert_eq!(account_set_flags::ASF_REQUIRE_AUTH, 2);
        assert_eq!(account_set_flags::ASF_DISALLOW_CALL, 3);
        assert_eq!(account_set_flags::ASF_DISABLE_MASTER, 4);
        assert_eq!(account_set_flags::ASF_ACCOUNT_TXN_ID, 5);
        assert_eq!(account_set_flags::ASF_NO_FREEZE, 6);
        assert_eq!(account_set_flags::ASF_GLOBAL_FREEZE, 7);
        assert_eq!(account_set_flags::ASF_DEFAULT_CALL, 8);
        assert_eq!(account_set_flags::ASF_DEPOSIT_AUTH, 9);
    }

    #[test]
    fn test_deposit_authorization() {
        let account = AccountID::new([1u8; 20]);
        let mut account_root = AccountRoot::new(account);

        // By default, deposit authorization is not required
        assert!(!account_root.requires_deposit_auth());

        // Enable deposit authorization
        account_root.set_flag(account_flags::LSF_DEPOSIT_AUTH);
        assert!(account_root.requires_deposit_auth());

        // Disable deposit authorization
        account_root.clear_flag(account_flags::LSF_DEPOSIT_AUTH);
        assert!(!account_root.requires_deposit_auth());
    }
}
