//! Transaction signing for Callchain
//!
//! This module provides functionality to sign Callchain transactions locally
//! without requiring an RPC call to the node.

use crate::keys::PrivateKey;
use primitives::AccountID;
use serialization::{Amount, Serializer, STObject, STValue};
use serialization::types::sf;

/// Transaction type identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum TransactionType {
    Payment = 0,
    AccountSet = 3,
    SetRegularKey = 5,
    NicknameSet = 6,
    OfferCreate = 7,
    OfferCancel = 8,
    SignerListSet = 12,
    IssueSet = 16,
    TrustSet = 20,
}

impl TransactionType {
    pub fn as_u16(&self) -> u16 {
        *self as u16
    }
}

/// A signable transaction structure
#[derive(Debug, Clone)]
pub struct SignableTransaction {
    pub tx_type: TransactionType,
    pub account: AccountID,
    pub sequence: u32,
    pub fee: u64,
    pub network_id: Option<u32>,
    pub flags: Option<u32>,
    // Payment fields
    pub destination: Option<AccountID>,
    pub amount: Option<AssetAmount>,
    pub destination_tag: Option<u32>,
    // TrustSet fields
    pub limit_amount: Option<AssetAmount>,
    // Offer fields
    pub taker_pays: Option<AssetAmount>,
    pub taker_gets: Option<AssetAmount>,
    pub offer_sequence: Option<u32>,
    // AccountSet fields
    pub domain: Option<Vec<u8>>,
    pub set_flag: Option<u32>,
    pub clear_flag: Option<u32>,
    // SetRegularKey fields
    pub regular_key: Option<AccountID>,
    // SignerListSet fields
    pub signer_quorum: Option<u32>,
    pub signers: Vec<SignerEntry>,
}

/// Asset amount (native or issued)
#[derive(Debug, Clone)]
pub struct AssetAmount {
    pub value: i64,
    pub currency: [u8; 20],
    pub issuer: AccountID,
}

impl AssetAmount {
    /// Create a native CALL amount (in drops)
    pub fn native(drops: u64) -> Self {
        Self {
            value: drops as i64,
            currency: [0u8; 20],
            issuer: AccountID::new([0u8; 20]),
        }
    }

    /// Create an issued currency amount
    pub fn issued(value: i64, currency_code: &str, issuer: AccountID) -> Self {
        let mut currency = [0u8; 20];
        if currency_code.len() == 3 {
            // Standard currency code
            currency[12] = currency_code.as_bytes()[0];
            currency[13] = currency_code.as_bytes()[1];
            currency[14] = currency_code.as_bytes()[2];
        } else if currency_code.len() == 40 {
            // Hex currency code
            if let Ok(hex_bytes) = hex::decode(currency_code) {
                currency.copy_from_slice(&hex_bytes);
            }
        }
        Self {
            value,
            currency,
            issuer,
        }
    }

    /// Check if this is a native amount
    pub fn is_native(&self) -> bool {
        self.currency == [0u8; 20]
    }

    /// Convert to serialization Amount
    pub fn to_amount(&self) -> Amount {
        if self.is_native() {
            Amount::call(self.value as u64)
        } else {
            let currency = primitives::Currency::new(self.currency);
            // Create issued amount directly without validation
            // to ensure currency and issuer are preserved
            Amount {
                mantissa: self.value,
                exponent: Amount::CALL_EXPONENT,
                currency,
                issuer: self.issuer,
                is_native: false,
                is_negative: self.value < 0,
            }
        }
    }
}

/// Signer entry for SignerListSet
#[derive(Debug, Clone)]
pub struct SignerEntry {
    pub account: AccountID,
    pub weight: u16,
}

impl SignableTransaction {
    /// Create a new payment transaction
    pub fn new_payment(
        account: AccountID,
        destination: AccountID,
        amount_drops: u64,
        sequence: u32,
    ) -> Self {
        Self {
            tx_type: TransactionType::Payment,
            account,
            sequence,
            fee: 10,
            network_id: None,
            flags: None,
            destination: Some(destination),
            amount: Some(AssetAmount::native(amount_drops)),
            destination_tag: None,
            limit_amount: None,
            taker_pays: None,
            taker_gets: None,
            offer_sequence: None,
            domain: None,
            set_flag: None,
            clear_flag: None,
            regular_key: None,
            signer_quorum: None,
            signers: Vec::new(),
        }
    }

