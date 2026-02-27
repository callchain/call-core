//! Fee Voting System
//!
//! Validators propose fee changes and the network reaches consensus
//! on the new fee structure through weighted averaging.

use primitives::NodeID;
use std::collections::HashMap;

/// A fee vote from a validator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeeVote {
    /// Proposed base fee (drops)
    pub base_fee: u64,
    /// Proposed reserve base (drops)
    pub reserve_base: u64,
    /// Proposed reserve increment (drops)
    pub reserve_increment: u64,
    /// Vote timestamp
    pub timestamp: u32,
}

impl FeeVote {
    pub fn new(base_fee: u64, reserve_base: u64, reserve_increment: u64, timestamp: u32) -> Self {
        Self {
            base_fee,
            reserve_base,
            reserve_increment,
            timestamp,
        }
    }

    /// Default fee vote (no change)
    pub fn default() -> Self {
        Self {
            base_fee: 10,
            reserve_base: 10_000_000,
            reserve_increment: 2_000_000,
            timestamp: 0,
        }
    }
}

/// Fee voting tracker
pub struct FeeVoting {
    /// Current fee votes by validator
    votes: HashMap<NodeID, FeeVote>,
    /// Current base fee (consensus value)
    current_base_fee: u64,
    /// Current reserve base
    current_reserve_base: u64,
    /// Current reserve increment
    current_reserve_increment: u64,
    /// Last update ledger index
    last_update_ledger: u32,
    /// Update interval (ledgers between fee updates)
    update_interval: u32,
}

impl FeeVoting {
    pub fn new() -> Self {
        Self {
            votes: HashMap::new(),
            current_base_fee: 10,
            current_reserve_base: 10_000_000,
            current_reserve_increment: 2_000_000,
            last_update_ledger: 0,
            update_interval: 256, // Update every 256 ledgers (~20 minutes)
        }
    }

    /// Submit a fee vote
    pub fn submit_vote(&mut self, validator: NodeID, vote: FeeVote) {
        self.votes.insert(validator, vote);
    }

    /// Remove a validator's vote
    pub fn remove_vote(&mut self, validator: &NodeID) {
        self.votes.remove(validator);
    }

    /// Calculate the consensus fee values using median
    pub fn calculate_consensus_fees(&self) -> FeeVote {
        if self.votes.is_empty() {
            return FeeVote::default();
        }

        let mut base_fees: Vec<u64> = self.votes.values().map(|v| v.base_fee).collect();
        let mut reserve_bases: Vec<u64> = self.votes.values().map(|v| v.reserve_base).collect();
        let mut reserve_increments: Vec<u64> = self.votes.values().map(|v| v.reserve_increment).collect();

        // Sort to find median
        base_fees.sort();
        reserve_bases.sort();
        reserve_increments.sort();

        let median = |v: &Vec<u64>| -> u64 {
            let mid = v.len() / 2;
            if v.len() % 2 == 0 {
                (v[mid - 1] + v[mid]) / 2
            } else {
                v[mid]
            }
        };

        FeeVote {
            base_fee: median(&base_fees),
            reserve_base: median(&reserve_bases),
            reserve_increment: median(&reserve_increments),
            timestamp: 0,
        }
    }

    /// Check if fees should be updated
    pub fn should_update(&self, current_ledger: u32) -> bool {
        current_ledger >= self.last_update_ledger + self.update_interval
    }

    /// Apply consensus fees and reset votes
    pub fn apply_consensus(&mut self, current_ledger: u32) -> FeeVote {
        let consensus = self.calculate_consensus_fees();

        self.current_base_fee = consensus.base_fee;
        self.current_reserve_base = consensus.reserve_base;
        self.current_reserve_increment = consensus.reserve_increment;
        self.last_update_ledger = current_ledger;

        // Clear old votes
        self.votes.clear();

        consensus
    }

    /// Get current fee values
    pub fn get_current_fees(&self) -> FeeVote {
        FeeVote {
            base_fee: self.current_base_fee,
            reserve_base: self.current_reserve_base,
            reserve_increment: self.current_reserve_increment,
            timestamp: 0,
        }
    }

    /// Get vote count
    pub fn vote_count(&self) -> usize {
        self.votes.len()
    }

    /// Get average proposed base fee
    pub fn get_average_proposed_base(&self) -> Option<u64> {
        if self.votes.is_empty() {
            return None;
        }
        let sum: u64 = self.votes.values().map(|v| v.base_fee).sum();
        Some(sum / self.votes.len() as u64)
    }
}

impl Default for FeeVoting {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fee_vote_submission() {
        let mut voting = FeeVoting::new();
        let validator = NodeID::new([1u8; 32]);

        assert_eq!(voting.vote_count(), 0);

        voting.submit_vote(validator, FeeVote::new(20, 20_000_000, 4_000_000, 1000));
        assert_eq!(voting.vote_count(), 1);
    }

    #[test]
    fn test_consensus_calculation() {
        let mut voting = FeeVoting::new();

        // Add votes from multiple validators
        for i in 0..5 {
            let validator = NodeID::new([i as u8; 32]);
            let vote = FeeVote::new(
                10 + (i as u64 * 5), // 10, 15, 20, 25, 30
                10_000_000,
                2_000_000,
                1000,
            );
            voting.submit_vote(validator, vote);
        }

        let consensus = voting.calculate_consensus_fees();
        // Median of [10, 15, 20, 25, 30] is 20
        assert_eq!(consensus.base_fee, 20);
    }

    #[test]
    fn test_empty_votes() {
        let voting = FeeVoting::new();
        let consensus = voting.calculate_consensus_fees();
        assert_eq!(consensus.base_fee, 10); // Default value
    }

    #[test]
    fn test_apply_consensus() {
        let mut voting = FeeVoting::new();
        let validator = NodeID::new([1u8; 32]);

        voting.submit_vote(validator, FeeVote::new(50, 50_000_000, 10_000_000, 1000));

        let result = voting.apply_consensus(256);
        assert_eq!(result.base_fee, 50);
        assert_eq!(voting.vote_count(), 0); // Votes cleared
        assert_eq!(voting.last_update_ledger, 256);
    }

    #[test]
    fn test_should_update() {
        let mut voting = FeeVoting::new();
        voting.last_update_ledger = 0;
        voting.update_interval = 256;

        assert!(!voting.should_update(100));
        assert!(!voting.should_update(255));
        assert!(voting.should_update(256));
        assert!(voting.should_update(300));
    }
}
