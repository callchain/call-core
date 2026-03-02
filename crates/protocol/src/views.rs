/// Views provide read/write access to ledger data
use crate::ledger::LedgerInfo;
use crate::ledger_entries::{AccountRoot, CallState, OfferEntry};
use primitives::{AccountID, Currency, UInt256};

/// Trait for ledger access during transaction processing
pub trait LedgerView {
    /// Get an account root
    fn get_account_root(&self, account: &AccountID) -> Option<AccountRoot>;

    /// Set an account root
    fn set_account_root(&mut self, account: &AccountRoot);

    /// Get a CallState (trust line)
    fn get_call_state(
        &self,
        account: &AccountID,
        issuer: &AccountID,
        currency: &Currency,
    ) -> Option<CallState>;

    /// Set a CallState
    fn set_call_state(&mut self, state: &CallState);

    /// Get an offer
    fn get_offer(&self, account: &AccountID, sequence: u32) -> Option<OfferEntry>;

    /// Set an offer
    fn set_offer(&mut self, offer: &OfferEntry);

    /// Delete an offer
    fn delete_offer(&mut self, account: &AccountID, sequence: u32);

    /// Get ledger info
    fn get_ledger_info(&self) -> LedgerInfo;

    /// Get a SignerList
    fn get_signer_list(&self, account: &AccountID) -> Option<crate::ledger_entries::SignerList>;

    /// Set a SignerList
    fn set_signer_list(&mut self, signer_list: &crate::ledger_entries::SignerList);

    /// Get a NicknameEntry by index
    fn get_nickname_entry(
        &self,
        nickname_index: &UInt256,
    ) -> Option<crate::ledger_entries::NicknameEntry>;

    /// Set a NicknameEntry
    fn set_nickname_entry(&mut self, nickname: &crate::ledger_entries::NicknameEntry);

    /// Get all nicknames owned by an account
    fn get_account_nicknames(
        &self,
        account: &AccountID,
    ) -> Vec<crate::ledger_entries::NicknameEntry>;

    /// Get a deposit preauthorization
    fn get_deposit_preauth(
        &self,
        account: &AccountID,
        authorize: &AccountID,
    ) -> Option<crate::ledger_entries::DepositPreauth>;

    /// Set a deposit preauthorization
    fn set_deposit_preauth(&mut self, preauth: &crate::ledger_entries::DepositPreauth);

    /// Delete a deposit preauthorization
    fn delete_deposit_preauth(&mut self, account: &AccountID, authorize: &AccountID);

    /// Check if a sender is authorized to send deposits to a recipient
    fn is_authorized_to_send(&self, sender: &AccountID, recipient: &AccountID) -> bool;
}

/// Simple ledger view implementation for testing
#[derive(Debug, Clone)]
pub struct BasicLedgerView {
    pub ledger_hash: UInt256,
    pub ledger_index: u32,
}

impl BasicLedgerView {
    pub fn new(ledger_hash: UInt256, ledger_index: u32) -> Self {
        Self {
            ledger_hash,
            ledger_index,
        }
    }
}

impl LedgerView for BasicLedgerView {
    fn get_account_root(&self, _account: &AccountID) -> Option<AccountRoot> {
        None
    }

    fn set_account_root(&mut self, _account: &AccountRoot) {}

    fn get_call_state(
        &self,
        _account: &AccountID,
        _issuer: &AccountID,
        _currency: &Currency,
    ) -> Option<CallState> {
        None
    }

    fn set_call_state(&mut self, _state: &CallState) {}

    fn get_offer(&self, _account: &AccountID, _sequence: u32) -> Option<OfferEntry> {
        None
    }

    fn set_offer(&mut self, _offer: &OfferEntry) {}

    fn delete_offer(&mut self, _account: &AccountID, _sequence: u32) {}

    fn get_ledger_info(&self) -> LedgerInfo {
        LedgerInfo::default()
    }

    fn get_signer_list(&self, _account: &AccountID) -> Option<crate::ledger_entries::SignerList> {
        None
    }

    fn set_signer_list(&mut self, _signer_list: &crate::ledger_entries::SignerList) {}

