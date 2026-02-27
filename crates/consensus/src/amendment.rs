//! Amendment System
//!
//! Tracks feature amendments that require validator consensus to activate.
//! Similar to XRP Ledger's amendment system.

use primitives::{NodeID, UInt256};
use std::collections::{HashMap, HashSet};

/// An amendment represents a protocol change
#[derive(Debug, Clone)]
pub struct Amendment {
    /// Unique identifier (hash of amendment name)
    pub id: UInt256,
    /// Human-readable name
    pub name: String,
    /// Description of the change
    pub description: String,
    /// Minimum required support percentage (0-100)
    pub min_support_percent: u8,
    /// Minimum time before activation (seconds)
    pub min_wait_time: u64,
    /// Amendment status
    pub status: AmendmentStatus,
    /// Ledger index when locked in (for calculating activation time)
    pub locked_in_ledger: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmendmentStatus {
    /// Proposed but not yet supported
    Proposed,
    /// Supported by validators, waiting for consensus
    Supported,
    /// Locked in, will activate
    LockedIn,
    /// Active on the network
    Active,
    /// Rejected or failed
    Rejected,
}

impl Amendment {
    pub fn new(name: &str, description: &str, min_support: u8, min_wait: u64) -> Self {
        // Generate ID from name hash
        use crypto::sha512_half;
        let id = sha512_half(name.as_bytes());

        Self {
            id,
            name: name.to_string(),
            description: description.to_string(),
            min_support_percent: min_support,
            min_wait_time: min_wait,
            status: AmendmentStatus::Proposed,
            locked_in_ledger: None,
        }
    }

    /// Create a standard amendment with 80% threshold and 2 week wait
    pub fn standard(name: &str, description: &str) -> Self {
        Self::new(name, description, 80, 14 * 24 * 60 * 60) // 2 weeks
    }
}

/// Amendment voting tracker
pub struct AmendmentTable {
    /// Known amendments by ID
    amendments: HashMap<UInt256, Amendment>,
    /// Votes by validator (validator -> set of amendment IDs they support)
    votes: HashMap<NodeID, HashSet<UInt256>>,
    /// Last vote ledger by validator
    last_vote_ledger: HashMap<NodeID, u32>,
    /// Current ledger index
    current_ledger: u32,
    /// Vote staleness threshold (ledgers)
    vote_staleness: u32,
}

impl AmendmentTable {
    pub fn new() -> Self {
        Self {
            amendments: HashMap::new(),
            votes: HashMap::new(),
            last_vote_ledger: HashMap::new(),
            current_ledger: 0,
            vote_staleness: 256, // Votes expire after 256 ledgers
        }
    }

    /// Register a new amendment
    pub fn register_amendment(&mut self, amendment: Amendment) {
        self.amendments.insert(amendment.id, amendment);
    }

    /// Submit a vote for amendments
    pub fn submit_vote(&mut self, validator: NodeID, supported: HashSet<UInt256>, ledger: u32) {
        self.votes.insert(validator, supported);
        self.last_vote_ledger.insert(validator, ledger);
        self.current_ledger = ledger;
    }

    /// Remove stale votes from validators who haven't voted recently
    pub fn remove_stale_votes(&mut self) {
        let stale: Vec<NodeID> = self
            .last_vote_ledger
            .iter()
            .filter(|(_, last_ledger)| **last_ledger + self.vote_staleness < self.current_ledger)
            .map(|(node, _)| *node)
            .collect();

        for node in stale {
            self.votes.remove(&node);
            self.last_vote_ledger.remove(&node);
        }
    }

    /// Calculate support percentage for an amendment
    pub fn get_support_percent(&self, amendment_id: &UInt256) -> u8 {
        let total_votes = self.votes.len();
        if total_votes == 0 {
            return 0;
        }

        let supporting = self
            .votes
            .values()
            .filter(|supported| supported.contains(amendment_id))
            .count();

        ((supporting as f64 / total_votes as f64) * 100.0) as u8
    }

    /// Check if an amendment has enough support to be locked in
    pub fn can_lock_in(&self, amendment_id: &UInt256) -> bool {
        if let Some(amendment) = self.amendments.get(amendment_id) {
            let support = self.get_support_percent(amendment_id);
            support >= amendment.min_support_percent
        } else {
            false
        }
    }

    /// Update amendment statuses based on votes
    pub fn process_amendments(&mut self, ledger: u32) {
        self.current_ledger = ledger;
        self.remove_stale_votes();

        let amendment_ids: Vec<UInt256> = self.amendments.keys().copied().collect();

        for id in amendment_ids {
            // Calculate support first (immutable borrow)
            let can_lock_in = self.can_lock_in(&id);
            let support_percent = self.get_support_percent(&id);

            if let Some(amendment) = self.amendments.get_mut(&id) {
                match amendment.status {
                    AmendmentStatus::Proposed | AmendmentStatus::Supported => {
                        if can_lock_in {
                            amendment.status = AmendmentStatus::LockedIn;
                            amendment.locked_in_ledger = Some(ledger);
                        } else if support_percent > 0 {
                            amendment.status = AmendmentStatus::Supported;
                        }
                    }
                    AmendmentStatus::LockedIn => {
                        // Activate after at least one ledger has passed since lock-in
                        if let Some(locked_ledger) = amendment.locked_in_ledger {
                            if ledger > locked_ledger {
                                amendment.status = AmendmentStatus::Active;
                            }
                        }
                    }
                    _ => {} // No change for Active or Rejected
                }
            }
        }
    }

