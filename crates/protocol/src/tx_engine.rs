//! Transaction Processing Engine
//!
//! This module implements the core transaction validation and application logic.
//! Each transaction type follows a three-phase process:
//!
//! 1. **Preflight**: Static validation (format, signature syntax, basic checks)
//! 2. **Preclaim**: State-based validation (sufficient balance, sequence, etc.)
//! 3. **Apply**: Execute the transaction and update ledger state

use crate::ledger_entries::{AccountRoot, CallState, OfferEntry};
use crate::transactions::{TER, Transaction, TxType};
use crate::views::LedgerView;
use crypto::{PublicKey, KeyType};
use primitives::{AccountID, UInt256};

/// Rules for transaction processing
#[derive(Debug, Clone)]
pub struct ApplyRules {
    pub max_fee: u64,
    pub max_paths: usize,
    pub max_signers: usize,
    pub enable_amendments: bool,
}

impl Default for ApplyRules {
    fn default() -> Self {
        Self {
            max_fee: 10_000_000, // 10 CALL maximum fee
            max_paths: 8,
            max_signers: 8,
            enable_amendments: true,
        }
    }
}

/// Context for transaction application
pub struct ApplyContext<'a> {
    pub ledger: &'a mut dyn LedgerView,
    pub rules: ApplyRules,
    pub ledger_seq: u32,
    pub parent_ledger_hash: UInt256,
}

impl<'a> std::fmt::Debug for ApplyContext<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApplyContext")
            .field("rules", &self.rules)
            .field("ledger_seq", &self.ledger_seq)
            .field("parent_ledger_hash", &self.parent_ledger_hash)
            .finish_non_exhaustive()
    }
}

/// Transaction processing result
#[derive(Debug, Clone)]
pub struct TxResult {
    pub ter: TER,
    pub fee_charged: u64,
    pub affected_accounts: Vec<AccountID>,
    pub affected_nodes: Vec<AffectedLedgerNode>,
}

/// Affected node record for metadata
#[derive(Debug, Clone)]
pub enum AffectedLedgerNode {
    Created { ledger_index: UInt256, data: Vec<u8> },
    Modified { ledger_index: UInt256, previous: Vec<u8>, final_data: Vec<u8> },
    Deleted { ledger_index: UInt256, previous: Vec<u8> },
}

impl Default for TxResult {
    fn default() -> Self {
        Self {
            ter: TER::tesSUCCESS,
            fee_charged: 0,
            affected_accounts: Vec::new(),
            affected_nodes: Vec::new(),
        }
    }
}

/// Transaction processing engine
pub struct TransactionEngine;

impl TransactionEngine {
    /// Create a new transaction engine
    pub fn new() -> Self {
        Self
    }

    /// Process a transaction through all phases
    pub fn process(
        &self,
        ctx: &mut ApplyContext,
        tx: &Transaction,
    ) -> TxResult {
        // Phase 1: Preflight checks (static validation)
        let result = self.preflight(ctx, tx);
        if result != TER::tesSUCCESS {
            return TxResult {
                ter: result,
                fee_charged: 0,
                affected_accounts: Vec::new(),
                affected_nodes: Vec::new(),
            };
        }

        // Phase 2: Preclaim checks (state-based validation)
        let result = self.preclaim(ctx, tx);
        if result != TER::tesSUCCESS {
            return TxResult {
                ter: result,
                fee_charged: 0,
                affected_accounts: Vec::new(),
                affected_nodes: Vec::new(),
            };
        }

        // Phase 3: Apply the transaction
        self.apply(ctx, tx)
    }

    /// Preflight checks - static validation
    fn preflight(&self, ctx: &ApplyContext, tx: &Transaction) -> TER {
        // Check transaction type is valid
        match tx.tx_type {
            TxType::Payment
            | TxType::IssueSet
            | TxType::TrustSet
            | TxType::OfferCreate
            | TxType::OfferCancel
            | TxType::AccountSet
            | TxType::SetRegularKey
            | TxType::SignerListSet => {}
            _ => return TER::temINVALID_TRANSACTION_TYPE,
        }

        // Check fee is reasonable
        if tx.fee > ctx.rules.max_fee {
            return TER::telINSUFFICIENT_FEE;
        }

        // Check sequence is valid (must be > 0)
        if tx.sequence == 0 {
            return TER::temBAD_SEQUENCE;
        }

        // Validate signing public key if present
        if let Some(pk) = &tx.signing_pub_key {
            if pk.len() != 33 && pk.len() != 32 {
                return TER::temBAD_SIGNATURE;
            }
        }

        // Check transaction signature exists and verify it
        if tx.txn_signature.is_none() {
            return TER::temEMPTY_SIGNER;
        }

        // Verify the transaction signature cryptographically
        if !self.verify_transaction_signature(tx) {
            return TER::temBAD_SIGNATURE;
        }

        // Type-specific preflight checks
        match tx.tx_type {
            TxType::Payment => self.preflight_payment(ctx, tx),
            TxType::IssueSet => self.preflight_issue_set(ctx, tx),
            TxType::TrustSet => self.preflight_trust_set(ctx, tx),
            TxType::OfferCreate => self.preflight_offer_create(ctx, tx),
            TxType::OfferCancel => self.preflight_offer_cancel(ctx, tx),
            TxType::AccountSet => self.preflight_account_set(ctx, tx),
            TxType::SetRegularKey => self.preflight_set_regular_key(ctx, tx),
            TxType::SignerListSet => self.preflight_signer_list_set(ctx, tx),
            TxType::NicknameSet => self.preflight_nickname_set(ctx, tx),
            _ => TER::temINVALID_TRANSACTION_TYPE,
        }
    }

