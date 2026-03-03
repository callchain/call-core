use crate::params::ConsensusParms;
use crate::types::{ConsensusMode, ConsensusPhase, PeerPosition, Proposal, Validation};
use primitives::{LedgerIndex, NodeID, UInt256};
use std::collections::{HashMap, HashSet};

/// Tracks Byzantine faults detected during consensus
#[derive(Debug, Clone)]
pub struct ByzantineFault {
    pub node_id: NodeID,
    pub fault_type: FaultType,
    pub evidence: Vec<FaultEvidence>,
}

#[derive(Debug, Clone)]
pub enum FaultType {
    /// Validator sent conflicting proposals for same round
    ConflictingProposals,
    /// Validator sent multiple validations for different ledgers at same index
    ConflictingValidations,
    /// Validator not in UNL tried to participate
    UntrustedValidator,
    /// Invalid signature on proposal/validation
    InvalidSignature,
}

#[derive(Debug, Clone)]
pub struct FaultEvidence {
    pub round_id: u64,
    pub ledger_index: LedgerIndex,
    pub data: Vec<u8>,
}

/// Tracks the state of a single consensus round
#[derive(Debug)]
pub struct ConsensusState {
    pub previous_ledger: UInt256,
    pub close_time: u32,
    pub our_position: Option<UInt256>,
    pub peer_positions: HashMap<NodeID, PeerPosition>,
    pub disputed_txs: Vec<UInt256>,
    /// Validator weights for weighted consensus
    validator_weights: HashMap<NodeID, u32>,
    /// Total weight of all validators in UNL
    total_weight: u32,
    /// Byzantine faults detected this round
    pub faults: Vec<ByzantineFault>,
    /// Proposals seen from each validator (for conflict detection)
    proposals_by_validator: HashMap<NodeID, Vec<Proposal>>,
}

impl ConsensusState {
    pub fn new(previous_ledger: UInt256, close_time: u32) -> Self {
        Self {
            previous_ledger,
            close_time,
            our_position: None,
            peer_positions: HashMap::new(),
            disputed_txs: Vec::new(),
            validator_weights: HashMap::new(),
            total_weight: 0,
            faults: Vec::new(),
            proposals_by_validator: HashMap::new(),
        }
    }

    /// Set validator weights from UNL
    pub fn set_validator_weights(&mut self, weights: HashMap<NodeID, u32>) {
        self.validator_weights = weights.clone();
        self.total_weight = weights.values().sum();
    }

    /// Get the weight for a validator
    pub fn get_validator_weight(&self, node_id: &NodeID) -> u32 {
        self.validator_weights.get(node_id).copied().unwrap_or(1)
    }

    /// Get total weight of all validators
    pub fn get_total_weight(&self) -> u32 {
        self.total_weight
    }

