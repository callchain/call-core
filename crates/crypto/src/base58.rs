//! Base58 encoding/decoding with Callchain alphabet
//!
//! Callchain uses a custom base58 alphabet:
//! 'cpshnaf39wBUDNEGHJKLM4PQRST7VWXYZ2brdeCg65jkm8oFqi1tuvAxyz'
//!
//! This is similar to Bitcoin's base58 but with a different character ordering.

use std::fmt;

/// Callchain base58 alphabet
pub const CALLCHAIN_ALPHABET: &[u8; 58] = b"cpshnaf39wBUDNEGHJKLM4PQRST7VWXYZ2brdeCg65jkm8oFqi1tuvAxyz";

/// Bitcoin base58 alphabet (for reference)
pub const BITCOIN_ALPHABET: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

/// Base58 encoding error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Base58Error {
    InvalidCharacter(char),
    InvalidLength,
    ChecksumMismatch,
}

impl fmt::Display for Base58Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Base58Error::InvalidCharacter(c) => write!(f, "Invalid base58 character: {}", c),
            Base58Error::InvalidLength => write!(f, "Invalid base58 string length"),
            Base58Error::ChecksumMismatch => write!(f, "Base58 checksum mismatch"),
        }
    }
}

impl std::error::Error for Base58Error {}

/// Encode bytes to base58 string using Callchain alphabet
pub fn encode(data: &[u8]) -> String {
    encode_with_alphabet(data, CALLCHAIN_ALPHABET)
}

/// Encode bytes to base58 string using specified alphabet
pub fn encode_with_alphabet(data: &[u8], alphabet: &[u8; 58]) -> String {
    if data.is_empty() {
        return String::new();
    }

    // Count leading zeros
    let leading_zeros = data.iter().take_while(|&&b| b == 0).count();

    // Convert to base58
    let mut result = Vec::new();
    let mut num: Vec<u8> = data.to_vec();

    while !num.is_empty() && num.iter().any(|&b| b != 0) {
        let mut remainder = 0u16;
        let mut new_num = Vec::new();

        for &byte in &num {
            let current = remainder * 256 + byte as u16;
            let quotient = (current / 58) as u8;
            remainder = current % 58;

            if !new_num.is_empty() || quotient != 0 {
                new_num.push(quotient);
            }
        }

        result.push(alphabet[remainder as usize]);
        num = new_num;
    }

    // Add leading zeros
    for _ in 0..leading_zeros {
        result.push(alphabet[0]);
    }

    // Reverse to get correct order
    result.reverse();

    String::from_utf8(result).expect("Alphabet is valid UTF-8")
}

/// Decode base58 string to bytes using Callchain alphabet
pub fn decode(s: &str) -> Result<Vec<u8>, Base58Error> {
    decode_with_alphabet(s, CALLCHAIN_ALPHABET)
}

/// Decode base58 string to bytes using specified alphabet
pub fn decode_with_alphabet(s: &str, alphabet: &[u8; 58]) -> Result<Vec<u8>, Base58Error> {
    if s.is_empty() {
        return Ok(Vec::new());
    }

    // Build reverse lookup table
    let mut reverse_table = [255u8; 256];
    for (i, &c) in alphabet.iter().enumerate() {
        reverse_table[c as usize] = i as u8;
    }

    // Count leading zeros (represented by first char of alphabet)
    let leading_zeros = s.bytes().take_while(|&b| b == alphabet[0]).count();

    // Convert from base58
    let mut result = vec![0u8];

    for c in s.bytes() {
        let value = reverse_table[c as usize];
        if value == 255 {
            return Err(Base58Error::InvalidCharacter(c as char));
        }

        // Multiply by 58 and add value
        let mut carry = value as u16;
        for byte in result.iter_mut() {
            let current = (*byte as u16) * 58 + carry;
            *byte = (current % 256) as u8;
            carry = current / 256;
        }

        while carry > 0 {
            result.push((carry % 256) as u8);
            carry /= 256;
        }
    }

    // Add leading zeros
    for _ in 0..leading_zeros {
        result.push(0);
    }

    // Reverse to get correct order
    result.reverse();

    Ok(result)
}

/// Encode with checksum (4 bytes of SHA-256)
pub fn encode_check(data: &[u8]) -> String {
    let mut with_checksum = data.to_vec();
    let hash = sha256(&sha256(data));
    with_checksum.extend_from_slice(&hash[..4]);
    encode(&with_checksum)
}

/// Decode with checksum verification
pub fn decode_check(s: &str) -> Result<Vec<u8>, Base58Error> {
    let decoded = decode(s)?;

    if decoded.len() < 4 {
        return Err(Base58Error::InvalidLength);
    }

    let (data, checksum) = decoded.split_at(decoded.len() - 4);
    let hash = sha256(&sha256(data));

    if &hash[..4] != checksum {
        return Err(Base58Error::ChecksumMismatch);
    }

    Ok(data.to_vec())
}

/// Generate a random seed (16 bytes) and encode as base58
pub fn generate_seed() -> String {
    use rand::RngCore;
    use rand::rngs::OsRng;

    let mut seed = [0u8; 16];
    OsRng.fill_bytes(&mut seed);
    encode(&seed)
}

/// Generate a seed from a specific entropy source (for testing)
pub fn generate_seed_from_entropy(entropy: &[u8]) -> String {
    encode(entropy)
}

/// Validate a base58-encoded seed
pub fn validate_seed(seed: &str) -> bool {
    decode(seed).is_ok()
}

/// Double SHA-256 hash
fn sha256(data: &[u8]) -> [u8; 32] {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode() {
        let data = b"hello world";
        let encoded = encode(data);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(data.to_vec(), decoded);
    }

    #[test]
    fn test_encode_decode_empty() {
        let data = b"";
        let encoded = encode(data);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(data.to_vec(), decoded);
    }

    #[test]
    fn test_encode_decode_zeros() {
        let data = vec![0, 0, 0, 1, 2, 3];
        let encoded = encode(&data);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_callchain_alphabet() {
        // Verify the Callchain alphabet is correctly defined
        let expected = b"cpshnaf39wBUDNEGHJKLM4PQRST7VWXYZ2brdeCg65jkm8oFqi1tuvAxyz";
        assert_eq!(CALLCHAIN_ALPHABET, expected);
        assert_eq!(CALLCHAIN_ALPHABET.len(), 58);
    }

    #[test]
    fn test_decode_invalid_char() {
        // '0' is not in the Callchain alphabet
        let result = decode("c0ps");
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_check() {
        let data = b"test data";
        let encoded = encode_check(data);
        let decoded = decode_check(&encoded).unwrap();
        assert_eq!(data.to_vec(), decoded);
    }

    #[test]
    fn test_generate_seed() {
        let seed = generate_seed();
        assert!(!seed.is_empty());
        assert!(validate_seed(&seed));
    }

    #[test]
    fn test_known_vectors() {
        // Test with known input/output
        let data = vec![0x00, 0x01, 0x02, 0x03];
        let encoded = encode(&data);
        assert_eq!(encoded.chars().next().unwrap(), 'c'); // Leading zero -> first char of alphabet

        let decoded = decode(&encoded).unwrap();
        assert_eq!(data, decoded);
    }
}
