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
///
/// This implementation provides an in-memory ledger view suitable for unit tests.
/// It stores all ledger entry types in HashMaps for fast lookup and modification.
///
/// ## Usage
/// ```rust
/// let mut view = BasicLedgerView::new(ledger_hash, ledger_index);
/// view.set_account_root(&account_root);
/// let account = view.get_account_root(&account_id);
/// ```
#[derive(Debug, Clone)]
pub struct BasicLedgerView {
    pub ledger_hash: UInt256,
    pub ledger_index: u32,
    /// Account storage: AccountID -> AccountRoot
    accounts: std::collections::HashMap<AccountID, AccountRoot>,
    /// CallState storage: (AccountID, Issuer, Currency) -> CallState
    call_states: std::collections::HashMap<(AccountID, AccountID, Currency), CallState>,
    /// Offer storage: (AccountID, sequence) -> OfferEntry
    offers: std::collections::HashMap<(AccountID, u32), OfferEntry>,
    /// SignerList storage: AccountID -> SignerList
    signer_lists: std::collections::HashMap<AccountID, crate::ledger_entries::SignerList>,
    /// Nickname storage: nickname_index -> NicknameEntry
    nicknames: std::collections::HashMap<UInt256, crate::ledger_entries::NicknameEntry>,
    /// Account nicknames index: AccountID -> Vec<NicknameEntry>
    account_nicknames: std::collections::HashMap<AccountID, Vec<crate::ledger_entries::NicknameEntry>>,
    /// DepositPreauth storage: (AccountID, Authorize) -> DepositPreauth
    deposit_preauths: std::collections::HashMap<(AccountID, AccountID), crate::ledger_entries::DepositPreauth>,
}

impl BasicLedgerView {
    pub fn new(ledger_hash: UInt256, ledger_index: u32) -> Self {
        Self {
            ledger_hash,
            ledger_index,
            accounts: std::collections::HashMap::new(),
            call_states: std::collections::HashMap::new(),
            offers: std::collections::HashMap::new(),
            signer_lists: std::collections::HashMap::new(),
            nicknames: std::collections::HashMap::new(),
            account_nicknames: std::collections::HashMap::new(),
            deposit_preauths: std::collections::HashMap::new(),
        }
    }

    /// Create a new BasicLedgerView with a funded account for testing
    pub fn new_with_funded_account(ledger_hash: UInt256, ledger_index: u32, account: AccountID, balance: u64) -> Self {
        let mut view = Self::new(ledger_hash, ledger_index);
        let account_root = AccountRoot::new(account).with_balance(balance);
        view.set_account_root(&account_root);
        view
    }

    /// Get the number of accounts stored
    pub fn account_count(&self) -> usize {
        self.accounts.len()
    }

    /// Get the number of offers stored
    pub fn offer_count(&self) -> usize {
        self.offers.len()
    }

    /// Clear all stored state (useful for testing)
    pub fn clear(&mut self) {
        self.accounts.clear();
        self.call_states.clear();
        self.offers.clear();
        self.signer_lists.clear();
        self.nicknames.clear();
        self.account_nicknames.clear();
        self.deposit_preauths.clear();
    }
}

impl LedgerView for BasicLedgerView {
    fn get_account_root(&self, account: &AccountID) -> Option<AccountRoot> {
        self.accounts.get(account).cloned()
    }

    fn set_account_root(&mut self, account: &AccountRoot) {
        self.accounts.insert(account.account, account.clone());
    }

    fn get_call_state(
        &self,
        account: &AccountID,
        issuer: &AccountID,
        currency: &Currency,
    ) -> Option<CallState> {
        self.call_states
            .get(&(*account, *issuer, *currency))
            .cloned()
    }

    fn set_call_state(&mut self, state: &CallState) {
        let key = (state.account, state.issuer, state.currency);
        self.call_states.insert(key, state.clone());
    }

    fn get_offer(&self, account: &AccountID, sequence: u32) -> Option<OfferEntry> {
        self.offers.get(&(*account, sequence)).cloned()
    }

    fn set_offer(&mut self, offer: &OfferEntry) {
        self.offers.insert((offer.account, offer.sequence), offer.clone());
    }