    pub fn add_peer_proposal(&mut self, proposal: Proposal, now: u64) {
        // Track proposal for conflict detection
        self.proposals_by_validator
            .entry(proposal.node_id)
            .or_default()
            .push(proposal.clone());

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

    /// Calculate unweighted consensus percentage
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

    /// Calculate weighted consensus percentage
    /// Returns the percentage of trusted weight that agrees with our position
    pub fn weighted_consensus_pct(&self, our_node_id: NodeID) -> f64 {
        if self.our_position.is_none() {
            return 0.0;
        }

        let trusted_weight = self.get_trusted_weight();
        if trusted_weight == 0 {
            return 0.0;
        }

        let our_pos = self.our_position.unwrap();
        let mut matching_weight = 0u32;

        // Count matching peer weight (only from non-faulty validators)
        for peer in self.peer_positions.values() {
            if self.is_validator_faulty(&peer.node_id) {
                continue;
            }
            if peer.proposal.position == our_pos {
                matching_weight = matching_weight.saturating_add(
                    self.get_validator_weight(&peer.node_id)
                );
            }
        }

        // Add our own weight if we're not faulty
        if !self.is_validator_faulty(&our_node_id) {
            matching_weight = matching_weight.saturating_add(
                self.get_validator_weight(&our_node_id)
            );
        }

        (matching_weight as f64 / trusted_weight as f64) * 100.0
    }

    /// Detect conflicting proposals from the same validator (Byzantine fault)
    pub fn detect_conflicting_proposals(&mut self) -> Vec<ByzantineFault> {
        let mut new_faults = Vec::new();

        for (node_id, proposals) in &self.proposals_by_validator {
            if proposals.len() < 2 {
                continue;
            }

            // Check for conflicting positions in the same round
            let first_position = &proposals[0].position;
            let first_ledger = &proposals[0].previous_ledger;

            for proposal in proposals.iter().skip(1) {
                // Same round = same previous_ledger, different position
                if proposal.previous_ledger == *first_ledger && proposal.position != *first_position {
                    let evidence = proposals.iter().map(|p| FaultEvidence {
                        round_id: p.propose_seq as u64,
                        ledger_index: 0,
                        data: p.position.as_bytes().to_vec(),
                    }).collect();

                    new_faults.push(ByzantineFault {
                        node_id: *node_id,
                        fault_type: FaultType::ConflictingProposals,
                        evidence,
                    });
                    break;
                }
            }
        }

        self.faults.extend(new_faults.clone());
        new_faults
    }

    /// Check if a validator has been detected as faulty
    pub fn is_validator_faulty(&self, node_id: &NodeID) -> bool {
        self.faults.iter().any(|f| &f.node_id == node_id)
    }

    /// Get total weight of non-faulty validators
    pub fn get_trusted_weight(&self) -> u32 {
        self.validator_weights
            .iter()
            .filter(|(node_id, _)| !self.is_validator_faulty(node_id))
            .map(|(_, weight)| *weight)
            .sum()
    }
}

/// Main consensus manager
/// Validator info for UNL
#[derive(Debug, Clone)]
pub struct ValidatorInfo {
    pub node_id: NodeID,
    pub public_key: Vec<u8>,
    pub domain: Option<String>,
    pub name: Option<String>,
    pub trusted: bool,
    /// Validator weight for weighted consensus (default: 1)
    pub weight: u32,
}

impl ValidatorInfo {
    /// Create a new validator with default weight of 1
    pub fn new(node_id: NodeID, public_key: Vec<u8>, trusted: bool) -> Self {
        Self {
            node_id,
            public_key,
            domain: None,
            name: None,
            trusted,
            weight: 1,
        }
    }

    /// Set validator weight
    pub fn with_weight(mut self, weight: u32) -> Self {
        self.weight = weight;
        self
    }
}

pub struct Consensus {
    params: ConsensusParms,
    mode: ConsensusMode,
    phase: ConsensusPhase,
    node_id: NodeID,
    state: Option<ConsensusState>,
    validations: HashMap<UInt256, Vec<Validation>>,
    ledger_index: LedgerIndex,
    round_id: u64,
    /// Unique Node List - trusted validators
    unl: Vec<ValidatorInfo>,
    /// Time when the current ledger was opened
    ledger_open_time: Option<std::time::Instant>,
    /// Current transaction count in open ledger
    current_tx_count: usize,
    /// Current ledger size in bytes
    current_ledger_size: usize,
}

impl Consensus {
    pub fn new(node_id: NodeID, params: ConsensusParms) -> Self {
        let mut consensus = Self {
            params,
            mode: ConsensusMode::Observing,
            phase: ConsensusPhase::Open,
            node_id,
            state: None,
            validations: HashMap::new(),
            ledger_index: 0,
            round_id: 0,
            unl: Vec::new(),
            ledger_open_time: Some(std::time::Instant::now()),
            current_tx_count: 0,
            current_ledger_size: 0,
        };

        // Add self as a validator
        consensus.add_validator(node_id, Vec::new(), None, None, true);

        consensus
    }

    /// Add a validator to the UNL
    pub fn add_validator(
        &mut self,
        node_id: NodeID,
        public_key: Vec<u8>,
        domain: Option<String>,
        name: Option<String>,
        trusted: bool,
    ) {
        self.add_validator_with_weight(node_id, public_key, domain, name, trusted, 1);
    }