    /// Preclaim checks - state-based validation
    fn preclaim(&self, ctx: &ApplyContext, tx: &Transaction) -> TER {
        // Check source account exists
        let account = match ctx.ledger.get_account_root(&tx.account) {
            Some(acc) => acc,
            None => return TER::terNO_ACCOUNT,
        };

        // Check sequence number matches
        if tx.sequence != account.sequence {
            return TER::terPRE_SEQ;
        }

        // Check sufficient balance for fee
        let fee_amount = tx.fee as i64;
        if account.balance.mantissa < fee_amount {
            return TER::terINSUFF_FEE;
        }

        // Type-specific preclaim checks
        match tx.tx_type {
            TxType::Payment => self.preclaim_payment(ctx, tx, &account),
            TxType::IssueSet => self.preclaim_issue_set(ctx, tx, &account),
            TxType::TrustSet => self.preclaim_trust_set(ctx, tx, &account),
            TxType::OfferCreate => self.preclaim_offer_create(ctx, tx, &account),
            TxType::OfferCancel => self.preclaim_offer_cancel(ctx, tx, &account),
            TxType::AccountSet => self.preclaim_account_set(ctx, tx, &account),
            TxType::SetRegularKey => self.preclaim_set_regular_key(ctx, tx, &account),
            TxType::SignerListSet => self.preclaim_signer_list_set(ctx, tx, &account),
            TxType::NicknameSet => self.preclaim_nickname_set(ctx, tx, &account),
            _ => TER::temINVALID_TRANSACTION_TYPE,
        }
    }

    /// Apply the transaction
    fn apply(&self, ctx: &mut ApplyContext, tx: &Transaction) -> TxResult {
        let mut result = TxResult::default();
        let fee = tx.fee;
        result.fee_charged = fee;

        // Get the source account (we know it exists from preclaim)
        let mut account = ctx.ledger.get_account_root(&tx.account).unwrap();

        // Increment sequence
        account.increment_sequence();

        // Deduct fee
        let fee_i64 = fee as i64;
        account.balance.mantissa = account.balance.mantissa.saturating_sub(fee_i64);

        // Update transaction tracking
        account.update_previous_txn(tx.get_hash(), ctx.ledger_seq);

        // Add to affected accounts
        result.affected_accounts.push(tx.account);

        // Type-specific application
        let type_result = match tx.tx_type {
            TxType::Payment => self.apply_payment(ctx, tx, &mut account, &mut result),
            TxType::IssueSet => self.apply_issue_set(ctx, tx, &mut account, &mut result),
            TxType::TrustSet => self.apply_trust_set(ctx, tx, &mut account, &mut result),
            TxType::OfferCreate => self.apply_offer_create(ctx, tx, &mut account, &mut result),
            TxType::OfferCancel => self.apply_offer_cancel(ctx, tx, &mut account, &mut result),
            TxType::AccountSet => self.apply_account_set(ctx, tx, &mut account, &mut result),
            TxType::SetRegularKey => self.apply_set_regular_key(ctx, tx, &mut account, &mut result),
            TxType::SignerListSet => self.apply_signer_list_set(ctx, tx, &mut account, &mut result),
            TxType::NicknameSet => self.apply_nickname_set(ctx, tx, &mut account, &mut result),
            _ => TER::tecINTERNAL,
        };

        result.ter = type_result;

        // Update the account root regardless of result
        ctx.ledger.set_account_root(&account);

        result
    }
}

// Payment transaction implementation
impl TransactionEngine {
    fn preflight_payment(&self, _ctx: &ApplyContext, tx: &Transaction) -> TER {
        // Must have destination
        if tx.destination.is_none() {
            return TER::temDST_NEEDED;
        }

        // Must have amount
        if tx.amount.is_none() {
            return TER::temBAD_AMOUNT;
        }

        // Cannot send to self
        if tx.destination == Some(tx.account) {
            return TER::temREDUNDANT;
        }

        TER::tesSUCCESS
    }

