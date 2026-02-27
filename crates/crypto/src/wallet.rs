//! Wallet generation for Callchain
//!
//! Implements Ripple/XRP-style address and seed generation using Callchain's base58 alphabet.
//!
//! Address format:
//! - Version byte (0x00) + RIPEMD160(SHA256(public_key)) + 4-byte checksum
//! - Encoded with Callchain base58 alphabet -> starts with 'c'
//!
//! Seed format:
//! - Version byte (0x21) + 16 random bytes + 4-byte checksum
//! - Encoded with Callchain base58 alphabet -> starts with 's'

use crate::base58::{encode, decode};
use crate::{PrivateKey, PublicKey, KeyType};
use rand::RngCore;
use rand::rngs::OsRng;
use sha2::{Sha256, Digest};
use ripemd::Ripemd160;

/// Version byte for Callchain addresses (produces 'c' prefix)
pub const ADDRESS_VERSION: u8 = 0x00;

/// Version byte for Callchain seeds (produces 's' prefix)
pub const SEED_VERSION: u8 = 0x21;

/// A generated wallet with seed, address, and keys
#[derive(Debug, Clone)]
pub struct Wallet {
    /// The seed (starts with 's')
    pub seed: String,
    /// The address (starts with 'c')
    pub address: String,
    /// The public key (hex)
    pub public_key: String,
    /// The private key (not exposed as string for security)
    pub private_key: PrivateKey,
}

impl Wallet {
    /// Generate a new random wallet
    pub fn generate() -> Self {
        // Generate seed
        let seed = generate_seed();

        // Derive keys from seed
        let (private_key, public_key) = derive_keys_from_seed(&seed);

        // Generate address from public key
        let address = derive_address(&public_key);

        Self {
            seed,
            address,
            public_key: hex::encode(public_key.as_bytes()),
            private_key,
        }
    }

    /// Create wallet from existing seed
    pub fn from_seed(seed: &str) -> Option<Self> {
        // Validate and decode seed
        let seed_bytes = decode_seed(seed)?;

        // Derive keys
        let (private_key, public_key) = derive_keys_from_seed_bytes(&seed_bytes);

        // Generate address
        let address = derive_address(&public_key);

        Some(Self {
            seed: seed.to_string(),
            address,
            public_key: hex::encode(public_key.as_bytes()),
            private_key,
        })
    }
}

/// Generate a new random seed (starts with 's')
pub fn generate_seed() -> String {
    let mut entropy = [0u8; 16];
    OsRng.fill_bytes(&mut entropy);
    encode_seed(&entropy)
}

/// Encode entropy as a seed with version byte and checksum
fn encode_seed(entropy: &[u8; 16]) -> String {
    // Format: version (1) + entropy (16) + checksum (4) = 21 bytes
    let mut data = Vec::with_capacity(21);
    data.push(SEED_VERSION);
    data.extend_from_slice(entropy);

    // Add checksum (first 4 bytes of SHA256(SHA256(data)))
    let checksum = double_sha256_checksum(&data);
    data.extend_from_slice(&checksum);

    encode(&data)
}

/// Decode a seed string to entropy bytes
pub fn decode_seed(seed: &str) -> Option<[u8; 16]> {
    let data = decode(seed).ok()?;

    // Must be 21 bytes: version (1) + entropy (16) + checksum (4)
    if data.len() != 21 {
        return None;
    }

    // Check version byte
    if data[0] != SEED_VERSION {
        return None;
    }

    // Verify checksum
    let (payload, checksum) = data.split_at(17);
    let expected_checksum = double_sha256_checksum(payload);
    if checksum != expected_checksum {
        return None;
    }

    // Extract entropy
    let mut entropy = [0u8; 16];
    entropy.copy_from_slice(&data[1..17]);
    Some(entropy)
}

/// Derive private and public keys from seed string
fn derive_keys_from_seed(seed: &str) -> (PrivateKey, PublicKey) {
    let seed_bytes = decode_seed(seed).expect("Valid seed");
    derive_keys_from_seed_bytes(&seed_bytes)
}

/// Derive keys from seed entropy
fn derive_keys_from_seed_bytes(seed_bytes: &[u8; 16]) -> (PrivateKey, PublicKey) {
    // Use seed bytes to generate private key
    // Hash with SHA256 to get 32 bytes for private key
    let mut hasher = Sha256::new();
    hasher.update(seed_bytes);
    let key_hash: [u8; 32] = hasher.finalize().into();

    let private_key = PrivateKey::from_bytes(KeyType::Secp256k1, &key_hash)
        .expect("Valid private key from 32 bytes");
    let public_key = private_key.to_public_key();

    (private_key, public_key)
}

