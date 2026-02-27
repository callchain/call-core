pub struct ConsensusParms {
    pub ledger_min_consensus: usize,
    pub ledger_max_consensus: usize,
    pub ledger_min_consensus_pct: u32,
    pub validation_quorum: usize,
    pub amendment_quorum: usize,
    pub min_propose_time: u32,
    pub max_propose_time: u32,
}

impl Default for ConsensusParms {
    fn default() -> Self {
        Self {
            ledger_min_consensus: 2,
            ledger_max_consensus: 50,
            ledger_min_consensus_pct: 80,
            validation_quorum: 28,
            amendment_quorum: 28,
            min_propose_time: 3,
            max_propose_time: 30,
        }
    }
}
