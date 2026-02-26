/// Views provide read-only access to ledger data
use primitives::UInt256;

#[derive(Debug, Clone)]
pub struct LedgerView {
    pub ledger_hash: UInt256,
    pub ledger_index: u32,
}

impl LedgerView {
    pub fn new(ledger_hash: UInt256, ledger_index: u32) -> Self {
        Self {
            ledger_hash,
            ledger_index,
        }
    }
}
