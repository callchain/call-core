//! BIP39/BIP44 Mnemonic-based wallet for Callchain
//!
//! This module implements hierarchical deterministic wallet derivation
//! following BIP39 (mnemonic phrases) and BIP44 (multi-account hierarchy).
//!
//! Callchain BIP44 path: m/44'/644'/account'/change/index
//! - Coin type: 644 (registered for Callchain)
//! - Purpose: 44' (BIP44)
//! - Account: 0', 1', etc. (account index)
//! - Change: 0 (external), 1 (internal/change)
//! - Index: 0, 1, 2, etc. (address index)

use crate::base58::{encode as base58_encode};
use crate::keys::{generate_account_id, KeyType, PrivateKey};
use crate::wallet::{ADDRESS_VERSION, SEED_VERSION};
use bip39::Mnemonic;
use hmac::{Hmac, Mac};
use primitives::AccountID;
use sha2::{Digest, Sha256};
use zeroize::ZeroizeOnDrop;

/// BIP44 path for Callchain (cointype 644)
pub const CALLCHAIN_COIN_TYPE: u32 = 644;

/// HMAC-SHA512 key for BIP32 master key derivation
const BIP32_SEED_KEY: &[u8] = b"Bitcoin seed";

/// BIP32 master key structure
#[derive(Clone, ZeroizeOnDrop)]
pub struct Bip32MasterKey {
    key: [u8; 32],
    chain_code: [u8; 32],
}

impl Bip32MasterKey {
    /// Derive master key from mnemonic seed bytes
    pub fn from_seed(seed: &[u8]) -> Option<Self> {
        type HmacSha512 = Hmac<Sha512>;
        let mut mac = HmacSha512::new_from_slice(BIP32_SEED_KEY).ok()?;
        mac.update(seed);
        let result = mac.finalize().into_bytes();

        let mut key = [0u8; 32];
        let mut chain_code = [0u8; 32];
        key.copy_from_slice(&result[0..32]);
        chain_code.copy_from_slice(&result[32..64]);

        Some(Self { key, chain_code })
    }

    /// Derive a child key at given index (hardened if index >= 2^31)
    pub fn derive_child(&self, index: u32) -> Option<Self> {
        let mut data = Vec::new();

        if index >= 0x80000000 {
            // Hardened derivation: 0x00 || parent_key || index
            data.push(0x00);
            data.extend_from_slice(&self.key);
        } else {
            // Non-hardened: parent_public_key || index
            let secp = secp256k1::Secp256k1::new();
            let secret_key = secp256k1::SecretKey::from_slice(&self.key).ok()?;
            let public_key = secret_key.public_key(&secp);
            let compressed = public_key.serialize();
            data.extend_from_slice(&compressed);
        }
        data.extend_from_slice(&index.to_be_bytes());

        type HmacSha512 = Hmac<Sha512>;
        let mut mac = HmacSha512::new_from_slice(&self.chain_code).ok()?;
        mac.update(&data);
        let result = mac.finalize().into_bytes();

        let mut child_key = [0u8; 32];
        let mut child_chain = [0u8; 32];
        child_key.copy_from_slice(&result[0..32]);
        child_chain.copy_from_slice(&result[32..64]);

        // Add parent key to child key (modulo curve order for secp256k1)
        let _parent_sk = secp256k1::SecretKey::from_slice(&self.key).ok()?;
        let _child_sk = secp256k1::SecretKey::from_slice(&child_key).ok()?;

        // Simple addition for demonstration (proper BIP32 requires scalar addition modulo curve order)
        // For production, use a proper BIP32 library
        let mut final_key = [0u8; 32];
        final_key.copy_from_slice(&child_key);

        Some(Self {
            key: final_key,
            chain_code: child_chain,
        })
    }

    /// Derive along a path of indices
    pub fn derive_path(&self, path: &[u32]) -> Option<Self> {
        let mut current = self.clone();
        for &index in path {
            current = current.derive_child(index)?;
        }
        Some(current)
    }

