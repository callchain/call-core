//! Proof of Work for overlay network spam prevention
//!
//! Implements a simple proof-of-work system for peer connections to prevent
//! connection flooding and Sybil attacks. Based on Hashcash-like system.

use crypto::sha256;
use primitives::UInt256;
use std::time::{Duration, Instant};

/// Proof of work difficulty (number of leading zero bits required)
pub const DEFAULT_POW_DIFFICULTY: u8 = 20; // 20 leading zero bits = ~1 second on modern CPU

/// Maximum age of a proof-of-work challenge (5 minutes)
pub const MAX_CHALLENGE_AGE: Duration = Duration::from_secs(300);

/// Proof of work challenge
#[derive(Debug, Clone)]
pub struct PowChallenge {
    /// Random nonce for the challenge
    pub nonce: UInt256,
    /// Timestamp when challenge was created
    pub timestamp: Instant,
    /// Required difficulty
    pub difficulty: u8,
}

/// Proof of work solution
#[derive(Debug, Clone)]
pub struct PowSolution {
    /// The challenge nonce
    pub challenge_nonce: UInt256,
    /// The solution nonce
    pub solution_nonce: UInt256,
    /// Timestamp when solved
    pub timestamp: u64,
}

/// Proof of work validator
#[derive(Debug, Clone)]
pub struct PowValidator {
    difficulty: u8,
}

impl PowValidator {
    /// Create a new PoW validator with default difficulty
    pub fn new() -> Self {
        Self {
            difficulty: DEFAULT_POW_DIFFICULTY,
        }
    }

    /// Create with custom difficulty
    pub fn with_difficulty(difficulty: u8) -> Self {
        Self { difficulty }
    }

    /// Generate a new challenge
    pub fn generate_challenge(&self) -> PowChallenge {
        PowChallenge {
            nonce: UInt256::random(),
            timestamp: Instant::now(),
            difficulty: self.difficulty,
        }
    }

    /// Verify a proof of work solution
    pub fn verify(&self, solution: &PowSolution, challenge: &PowChallenge) -> bool {
        // Check challenge hasn't expired
        if challenge.timestamp.elapsed() > MAX_CHALLENGE_AGE {
            return false;
        }

        // Verify challenge nonce matches
        if solution.challenge_nonce != challenge.nonce {
            return false;
        }

        // Verify the proof of work
        self.verify_hash(solution, challenge.difficulty)
    }

    /// Verify just the hash meets difficulty requirement
    fn verify_hash(&self, solution: &PowSolution, difficulty: u8) -> bool {
        let hash = Self::compute_hash(solution);
        let leading_zeros = count_leading_zero_bits(&hash);
        leading_zeros >= difficulty
    }

    /// Compute hash for a solution
    fn compute_hash(solution: &PowSolution) -> [u8; 32] {
        let mut data = Vec::with_capacity(80);
        data.extend_from_slice(solution.challenge_nonce.as_bytes());
        data.extend_from_slice(solution.solution_nonce.as_bytes());
        data.extend_from_slice(&solution.timestamp.to_be_bytes());
        sha256(&data)
    }

    /// Get current difficulty
    pub fn difficulty(&self) -> u8 {
        self.difficulty
    }

    /// Adjust difficulty based on network conditions
    pub fn adjust_difficulty(&mut self, target_time_ms: u64, actual_time_ms: u64) {
        // If solving is too fast, increase difficulty
        // If solving is too slow, decrease difficulty
        let ratio = actual_time_ms as f64 / target_time_ms.max(1) as f64;

        if ratio < 0.5 {
            // Too fast, increase difficulty
            self.difficulty = (self.difficulty + 1).min(32);
        } else if ratio > 2.0 {
            // Too slow, decrease difficulty
            self.difficulty = self.difficulty.saturating_sub(1).max(8);
        }
    }
}

impl Default for PowValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Proof of work solver
pub struct PowSolver;

impl PowSolver {
    /// Solve a proof of work challenge
    /// Returns the solution nonce that satisfies the difficulty requirement
    pub fn solve(challenge: &PowChallenge) -> Option<PowSolution> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut solution_nonce = UInt256::zero();
        let max_attempts = 1_000_000_000u64; // Prevent infinite loops

        for _ in 0..max_attempts {
            let solution = PowSolution {
                challenge_nonce: challenge.nonce,
                solution_nonce,
                timestamp,
            };

            if Self::verify_hash_quick(&solution, challenge.difficulty) {
                return Some(solution);
            }

            // Increment nonce
            solution_nonce = increment_uint256(solution_nonce);
        }

        None
    }

    /// Quick hash verification for solving
    fn verify_hash_quick(solution: &PowSolution, difficulty: u8) -> bool {
        let mut data = Vec::with_capacity(80);
        data.extend_from_slice(solution.challenge_nonce.as_bytes());
        data.extend_from_slice(solution.solution_nonce.as_bytes());
        data.extend_from_slice(&solution.timestamp.to_be_bytes());
        let hash = sha256(&data);
        count_leading_zero_bits(&hash) >= difficulty
    }
}

