//! Signature Cache for Transaction Verification
//!
//! This module implements a cache for transaction signature verification results,
//! similar to the HashRouter in the old calld project.
//!
//! The cache stores signature verification results keyed by transaction hash,
//! allowing the system to skip redundant signature verification during consensus.
//!
//! ## Usage
//!
//! When a transaction is first submitted via RPC:
//! 1. Signature is verified
//! 2. Result is cached (SignatureState::Good or SignatureState::Bad)
//!
//! When the same transaction is processed during consensus:
//! 1. Check cache for signature state
//! 2. If Good, skip verification (use ApplyFlags::no_check_sign())
//! 3. If Bad, reject immediately
//! 4. If Unknown, verify and cache result

use primitives::UInt256;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Signature verification state for a transaction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureState {
    /// Signature not yet checked
    Unknown,
    /// Signature verified as valid
    Good,
    /// Signature verified as invalid
    Bad,
}

/// Entry in the signature cache
#[derive(Debug, Clone)]
struct CacheEntry {
    state: SignatureState,
    timestamp: Instant,
}

impl CacheEntry {
    fn new(state: SignatureState) -> Self {
        Self {
            state,
            timestamp: Instant::now(),
        }
    }

    fn is_expired(&self, ttl: Duration) -> bool {
        self.timestamp.elapsed() > ttl
    }
}

/// Signature cache for transaction verification results
///
/// This cache stores the signature verification state for transactions,
/// keyed by transaction hash. It helps avoid redundant signature
/// verification during consensus processing.
#[derive(Debug, Clone)]
pub struct SignatureCache {
    /// Map from transaction hash to signature state
    entries: Arc<Mutex<HashMap<UInt256, CacheEntry>>>,
    /// Time-to-live for cache entries
    ttl: Duration,
    /// Maximum number of entries to store
    max_entries: usize,
}

impl SignatureCache {
    /// Create a new signature cache with default settings
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
            ttl: Duration::from_secs(300), // 5 minutes default TTL
            max_entries: 10000,
        }
    }

    /// Create a new signature cache with custom settings
    pub fn with_settings(ttl_secs: u64, max_entries: usize) -> Self {
        Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
            ttl: Duration::from_secs(ttl_secs),
            max_entries,
        }
    }

    /// Get the signature state for a transaction
    /// Returns Unknown if not in cache or if entry is expired
    pub fn get_state(&self, tx_hash: &UInt256) -> SignatureState {
        let mut entries = self.entries.lock().unwrap();

        if let Some(entry) = entries.get(tx_hash) {
            if entry.is_expired(self.ttl) {
                entries.remove(tx_hash);
                SignatureState::Unknown
            } else {
                entry.state
            }
        } else {
            SignatureState::Unknown
        }
    }

    /// Set the signature state for a transaction
    pub fn set_state(&self, tx_hash: UInt256, state: SignatureState) {
        let mut entries = self.entries.lock().unwrap();

        // Evict oldest entries if at capacity (simple eviction)
        if entries.len() >= self.max_entries && !entries.contains_key(&tx_hash) {
            // Remove expired entries first
            let expired: Vec<UInt256> = entries
                .iter()
                .filter(|(_, e)| e.is_expired(self.ttl))
                .map(|(k, _)| *k)
                .collect();

            for key in expired {
                entries.remove(&key);
            }

            // If still at capacity, remove oldest entry
            if entries.len() >= self.max_entries {
                if let Some(oldest) = entries
                    .iter()
                    .min_by_key(|(_, e)| e.timestamp)
                    .map(|(k, _)| *k)
                {
                    entries.remove(&oldest);
                }
            }
        }

        entries.insert(tx_hash, CacheEntry::new(state));
    }

    /// Mark a transaction signature as good (verified)
    pub fn set_good(&self, tx_hash: UInt256) {
        self.set_state(tx_hash, SignatureState::Good);
    }

    /// Mark a transaction signature as bad (failed verification)
    pub fn set_bad(&self, tx_hash: UInt256) {
        self.set_state(tx_hash, SignatureState::Bad);
    }

    /// Check if signature is good (convenience method)
    pub fn is_good(&self, tx_hash: &UInt256) -> bool {
        self.get_state(tx_hash) == SignatureState::Good
    }

    /// Check if signature is bad (convenience method)
    pub fn is_bad(&self, tx_hash: &UInt256) -> bool {
        self.get_state(tx_hash) == SignatureState::Bad
    }

    /// Clear all entries from the cache
    pub fn clear(&self) {
        let mut entries = self.entries.lock().unwrap();
        entries.clear();
    }

    /// Get the number of entries in the cache
    pub fn len(&self) -> usize {
        let entries = self.entries.lock().unwrap();
        entries.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Remove expired entries (can be called periodically)
    pub fn cleanup(&self) {
        let mut entries = self.entries.lock().unwrap();
        let expired: Vec<UInt256> = entries
            .iter()
            .filter(|(_, e)| e.is_expired(self.ttl))
            .map(|(k, _)| *k)
            .collect();

        for key in expired {
            entries.remove(&key);
        }
    }
}

impl Default for SignatureCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe reference to a signature cache
pub type SharedSignatureCache = Arc<SignatureCache>;

/// Create a new shared signature cache
pub fn create_signature_cache() -> SharedSignatureCache {
    Arc::new(SignatureCache::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_cache_basic() {
        let cache = SignatureCache::new();
        let tx_hash = UInt256::from_hex("abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234").unwrap();

        // Initially unknown
        assert_eq!(cache.get_state(&tx_hash), SignatureState::Unknown);
        assert!(!cache.is_good(&tx_hash));
        assert!(!cache.is_bad(&tx_hash));

        // Set as good
        cache.set_good(tx_hash);
        assert_eq!(cache.get_state(&tx_hash), SignatureState::Good);
        assert!(cache.is_good(&tx_hash));
        assert!(!cache.is_bad(&tx_hash));

        // Set as bad
        cache.set_bad(tx_hash);
        assert_eq!(cache.get_state(&tx_hash), SignatureState::Bad);
        assert!(!cache.is_good(&tx_hash));
        assert!(cache.is_bad(&tx_hash));
    }

    #[test]
    fn test_signature_cache_expiration() {
        // Create cache with very short TTL
        let cache = SignatureCache::with_settings(0, 100);
        let tx_hash = UInt256::from_hex("abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234").unwrap();

        cache.set_good(tx_hash);
        assert!(cache.is_good(&tx_hash));

        // Wait a bit and cleanup
        std::thread::sleep(Duration::from_millis(10));
        cache.cleanup();

        // Should be expired
        assert_eq!(cache.get_state(&tx_hash), SignatureState::Unknown);
    }
}