/// Derive address from public key (starts with 'c')
pub fn derive_address(public_key: &PublicKey) -> String {
    // Step 1: SHA256 of public key
    let mut hasher = Sha256::new();
    hasher.update(public_key.as_bytes());
    let sha256_hash: [u8; 32] = hasher.finalize().into();

    // Step 2: RIPEMD160 of SHA256 hash
    let mut ripemd = Ripemd160::new();
    ripemd.update(sha256_hash);
    let account_id: [u8; 20] = ripemd.finalize().into();

    // Step 3: Add version byte
    let mut data = Vec::with_capacity(25);
    data.push(ADDRESS_VERSION);
    data.extend_from_slice(&account_id);

    // Step 4: Add checksum (first 4 bytes of double SHA256)
    let checksum = double_sha256_checksum(&data);
    data.extend_from_slice(&checksum);

    // Step 5: Base58 encode
    encode(&data)
}

/// Calculate double SHA256 checksum (first 4 bytes)
fn double_sha256_checksum(data: &[u8]) -> [u8; 4] {
    let hash1 = sha256(data);
    let hash2 = sha256(&hash1);
    [hash2[0], hash2[1], hash2[2], hash2[3]]
}

/// Simple SHA256 hash
fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Validate a Callchain address
pub fn validate_address(address: &str) -> bool {
    // Address should start with 'c'
    if !address.starts_with('c') {
        return false;
    }

    let data = match decode(address) {
        Ok(d) => d,
        Err(_) => return false,
    };

    // Must be 25 bytes: version (1) + account_id (20) + checksum (4)
    if data.len() != 25 {
        return false;
    }

    // Check version byte
    if data[0] != ADDRESS_VERSION {
        return false;
    }

    // Verify checksum
    let (payload, checksum) = data.split_at(21);
    let expected_checksum = double_sha256_checksum(payload);
    checksum == expected_checksum
}