    fn preclaim_payment(&self, ctx: &ApplyContext, tx: &Transaction, account: &AccountRoot) -> TER {
        let amount = tx.amount.as_ref().unwrap();

        // Check sufficient balance for amount + fee
        // Amount mantissa is i64, fee is u64 - convert carefully
        let amount_val = amount.mantissa.max(0) as u64;
        let total_needed = amount_val.saturating_add(tx.fee);
        let balance_val = account.balance.mantissa.max(0) as u64;
        if balance_val < total_needed {
            return TER::terINSUFF_FEE;
        }

        // Check destination exists (unless creating it)
        let destination = tx.destination.as_ref().unwrap();
        let dest_exists = ctx.ledger.get_account_root(destination).is_some();

        if !dest_exists {
            // Destination doesn't exist - need to check if we're creating it
            // Account creation requires minimum reserve (currently 10 CALL)
            const ACCOUNT_CREATION_RESERVE: i64 = 10_000_000; // 10 CALL in drops

            if amount.mantissa < ACCOUNT_CREATION_RESERVE {
                return TER::tecNO_DST_INSUF_CALL;
            }

            // Also check sender has enough for their own reserve
            // Owner count would increase if sender doesn't have enough reserve
            // Default reserve values: 10 CALL base, 2 CALL increment
            let reserve_base: u64 = 10_000_000; // 10 CALL in drops
            let reserve_inc: u64 = 2_000_000;   // 2 CALL in drops
            let sender_reserve_needed = reserve_base
                + (account.owner_count as u64 * reserve_inc);

            let sender_balance_after = account.balance.mantissa.saturating_sub(amount_val as i64);
            if (sender_balance_after as u64) < sender_reserve_needed {
                return TER::tecUNFUNDED_PAYMENT;
            }
        }

        TER::tesSUCCESS
    }

    fn apply_payment(
        &self,
        ctx: &mut ApplyContext,
        tx: &Transaction,
        sender: &mut AccountRoot,
        result: &mut TxResult,
    ) -> TER {
        let amount = tx.amount.clone().unwrap();
        let destination = tx.destination.unwrap();

        // Deduct from sender (amount.mantissa is i64, balance.mantissa is i64)
        let amount_val = amount.mantissa.max(0);
        sender.balance.mantissa = sender.balance.mantissa.saturating_sub(amount_val);

        // Get or create destination account
        let mut recipient = match ctx.ledger.get_account_root(&destination) {
            Some(acc) => acc,
            None => {
                // Create new account (if minimum reserve met)
                AccountRoot::new(destination)
            }
        };

        // Add to recipient
        recipient.balance.mantissa = recipient.balance.mantissa.saturating_add(amount_val);

        // Store updated accounts
        ctx.ledger.set_account_root(&recipient);

        if destination != sender.account {
            result.affected_accounts.push(destination);
        }

        TER::tesSUCCESS
    }
}

// IssueSet transaction implementation
impl TransactionEngine {
    fn preflight_issue_set(&self, _ctx: &ApplyContext, tx: &Transaction) -> TER {
        // Must have amount
        if tx.amount.is_none() {
            return TER::temBAD_AMOUNT;
        }

        TER::tesSUCCESS
    }

    fn preclaim_issue_set(&self, _ctx: &ApplyContext, _tx: &Transaction, _account: &AccountRoot) -> TER {
        // Any account can issue tokens
        TER::tesSUCCESS
    }

    fn apply_issue_set(
        &self,
        _ctx: &mut ApplyContext,
        tx: &Transaction,
        issuer: &mut AccountRoot,
        _result: &mut TxResult,
    ) -> TER {
        let amount = tx.amount.clone().unwrap();

        // Mint new tokens - increase issuer's balance
        let amount_val = amount.mantissa.max(0);
        issuer.balance.mantissa = issuer.balance.mantissa.saturating_add(amount_val);

        TER::tesSUCCESS
    }
}

// TrustSet transaction implementation
impl TransactionEngine {
    fn preflight_trust_set(&self, _ctx: &ApplyContext, tx: &Transaction) -> TER {
        // Must have limit amount
        if tx.limit_amount.is_none() {
            return TER::temBAD_AMOUNT;
        }

        TER::tesSUCCESS
    }

    fn preclaim_trust_set(&self, _ctx: &ApplyContext, _tx: &Transaction, _account: &AccountRoot) -> TER {
        TER::tesSUCCESS
    }

