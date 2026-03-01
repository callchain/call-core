//! Transaction signing for Callchain
//!
//! This module provides functionality to sign Callchain transactions locally
//! without requiring an RPC call to the node.

use crate::keys::PrivateKey;
use primitives::AccountID;

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
    // Payment fields
    pub destination: Option<AccountID>,
    pub amount: Option<u64>, // Amount in drops for native payments
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

    /// Check if this is a native amount
    pub fn is_native(&self) -> bool {
        self.currency == [0u8; 20]
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
            destination: Some(destination),
            amount: Some(amount_drops),
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

    /// Serialize transaction to bytes for signing
    /// This is a simplified serialization - proper implementation would use STObject
    pub fn serialize_for_signing(&self) -> Vec<u8> {
        // Simple binary serialization for demonstration
        // In production, use proper serialization::Serializer
        let mut result = Vec::new();

        // Transaction type (2 bytes, big-endian)
        result.extend_from_slice(&self.tx_type.as_u16().to_be_bytes());

        // Account (20 bytes)
        result.extend_from_slice(self.account.as_bytes());

        // Sequence (4 bytes, big-endian)
        result.extend_from_slice(&self.sequence.to_be_bytes());

        // Fee (8 bytes, big-endian)
        result.extend_from_slice(&self.fee.to_be_bytes());

        // Transaction-specific fields
        match self.tx_type {
            TransactionType::Payment => {
                if let Some(dest) = self.destination {
                    result.extend_from_slice(dest.as_bytes());
                }
                if let Some(amt) = self.amount {
                    result.extend_from_slice(&amt.to_be_bytes());
                }
            }
            TransactionType::AccountSet => {
                if let Some(ref domain) = self.domain {
                    result.extend_from_slice(domain);
                }
            }
            TransactionType::TrustSet => {
                // Serialize limit amount
            }
            TransactionType::OfferCreate => {
                // Serialize offer amounts
            }
            TransactionType::OfferCancel => {
                if let Some(seq) = self.offer_sequence {
                    result.extend_from_slice(&seq.to_be_bytes());
                }
            }
            TransactionType::SetRegularKey => {
                if let Some(key) = self.regular_key {
                    result.extend_from_slice(key.as_bytes());
                }
            }
            TransactionType::SignerListSet => {
                if let Some(quorum) = self.signer_quorum {
                    result.extend_from_slice(&quorum.to_be_bytes());
                }
            }
            _ => {}
        }

        result
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
        // Serialize transaction
        let tx_bytes = tx.serialize_for_signing();

        // Get the public key for SigningPubKey field
        let public_key = private_key.to_public_key();

        // Create signing payload (prefix + tx bytes)
        let prefix = crate::hash::HashPrefix::TxSign.as_bytes();
        let mut sign_data = Vec::with_capacity(prefix.len() + tx_bytes.len());
        sign_data.extend_from_slice(prefix);
        sign_data.extend_from_slice(&tx_bytes);

        // Sign the transaction hash
        let signature = private_key.sign(&sign_data);

        // Build the signed transaction blob
        // This includes all fields plus SigningPubKey and TxnSignature
        let signed_blob = Self::build_signed_blob(tx, &public_key.as_bytes(), signature.as_bytes())?;

        Ok(hex::encode(&signed_blob))
    }

    /// Build the final signed transaction blob
    fn build_signed_blob(
        tx: &SignableTransaction,
        public_key: &[u8],
        signature: &[u8],
    ) -> Result<Vec<u8>, SignError> {
        // Simplified blob construction
        // In production, use proper STObject serialization

        let mut blob = Vec::new();

        // Transaction type
        blob.extend_from_slice(&tx.tx_type.as_u16().to_be_bytes());

        // Flags (optional, omitted for simplicity)

        // Sequence
        blob.extend_from_slice(&tx.sequence.to_be_bytes());

        // Fee (as Amount)
        // For simplicity, using 8-byte representation
        blob.extend_from_slice(&tx.fee.to_be_bytes());

        // SigningPubKey (variable length, length-prefixed)
        blob.push(public_key.len() as u8);
        blob.extend_from_slice(public_key);

        // Account
        blob.extend_from_slice(tx.account.as_bytes());

        // Transaction-specific fields
        match tx.tx_type {
            TransactionType::Payment => {
                if let Some(dest) = tx.destination {
                    blob.extend_from_slice(dest.as_bytes());
                }
                if let Some(amt) = tx.amount {
                    // Amount field (8 bytes for native)
                    blob.extend_from_slice(&amt.to_be_bytes());
                }
            }
            _ => {}
        }

        // TxnSignature (variable length, length-prefixed)
        blob.push(signature.len() as u8);
        blob.extend_from_slice(signature);

        Ok(blob)
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
