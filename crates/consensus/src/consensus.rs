use crate::params::ConsensusParms;
use crate::types::{ConsensusMode, ConsensusPhase, PeerPosition, Proposal, Validation};
use primitives::{LedgerIndex, NodeID, UInt256};
use std::collections::HashMap;

/// Tracks the state of a single consensus round
#[derive(Debug)]
pub struct ConsensusState {
    pub previous_ledger: UInt256,
    pub close_time: u32,
    pub our_position: Option<UInt256>,
    pub peer_positions: HashMap<NodeID, PeerPosition>,
    pub disputed_txs: Vec<UInt256>,
}

impl ConsensusState {
    pub fn new(previous_ledger: UInt256, close_time: u32) -> Self {
        Self {
            previous_ledger,
            close_time,
            our_position: None,
            peer_positions: HashMap::new(),
            disputed_txs: Vec::new(),
        }
    }

    pub fn add_peer_proposal(&mut self, proposal: Proposal, now: u64) {
        let position = PeerPosition {
            node_id: proposal.node_id,
            last_update: now,
            proposal,
        };
        self.peer_positions.insert(position.node_id, position);
    }

    pub fn update_peer_proposal(&mut self, proposal: Proposal, now: u64) {
        if let Some(existing) = self.peer_positions.get_mut(&proposal.node_id) {
            existing.proposal = proposal;
            existing.last_update = now;
        }
    }

    pub fn remove_stale_peers(&mut self, cutoff: u64) {
        self.peer_positions.retain(|_, p| p.last_update >= cutoff);
    }

    pub fn peer_count(&self) -> usize {
        self.peer_positions.len()
    }

    pub fn consensus_pct(&self) -> f64 {
        if self.peer_positions.is_empty() || self.our_position.is_none() {
            return 0.0;
        }

        let our_pos = self.our_position.unwrap();
        let matching = self
            .peer_positions
            .values()
            .filter(|p| p.proposal.position == our_pos)
            .count();

        (matching as f64 / self.peer_positions.len() as f64) * 100.0
    }
}

/// Main consensus manager
pub struct Consensus {
    params: ConsensusParms,
    mode: ConsensusMode,
    phase: ConsensusPhase,
    node_id: NodeID,
    state: Option<ConsensusState>,
    validations: HashMap<UInt256, Vec<Validation>>,
    ledger_index: LedgerIndex,
    round_id: u64,
}

impl Consensus {
    pub fn new(node_id: NodeID, params: ConsensusParms) -> Self {
        Self {
            params,
            mode: ConsensusMode::Observing,
            phase: ConsensusPhase::Open,
            node_id,
            state: None,
            validations: HashMap::new(),
            ledger_index: 0,
            round_id: 0,
        }
    }

