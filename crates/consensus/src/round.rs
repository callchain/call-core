//! Consensus round management
//!
//! Manages a single consensus round from open phase through processing.
//! Implements the RPCA (Ripple Protocol Consensus Algorithm) phases.

use crate::params::ConsensusParms;
use crate::types::{ConsensusPhase, PeerPosition, Proposal, Validation};
use crypto::PrivateKey;
use primitives::{LedgerIndex, NodeID, UInt256};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// A consensus round manages one ledger close cycle
#[derive(Debug)]
pub struct ConsensusRound {
    /// Round identifier
    pub round_id: u64,
    /// Ledger we're building on
    pub previous_ledger: UInt256,
    /// Ledger index we're creating
    pub ledger_index: LedgerIndex,
    /// Current phase
    pub phase: ConsensusPhase,
    /// When the round started
    pub start_time: Instant,
    /// Ledger close time (network time)
    pub close_time: u32,
    /// Our proposed transaction set hash
    pub our_position: Option<UInt256>,
    /// Peer positions for this round
    pub peer_positions: HashMap<NodeID, PeerPosition>,
    /// Received validations
    pub validations: HashMap<UInt256, Vec<Validation>>,
    /// Disputed transactions
    pub disputed_txs: Vec<UInt256>,
    /// Whether we've accepted a position
    pub accepted: bool,
    /// Proposal sequence number
    pub propose_seq: u32,
    /// Last time we broadcast our position
    pub last_broadcast: Instant,
}

impl ConsensusRound {
    /// Create a new consensus round
    pub fn new(
        round_id: u64,
        previous_ledger: UInt256,
        ledger_index: LedgerIndex,
        close_time: u32,
    ) -> Self {
        Self {
            round_id,
            previous_ledger,
            ledger_index,
            phase: ConsensusPhase::Open,
            start_time: Instant::now(),
            close_time,
            our_position: None,
            peer_positions: HashMap::new(),
            validations: HashMap::new(),
            disputed_txs: Vec::new(),
            accepted: false,
            propose_seq: 0,
            last_broadcast: Instant::now(),
        }
    }

    /// Close the ledger and start consensus
    pub fn close_ledger(&mut self, tx_set_hash: UInt256) {
        if self.phase != ConsensusPhase::Open {
            warn!("Cannot close ledger in phase {:?}", self.phase);
            return;
        }

        self.our_position = Some(tx_set_hash);
        self.phase = ConsensusPhase::Establish;
        self.propose_seq = 1;

        info!(
            round = self.round_id,
            ledger_index = self.ledger_index,
            position = ?tx_set_hash,
            "Ledger closed, entering consensus"
        );
    }

    /// Process a proposal from a peer
    pub fn process_proposal(&mut self, proposal: Proposal, now: u64) {
        // Only accept proposals for the correct previous ledger
        if proposal.previous_ledger != self.previous_ledger {
            debug!("Ignoring proposal for wrong previous ledger");
            return;
        }

        // Only accept proposals in Establish phase
        if self.phase != ConsensusPhase::Establish {
            return;
        }

        let node_id = proposal.node_id;

        if let Some(existing) = self.peer_positions.get_mut(&node_id) {
            // Update existing position
            if proposal.propose_seq > existing.proposal.propose_seq {
                existing.proposal = proposal;
                existing.last_update = now;
            }
        } else {
            // New peer position
            let position = PeerPosition {
                node_id,
                last_update: now,
                proposal,
            };
            self.peer_positions.insert(node_id, position);
        }
    }

    /// Process a validation
    pub fn process_validation(&mut self, validation: Validation) {
        // Only accept validations for our ledger index
        if validation.ledger_index != self.ledger_index {
            return;
        }

        self.validations
            .entry(validation.ledger_hash)
            .or_default()
            .push(validation);
    }

    /// Update our position (if we change our mind during consensus)
    pub fn update_position(&mut self, new_position: UInt256) {
        if self.phase != ConsensusPhase::Establish {
            return;
        }

        if self.our_position != Some(new_position) {
            self.our_position = Some(new_position);
            self.propose_seq += 1;
            self.last_broadcast = Instant::now();

            info!(
                round = self.round_id,
                seq = self.propose_seq,
                "Updated consensus position"
            );
        }
    }

    /// Check if we should accept the current position
    pub fn should_accept(&self, params: &ConsensusParms) -> bool {
        if self.phase != ConsensusPhase::Establish {
            return false;
        }

        let peer_count = self.peer_positions.len();
        if peer_count < params.ledger_min_consensus as usize {
            return false;
        }

        // Calculate consensus percentage
        let consensus_pct = self.consensus_percentage();
        consensus_pct >= params.ledger_min_consensus_pct as f64
    }

    /// Accept the current position and move to processing
    pub fn accept(&mut self) {
        if self.phase != ConsensusPhase::Establish {
            return;
        }

        self.phase = ConsensusPhase::Processing;
        self.accepted = true;

        info!(
            round = self.round_id,
            ledger_index = self.ledger_index,
            "Consensus achieved, accepting ledger"
        );
    }