    /// Create a new AccountSet transaction
    pub fn new_account_set(account: AccountID, sequence: u32) -> Self {
        Self {
            tx_type: TransactionType::AccountSet,
            account,
            sequence,
            fee: 10,
            network_id: None,
            flags: None,
            destination: None,
            amount: None,
            destination_tag: None,
            limit_amount: None,
            taker_pays: None,
            taker_gets: None,
            offer_sequence: None,
            domain: None,
            set_flag: None,
            clear_flag: None,
            regular_key: None,
            signer_quorum: None,
            signers: Vec::new(),
        }
    }

    /// Set domain (AccountSet)
    pub fn set_domain(mut self, domain: &[u8]) -> Self {
        self.domain = Some(domain.to_vec());
        self
    }

    /// Create a new TrustSet transaction
    pub fn new_trust_set(
        account: AccountID,
        issuer: AccountID,
        currency: [u8; 20],
        limit: i64,
        sequence: u32,
    ) -> Self {
        Self {
            tx_type: TransactionType::TrustSet,
            account,
            sequence,
            fee: 10,
            network_id: None,
            flags: None,
            destination: None,
            amount: None,
            destination_tag: None,
            limit_amount: Some(AssetAmount {
                value: limit,
                currency,
                issuer,
            }),
            taker_pays: None,
            taker_gets: None,
            offer_sequence: None,
            domain: None,
            set_flag: None,
            clear_flag: None,
            regular_key: None,
            signer_quorum: None,
            signers: Vec::new(),
        }
    }

    /// Create a new OfferCreate transaction
    pub fn new_offer_create(
        account: AccountID,
        taker_pays: AssetAmount,
        taker_gets: AssetAmount,
        sequence: u32,
    ) -> Self {
        Self {
            tx_type: TransactionType::OfferCreate,
            account,
            sequence,
            fee: 10,
            network_id: None,
            flags: None,
            destination: None,
            amount: None,
            destination_tag: None,
            limit_amount: None,
            taker_pays: Some(taker_pays),
            taker_gets: Some(taker_gets),
            offer_sequence: None,
            domain: None,
            set_flag: None,
            clear_flag: None,
            regular_key: None,
            signer_quorum: None,
            signers: Vec::new(),
        }
    }

    /// Create a new OfferCancel transaction
    pub fn new_offer_cancel(account: AccountID, offer_sequence: u32, sequence: u32) -> Self {
        Self {
            tx_type: TransactionType::OfferCancel,
            account,
            sequence,
            fee: 10,
            network_id: None,
            flags: None,
            destination: None,
            amount: None,
            destination_tag: None,
            limit_amount: None,
            taker_pays: None,
            taker_gets: None,
            offer_sequence: Some(offer_sequence),
            domain: None,
            set_flag: None,
            clear_flag: None,
            regular_key: None,
            signer_quorum: None,
            signers: Vec::new(),
        }
    }

    /// Create a new SetRegularKey transaction
    pub fn new_set_regular_key(
        account: AccountID,
        regular_key: AccountID,
        sequence: u32,
    ) -> Self {
        Self {
            tx_type: TransactionType::SetRegularKey,
            account,
            sequence,
            fee: 10,
            network_id: None,
            flags: None,
            destination: None,
            amount: None,
            destination_tag: None,
            limit_amount: None,
            taker_pays: None,
            taker_gets: None,
            offer_sequence: None,
            domain: None,
            set_flag: None,
            clear_flag: None,
            regular_key: Some(regular_key),
            signer_quorum: None,
            signers: Vec::new(),
        }
    }

