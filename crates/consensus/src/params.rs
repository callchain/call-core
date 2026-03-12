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
            ledger_min_consensus_pct: 66,  // 66% for fast 3-node devnet consensus
            validation_quorum: 80,         // 80% quorum for BFT consensus
            amendment_quorum: 2,
            min_propose_time: 1,           // Propose solutions faster
            max_propose_time: 10,          // Cap proposal time
            ledger_min_close_time: 1,      // 1 second minimum - open ledgers faster
            ledger_max_close_time: 5,      // 5 seconds maximum - force close sooner
            ledger_max_tx_count: 10000,    // Max 10000 transactions - higher throughput
            ledger_max_size: 50_000_000,   // 50MB max ledger size - larger ledgers
        }
    }
}