    fn apply_trust_set(
        &self,
        ctx: &mut ApplyContext,
        tx: &Transaction,
        _account: &mut AccountRoot,
        _result: &mut TxResult,
    ) -> TER {
        let limit = tx.limit_amount.clone().unwrap();

        // Get or create the CallState (trust line)
        // Use the currency from the limit_amount
        let currency = limit.currency;
        let issuer = tx.issuer.unwrap_or(tx.account);

        let mut call_state = ctx
            .ledger
            .get_call_state(&tx.account, &issuer, &currency)
            .unwrap_or_else(|| CallState::new(tx.account, issuer, currency));

        // Update limit
        call_state.limit = limit;

        ctx.ledger.set_call_state(&call_state);

        TER::tesSUCCESS
    }
}

// OfferCreate transaction implementation
impl TransactionEngine {
    fn preflight_offer_create(&self, _ctx: &ApplyContext, tx: &Transaction) -> TER {
        // Must have both taker_pays and taker_gets
        if tx.taker_pays.is_none() || tx.taker_gets.is_none() {
            return TER::temBAD_OFFER;
        }

        // Cannot create an offer where taker_pays and taker_gets are the same currency
        // and same issuer (would be a trivial exchange)
        let taker_pays = tx.taker_pays.as_ref().unwrap();
        let taker_gets = tx.taker_gets.as_ref().unwrap();

        // Check if currencies are the same
        if taker_pays.currency == taker_gets.currency {
            // Same currency is allowed only if it's a different issuer (for trust line trading)
            // or if the amounts are different (allowing for price discovery)
            // For now, we allow same currency offers as they can be useful for order book depth
        }

        // Check that amounts are positive
        if taker_pays.mantissa <= 0 || taker_gets.mantissa <= 0 {
            return TER::temBAD_AMOUNT;
        }

        TER::tesSUCCESS
    }

    fn preclaim_offer_create(&self, _ctx: &ApplyContext, tx: &Transaction, account: &AccountRoot) -> TER {
        // Check sufficient balance for what the taker is offering
        let taker_gets = tx.taker_gets.as_ref().unwrap();

        // Check owner reserve (if creating a new offer)
        let taker_gets_val = taker_gets.mantissa.max(0);
        let balance_val = account.balance.mantissa.max(0);
        if taker_gets_val > balance_val {
            return TER::terINSUFF_FEE;
        }

        TER::tesSUCCESS
    }

    fn apply_offer_create(
        &self,
        ctx: &mut ApplyContext,
        tx: &Transaction,
        account: &mut AccountRoot,
        _result: &mut TxResult,
    ) -> TER {
        let taker_pays = tx.taker_pays.clone().unwrap();
        let taker_gets = tx.taker_gets.clone().unwrap();

        // Create offer entry
        let offer = OfferEntry::new(tx.account, tx.sequence, taker_pays, taker_gets);

        ctx.ledger.set_offer(&offer);

        // Increment owner count
        account.add_owner_count(1);

        TER::tesSUCCESS
    }
}

// OfferCancel transaction implementation
impl TransactionEngine {
    fn preflight_offer_cancel(&self, _ctx: &ApplyContext, tx: &Transaction) -> TER {
        // Must have offer sequence
        if tx.offer_sequence == 0 {
            return TER::temBAD_OFFER;
        }

        TER::tesSUCCESS
    }

    fn preclaim_offer_cancel(&self, ctx: &ApplyContext, tx: &Transaction, _account: &AccountRoot) -> TER {
        // Check offer exists
        if ctx.ledger.get_offer(&tx.account, tx.offer_sequence).is_none() {
            return TER::terNO_OFFER;
        }

        TER::tesSUCCESS
    }

    fn apply_offer_cancel(
        &self,
        ctx: &mut ApplyContext,
        tx: &Transaction,
        account: &mut AccountRoot,
        _result: &mut TxResult,
    ) -> TER {
        // Delete the offer
        ctx.ledger.delete_offer(&tx.account, tx.offer_sequence);

        // Decrement owner count
        account.subtract_owner_count(1);

        TER::tesSUCCESS
    }
}

// AccountSet transaction implementation
impl TransactionEngine {
    fn preflight_account_set(&self, _ctx: &ApplyContext, tx: &Transaction) -> TER {
        // Can set domain, email hash, message key, transfer rate, tick size
        // Validate tick size if present (1-15)
        if let Some(tick_size) = tx.tick_size {
            if tick_size == 0 || tick_size > 15 {
                return TER::temBAD_TICK_SIZE;
            }
        }

        TER::tesSUCCESS
    }

    fn preclaim_account_set(&self, _ctx: &ApplyContext, _tx: &Transaction, _account: &AccountRoot) -> TER {
        TER::tesSUCCESS
    }

