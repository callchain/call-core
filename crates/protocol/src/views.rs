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
}