    /// Create a new SignerListSet transaction
    pub fn new_signer_list_set(
        account: AccountID,
        quorum: u32,
        signers: Vec<SignerEntry>,
        sequence: u32,
    ) -> Self {
        Self {
            tx_type: TransactionType::SignerListSet,
            account,
            sequence,
            fee: 10,
            network_id: None,
            flags: None,
            destination: None,
            amount: None,
            destination_tag: None,
            limit_amount: None,
            taker_pays: None,
            taker_gets: None,
            offer_sequence: None,
            domain: None,
            set_flag: None,
            clear_flag: None,
            regular_key: None,
            signer_quorum: Some(quorum),
            signers,
        }
    }

    /// Build the STObject representation of this transaction
    /// This creates the transaction data without signature fields
    pub fn to_stobject(&self, include_signature_fields: bool, public_key: Option<&[u8]>, signature: Option<&[u8]>) -> STObject {
        let mut obj = STObject::new();

        // TransactionType (always first for canonical ordering)
        obj.insert(sf::TRANSACTION_TYPE, STValue::UInt16(self.tx_type.as_u16()));

        // Account
        obj.insert(sf::ACCOUNT, STValue::Account(self.account));

        // Sequence
        obj.insert(sf::SEQUENCE, STValue::UInt32(self.sequence));

        // Fee (as Amount)
        obj.insert(sf::FEE, STValue::Amount(Amount::call(self.fee)));

        // NetworkID (optional)
        if let Some(network_id) = self.network_id {
            // Note: sf::NETWORK_ID might not exist, skipping for now
        }

        // Flags (optional)
        if let Some(flags) = self.flags {
            obj.insert(sf::FLAGS, STValue::UInt32(flags));
        }

        // Transaction-specific fields
        match self.tx_type {
            TransactionType::Payment => {
                if let Some(dest) = self.destination {
                    obj.insert(sf::DESTINATION, STValue::Account(dest));
                }
                if let Some(ref amt) = self.amount {
                    obj.insert(sf::AMOUNT, STValue::Amount(amt.to_amount()));
                }
                if let Some(tag) = self.destination_tag {
                    obj.insert(sf::DESTINATION_TAG, STValue::UInt32(tag));
                }
            }
            TransactionType::AccountSet => {
                if let Some(ref domain) = self.domain {
                    obj.insert(sf::DOMAIN, STValue::VL(domain.clone()));
                }
                if let Some(flag) = self.set_flag {
                    obj.insert(sf::SET_FLAG, STValue::UInt32(flag));
                }
                if let Some(flag) = self.clear_flag {
                    obj.insert(sf::CLEAR_FLAG, STValue::UInt32(flag));
                }
            }
            TransactionType::TrustSet => {
                if let Some(ref limit) = self.limit_amount {
                    obj.insert(sf::LIMIT_AMOUNT, STValue::Amount(limit.to_amount()));
                }
            }
            TransactionType::OfferCreate => {
                if let Some(ref pays) = self.taker_pays {
                    obj.insert(sf::TAKER_PAYS, STValue::Amount(pays.to_amount()));
                }
                if let Some(ref gets) = self.taker_gets {
                    obj.insert(sf::TAKER_GETS, STValue::Amount(gets.to_amount()));
                }
            }
            TransactionType::OfferCancel => {
                if let Some(seq) = self.offer_sequence {
                    obj.insert(sf::OFFER_SEQUENCE, STValue::UInt32(seq));
                }
            }
            TransactionType::SetRegularKey => {
                if let Some(key) = self.regular_key {
                    obj.insert(sf::REGULAR_KEY, STValue::Account(key));
                }
            }
            TransactionType::SignerListSet => {
                if let Some(quorum) = self.signer_quorum {
                    obj.insert(sf::SIGNER_QUORUM, STValue::UInt32(quorum));
                }
                if !self.signers.is_empty() {
                    let signer_values: Vec<STValue> = self.signers.iter().map(|s| {
                        let mut signer_obj = STObject::new();
                        signer_obj.insert(sf::ACCOUNT, STValue::Account(s.account));
                        signer_obj.insert(sf::SIGNER_WEIGHT, STValue::UInt16(s.weight));
                        STValue::Object(signer_obj)
                    }).collect();
                    obj.insert(sf::SIGNER_ENTRIES, STValue::Array(signer_values));
                }
            }
            _ => {}
        }

        // Add signature fields if requested
        if include_signature_fields {
            if let Some(pubkey) = public_key {
                obj.insert(sf::SIGNING_PUB_KEY, STValue::VL(pubkey.to_vec()));
            }
            if let Some(sig) = signature {
                obj.insert(sf::TXN_SIGNATURE, STValue::VL(sig.to_vec()));
            }
        }

        obj
    }
}