    fn apply_account_set(
        &self,
        _ctx: &mut ApplyContext,
        tx: &Transaction,
        account: &mut AccountRoot,
        _result: &mut TxResult,
    ) -> TER {
        // Update domain if provided
        if let Some(domain) = &tx.domain {
            account.domain = Some(domain.clone());
        }

        // Update email hash if provided
        if let Some(email_hash) = tx.email_hash {
            account.email_hash = Some(email_hash);
        }

        // Update message key if provided
        if let Some(message_key) = &tx.message_key {
            account.message_key = Some(message_key.clone());
        }

        // Update transfer rate if provided
        if let Some(transfer_rate) = tx.transfer_rate {
            account.transfer_rate = Some(transfer_rate);
        }

        // Update tick size if provided
        if let Some(tick_size) = tx.tick_size {
            account.tick_size = Some(tick_size);
        }

        TER::tesSUCCESS
    }
}

// SetRegularKey transaction implementation
impl TransactionEngine {
    fn preflight_set_regular_key(&self, _ctx: &ApplyContext, tx: &Transaction) -> TER {
        // Must have regular key if setting
        if tx.regular_key.is_none() {
            return TER::temBAD_REGULAR_KEY;
        }

        TER::tesSUCCESS
    }

    fn preclaim_set_regular_key(&self, _ctx: &ApplyContext, _tx: &Transaction, _account: &AccountRoot) -> TER {
        TER::tesSUCCESS
    }

    fn apply_set_regular_key(
        &self,
        _ctx: &mut ApplyContext,
        tx: &Transaction,
        account: &mut AccountRoot,
        _result: &mut TxResult,
    ) -> TER {
        account.regular_key = tx.regular_key;
        TER::tesSUCCESS
    }
}

// SignerListSet transaction implementation
impl TransactionEngine {
    fn preflight_signer_list_set(&self, ctx: &ApplyContext, tx: &Transaction) -> TER {
        // Check number of signers
        if tx.signers.len() > ctx.rules.max_signers {
            return TER::temBAD_SIGNER_LIST;
        }

        // Validate signer weights
        let mut total_weight = 0u32;
        for signer in &tx.signers {
            total_weight += signer.weight as u32;
        }

        // Total weight must be >= quorum
        if total_weight < tx.signer_quorum {
            return TER::temBAD_SIGNER_LIST;
        }

        TER::tesSUCCESS
    }

    fn preclaim_signer_list_set(&self, _ctx: &ApplyContext, _tx: &Transaction, _account: &AccountRoot) -> TER {
        // Check owner reserve for signers
        // Each signer entry requires reserve
        TER::tesSUCCESS
    }

    fn apply_signer_list_set(
        &self,
        ctx: &mut ApplyContext,
        tx: &Transaction,
        account: &mut AccountRoot,
        _result: &mut TxResult,
    ) -> TER {
        use crate::ledger_entries::SignerList;

        // Create or update the SignerList ledger entry
        let mut signer_list = SignerList::new(account.account, tx.signer_quorum);

        // Add all signers from the transaction
        for signer_entry in &tx.signers {
            signer_list.add_signer(signer_entry.clone());
        }

        // Update previous txn info
        // Note: Would use actual transaction hash if available in result
        signer_list.update_previous_txn(UInt256::zero(), ctx.ledger_seq);

        // Store the signer list in the ledger state
        ctx.ledger.set_signer_list(&signer_list);

        // Update account owner count
        account.owner_count = account.owner_count.saturating_add(1);

        TER::tesSUCCESS
    }
}

// NicknameSet transaction implementation
impl TransactionEngine {
    fn preflight_nickname_set(&self, _ctx: &ApplyContext, tx: &Transaction) -> TER {
        // Must have a nickname
        if tx.nickname.is_none() {
            return TER::temBAD_SIGNATURE; // Using appropriate error code
        }

        let nickname = tx.nickname.as_ref().unwrap();

        // Nickname must not be empty
        if nickname.is_empty() {
            return TER::temBAD_SIGNATURE;
        }

        // Nickname must not be too long (max 128 bytes)
        if nickname.len() > 128 {
            return TER::temBAD_SIGNATURE;
        }

        // Validate nickname characters (alphanumeric, underscore, hyphen, dot)
        for byte in nickname {
            if !byte.is_ascii_alphanumeric()
                && *byte != b'_'
                && *byte != b'-'
                && *byte != b'.'
            {
                return TER::temBAD_SIGNATURE;
            }
        }

        TER::tesSUCCESS
    }

