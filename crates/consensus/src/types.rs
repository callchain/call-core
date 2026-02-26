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