    fn delete_offer(&mut self, account: &AccountID, sequence: u32) {
        self.offers.remove(&(*account, sequence));
    }

    fn get_ledger_info(&self) -> LedgerInfo {
        LedgerInfo::default()
    }

    fn get_signer_list(&self, account: &AccountID) -> Option<crate::ledger_entries::SignerList> {
        self.signer_lists.get(account).cloned()
    }

    fn set_signer_list(&mut self, signer_list: &crate::ledger_entries::SignerList) {
        self.signer_lists
            .insert(signer_list.account, signer_list.clone());
    }

    fn get_nickname_entry(
        &self,
        nickname_index: &UInt256,
    ) -> Option<crate::ledger_entries::NicknameEntry> {
        self.nicknames.get(nickname_index).cloned()
    }

    fn set_nickname_entry(&mut self, nickname: &crate::ledger_entries::NicknameEntry) {
        // Update nicknames index
        let index = nickname.ledger_index();
        self.nicknames.insert(index, nickname.clone());

        // Update account nicknames index
        self.account_nicknames
            .entry(nickname.account)
            .or_default()
            .push(nickname.clone());
    }

    fn get_account_nicknames(
        &self,
        account: &AccountID,
    ) -> Vec<crate::ledger_entries::NicknameEntry> {
        self.account_nicknames
            .get(account)
            .cloned()
            .unwrap_or_default()
    }

    fn get_deposit_preauth(
        &self,
        account: &AccountID,
        authorize: &AccountID,
    ) -> Option<crate::ledger_entries::DepositPreauth> {
        self.deposit_preauths.get(&(*account, *authorize)).cloned()
    }

    fn set_deposit_preauth(&mut self, preauth: &crate::ledger_entries::DepositPreauth) {
        self.deposit_preauths
            .insert((preauth.account, preauth.authorize), preauth.clone());
    }

    fn delete_deposit_preauth(&mut self, account: &AccountID, authorize: &AccountID) {
        self.deposit_preauths.remove(&(*account, *authorize));
    }