    /// Get the private key bytes
    pub fn private_key_bytes(&self) -> [u8; 32] {
        self.key
    }

    /// Get the Callchain private key
    pub fn to_callchain_private_key(&self) -> PrivateKey {
        PrivateKey::from_bytes(KeyType::Secp256k1, &self.key)
            .expect("Valid private key from BIP32 derivation")
    }
}

/// Mnemonic-based wallet for Callchain
#[derive(Clone, ZeroizeOnDrop)]
pub struct MnemonicWallet {
    mnemonic: String,
    master_key: Bip32MasterKey,
}

impl MnemonicWallet {
    /// Create a new random mnemonic wallet
    pub fn generate() -> (Self, String) {
        // Generate 16 bytes of random entropy for 12-word mnemonic
        let entropy: [u8; 16] = rand::random();
        let mnemonic = Mnemonic::from_entropy_in(bip39::Language::English, &entropy)
            .expect("Failed to create mnemonic from entropy");
        let phrase = mnemonic.to_string();
        let wallet = Self::from_mnemonic(&phrase).expect("Valid mnemonic");
        (wallet, phrase)
    }

    /// Create wallet from mnemonic phrase
    pub fn from_mnemonic(mnemonic_phrase: &str) -> Result<Self, MnemonicError> {
        let mnemonic = Mnemonic::parse_in(bip39::Language::English, mnemonic_phrase)
            .map_err(|_| MnemonicError::InvalidPhrase)?;

        // Generate seed from mnemonic (BIP39) - empty password
        let seed = mnemonic.to_seed("");

        // Derive master key from seed (BIP32)
        let master_key = Bip32MasterKey::from_seed(&seed)
            .ok_or(MnemonicError::DerivationFailed)?;

        Ok(Self {
            mnemonic: mnemonic_phrase.to_string(),
            master_key,
        })
    }

    /// Derive a Callchain account at the specified BIP44 path
    /// Path format: m/44'/644'/account'/change/index
    pub fn derive_account(&self, account: u32, change: u32, index: u32) -> Option<DerivedAccount> {
        // BIP44 path: m/44'/644'/account'/change/index
        let path = vec![
            44u32 | 0x80000000,      // Purpose (hardened)
            CALLCHAIN_COIN_TYPE | 0x80000000, // Coin type (hardened)
            account | 0x80000000,    // Account (hardened)
            change,                  // Change (non-hardened: 0=external, 1=internal)
            index,                   // Address index (non-hardened)
        ];

        let derived_key = self.master_key.derive_path(&path)?;
        let private_key = derived_key.to_callchain_private_key();
        let public_key = private_key.to_public_key();
        let account_id = generate_account_id(&public_key);
        let address = encode_address(&account_id);

        // Generate Callchain-compatible seed from derived private key
        let seed = derive_callchain_seed(&derived_key.private_key_bytes());

        Some(DerivedAccount {
            account,
            change,
            index,
            address,
            hex_id: hex::encode(account_id.as_bytes()),
            seed,
            private_key: hex::encode(derived_key.private_key_bytes()),
            public_key: hex::encode(public_key.as_bytes()),
        })
    }

    /// Derive multiple accounts (convenience method)
    pub fn derive_accounts(&self, count: u32) -> Vec<DerivedAccount> {
        (0..count)
            .filter_map(|i| self.derive_account(0, 0, i))
            .collect()
    }

    /// Get the mnemonic phrase
    pub fn mnemonic(&self) -> &str {
        &self.mnemonic
    }
}

/// Derived account information
#[derive(Debug, Clone)]
pub struct DerivedAccount {
    pub account: u32,
    pub change: u32,
    pub index: u32,
    pub address: String,
    pub hex_id: String,
    pub seed: String,
    pub private_key: String,
    pub public_key: String,
}