    fn preclaim_nickname_set(&self, ctx: &ApplyContext, tx: &Transaction, account: &AccountRoot) -> TER {
        // Check if nickname already exists for another account
        if let Some(ref nickname) = tx.nickname {
            let nickname_hash = crypto::sha256(nickname);
            let nickname_index = UInt256::new(nickname_hash);

            // Check if this nickname is already taken by another account
            if let Some(existing_nickname) = ctx.ledger.get_nickname_entry(&nickname_index) {
                if existing_nickname.account != tx.account {
                    return TER::terDUPLICATE;
                }
            }
        }

        // Check if account has sufficient reserve for creating new ledger entry
        // Only if they don't already have a nickname
        let has_existing_nickname = ctx.ledger.get_account_nicknames(&tx.account).len() > 0;

        if !has_existing_nickname {
            // Creating new nickname entry requires reserve
            // Default reserve: 10 CALL base + 2 CALL per owner object
            let reserve_base: u64 = 10_000_000; // 10 CALL in drops
            let reserve_inc: u64 = 2_000_000;   // 2 CALL in drops
            let owner_reserve = reserve_base + (account.owner_count as u64 * reserve_inc);

            if (account.balance.mantissa.max(0) as u64) < owner_reserve + tx.fee {
                return TER::tecINSUF_RESERVE_OFFER;
            }
        }

        TER::tesSUCCESS
    }

    fn apply_nickname_set(
        &self,
        ctx: &mut ApplyContext,
        tx: &Transaction,
        account: &mut AccountRoot,
        result: &mut TxResult,
    ) -> TER {
        use crate::ledger_entries::NicknameEntry;

        let nickname = match &tx.nickname {
            Some(n) => n.clone(),
            None => return TER::tecINTERNAL,
        };

        // Compute nickname index (hash of nickname)
        let nickname_hash = crypto::sha256(&nickname);
        let nickname_index = UInt256::new(nickname_hash);

        // Check if nickname already exists for this account
        let is_new = match ctx.ledger.get_nickname_entry(&nickname_index) {
            Some(existing) if existing.account == tx.account => false,
            Some(_) => return TER::tecDUPLICATE, // Should have been caught in preclaim
            None => true,
        };

        // Create or update nickname entry
        let mut nickname_entry = NicknameEntry::new(
            nickname,
            account.account,
        );

        // Set minimum offer if provided
        if let Some(min_offer) = &tx.min_offer {
            nickname_entry.min_offer = Some(min_offer.clone());
        }

        // Update previous txn info
        nickname_entry.update_previous_txn(tx.get_hash(), ctx.ledger_seq);

        // Store the nickname entry
        ctx.ledger.set_nickname_entry(&nickname_entry);

        // Update account owner count if this is a new nickname
        if is_new {
            account.owner_count = account.owner_count.saturating_add(1);
        }

        // Track affected nodes for metadata
        result.affected_nodes.push(AffectedLedgerNode::Created {
            ledger_index: nickname_index,
            data: vec![], // Would serialize the entry in full implementation
        });

        TER::tesSUCCESS
    }

    /// Verify transaction signature cryptographically
    fn verify_transaction_signature(&self, tx: &Transaction) -> bool {
        // Get the signing public key
        let pk_bytes = match &tx.signing_pub_key {
            Some(pk) => pk,
            None => return false,
        };

        // Get the signature
        let sig_bytes = match &tx.txn_signature {
            Some(sig) => sig,
            None => return false,
        };

        // Determine key type from public key length
        // Secp256k1: 33 bytes (compressed), Ed25519: 32 bytes
        let key_type = if pk_bytes.len() == 33 {
            KeyType::Secp256k1
        } else if pk_bytes.len() == 32 {
            KeyType::Ed25519
        } else {
            return false;
        };

        // Create public key
        let public_key = match key_type {
            KeyType::Secp256k1 => {
                if pk_bytes.len() != 33 {
                    return false;
                }
                PublicKey::from_bytes(KeyType::Secp256k1, pk_bytes)
            }
            KeyType::Ed25519 => {
                if pk_bytes.len() != 32 {
                    return false;
                }
                PublicKey::from_bytes(KeyType::Ed25519, pk_bytes)
            }
        };

        let public_key = match public_key {
            Some(pk) => pk,
            None => return false,
        };

        // Create signature
        let signature = crypto::Signature::new(key_type, sig_bytes.clone());

        // Get the transaction data to verify (excluding the signature itself)
        let message_hash = self.get_transaction_signing_hash(tx);

        // Verify the signature
        public_key.verify(message_hash.as_bytes(), &signature)
    }

