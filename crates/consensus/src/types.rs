use primitives::{LedgerIndex, NodeID, UInt256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsensusMode {
    Proposing,
    Observing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsensusPhase {
    Open,
    Establish,
    Processing,
}

/// A proposal from a validator during consensus
#[derive(Debug, Clone)]
pub struct Proposal {
    pub node_id: NodeID,
    pub previous_ledger: UInt256,
    pub position: UInt256,
    pub propose_seq: u32,
    pub close_time: u32,
}

impl Proposal {
    pub fn new(
        node_id: NodeID,
        previous_ledger: UInt256,
        position: UInt256,
        propose_seq: u32,
        close_time: u32,
    ) -> Self {
        Self {
            node_id,
            previous_ledger,
            position,
            propose_seq,
            close_time,
        }
    }
}

/// A validation (final agreement) from a validator
#[derive(Debug, Clone)]
pub struct Validation {
    pub node_id: NodeID,
    pub ledger_hash: UInt256,
    pub ledger_index: LedgerIndex,
    pub sign_time: u32,
    pub close_time: u32,
}

impl Validation {
    pub fn new(
        node_id: NodeID,
        ledger_hash: UInt256,
        ledger_index: LedgerIndex,
        close_time: u32,
    ) -> Self {
        Self {
            node_id,
            ledger_hash,
            ledger_index,
            sign_time: 0,
            close_time,
        }
    }
}

/// Position of a peer during consensus
#[derive(Debug, Clone)]
pub struct PeerPosition {
    pub node_id: NodeID,
    pub proposal: Proposal,
    pub last_update: u64,
}