/// Transaction signer
pub struct TransactionSigner;

impl TransactionSigner {
    /// Sign a transaction with the given private key
    /// Returns the transaction blob (hex-encoded) ready for submission
    pub fn sign_transaction(
        tx: &SignableTransaction,
        private_key: &PrivateKey,
    ) -> Result<String, SignError> {
        // Get the public key for SigningPubKey field
        let public_key = private_key.to_public_key();
        let pubkey_bytes = public_key.as_bytes();

        // Build the STObject for signing (without signature fields)
        let obj_for_signing = tx.to_stobject(false, None, None);

        // Serialize the object
        let mut serializer = Serializer::new();
        serializer.add_object(&obj_for_signing)
            .map_err(|_| SignError::SerializationFailed)?;
        let serialized_tx = serializer.finish();

        // Create signing payload (prefix + serialized tx)
        let prefix = crate::hash::HashPrefix::TxSign.as_bytes();
        let mut sign_data = Vec::with_capacity(prefix.len() + serialized_tx.len());
        sign_data.extend_from_slice(prefix);
        sign_data.extend_from_slice(&serialized_tx);

        // Sign the transaction hash
        let signature = private_key.sign(&sign_data);
        let sig_bytes = signature.as_bytes();

        // Build the final STObject with signature fields
        let signed_obj = tx.to_stobject(true, Some(&pubkey_bytes), Some(sig_bytes));

        // Serialize the final object
        let mut final_serializer = Serializer::new();
        final_serializer.add_object(&signed_obj)
            .map_err(|_| SignError::SerializationFailed)?;
        let signed_blob = final_serializer.finish();

        Ok(hex::encode(&signed_blob))
    }

    /// Create and sign a payment transaction (convenience method)
    pub fn sign_payment(
        sender: AccountID,
        destination: AccountID,
        amount_drops: u64,
        sequence: u32,
        private_key: &PrivateKey,
    ) -> Result<String, SignError> {
        let tx = SignableTransaction::new_payment(sender, destination, amount_drops, sequence);
        Self::sign_transaction(&tx, private_key)
    }

    /// Create and sign an AccountSet transaction (convenience method)
    pub fn sign_account_set(
        account: AccountID,
        sequence: u32,
        private_key: &PrivateKey,
    ) -> Result<String, SignError> {
        let tx = SignableTransaction::new_account_set(account, sequence);
        Self::sign_transaction(&tx, private_key)
    }
}

/// Signing errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignError {
    SerializationFailed,
    InvalidKey,
    SigningFailed,
}

impl std::fmt::Display for SignError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SignError::SerializationFailed => write!(f, "Transaction serialization failed"),
            SignError::InvalidKey => write!(f, "Invalid signing key"),
            SignError::SigningFailed => write!(f, "Signing operation failed"),
        }
    }
}

impl std::error::Error for SignError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::PrivateKey;

    #[test]
    fn test_sign_payment() {
        let sender = AccountID::new([1u8; 20]);
        let dest = AccountID::new([2u8; 20]);
        let private_key = PrivateKey::generate_secp256k1();

        let result = TransactionSigner::sign_payment(
            sender,
            dest,
            1000000, // 1 CALL
            1,
            &private_key,
        );

        assert!(result.is_ok());
        let tx_blob = result.unwrap();
        assert!(!tx_blob.is_empty());
        // Should be valid hex
        assert!(hex::decode(&tx_blob).is_ok());
    }

    #[test]
    fn test_sign_account_set() {
        let account = AccountID::new([1u8; 20]);
        let private_key = PrivateKey::generate_secp256k1();

        let result = TransactionSigner::sign_account_set(account, 1, &private_key);

        assert!(result.is_ok());
        let tx_blob = result.unwrap();
        assert!(!tx_blob.is_empty());
    }
}
