use consensus::{Proposal, Validation};
use primitives::{LedgerIndex, UInt256};
use protocol::Transaction;
use serialization::Serializer;

/// Protocol message types matching calld message types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    // Handshake
    Hello = 1,
    StatusChange = 2,

    // Consensus
    Propose = 3,
    Validation = 4,

    // Transactions
    Transaction = 5,
    HaveTransactions = 6,
    GetTransactions = 7,

    // Ledger data
    GetLedger = 8,
    LedgerData = 9,
    HaveTransactionSet = 10,
    GetTransactionSet = 11,

    // Peers
    Ping = 12,
    Cluster = 13,
}

/// Network message wrapper
#[derive(Debug, Clone)]
pub struct Message {
    pub message_type: MessageType,
    pub payload: Vec<u8>,
}

impl Message {
    pub fn new(message_type: MessageType, payload: Vec<u8>) -> Self {
        Self {
            message_type,
            payload,
        }
    }

    pub fn get_type(&self) -> MessageType {
        self.message_type
    }

    pub fn get_payload(&self) -> &[u8] {
        &self.payload
    }

    /// Create a validation message
    pub fn validation(v: &Validation) -> Self {
        // Serialize validation using the serialization crate
        let mut serializer = Serializer::new();
        // Add validation fields to serializer
        // This is a simplified placeholder
        let payload = serializer.finish();
        Self::new(MessageType::Validation, payload)
    }

    /// Create a proposal message
    pub fn propose(p: &Proposal) -> Self {
        // Serialize proposal
        let mut serializer = Serializer::new();
        // Add proposal fields
        let payload = serializer.finish();
        Self::new(MessageType::Propose, payload)
    }

    /// Create a transaction message
    pub fn transaction(tx: &Transaction) -> Self {
        // Serialize transaction
        let mut serializer = Serializer::new();
        // Add transaction fields
        let payload = serializer.finish();
        Self::new(MessageType::Transaction, payload)
    }

    /// Create a ping message
    pub fn ping() -> Self {
        Self::new(MessageType::Ping, vec![])
    }
}

/// Hello message for handshake
#[derive(Debug, Clone)]
pub struct HelloMessage {
    pub protocol_version: u32,
    pub node_public_key: Vec<u8>,
    pub node_id: Vec<u8>,
    pub ledger_index: LedgerIndex,
    pub ledger_hash: UInt256,
    pub network_time: u64,
}

impl HelloMessage {
    pub fn new(protocol_version: u32, node_public_key: Vec<u8>) -> Self {
        Self {
            protocol_version,
            node_public_key,
            node_id: Vec::new(),
            ledger_index: 0,
            ledger_hash: UInt256::zero(),
            network_time: 0,
        }
    }
}

/// Status change message
#[derive(Debug, Clone)]
pub struct StatusChangeMessage {
    pub ledger_index: LedgerIndex,
    pub ledger_hash: UInt256,
    pub network_time: u64,
}

/// Have transactions message
#[derive(Debug, Clone)]
pub struct HaveTransactionsMessage {
    pub tx_hashes: Vec<UInt256>,
}

/// Get transactions message
#[derive(Debug, Clone)]
pub struct GetTransactionsMessage {
    pub tx_hashes: Vec<UInt256>,
}

/// Get ledger message
#[derive(Debug, Clone)]
pub struct GetLedgerMessage {
    pub ledger_hash: Option<UInt256>,
    pub ledger_index: Option<LedgerIndex>,
    pub query_type: QueryType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryType {
    Full,
    Transactions,
    Accounts,
}

impl GetLedgerMessage {
    pub fn by_hash(hash: UInt256) -> Self {
        Self {
            ledger_hash: Some(hash),
            ledger_index: None,
            query_type: QueryType::Full,
        }
    }

    pub fn by_index(index: LedgerIndex) -> Self {
        Self {
            ledger_hash: None,
            ledger_index: Some(index),
            query_type: QueryType::Full,
        }
    }
}

/// Ledger data message
#[derive(Debug, Clone)]
pub struct LedgerDataMessage {
    pub ledger_hash: UInt256,
    pub ledger_index: LedgerIndex,
    pub data: Vec<u8>,
}
