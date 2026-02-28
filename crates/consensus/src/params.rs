pub struct ConsensusParms {
    pub ledger_min_consensus: usize,
    pub ledger_max_consensus: usize,
    pub ledger_min_consensus_pct: u32,
    pub validation_quorum: usize,
    pub amendment_quorum: usize,
    pub min_propose_time: u32,
    pub max_propose_time: u32,
    /// Minimum time ledger remains open before closing (in seconds)
    pub ledger_min_close_time: u32,
    /// Maximum time ledger can remain open (in seconds)
    pub ledger_max_close_time: u32,
    /// Maximum number of transactions per ledger
    pub ledger_max_tx_count: usize,
    /// Maximum ledger size in bytes
    pub ledger_max_size: usize,
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
            ledger_min_close_time: 2,  // 2 seconds minimum
            ledger_max_close_time: 20, // 20 seconds maximum
            ledger_max_tx_count: 5000, // Max 5000 transactions
            ledger_max_size: 10_000_000, // 10MB max ledger size
        }
    }
}