    /// Get the hash of transaction data that should be signed
    fn get_transaction_signing_hash(&self, tx: &Transaction) -> UInt256 {
        use crypto::sha512_half;

        // Build signing data by combining critical transaction fields
        let mut data = Vec::new();

        // Add transaction type (as bytes)
        data.extend_from_slice(&(tx.tx_type as u16).to_be_bytes());

        // Add account
        data.extend_from_slice(tx.account.as_bytes());

        // Add sequence
        data.extend_from_slice(&tx.sequence.to_be_bytes());

        // Add fee
        data.extend_from_slice(&tx.fee.to_be_bytes());

        // Add transaction-specific fields based on type
        match tx.tx_type {
            TxType::Payment => {
                if let Some(dest) = &tx.destination {
                    data.extend_from_slice(dest.as_bytes());
                }
                if let Some(amt) = &tx.amount {
                    // Add amount fields directly
                    data.extend_from_slice(&amt.mantissa.to_be_bytes());
                    data.extend_from_slice(&amt.exponent.to_be_bytes());
                    data.extend_from_slice(amt.currency.as_bytes());
                }
            }
            TxType::TrustSet => {
                if let Some(issuer) = &tx.issuer {
                    data.extend_from_slice(issuer.as_bytes());
                }
                if let Some(limit) = &tx.limit_amount {
                    data.extend_from_slice(&limit.mantissa.to_be_bytes());
                    data.extend_from_slice(&limit.exponent.to_be_bytes());
                    data.extend_from_slice(limit.currency.as_bytes());
                }
            }
            TxType::OfferCreate => {
                if let Some(pays) = &tx.taker_pays {
                    data.extend_from_slice(&pays.mantissa.to_be_bytes());
                    data.extend_from_slice(&pays.exponent.to_be_bytes());
                    data.extend_from_slice(pays.currency.as_bytes());
                }
                if let Some(gets) = &tx.taker_gets {
                    data.extend_from_slice(&gets.mantissa.to_be_bytes());
                    data.extend_from_slice(&gets.exponent.to_be_bytes());
                    data.extend_from_slice(gets.currency.as_bytes());
                }
                data.extend_from_slice(&tx.offer_sequence.to_be_bytes());
            }
            TxType::OfferCancel => {
                data.extend_from_slice(&tx.offer_sequence.to_be_bytes());
            }
            TxType::AccountSet => {
                if let Some(domain) = &tx.domain {
                    data.extend_from_slice(domain.as_slice());
                }
                if let Some(set_flag) = tx.set_flag {
                    data.extend_from_slice(&set_flag.to_be_bytes());
                }
                if let Some(clear_flag) = tx.clear_flag {
                    data.extend_from_slice(&clear_flag.to_be_bytes());
                }
            }
            TxType::SetRegularKey => {
                if let Some(key) = &tx.regular_key {
                    data.extend_from_slice(key.as_bytes());
                }
            }
            TxType::SignerListSet => {
                data.extend_from_slice(&tx.signer_quorum.to_be_bytes());
            }
            _ => {}
        }

        // Add signing public key
        if let Some(pk) = &tx.signing_pub_key {
            data.extend_from_slice(pk.as_slice());
        }

        // Hash with SHA-512/256 for the final signing message
        sha512_half(&data)
    }
}