    /// Add a validator with a specific weight
    pub fn add_validator_with_weight(
        &mut self,
        node_id: NodeID,
        public_key: Vec<u8>,
        domain: Option<String>,
        name: Option<String>,
        trusted: bool,
        weight: u32,
    ) {
        // Check if validator already exists
        if self.unl.iter().any(|v| v.node_id == node_id) {
            return;
        }

        self.unl.push(ValidatorInfo {
            node_id,
            public_key,
            domain,
            name,
            trusted,
            weight,
        });
    }

    /// Get the list of validators
    pub fn get_validators(&self) -> &[ValidatorInfo] {
        &self.unl
    }

    /// Get the count of trusted validators
    pub fn get_trusted_validator_count(&self) -> usize {
        self.unl.iter().filter(|v| v.trusted).count()
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
        let mut state = ConsensusState::new(previous_ledger, 0);

        // Initialize validator weights from UNL
        let mut weights = HashMap::new();
        let mut total_weight = 0u32;
        for validator in &self.unl {
            if validator.trusted {
                weights.insert(validator.node_id, validator.weight);
                total_weight = total_weight.saturating_add(validator.weight);
            }
        }
        state.set_validator_weights(weights);

        self.state = Some(state);
        self.validations.clear();
        // Reset ledger open time and transaction counts for new round
        self.ledger_open_time = Some(std::time::Instant::now());
        self.current_tx_count = 0;
        self.current_ledger_size = 0;
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

    /// Check if we have consensus using weighted BFT algorithm
    /// Requires agreement from at least 80% of validator weight
    pub fn have_consensus(&self) -> bool {
        if self.phase != ConsensusPhase::Establish {
            return false;
        }

        let state = match &self.state {
            Some(s) => s,
            None => return false,
        };

        // Must have a position
        if state.our_position.is_none() {
            return false;
        }

        let peer_count = state.peer_count();
        let trusted_count = self.get_trusted_validator_count();

        // Single-node case: if no peers and we have a position, accept it
        if trusted_count <= 1 && state.our_position.is_some() {
            return true;
        }

        // Need minimum number of peer proposals
        if peer_count < self.params.ledger_min_consensus {
            return false;
        }

        // Use weighted consensus for BFT
        let weighted_pct = state.weighted_consensus_pct(self.node_id);
        weighted_pct >= 80.0
    }

    /// Run Byzantine fault detection on current proposals
    pub fn detect_byzantine_faults(&mut self) -> Vec<ByzantineFault> {
        if let Some(state) = &mut self.state {
            state.detect_conflicting_proposals()
        } else {
            Vec::new()
        }
    }

    /// Get current Byzantine faults detected
    pub fn get_faults(&self) -> Vec<ByzantineFault> {
        self.state.as_ref()
            .map(|s| s.faults.clone())
            .unwrap_or_default()
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

    /// Get current unweighted consensus percentage (for debugging)
    pub fn get_consensus_pct(&self) -> f64 {
        self.state.as_ref().map(|s| s.consensus_pct()).unwrap_or(0.0)
    }

    /// Get current weighted consensus percentage (used for BFT consensus)
    pub fn get_weighted_consensus_pct(&self) -> f64 {
        self.state.as_ref()
            .map(|s| s.weighted_consensus_pct(self.node_id))
            .unwrap_or(0.0)
    }

    /// Check if we should close the ledger (timeout or full)
    pub fn should_close_ledger(&self) -> bool {
        // Only check if ledger is open
        if self.phase != ConsensusPhase::Open {
            return false;
        }

        // Check transaction count limit
        if self.current_tx_count >= self.params.ledger_max_tx_count {
            return true;
        }

        // Check ledger size limit
        if self.current_ledger_size >= self.params.ledger_max_size {
            return true;
        }

        // Check time-based constraints
        if let Some(open_time) = self.ledger_open_time {
            let elapsed = open_time.elapsed().as_secs() as u32;

            // Must stay open for minimum time
            if elapsed < self.params.ledger_min_close_time {
                return false;
            }

            // Must close after maximum time
            if elapsed >= self.params.ledger_max_close_time {
                return true;
            }
        }

        // Check if we have enough transactions for minimum close
        // This prevents closing empty ledgers too quickly
        self.current_tx_count >= self.params.ledger_min_consensus
    }

    /// Add a transaction to the open ledger
    pub fn add_transaction(&mut self, tx_size: usize) {
        self.current_tx_count += 1;
        self.current_ledger_size += tx_size;
    }

    /// Reset ledger tracking when opening a new ledger
    pub fn reset_ledger_open(&mut self) {
        self.ledger_open_time = Some(std::time::Instant::now());
        self.current_tx_count = 0;
        self.current_ledger_size = 0;
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

    /// Vote on disputed transactions using weighted voting
    /// Returns list of (transaction_hash, accepted) tuples
    pub fn vote_on_disputes(
        &mut self,
        tx_inclusion: &HashMap<UInt256, Vec<NodeID>>,
        acceptance_threshold: f64,
    ) -> Vec<(UInt256, bool)> {
        let mut results = Vec::new();

        let disputed = self
            .state
            .as_ref()
            .map(|s| s.disputed_txs.clone())
            .unwrap_or_default();

        for tx in &disputed {
            if let Some(state) = &self.state {
                let mut votes_for: u32 = 0;

                // Count weighted votes from validators who include this tx
                if let Some(supporters) = tx_inclusion.get(tx) {
                    for node_id in supporters {
                        // Skip faulty validators
                        if state.is_validator_faulty(node_id) {
                            continue;
                        }
                        votes_for = votes_for.saturating_add(state.get_validator_weight(node_id));
                    }
                }

                // Calculate acceptance rate based on total trusted weight
                let total_weight = state.get_trusted_weight();
                if total_weight > 0 {
                    let acceptance_rate = votes_for as f64 / total_weight as f64;
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

    /// Check if close time has reached consensus using weighted voting
    pub fn close_time_consensus(&self, threshold_pct: f64) -> bool {
        if let Some(state) = &self.state {
            let target_time = self.calculate_close_time();
            let trusted_weight = state.get_trusted_weight();

            if trusted_weight == 0 {
                return false;
            }

            // Count weighted matching votes (only from non-faulty validators)
            let mut matching_weight: u32 = 0;
            if !state.is_validator_faulty(&self.node_id) {
                matching_weight = state.get_validator_weight(&self.node_id);
            }

            for peer in state.peer_positions.values() {
                // Skip faulty validators
                if state.is_validator_faulty(&peer.node_id) {
                    continue;
                }
                if peer.proposal.close_time == target_time {
                    matching_weight = matching_weight.saturating_add(
                        state.get_validator_weight(&peer.node_id)
                    );
                }
            }

            let agreement_pct = (matching_weight as f64 / trusted_weight as f64) * 100.0;
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

    #[test]
    fn test_validator_weights() {
        let ledger = UInt256::zero();
        let mut state = ConsensusState::new(ledger, 0);

        // Set validator weights
        let mut weights = HashMap::new();
        let v1 = NodeID::new([1u8; 32]);
        let v2 = NodeID::new([2u8; 32]);
        let v3 = NodeID::new([3u8; 32]);

        weights.insert(v1, 5);
        weights.insert(v2, 3);
        weights.insert(v3, 2);

        state.set_validator_weights(weights);

        assert_eq!(state.get_total_weight(), 10);
        assert_eq!(state.get_validator_weight(&v1), 5);
        assert_eq!(state.get_validator_weight(&v2), 3);
        assert_eq!(state.get_validator_weight(&v3), 2);
        // Unknown validator should default to 1
        let unknown = NodeID::new([99u8; 32]);
        assert_eq!(state.get_validator_weight(&unknown), 1);
    }

    #[test]
    fn test_weighted_consensus() {
        let ledger = UInt256::zero();
        let our_position = UInt256::new([1u8; 32]);
        let other_position = UInt256::new([2u8; 32]);

        let mut state = ConsensusState::new(ledger, 0);
        let our_node_id = NodeID::new([0u8; 32]);

        // Set validator weights (total = 10)
        let mut weights = HashMap::new();
        let v1 = NodeID::new([1u8; 32]);
        let v2 = NodeID::new([2u8; 32]);
        let v3 = NodeID::new([3u8; 32]);

        weights.insert(our_node_id, 5); // Us
        weights.insert(v1, 3);          // Validator 1
        weights.insert(v2, 2);          // Validator 2
        weights.insert(v3, 1);          // Validator 3 (faulty, will disagree)
        state.set_validator_weights(weights);

        state.our_position = Some(our_position);

        // Add peer proposals
        // v1 agrees with us (weight 3)
        state.add_peer_proposal(
            Proposal::new(v1, ledger, our_position, 1, 0),
            1000,
        );

        // v2 disagrees (weight 2)
        state.add_peer_proposal(
            Proposal::new(v2, ledger, other_position, 1, 0),
            1000,
        );

        // v3 agrees with us (weight 1)
        state.add_peer_proposal(
            Proposal::new(v3, ledger, our_position, 1, 0),
            1000,
        );

        // Weighted consensus: (5 + 3 + 1) / 11 = 81.8%
        let pct = state.weighted_consensus_pct(our_node_id);
        assert!(pct > 80.0, "Expected > 80%, got {}", pct);
    }

    #[test]
    fn test_byzantine_fault_detection() {
        let ledger = UInt256::zero();
        let mut state = ConsensusState::new(ledger, 0);

        let validator = NodeID::new([1u8; 32]);
        let pos1 = UInt256::new([1u8; 32]);
        let pos2 = UInt256::new([2u8; 32]);

        // First proposal
        state.add_peer_proposal(
            Proposal::new(validator, ledger, pos1, 1, 0),
            1000,
        );

        // Second conflicting proposal (same round, different position)
        state.add_peer_proposal(
            Proposal::new(validator, ledger, pos2, 1, 0),
            1001,
        );

        // Detect faults
        let faults = state.detect_conflicting_proposals();
        assert_eq!(faults.len(), 1);
        assert!(matches!(faults[0].fault_type, FaultType::ConflictingProposals));
        assert_eq!(faults[0].node_id, validator);

        // Check validator is marked faulty
        assert!(state.is_validator_faulty(&validator));
    }

    #[test]
    fn test_byzantine_fault_excludes_from_consensus() {
        let ledger = UInt256::zero();
        let our_position = UInt256::new([1u8; 32]);

        let mut state = ConsensusState::new(ledger, 0);
        let our_node_id = NodeID::new([0u8; 32]);

        // Set validator weights
        let mut weights = HashMap::new();
        let v1 = NodeID::new([1u8; 32]);
        let v2 = NodeID::new([2u8; 32]);

        weights.insert(our_node_id, 5);
        weights.insert(v1, 3); // Will become faulty
        weights.insert(v2, 2);
        state.set_validator_weights(weights);

        state.our_position = Some(our_position);

        // Add conflicting proposals from v1 (Byzantine fault)
        state.add_peer_proposal(
            Proposal::new(v1, ledger, our_position, 1, 0),
            1000,
        );
        state.add_peer_proposal(
            Proposal::new(v1, ledger, UInt256::new([99u8; 32]), 1, 0),
            1001,
        );

        // Add agreement from v2
        state.add_peer_proposal(
            Proposal::new(v2, ledger, our_position, 1, 0),
            1000,
        );

        // Detect faults
        state.detect_conflicting_proposals();

        // Trusted weight should exclude faulty validator
        // Total: 10, Faulty: 3, Trusted: 7
        let trusted_weight = state.get_trusted_weight();
        assert_eq!(trusted_weight, 7);

        // Our weight (5) + v2 weight (2) = 7 out of 7 trusted = 100%
        let pct = state.weighted_consensus_pct(our_node_id);
        assert_eq!(pct, 100.0);
    }

    #[test]
    fn test_bft_consensus_requirements() {
        let node_id = NodeID::new([0u8; 32]);
        let params = ConsensusParms::default();
        let mut consensus = Consensus::new(node_id, params);

        // Add validators with different weights
        let v1 = NodeID::new([1u8; 32]);
        let v2 = NodeID::new([2u8; 32]);
        let v3 = NodeID::new([3u8; 32]);

        // Self has weight 1 (added in Consensus::new)
        // Add validators with weights that will let us reach 80%
        consensus.add_validator_with_weight(v1, vec![1], None, None, true, 4);
        consensus.add_validator_with_weight(v2, vec![2], None, None, true, 4);
        consensus.add_validator_with_weight(v3, vec![3], None, None, true, 2);
        // Total weight = 1 (us) + 4 + 4 + 2 = 11
        // 80% of 11 = 8.8, so we need 9 weight

        let prev_ledger = UInt256::zero();
        consensus.start_round(prev_ledger, 1);

        let our_position = UInt256::new([1u8; 32]);
        consensus.close_ledger(our_position, 0);

        // Initially no consensus (only us with weight 1 = 9%)
        assert!(!consensus.have_consensus());

        // Add proposals from validators
        // v1 agrees (weight 4) -> total 5, not enough (45%)
        consensus.process_proposal(
            Proposal::new(v1, prev_ledger, our_position, 1, 0),
            1000,
        );
        assert!(!consensus.have_consensus());

        // v2 agrees (weight 4) -> total 9, enough for 80% (9/11 = 82%)
        consensus.process_proposal(
            Proposal::new(v2, prev_ledger, our_position, 1, 0),
            1000,
        );
        assert!(consensus.have_consensus());
    }

    #[test]
    fn test_close_time_consensus_weighted() {
        let node_id = NodeID::new([0u8; 32]);
        let params = ConsensusParms::default();
        let mut consensus = Consensus::new(node_id, params);

        let v1 = NodeID::new([1u8; 32]);
        let v2 = NodeID::new([2u8; 32]);
        let v3 = NodeID::new([3u8; 32]);
        let v4 = NodeID::new([4u8; 32]);
        let v5 = NodeID::new([5u8; 32]);

        // Self has weight 1 (added in Consensus::new)
        // Add all validators BEFORE starting the round (weights are captured at start_round)
        consensus.add_validator_with_weight(v1, vec![1], None, None, true, 5);
        consensus.add_validator_with_weight(v2, vec![2], None, None, true, 5);
        consensus.add_validator_with_weight(v3, vec![3], None, None, true, 5);
        consensus.add_validator_with_weight(v4, vec![4], None, None, true, 5);
        consensus.add_validator_with_weight(v5, vec![5], None, None, true, 5);
        // Total weight = 1 (us) + 5 + 5 + 5 + 5 + 5 = 26
        // 80% of 26 = 20.8, so we need 21 weight

        let prev_ledger = UInt256::zero();
        consensus.start_round(prev_ledger, 1);

        // Close ledger to enter Establish phase (required for process_proposal)
        consensus.close_ledger(UInt256::new([1u8; 32]), 1000);

        // We have weight 1, need 20 more to reach 21

        // v1 agrees (weight 5) -> total 6, not enough (23%)
        consensus.process_proposal(
            Proposal::new(v1, prev_ledger, UInt256::new([1u8; 32]), 1, 1000),
            1000,
        );
        assert!(!consensus.close_time_consensus(80.0));

        // v2 disagrees (weight 5) with different close time -> still 6
        consensus.process_proposal(
            Proposal::new(v2, prev_ledger, UInt256::new([2u8; 32]), 1, 2000),
            1000,
        );
        assert!(!consensus.close_time_consensus(80.0));

        // v3 agrees (weight 5) -> total 11 (42%)
        consensus.process_proposal(
            Proposal::new(v3, prev_ledger, UInt256::new([3u8; 32]), 1, 1000),
            1000,
        );
        assert!(!consensus.close_time_consensus(80.0));

        // v4 agrees (weight 5) -> total 16 (62%)
        consensus.process_proposal(
            Proposal::new(v4, prev_ledger, UInt256::new([4u8; 32]), 1, 1000),
            1000,
        );
        assert!(!consensus.close_time_consensus(80.0));

        // v5 agrees (weight 5) -> total 21 (81%)
        consensus.process_proposal(
            Proposal::new(v5, prev_ledger, UInt256::new([5u8; 32]), 1, 1000),
            1000,
        );

        // Should have close time consensus (21/26 = 81%)
        assert!(consensus.close_time_consensus(80.0));
    }
}