    fn get_nickname_entry(
        &self,
        _nickname_index: &UInt256,
    ) -> Option<crate::ledger_entries::NicknameEntry> {
        None
    }

    fn set_nickname_entry(&mut self, _nickname: &crate::ledger_entries::NicknameEntry) {}

    fn get_account_nicknames(
        &self,
        _account: &AccountID,
    ) -> Vec<crate::ledger_entries::NicknameEntry> {
        Vec::new()
    }

    fn get_deposit_preauth(
        &self,
        _account: &AccountID,
        _authorize: &AccountID,
    ) -> Option<crate::ledger_entries::DepositPreauth> {
        None
    }

    fn set_deposit_preauth(&mut self, _preauth: &crate::ledger_entries::DepositPreauth) {}

    fn delete_deposit_preauth(&mut self, _account: &AccountID, _authorize: &AccountID) {}

    fn is_authorized_to_send(&self, _sender: &AccountID, _recipient: &AccountID) -> bool {
        // BasicLedgerView always allows (no deposit auth check)
        true
    }
}

use crate::ledger_entries::LedgerState;

/// Ledger view implementation that wraps a mutable LedgerState reference
/// This is used during transaction processing to apply changes to the actual ledger
pub struct MutableLedgerView<'a> {
    state: &'a mut LedgerState,
    ledger_info: LedgerInfo,
}

impl<'a> MutableLedgerView<'a> {
    pub fn new(state: &'a mut LedgerState, ledger_info: LedgerInfo) -> Self {
        Self { state, ledger_info }
    }
}

impl<'a> LedgerView for MutableLedgerView<'a> {
    fn get_account_root(&self, account: &AccountID) -> Option<AccountRoot> {
        self.state.get_account_root(account)
    }

    fn set_account_root(&mut self, account: &AccountRoot) {
        self.state.set_account_root(account);
    }

    fn get_call_state(
        &self,
        account: &AccountID,
        issuer: &AccountID,
        currency: &Currency,
    ) -> Option<CallState> {
        self.state.get_call_state(account, issuer, currency)
    }

    fn set_call_state(&mut self, state: &CallState) {
        self.state.set_call_state(state);
    }

    fn get_offer(&self, account: &AccountID, sequence: u32) -> Option<OfferEntry> {
        self.state.get_offer(account, sequence)
    }

    fn set_offer(&mut self, offer: &OfferEntry) {
        self.state.set_offer(offer);
    }

    fn delete_offer(&mut self, account: &AccountID, sequence: u32) {
        self.state.delete_offer(account, sequence);
    }

    fn get_ledger_info(&self) -> LedgerInfo {
        self.ledger_info.clone()
    }

    fn get_signer_list(&self, account: &AccountID) -> Option<crate::ledger_entries::SignerList> {
        self.state.get_signer_list(account)
    }

    fn set_signer_list(&mut self, signer_list: &crate::ledger_entries::SignerList) {
        self.state.set_signer_list(signer_list);
    }

    fn get_nickname_entry(&self, nickname_index: &UInt256) -> Option<crate::ledger_entries::NicknameEntry> {
        self.state.get_nickname_entry(nickname_index)
    }

    fn set_nickname_entry(&mut self, nickname: &crate::ledger_entries::NicknameEntry) {
        self.state.set_nickname_entry(nickname);
    }

    fn get_account_nicknames(&self, account: &AccountID) -> Vec<crate::ledger_entries::NicknameEntry> {
        self.state.get_account_nicknames(account)
    }

    fn get_deposit_preauth(
        &self,
        account: &AccountID,
        authorize: &AccountID,
    ) -> Option<crate::ledger_entries::DepositPreauth> {
        self.state.get_deposit_preauth(account, authorize)
    }

    fn set_deposit_preauth(&mut self, preauth: &crate::ledger_entries::DepositPreauth) {
        self.state.set_deposit_preauth(preauth);
    }

    fn delete_deposit_preauth(&mut self, account: &AccountID, authorize: &AccountID) {
        self.state.delete_deposit_preauth(account, authorize);
    }

    fn is_authorized_to_send(&self, sender: &AccountID, recipient: &AccountID) -> bool {
        self.state.is_authorized_to_send(sender, recipient)
    }
}