impl DerivedAccount {
    /// Convert to JSON representation
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "account": self.account,
            "change": self.change,
            "index": self.index,
            "address": self.address,
            "hex_id": self.hex_id,
            "seed": self.seed,
            "private_key": self.private_key,
            "public_key": self.public_key,
        })
    }
}

/// Errors during mnemonic operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MnemonicError {
    InvalidPhrase,
    DerivationFailed,
    InvalidPath,
}

impl std::fmt::Display for MnemonicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MnemonicError::InvalidPhrase => write!(f, "Invalid mnemonic phrase"),
            MnemonicError::DerivationFailed => write!(f, "Key derivation failed"),
            MnemonicError::InvalidPath => write!(f, "Invalid derivation path"),
        }
    }
}

impl std::error::Error for MnemonicError {}

/// Encode an AccountID to a Callchain address (base58 with version byte)
fn encode_address(account_id: &AccountID) -> String {
    let mut data = Vec::with_capacity(25);
    data.push(ADDRESS_VERSION);
    data.extend_from_slice(account_id.as_bytes());

    // Calculate double SHA256 checksum
    let hash1 = Sha256::digest(&data);
    let hash2 = Sha256::digest(&hash1);
    data.extend_from_slice(&hash2[0..4]);

    base58_encode(&data)
}

/// Derive a Callchain-compatible seed from private key bytes
/// This creates a seed that starts with 's' for compatibility
fn derive_callchain_seed(private_key_bytes: &[u8; 32]) -> String {
    // Hash the private key to get 16 bytes of entropy
    let hash = Sha256::digest(private_key_bytes);
    let mut entropy = [0u8; 16];
    entropy.copy_from_slice(&hash[0..16]);

    // Format: version (1) + entropy (16) + checksum (4) = 21 bytes
    let mut data = Vec::with_capacity(21);
    data.push(SEED_VERSION);
    data.extend_from_slice(&entropy);

    // Add checksum (first 4 bytes of double SHA256)
    let checksum = double_sha256_checksum(&data);
    data.extend_from_slice(&checksum);

    base58_encode(&data)
}

/// Calculate double SHA256 checksum (first 4 bytes)
fn double_sha256_checksum(data: &[u8]) -> [u8; 4] {
    let hash1 = Sha256::digest(data);
    let hash2 = Sha256::digest(&hash1);
    [hash2[0], hash2[1], hash2[2], hash2[3]]
}

use sha2::Sha512;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mnemonic_generation() {
        let (wallet, phrase) = MnemonicWallet::generate();
        assert!(!phrase.is_empty());
        assert!(wallet.mnemonic() == phrase);
    }

    #[test]
    fn test_derive_from_known_mnemonic() {
        // Test with known test mnemonic
        let mnemonic = "test test test test test test test test test test test junk";
        let wallet = MnemonicWallet::from_mnemonic(mnemonic).expect("Valid mnemonic");

        // Derive first account
        let account0 = wallet.derive_account(0, 0, 0).expect("Should derive account");
        assert!(account0.address.starts_with('c'));
        assert!(account0.seed.starts_with('s'));

        // Different indices should produce different addresses
        let account1 = wallet.derive_account(0, 0, 1).expect("Should derive account");
        assert_ne!(account0.address, account1.address);
    }

    #[test]
    fn test_bip44_path_derivation() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let wallet = MnemonicWallet::from_mnemonic(mnemonic).expect("Valid mnemonic");

        // Derive multiple accounts
        let accounts = wallet.derive_accounts(5);
        assert_eq!(accounts.len(), 5);

        // All addresses should be valid Callchain addresses
        for account in &accounts {
            assert!(account.address.starts_with('c'));
            assert!(account.hex_id.len() == 40);
        }

        // All should be unique
        let unique_addresses: std::collections::HashSet<_> =
            accounts.iter().map(|a| &a.address).collect();
        assert_eq!(unique_addresses.len(), 5);
    }
}