    /// Check if a feature (amendment) is active
    pub fn is_feature_active(&self, amendment_id: &UInt256) -> bool {
        self.amendments
            .get(amendment_id)
            .map(|a| a.status == AmendmentStatus::Active)
            .unwrap_or(false)
    }

    /// Get all active amendments
    pub fn get_active_amendments(&self) -> Vec<&Amendment> {
        self.amendments
            .values()
            .filter(|a| a.status == AmendmentStatus::Active)
            .collect()
    }

    /// Get amendment by name
    pub fn get_by_name(&self, name: &str) -> Option<&Amendment> {
        self.amendments.values().find(|a| a.name == name)
    }

    /// Get vote summary for an amendment
    pub fn get_vote_summary(&self, amendment_id: &UInt256) -> VoteSummary {
        let total = self.votes.len();
        let supporting = self
            .votes
            .values()
            .filter(|supported| supported.contains(amendment_id))
            .count();

        VoteSummary {
            total_votes: total,
            supporting,
            opposing: total - supporting,
            support_percent: if total > 0 {
                ((supporting as f64 / total as f64) * 100.0) as u8
            } else {
                0
            },
        }
    }
}

impl Default for AmendmentTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary of voting for an amendment
#[derive(Debug, Clone)]
pub struct VoteSummary {
    pub total_votes: usize,
    pub supporting: usize,
    pub opposing: usize,
    pub support_percent: u8,
}

/// Predefined amendments
pub mod amendments {
    use super::*;

    /// Create the standard set of amendments
    pub fn standard_set() -> Vec<Amendment> {
        vec![
            Amendment::standard(
                "FeeEscalation",
                "Improved transaction fee escalation algorithm",
            ),
            Amendment::standard(
                "MultiSign",
                "Support for multi-signature transactions",
            ),
            Amendment::standard(
                "FlowV2",
                "Improved payment flow engine",
            ),
            Amendment::standard(
                "CryptoConditions",
                "Support for cryptoconditions in transactions",
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amendment_creation() {
        let amendment = Amendment::standard("TestAmendment", "Test description");
        assert_eq!(amendment.name, "TestAmendment");
        assert_eq!(amendment.status, AmendmentStatus::Proposed);
        assert_eq!(amendment.min_support_percent, 80);
    }

    #[test]
    fn test_vote_submission() {
        let mut table = AmendmentTable::new();
        let validator = NodeID::new([1u8; 32]);
        let amendment = Amendment::standard("Test", "Test");
        let id = amendment.id;

        table.register_amendment(amendment);

        let mut support = HashSet::new();
        support.insert(id);
        table.submit_vote(validator, support, 1);

        assert_eq!(table.votes.len(), 1);
    }

    #[test]
    fn test_support_calculation() {
        let mut table = AmendmentTable::new();
        let amendment = Amendment::standard("Test", "Test");
        let id = amendment.id;

        table.register_amendment(amendment);

        // 3 validators, 2 support the amendment
        for i in 0..3 {
            let validator = NodeID::new([i as u8; 32]);
            let mut support = HashSet::new();
            if i < 2 {
                support.insert(id);
            }
            table.submit_vote(validator, support, 1);
        }

        let support_pct = table.get_support_percent(&id);
        assert_eq!(support_pct, 66); // 2/3 = 66%
        assert!(!table.can_lock_in(&id)); // Need 80%

        // Add more support
        let validator4 = NodeID::new([4u8; 32]);
        let mut support = HashSet::new();
        support.insert(id);
        table.submit_vote(validator4, support, 1);

        let validator5 = NodeID::new([5u8; 32]);
        let mut support = HashSet::new();
        support.insert(id);
        table.submit_vote(validator5, support, 1);

        // Now 4/5 = 80%
        assert_eq!(table.get_support_percent(&id), 80);
        assert!(table.can_lock_in(&id));
    }

    #[test]
    fn test_stale_vote_removal() {
        let mut table = AmendmentTable::new();
        table.vote_staleness = 10;

        let validator = NodeID::new([1u8; 32]);
        let support = HashSet::new();
        table.submit_vote(validator, support, 1);

        assert_eq!(table.votes.len(), 1);

        // Advance past staleness threshold
        table.remove_stale_votes();
        table.current_ledger = 20;
        table.remove_stale_votes();

        assert_eq!(table.votes.len(), 0);
    }

    #[test]
    fn test_amendment_activation() {
        let mut table = AmendmentTable::new();
        let amendment = Amendment::standard("Test", "Test");
        let id = amendment.id;

        table.register_amendment(amendment);
        assert!(!table.is_feature_active(&id));

        // Add enough support
        for i in 0..5 {
            let validator = NodeID::new([i as u8; 32]);
            let mut support = HashSet::new();
            support.insert(id);
            table.submit_vote(validator, support, 1);
        }

        table.process_amendments(2);
        // Amendment is now locked in at ledger 2

        // Call again with higher ledger to activate
        table.process_amendments(3);
        assert!(table.is_feature_active(&id));
    }
}