    fn is_authorized_to_send(&self, sender: &AccountID, recipient: &AccountID) -> bool {
        // Check if recipient has deposit auth enabled
        if let Some(recipient_account) = self.get_account_root(recipient) {
            if recipient_account.has_flag(crate::ledger_entries::account_flags::LSF_DEPOSIT_AUTH) {
                // Recipient requires deposit authorization
                // Check if sender is pre-authorized
                return self
                    .get_deposit_preauth(recipient, sender)
                    .map(|p| p.is_active())
                    .unwrap_or(false);
            }
        }
        // No deposit auth required or recipient not found
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

#[cfg(test)]
mod tests {
    use super::*;
    use primitives::{AccountID, Currency};
    use serialization::Amount;

    #[test]
    fn test_basic_ledger_view_account_operations() {
        let mut view = BasicLedgerView::new(UInt256::zero(), 1);
        let account = AccountID::new([1u8; 20]);

        // Initially no account
        assert!(view.get_account_root(&account).is_none());

        // Set account
        let account_root = AccountRoot::new(account).with_balance(1_000_000);
        view.set_account_root(&account_root);

        // Get account
        let retrieved = view.get_account_root(&account).unwrap();
        assert_eq!(retrieved.account, account);
        assert_eq!(retrieved.balance.mantissa, 1_000_000);
        assert_eq!(view.account_count(), 1);

        // Clear and verify
        view.clear();
        assert!(view.get_account_root(&account).is_none());
        assert_eq!(view.account_count(), 0);
    }

    #[test]
    fn test_basic_ledger_view_new_with_funded_account() {
        let account = AccountID::new([1u8; 20]);
        let view = BasicLedgerView::new_with_funded_account(UInt256::zero(), 1, account, 5_000_000);

        let retrieved = view.get_account_root(&account).unwrap();
        assert_eq!(retrieved.balance.mantissa, 5_000_000);
    }

    #[test]
    fn test_basic_ledger_view_offer_operations() {
        let mut view = BasicLedgerView::new(UInt256::zero(), 1);
        let account = AccountID::new([1u8; 20]);

        // Create an offer
        let offer = OfferEntry::new(
            account,
            1,
            Amount::call(1000),
            Amount::issued(500, 0, Currency::new(*b"USD\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0"), AccountID::new([2u8; 20])).unwrap(),
        );

        // Set offer
        view.set_offer(&offer);
        assert_eq!(view.offer_count(), 1);

        // Get offer
        let retrieved = view.get_offer(&account, 1).unwrap();
        assert_eq!(retrieved.account, account);
        assert_eq!(retrieved.sequence, 1);

        // Delete offer
        view.delete_offer(&account, 1);
        assert!(view.get_offer(&account, 1).is_none());
        assert_eq!(view.offer_count(), 0);
    }

    #[test]
    fn test_basic_ledger_view_call_state_operations() {
        let mut view = BasicLedgerView::new(UInt256::zero(), 1);
        let account = AccountID::new([1u8; 20]);
        let issuer = AccountID::new([2u8; 20]);
        let currency = Currency::new(*b"USD\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0");

        // Initially no call state
        assert!(view.get_call_state(&account, &issuer, &currency).is_none());

        // Set call state
        let mut call_state = CallState::new(account, issuer, currency);
        call_state.balance = Amount::issued(1000, 0, currency, issuer).unwrap();
        view.set_call_state(&call_state);

        // Get call state
        let retrieved = view.get_call_state(&account, &issuer, &currency).unwrap();
        assert_eq!(retrieved.account, account);
        assert_eq!(retrieved.issuer, issuer);
        assert_eq!(retrieved.currency, currency);
    }

    #[test]
    fn test_basic_ledger_view_signer_list_operations() {
        let mut view = BasicLedgerView::new(UInt256::zero(), 1);
        let account = AccountID::new([1u8; 20]);

        // Initially no signer list
        assert!(view.get_signer_list(&account).is_none());

        // Set signer list
        let signer_list = crate::ledger_entries::SignerList::new(account, 3);
        view.set_signer_list(&signer_list);

        // Get signer list
        let retrieved = view.get_signer_list(&account).unwrap();
        assert_eq!(retrieved.account, account);
        assert_eq!(retrieved.signer_quorum, 3);
    }

    #[test]
    fn test_basic_ledger_view_deposit_preauth() {
        let mut view = BasicLedgerView::new(UInt256::zero(), 1);
        let account = AccountID::new([1u8; 20]);
        let authorize = AccountID::new([2u8; 20]);

        // Initially no preauth
        assert!(view.get_deposit_preauth(&account, &authorize).is_none());

        // Set preauth
        let preauth = crate::ledger_entries::DepositPreauth::new(account, authorize);
        view.set_deposit_preauth(&preauth);

        // Get preauth
        let retrieved = view.get_deposit_preauth(&account, &authorize).unwrap();
        assert_eq!(retrieved.account, account);
        assert_eq!(retrieved.authorize, authorize);

        // Delete preauth
        view.delete_deposit_preauth(&account, &authorize);
        assert!(view.get_deposit_preauth(&account, &authorize).is_none());
    }

    #[test]
    fn test_basic_ledger_view_is_authorized_to_send() {
        let mut view = BasicLedgerView::new(UInt256::zero(), 1);
        let sender = AccountID::new([1u8; 20]);
        let recipient = AccountID::new([2u8; 20]);

        // Without deposit auth flag, anyone can send
        assert!(view.is_authorized_to_send(&sender, &recipient));

        // Add recipient account with deposit auth flag
        let mut recipient_account = AccountRoot::new(recipient);
        recipient_account.flags = crate::ledger_entries::account_flags::LSF_DEPOSIT_AUTH;
        view.set_account_root(&recipient_account);

        // Now sender is NOT authorized (no preauth)
        assert!(!view.is_authorized_to_send(&sender, &recipient));

        // Add preauthorization
        let preauth = crate::ledger_entries::DepositPreauth::new(recipient, sender);
        view.set_deposit_preauth(&preauth);

        // Now sender IS authorized
        assert!(view.is_authorized_to_send(&sender, &recipient));
    }
}
