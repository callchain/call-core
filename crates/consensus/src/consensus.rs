use crate::params::ConsensusParms;
use crate::types::{ConsensusMode, ConsensusPhase};
use primitives::UInt256;

pub struct Consensus {
    params: ConsensusParms,
    mode: ConsensusMode,
    phase: ConsensusPhase,
}

impl Consensus {
    pub fn new(params: ConsensusParms) -> Self {
        Self {
            params,
            mode: ConsensusMode::Proposing,
            phase: ConsensusPhase::Open,
        }
    }

    pub fn get_mode(&self) -> ConsensusMode {
        self.mode
    }

    pub fn get_phase(&self) -> ConsensusPhase {
        self.phase
    }

    pub fn start_round(&mut self) {
        self.phase = ConsensusPhase::Open;
    }

    pub fn close_ledger(&mut self) {
        self.phase = ConsensusPhase::Establish;
    }
}