impl Default for TransactionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use primitives::{AccountID, Currency};
    use serialization::Amount;

    struct MockLedgerView {
        accounts: std::collections::HashMap<AccountID, AccountRoot>,
    }

    impl MockLedgerView {
        fn new() -> Self {
            Self {
                accounts: std::collections::HashMap::new(),
            }
        }
    }

    impl LedgerView for MockLedgerView {
        fn get_account_root(&self, account: &AccountID) -> Option<AccountRoot> {
            self.accounts.get(account).cloned()
        }

        fn set_account_root(&mut self, account: &AccountRoot) {
            self.accounts.insert(account.account, account.clone());
        }

        fn get_call_state(&self, _account: &AccountID, _issuer: &AccountID, _currency: &Currency) -> Option<CallState> {
            None
        }

        fn set_call_state(&mut self, _state: &CallState) {}

        fn get_offer(&self, _account: &AccountID, _sequence: u32) -> Option<OfferEntry> {
            None
        }

        fn set_offer(&mut self, _offer: &OfferEntry) {}

        fn delete_offer(&mut self, _account: &AccountID, _sequence: u32) {}

        fn get_ledger_info(&self) -> crate::ledger::LedgerInfo {
            crate::ledger::LedgerInfo::default()
        }

        fn get_signer_list(&self, _account: &AccountID) -> Option<crate::ledger_entries::SignerList> {
            None
        }

        fn set_signer_list(&mut self, _signer_list: &crate::ledger_entries::SignerList) {}

        fn get_nickname_entry(&self, _nickname_index: &UInt256) -> Option<crate::ledger_entries::NicknameEntry> {
            None
        }

        fn set_nickname_entry(&mut self, _nickname: &crate::ledger_entries::NicknameEntry) {}

        fn get_account_nicknames(&self, _account: &AccountID) -> Vec<crate::ledger_entries::NicknameEntry> {
            Vec::new()
        }
    }

    #[test]
    fn test_payment_validation() {
        use crypto::PrivateKey;

        let engine = TransactionEngine::new();
        let mut ledger = MockLedgerView::new();

        // Generate a key pair for signing
        let private_key = PrivateKey::generate_secp256k1();
        let public_key = private_key.to_public_key();

        // Create sender account (use a fixed account for testing)
        let sender = AccountID::new([1u8; 20]);
        let sender_root = AccountRoot::new(sender).with_balance(1_000_000);
        ledger.set_account_root(&sender_root);

        // Create destination account to avoid account creation reserve check
        let destination = AccountID::new([2u8; 20]);
        let dest_root = AccountRoot::new(destination).with_balance(1);
        ledger.set_account_root(&dest_root);

        let mut ctx = ApplyContext {
            ledger: &mut ledger,
            rules: ApplyRules::default(),
            ledger_seq: 1,
            parent_ledger_hash: UInt256::zero(),
        };

        // Create payment transaction
        let mut tx = Transaction::new_payment(
            sender,
            destination,
            Amount::call(100_000),
        );
        tx.sequence = 1;
        tx.fee = 1000;
        tx.signing_pub_key = Some(public_key.as_bytes().to_vec());

        // Should fail preflight without signature
        let result = engine.process(&mut ctx, &tx);
        assert_eq!(result.ter, TER::temEMPTY_SIGNER);

        // Sign the transaction
        let message = engine.get_transaction_signing_hash(&tx);
        let signature = private_key.sign(message.as_bytes());
        tx.txn_signature = Some(signature.as_bytes().to_vec());

        // Should succeed now
        let result = engine.process(&mut ctx, &tx);
        assert_eq!(result.ter, TER::tesSUCCESS);
        assert_eq!(result.fee_charged, 1000);
    }

    #[test]
    fn test_insufficient_funds() {
        use crypto::PrivateKey;

        let engine = TransactionEngine::new();
        let mut ledger = MockLedgerView::new();

        // Generate a key pair for signing
        let private_key = PrivateKey::generate_secp256k1();
        let public_key = private_key.to_public_key();

        // Create sender with low balance
        let sender = AccountID::new([1u8; 20]);
        let sender_root = AccountRoot::new(sender).with_balance(1000);
        ledger.set_account_root(&sender_root);

        let mut ctx = ApplyContext {
            ledger: &mut ledger,
            rules: ApplyRules::default(),
            ledger_seq: 1,
            parent_ledger_hash: UInt256::zero(),
        };

        // Create large payment
        let mut tx = Transaction::new_payment(
            sender,
            AccountID::new([2u8; 20]),
            Amount::call(100_000),
        );
        tx.sequence = 1;
        tx.fee = 1000;
        tx.signing_pub_key = Some(public_key.as_bytes().to_vec());

        // Sign the transaction
        let message = engine.get_transaction_signing_hash(&tx);
        let signature = private_key.sign(message.as_bytes());
        tx.txn_signature = Some(signature.as_bytes().to_vec());

        let result = engine.process(&mut ctx, &tx);
        assert_eq!(result.ter, TER::terINSUFF_FEE);
    }

    #[test]
    fn test_sequence_mismatch() {
        use crypto::PrivateKey;

        let engine = TransactionEngine::new();
        let mut ledger = MockLedgerView::new();

        // Generate a key pair for signing
        let private_key = PrivateKey::generate_secp256k1();
        let public_key = private_key.to_public_key();

        // Create sender with sequence 5
        let sender = AccountID::new([1u8; 20]);
        let mut sender_root = AccountRoot::new(sender).with_balance(1_000_000);
        for _ in 0..4 {
            sender_root.increment_sequence();
        }
        ledger.set_account_root(&sender_root);

        let mut ctx = ApplyContext {
            ledger: &mut ledger,
            rules: ApplyRules::default(),
            ledger_seq: 1,
            parent_ledger_hash: UInt256::zero(),
        };

        // Wrong sequence
        let mut tx = Transaction::new_payment(
            sender,
            AccountID::new([2u8; 20]),
            Amount::call(100_000),
        );
        tx.sequence = 1; // Should be 5
        tx.fee = 1000;
        tx.signing_pub_key = Some(public_key.as_bytes().to_vec());

        // Sign the transaction
        let message = engine.get_transaction_signing_hash(&tx);
        let signature = private_key.sign(message.as_bytes());
        tx.txn_signature = Some(signature.as_bytes().to_vec());

        let result = engine.process(&mut ctx, &tx);
        assert_eq!(result.ter, TER::terPRE_SEQ);
    }
}