    pub fn with_mode(mut self, mode: ConsensusMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn get_mode(&self) -> ConsensusMode {
        self.mode
    }

    pub fn get_phase(&self) -> ConsensusPhase {
        self.phase
    }

    pub fn get_round_id(&self) -> u64 {
        self.round_id
    }

    pub fn get_ledger_index(&self) -> LedgerIndex {
        self.ledger_index
    }

    /// Start a new consensus round
    pub fn start_round(&mut self, previous_ledger: UInt256, ledger_index: LedgerIndex) {
        self.round_id += 1;
        self.ledger_index = ledger_index;
        self.phase = ConsensusPhase::Open;
        self.state = Some(ConsensusState::new(previous_ledger, 0));
        self.validations.clear();
    }

    /// Close the open ledger and begin consensus
    pub fn close_ledger(&mut self, tx_set_hash: UInt256, close_time: u32) {
        if let Some(state) = &mut self.state {
            state.close_time = close_time;
            state.our_position = Some(tx_set_hash);
            self.phase = ConsensusPhase::Establish;
        }
    }

    /// Process a proposal from a peer
    pub fn process_proposal(&mut self, proposal: Proposal, now: u64) {
        if self.phase != ConsensusPhase::Establish {
            return;
        }

        if let Some(state) = &mut self.state {
            // Only accept proposals for the current round
            if proposal.previous_ledger == state.previous_ledger {
                if state.peer_positions.contains_key(&proposal.node_id) {
                    state.update_peer_proposal(proposal, now);
                } else {
                    state.add_peer_proposal(proposal, now);
                }
            }
        }
    }

    /// Process a validation from a peer
    pub fn process_validation(&mut self, validation: Validation) {
        // Only accept validations for the expected ledger index
        if validation.ledger_index != self.ledger_index + 1 {
            return;
        }

        self.validations
            .entry(validation.ledger_hash)
            .or_default()
            .push(validation);
    }

    /// Check if we have consensus
    pub fn have_consensus(&self) -> bool {
        if self.phase != ConsensusPhase::Establish {
            return false;
        }

        let state = match &self.state {
            Some(s) => s,
            None => return false,
        };

        let peer_count = state.peer_count();
        if peer_count < self.params.ledger_min_consensus {
            return false;
        }

        // Check if consensus percentage exceeds threshold
        state.consensus_pct() >= 80.0
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

    /// Move to processing phase
    pub fn accept_ledger(&mut self) {
        self.phase = ConsensusPhase::Processing;
    }

    /// Finish the round and return to open phase
    pub fn finish_round(&mut self) {
        self.phase = ConsensusPhase::Open;
        self.state = None;
    }

    /// Create our proposal
    pub fn create_proposal(&self, position: UInt256, propose_seq: u32) -> Option<Proposal> {
        let state = self.state.as_ref()?;
        Some(Proposal::new(
            self.node_id,
            state.previous_ledger,
            position,
            propose_seq,
            state.close_time,
        ))
    }

    /// Get the current consensus state
    pub fn get_state(&self) -> Option<&ConsensusState> {
        self.state.as_ref()
    }

    /// Get peer count
    pub fn get_peer_count(&self) -> usize {
        self.state.as_ref().map(|s| s.peer_count()).unwrap_or(0)
    }

    /// Get current consensus percentage
    pub fn get_consensus_pct(&self) -> f64 {
        self.state.as_ref().map(|s| s.consensus_pct()).unwrap_or(0.0)
    }

    /// Check if we should close the ledger (timeout or full)
    pub fn should_close_ledger(&self) -> bool {
        // In a real implementation, check if ledger is full or timeout reached
        // For now, always return true if in Open phase
        self.phase == ConsensusPhase::Open
    }

    /// Check if we should accept the current position
    pub fn should_accept(&self) -> bool {
        self.have_consensus()
    }

    /// Accept the current position and move to processing
    pub fn accept_position(&mut self) {
        self.accept_ledger();
    }

    /// Process the accepted ledger
    pub fn process_ledger(&mut self) -> anyhow::Result<()> {
        // Move to Accepted phase after processing
        if self.phase == ConsensusPhase::Processing {
            self.phase = ConsensusPhase::Accepted;
        }
        Ok(())
    }

    /// Check if the round is complete
    pub fn is_round_complete(&self) -> bool {
        self.phase == ConsensusPhase::Accepted
    }

    /// Detect and handle disputed transactions
    /// Returns transactions that need to be voted on
    pub fn detect_disputes(&mut self, tx_set: &[UInt256]) -> Vec<UInt256> {
        // Track votes per transaction
        let mut tx_votes: HashMap<UInt256, usize> = HashMap::new();

        // Count our votes
        for tx in tx_set {
            *tx_votes.entry(*tx).or_default() += 1;
        }

        // Count peer votes from their proposals
        if let Some(state) = &self.state {
            for _peer in state.peer_positions.values() {
                // In real impl, we'd extract transactions from peer's position
                // For now, mark all as potentially disputed if not matching
            }
        }

        // Find transactions with less than 50% agreement
        let total_peers = self.get_peer_count() + 1; // +1 for us
        let threshold = total_peers / 2;

        let disputed: Vec<UInt256> = tx_votes
            .iter()
            .filter(|(_, votes)| **votes <= threshold)
            .map(|(tx, _)| *tx)
            .collect();

        if let Some(state) = &mut self.state {
            state.disputed_txs = disputed.clone();
        }
        disputed
    }

    /// Vote on disputed transactions
    pub fn vote_on_disputes(&mut self, acceptance_threshold: f64) -> Vec<(UInt256, bool)> {
        let mut results = Vec::new();

        let disputed = self.state.as_ref().map(|s| s.disputed_txs.clone()).unwrap_or_default();
        for tx in &disputed {
            // Count votes for this transaction
            let mut votes_for = 0;
            let votes_against = 0;

            if let Some(state) = &self.state {
                // Count peer votes
                for _peer in state.peer_positions.values() {
                    // In real impl, check if peer's position includes this tx
                    // Simplified: assume 60% acceptance
                    votes_for += 1;
                }

                let total_votes = votes_for + votes_against;
                if total_votes > 0 {
                    let acceptance_rate = votes_for as f64 / total_votes as f64;
                    let accepted = acceptance_rate >= acceptance_threshold;
                    results.push((*tx, accepted));
                }
            }
        }

        results
    }

    /// Calculate network close time based on peer proposals
    pub fn calculate_close_time(&self) -> u32 {
        if let Some(state) = &self.state {
            let mut close_times: Vec<u32> = Vec::new();

            // Add our close time
            close_times.push(state.close_time);

            // Collect peer close times
            for peer in state.peer_positions.values() {
                close_times.push(peer.proposal.close_time);
            }

            if close_times.is_empty() {
                return state.close_time;
            }

            // Use median close time for consensus
            close_times.sort();
            let mid = close_times.len() / 2;
            if close_times.len() % 2 == 0 {
                // Even number - average the two middle values
                ((close_times[mid - 1] as u64 + close_times[mid] as u64) / 2) as u32
            } else {
                // Odd number - take the middle
                close_times[mid]
            }
        } else {
            0
        }
    }

    /// Check if close time has reached consensus
    pub fn close_time_consensus(&self, threshold_pct: f64) -> bool {
        if let Some(state) = &self.state {
            let target_time = self.calculate_close_time();
            let total_peers = state.peer_positions.len() + 1;

            let mut matching = 1; // Count ourselves
            for peer in state.peer_positions.values() {
                if peer.proposal.close_time == target_time {
                    matching += 1;
                }
            }

            let agreement_pct = (matching as f64 / total_peers as f64) * 100.0;
            agreement_pct >= threshold_pct
        } else {
            false
        }
    }

    /// Start a new consensus round
    pub fn start_new_round(&mut self) -> anyhow::Result<()> {
        // Get the winning ledger to start the next round
        let prev_ledger = self.get_winning_ledger().unwrap_or_else(|| {
            self.state.as_ref().map(|s| s.previous_ledger).unwrap_or_else(UInt256::zero)
        });
        let next_ledger_index = self.ledger_index + 1;
        self.start_round(prev_ledger, next_ledger_index);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consensus_state() {
        let ledger = UInt256::zero();
        let mut state = ConsensusState::new(ledger, 0);

        let node_id = NodeID::new([1u8; 32]);
        let proposal = Proposal::new(node_id, ledger, UInt256::zero(), 0, 0);

        state.add_peer_proposal(proposal.clone(), 1000);
        assert_eq!(state.peer_count(), 1);

        state.update_peer_proposal(proposal, 2000);
        assert_eq!(state.peer_count(), 1);
    }

    #[test]
    fn test_consensus_round() {
        let node_id = NodeID::new([0u8; 32]);
        let params = ConsensusParms::default();
        let mut consensus = Consensus::new(node_id, params);

        assert_eq!(consensus.get_phase(), ConsensusPhase::Open);

        let prev_ledger = UInt256::zero();
        consensus.start_round(prev_ledger, 1);
        assert_eq!(consensus.get_round_id(), 1);
        assert_eq!(consensus.get_ledger_index(), 1);

        consensus.close_ledger(UInt256::zero(), 0);
        assert_eq!(consensus.get_phase(), ConsensusPhase::Establish);
    }
}