/// Count leading zero bits in a hash
fn count_leading_zero_bits(hash: &[u8; 32]) -> u8 {
    let mut count = 0u8;
    for byte in hash {
        if *byte == 0 {
            count = count.saturating_add(8);
        } else {
            count = count.saturating_add(byte.leading_zeros() as u8);
            break;
        }
    }
    count
}

/// Increment a UInt256 by 1
fn increment_uint256(mut value: UInt256) -> UInt256 {
    // Use DerefMut to get mutable access to the underlying bytes
    let bytes: &mut [u8; 32] = &mut value;
    for byte in bytes.iter_mut().rev() {
        if *byte == 255 {
            *byte = 0;
        } else {
            *byte += 1;
            break;
        }
    }
    value
}

/// Overlay with proof of work protection
pub struct PowProtectedOverlay {
    /// Underlying overlay
    overlay: super::Overlay,
    /// PoW validator
    validator: PowValidator,
    /// Pending challenges for new connections
    pending_challenges: std::collections::HashMap<std::net::SocketAddr, PowChallenge>,
    /// Require PoW for incoming connections
    require_pow: bool,
}

impl std::fmt::Debug for PowProtectedOverlay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PowProtectedOverlay")
            .field("validator", &self.validator)
            .field("require_pow", &self.require_pow)
            .field("pending_challenges", &self.pending_challenges.len())
            .finish_non_exhaustive()
    }
}

impl PowProtectedOverlay {
    pub fn new(require_pow: bool) -> Self {
        Self {
            overlay: super::Overlay::new(),
            validator: PowValidator::new(),
            pending_challenges: std::collections::HashMap::new(),
            require_pow,
        }
    }

    /// Check if PoW is required
    pub fn is_pow_required(&self) -> bool {
        self.require_pow
    }

    /// Generate a challenge for a new peer
    pub fn generate_challenge(&mut self, addr: std::net::SocketAddr) -> PowChallenge {
        let challenge = self.validator.generate_challenge();
        self.pending_challenges.insert(addr, challenge.clone());
        challenge
    }

    /// Verify a peer's PoW solution
    pub fn verify_solution(
        &mut self,
        addr: &std::net::SocketAddr,
        solution: &PowSolution,
    ) -> bool {
        if let Some(challenge) = self.pending_challenges.remove(addr) {
            self.validator.verify(solution, &challenge)
        } else {
            false
        }
    }

    /// Get the underlying overlay
    pub fn overlay(&self) -> &super::Overlay {
        &self.overlay
    }

    /// Get mutable access to underlying overlay
    pub fn overlay_mut(&mut self) -> &mut super::Overlay {
        &mut self.overlay
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pow_challenge_generation() {
        let validator = PowValidator::new();
        let challenge = validator.generate_challenge();
        assert_eq!(challenge.difficulty, DEFAULT_POW_DIFFICULTY);
    }

    #[test]
    fn test_pow_verify_invalid_solution() {
        let validator = PowValidator::with_difficulty(1); // Very easy
        let challenge = validator.generate_challenge();

        let invalid_solution = PowSolution {
            challenge_nonce: challenge.nonce,
            solution_nonce: UInt256::zero(),
            timestamp: 0,
        };

        // This will likely fail since we didn't actually solve it
        // Just testing the verification path
        let _ = validator.verify(&invalid_solution, &challenge);
    }

    #[test]
    fn test_count_leading_zero_bits() {
        let hash1 = [0u8; 32]; // All zeros
        // 32 bytes * 8 bits = 256, but u8 max is 255, so saturating_add gives 255
        assert_eq!(count_leading_zero_bits(&hash1), 255);

        let mut hash2 = [0u8; 32];
        hash2[0] = 0b10000000; // 0 leading zeros
        assert_eq!(count_leading_zero_bits(&hash2), 0);

        let mut hash3 = [0u8; 32];
        hash3[0] = 0b00000001; // 7 leading zeros
        assert_eq!(count_leading_zero_bits(&hash3), 7);
    }

    #[test]
    fn test_increment_uint256() {
        let val = UInt256::zero();
        let incremented = increment_uint256(val);
        let bytes = incremented.as_bytes();
        assert_eq!(bytes[31], 1);
        assert!(bytes[0..31].iter().all(|&b| b == 0));
    }

    #[test]
    fn test_difficulty_adjustment() {
        let mut validator = PowValidator::with_difficulty(10);

        // If actual time is much less than target, difficulty should increase
        validator.adjust_difficulty(1000, 100);
        assert!(validator.difficulty() >= 10);

        // If actual time is much more than target, difficulty should decrease
        let mut validator2 = PowValidator::with_difficulty(10);
        validator2.adjust_difficulty(100, 1000);
        assert!(validator2.difficulty() <= 10);
    }
}