/// Validate a Callchain seed
pub fn validate_seed_format(seed: &str) -> bool {
    // Seed should start with 's'
    if !seed.starts_with('s') {
        return false;
    }

    decode_seed(seed).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_seed() {
        let seed = generate_seed();
        assert!(seed.starts_with('s'), "Seed should start with 's'");
        assert!(validate_seed_format(&seed), "Generated seed should be valid");
    }

    #[test]
    fn test_wallet_generation() {
        let wallet = Wallet::generate();

        assert!(wallet.seed.starts_with('s'), "Seed should start with 's'");
        assert!(wallet.address.starts_with('c'), "Address should start with 'c'");
        assert!(!wallet.public_key.is_empty(), "Public key should not be empty");

        // Validate the wallet
        assert!(validate_seed_format(&wallet.seed));
        assert!(validate_address(&wallet.address));
    }

    #[test]
    fn test_seed_roundtrip() {
        let wallet = Wallet::generate();
        let recovered = Wallet::from_seed(&wallet.seed);

        assert!(recovered.is_some(), "Should recover wallet from seed");
        let recovered = recovered.unwrap();

        assert_eq!(wallet.address, recovered.address, "Address should match");
        assert_eq!(wallet.public_key, recovered.public_key, "Public key should match");
    }

    #[test]
    fn test_invalid_address() {
        // Address with wrong prefix
        assert!(!validate_address("rN7n7otQDd6FczFgLdlqtyMVrn3HMfHgFj")); // Ripple address
        assert!(!validate_address("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa")); // Bitcoin address
    }

    #[test]
    fn test_invalid_seed() {
        // Seed not starting with 's'
        assert!(!validate_seed_format("cpshnaf39w"));
        // Empty seed
        assert!(!validate_seed_format(""));
    }

    #[test]
    fn test_address_from_pubkey() {
        // Test with a known public key
        let private_key = PrivateKey::generate_secp256k1();
        let public_key = private_key.to_public_key();

        let address = derive_address(&public_key);
        assert!(address.starts_with('c'), "Address should start with 'c'");
        assert!(validate_address(&address), "Generated address should be valid");
    }

    #[test]
    fn test_address_format() {
        // Test that addresses have the correct format
        let wallet = Wallet::generate();

        // Address should start with 'c'
        assert!(wallet.address.starts_with('c'), "Address must start with 'c'");

        // Address length should be between 25-35 characters (typical base58 address)
        assert!(
            wallet.address.len() >= 25 && wallet.address.len() <= 35,
            "Address length should be 25-35 chars, got {}",
            wallet.address.len()
        );

        // All characters should be from Callchain alphabet
        let alphabet: std::collections::HashSet<char> = crate::CALLCHAIN_ALPHABET.iter().map(|&c| c as char).collect();
        for c in wallet.address.chars() {
            assert!(
                alphabet.contains(&c),
                "Address contains invalid character: {}",
                c
            );
        }
    }

    #[test]
    fn test_seed_format() {
        // Test that seeds have the correct format
        let wallet = Wallet::generate();

        // Seed should start with 's'
        assert!(wallet.seed.starts_with('s'), "Seed must start with 's'");

        // Seed length should be between 25-35 characters (typical base58 seed)
        assert!(
            wallet.seed.len() >= 25 && wallet.seed.len() <= 35,
            "Seed length should be 25-35 chars, got {}",
            wallet.seed.len()
        );
    }

    #[test]
    fn test_deterministic_key_derivation() {
        // Test that the same seed always produces the same keys
        let seed = generate_seed();

        let wallet1 = Wallet::from_seed(&seed).unwrap();
        let wallet2 = Wallet::from_seed(&seed).unwrap();

        assert_eq!(wallet1.address, wallet2.address, "Same seed should produce same address");
        assert_eq!(wallet1.public_key, wallet2.public_key, "Same seed should produce same public key");
    }

    #[test]
    fn test_address_version_byte() {
        // Decode address and check version byte
        let wallet = Wallet::generate();
        let decoded = decode(&wallet.address).unwrap();

        // First byte should be ADDRESS_VERSION (0x00)
        assert_eq!(decoded[0], ADDRESS_VERSION, "Address version byte should be 0x00");

        // Total length should be 25 bytes (1 version + 20 account_id + 4 checksum)
        assert_eq!(decoded.len(), 25, "Decoded address should be 25 bytes");
    }

    #[test]
    fn test_seed_version_byte() {
        // Decode seed and check version byte
        let wallet = Wallet::generate();
        let decoded = decode(&wallet.seed).unwrap();

        // First byte should be SEED_VERSION (0x21)
        assert_eq!(decoded[0], SEED_VERSION, "Seed version byte should be 0x21");

        // Total length should be 21 bytes (1 version + 16 entropy + 4 checksum)
        assert_eq!(decoded.len(), 21, "Decoded seed should be 21 bytes");
    }

    #[test]
    fn test_address_checksum() {
        // Test that address checksum is valid
        let wallet = Wallet::generate();
        let data = decode(&wallet.address).unwrap();

        let (payload, checksum) = data.split_at(21);
        let expected_checksum = double_sha256_checksum(payload);

        assert_eq!(checksum, expected_checksum, "Address checksum should be valid");
    }

    #[test]
    fn test_seed_checksum() {
        // Test that seed checksum is valid
        let wallet = Wallet::generate();
        let data = decode(&wallet.seed).unwrap();

        let (payload, checksum) = data.split_at(17);
        let expected_checksum = double_sha256_checksum(payload);

        assert_eq!(checksum, expected_checksum, "Seed checksum should be valid");
    }

    #[test]
    fn test_corrupted_address_fails() {
        // Generate a valid address and corrupt it
        let wallet = Wallet::generate();
        let mut corrupted = wallet.address.clone();

        // Change the last character
        let last_char = corrupted.pop().unwrap();
        let new_char = if last_char == 'c' { 'p' } else { 'c' };
        corrupted.push(new_char);

        assert!(!validate_address(&corrupted), "Corrupted address should fail validation");
    }

    #[test]
    fn test_corrupted_seed_fails() {
        // Generate a valid seed and corrupt it
        let wallet = Wallet::generate();
        let mut corrupted = wallet.seed.clone();

        // Change the last character
        let last_char = corrupted.pop().unwrap();
        let new_char = if last_char == 'k' { 's' } else { 'k' };
        corrupted.push(new_char);

        assert!(!validate_seed_format(&corrupted), "Corrupted seed should fail validation");
    }

    #[test]
    fn test_multiple_wallet_generation() {
        // Generate multiple wallets and ensure they're all unique
        let mut addresses = std::collections::HashSet::new();
        let mut seeds = std::collections::HashSet::new();

        for _ in 0..10 {
            let wallet = Wallet::generate();
            addresses.insert(wallet.address.clone());
            seeds.insert(wallet.seed.clone());
        }

        // All 10 should be unique
        assert_eq!(addresses.len(), 10, "All addresses should be unique");
        assert_eq!(seeds.len(), 10, "All seeds should be unique");
    }

    #[test]
    fn test_known_vectors() {
        // Test with known entropy to verify deterministic output
        let entropy = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
                       0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10u8];

        let seed = encode_seed(&entropy);

        // Seed should start with 's' and be valid
        assert!(seed.starts_with('s'), "Seed should start with 's'");
        assert!(validate_seed_format(&seed), "Seed should be valid");

        // Decode and verify entropy matches
        let decoded_entropy = decode_seed(&seed).unwrap();
        assert_eq!(decoded_entropy.to_vec(), entropy.to_vec(), "Decoded entropy should match");
    }
}