    /// Check if the round is complete
    pub fn is_complete(&self) -> bool {
        matches!(self.phase, ConsensusPhase::Processing | ConsensusPhase::Accepted)
    }

    /// Get the winning ledger hash based on validations
    pub fn get_winning_ledger(&self) -> Option<UInt256> {
        self.validations
            .iter()
            .max_by_key(|(_, v)| v.len())
            .map(|(hash, _)| *hash)
    }

    /// Get validation count for a specific ledger
    pub fn get_validation_count(&self, ledger_hash: UInt256) -> usize {
        self.validations
            .get(&ledger_hash)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    /// Get the consensus percentage for our position
    pub fn consensus_percentage(&self) -> f64 {
        let our_pos = match self.our_position {
            Some(p) => p,
            None => return 0.0,
        };

        let peer_count = self.peer_positions.len();
        if peer_count == 0 {
            return 0.0;
        }

        let matching = self
            .peer_positions
            .values()
            .filter(|p| p.proposal.position == our_pos)
            .count();

        (matching as f64 / peer_count as f64) * 100.0
    }

    /// Get count of peers agreeing with our position
    pub fn agreeing_peers(&self) -> usize {
        let our_pos = match self.our_position {
            Some(p) => p,
            None => return 0,
        };

        self.peer_positions
            .values()
            .filter(|p| p.proposal.position == our_pos)
            .count()
    }

    /// Get total peer count
    pub fn peer_count(&self) -> usize {
        self.peer_positions.len()
    }

    /// Remove stale peer positions
    pub fn remove_stale_peers(&mut self, cutoff: u64) {
        let before = self.peer_positions.len();
        self.peer_positions.retain(|_, p| p.last_update >= cutoff);
        let after = self.peer_positions.len();

        if before != after {
            debug!("Removed {} stale peer positions", before - after);
        }
    }

    /// Create a proposal message for broadcasting
    pub fn create_proposal(&self, node_id: NodeID) -> Option<Proposal> {
        let position = self.our_position?;

        Some(Proposal::new(
            node_id,
            self.previous_ledger,
            position,
            self.propose_seq,
            self.close_time,
        ))
    }

    /// Create a validation for the accepted ledger
    pub fn create_validation(&self, node_id: NodeID, signing_key: &PrivateKey) -> Option<Validation> {
        let ledger_hash = self.get_winning_ledger()?;

        Some(Validation::with_signature(
            node_id,
            self.ledger_index,
            ledger_hash,
            self.close_time,
            signing_key,
        ))
    }

    /// Get duration since round started
    pub fn duration(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Check if we need to rebroadcast our position
    pub fn should_rebroadcast(&self, interval: Duration) -> bool {
        self.last_broadcast.elapsed() > interval
    }

    /// Mark that we've broadcast our position
    pub fn mark_broadcast(&mut self) {
        self.last_broadcast = Instant::now();
    }
}

/// Manager for consecutive consensus rounds
pub struct ConsensusRoundManager {
    /// Current or most recent round
    current_round: Option<ConsensusRound>,
    /// Round counter
    round_counter: u64,
    /// Current ledger index
    current_ledger_index: LedgerIndex,
    /// Last closed ledger hash
    last_closed_ledger: UInt256,
}

impl ConsensusRoundManager {
    pub fn new(genesis_ledger: UInt256) -> Self {
        Self {
            current_round: None,
            round_counter: 0,
            current_ledger_index: 1,
            last_closed_ledger: genesis_ledger,
        }
    }

    /// Start a new consensus round
    pub fn start_round(&mut self, close_time: u32) -> &mut ConsensusRound {
        self.round_counter += 1;

        let round = ConsensusRound::new(
            self.round_counter,
            self.last_closed_ledger,
            self.current_ledger_index,
            close_time,
        );

        self.current_round = Some(round);

        info!(
            round = self.round_counter,
            ledger_index = self.current_ledger_index,
            "Started new consensus round"
        );

        self.current_round.as_mut().unwrap()
    }

    /// Get the current round
    pub fn current_round(&self) -> Option<&ConsensusRound> {
        self.current_round.as_ref()
    }

    /// Get the current round mutably
    pub fn current_round_mut(&mut self) -> Option<&mut ConsensusRound> {
        self.current_round.as_mut()
    }

    /// Finish the current round and advance
    pub fn finish_round(&mut self, accepted_ledger: UInt256) {
        if let Some(round) = &self.current_round {
            info!(
                round = round.round_id,
                ledger_index = round.ledger_index,
                "Finished consensus round"
            );
        }

        self.last_closed_ledger = accepted_ledger;
        self.current_ledger_index += 1;
        self.current_round = None;
    }

    /// Get the current ledger index
    pub fn current_ledger_index(&self) -> LedgerIndex {
        self.current_ledger_index
    }

    /// Get the last closed ledger hash
    pub fn last_closed_ledger(&self) -> UInt256 {
        self.last_closed_ledger
    }

    /// Get the round counter
    pub fn round_counter(&self) -> u64 {
        self.round_counter
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_params() -> ConsensusParms {
        ConsensusParms {
            ledger_min_consensus: 3,
            ledger_min_consensus_pct: 80,
            ..Default::default()
        }
    }

    #[test]
    fn test_consensus_round_lifecycle() {
        let prev_ledger = UInt256::new([1u8; 32]);
        let mut round = ConsensusRound::new(1, prev_ledger, 2, 1000);

        assert_eq!(round.phase, ConsensusPhase::Open);
        assert!(round.our_position.is_none());

        // Close ledger
        let tx_set = UInt256::new([2u8; 32]);
        round.close_ledger(tx_set);

        assert_eq!(round.phase, ConsensusPhase::Establish);
        assert_eq!(round.our_position, Some(tx_set));
        assert_eq!(round.propose_seq, 1);
    }

    #[test]
    fn test_consensus_percentage() {
        let prev_ledger = UInt256::zero();
        let mut round = ConsensusRound::new(1, prev_ledger, 2, 1000);

        let our_pos = UInt256::new([1u8; 32]);
        round.close_ledger(our_pos);

        // No peers yet
        assert_eq!(round.consensus_percentage(), 0.0);

        // Add peers with different positions
        for i in 0..5 {
            let node_id = NodeID::new([i as u8; 32]);
            let pos = if i < 3 {
                our_pos // 3 agree
            } else {
                UInt256::new([2u8; 32]) // 2 disagree
            };

            let proposal = Proposal::new(node_id, prev_ledger, pos, 1, 1000);
            round.process_proposal(proposal, 2000);
        }

        // 3 out of 5 = 60%
        assert_eq!(round.consensus_percentage(), 60.0);
        assert_eq!(round.agreeing_peers(), 3);
    }

    #[test]
    fn test_should_accept() {
        let params = test_params();
        let prev_ledger = UInt256::zero();
        let mut round = ConsensusRound::new(1, prev_ledger, 2, 1000);

        let our_pos = UInt256::new([1u8; 32]);
        round.close_ledger(our_pos);

        // Not enough peers
        assert!(!round.should_accept(&params));

        // Add agreeing peers to reach consensus
        for i in 0..4 {
            let node_id = NodeID::new([i as u8; 32]);
            let proposal = Proposal::new(node_id, prev_ledger, our_pos, 1, 1000);
            round.process_proposal(proposal, 2000);
        }

        // 4 peers, all agree = 100% >= 80%
        assert!(round.should_accept(&params));
    }

    #[test]
    fn test_winning_ledger() {
        let prev_ledger = UInt256::zero();
        let mut round = ConsensusRound::new(1, prev_ledger, 2, 1000);

        let hash1 = UInt256::new([1u8; 32]);
        let hash2 = UInt256::new([2u8; 32]);

        // Add validations for hash1
        for i in 0..3 {
            let validation = Validation {
                node_id: NodeID::new([i as u8; 32]),
                ledger_index: 2,
                ledger_hash: hash1,
                close_time: 1000,
                sign_time: 1100,
                signature: None,
                signing_pub_key: None,
            };
            round.process_validation(validation);
        }

        // Add validation for hash2
        let validation = Validation {
            node_id: NodeID::new([3u8; 32]),
            ledger_index: 2,
            ledger_hash: hash2,
            close_time: 1000,
            sign_time: 1100,
            signature: None,
            signing_pub_key: None,
        };
        round.process_validation(validation);

        // hash1 should win (3 vs 1 validations)
        assert_eq!(round.get_winning_ledger(), Some(hash1));
    }

    #[test]
    fn test_round_manager() {
        let genesis = UInt256::new([0u8; 32]);
        let mut manager = ConsensusRoundManager::new(genesis);

        assert_eq!(manager.current_ledger_index(), 1);

        // Start round
        manager.start_round(1000);
        assert_eq!(manager.round_counter(), 1);

        let round = manager.current_round().unwrap();
        assert_eq!(round.ledger_index, 1);

        // Close and finish
        let accepted = UInt256::new([1u8; 32]);
        manager.finish_round(accepted);

        assert_eq!(manager.current_ledger_index(), 2);
        assert_eq!(manager.last_closed_ledger(), accepted);
        assert!(manager.current_round().is_none());
    }

    #[test]
    fn test_stale_peer_removal() {
        let prev_ledger = UInt256::zero();
        let mut round = ConsensusRound::new(1, prev_ledger, 2, 1000);

        // Close ledger to enter Establish phase (required for accepting proposals)
        round.close_ledger(UInt256::zero());
        assert_eq!(round.phase, ConsensusPhase::Establish);

        // Add peers
        for i in 0..3 {
            let node_id = NodeID::new([i as u8; 32]);
            let proposal = Proposal::new(node_id, prev_ledger, UInt256::zero(), 1, 1000);
            round.process_proposal(proposal, 1000);
        }

        assert_eq!(round.peer_count(), 3);

        // Remove peers older than 2500 (peers with last_update < 2500 are removed)
        round.remove_stale_peers(2500);
        assert_eq!(round.peer_count(), 0);
    }
}
