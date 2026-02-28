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
        let mut serializer = Serializer::with_capacity(256);

        // Add validation fields
        serializer.add256(v.node_id.0);
        serializer.add256(v.ledger_hash);
        serializer.add32(v.ledger_index as u32);
        serializer.add32(v.sign_time);
        serializer.add32(v.close_time);

        // Add optional signature (use VL encoding)
        if let Some(ref sig) = v.signature {
            serializer.add_vl(sig);
        } else {
            serializer.add_vl(&[]);
        }

        // Add optional signing public key
        if let Some(ref pk) = v.signing_pub_key {
            serializer.add_vl(pk);
        } else {
            serializer.add_vl(&[]);
        }

        let payload = serializer.finish();
        Self::new(MessageType::Validation, payload)
    }

    /// Create a proposal message
    pub fn propose(p: &Proposal) -> Self {
        // Serialize proposal
        let mut serializer = Serializer::with_capacity(256);

        // Add proposal fields
        serializer.add256(p.node_id.0);
        serializer.add256(p.previous_ledger);
        serializer.add256(p.position);
        serializer.add32(p.propose_seq);
        serializer.add32(p.close_time);

        // Add optional signature (use VL encoding)
        if let Some(ref sig) = p.signature {
            serializer.add_vl(sig);
        } else {
            serializer.add_vl(&[]);
        }

        // Add optional signing public key
        if let Some(ref pk) = p.signing_pub_key {
            serializer.add_vl(pk);
        } else {
            serializer.add_vl(&[]);
        }

        let payload = serializer.finish();
        Self::new(MessageType::Propose, payload)
    }

    /// Create a transaction message
    pub fn transaction(tx: &Transaction) -> Self {
        // Serialize transaction - uses STObject format for transaction data
        let mut serializer = Serializer::with_capacity(512);

        // Serialize transaction type
        serializer.add16(tx.tx_type.as_i16() as u16);

        // Serialize account
        serializer.add_account(tx.account);

        // Serialize sequence
        serializer.add32(tx.sequence);

        // Serialize fee
        serializer.add64(tx.fee);

        // Serialize signing public key if present
        if let Some(ref pk) = tx.signing_pub_key {
            serializer.add_vl(pk);
        } else {
            serializer.add_vl(&[]);
        }

        // Serialize transaction signature if present
        if let Some(ref sig) = tx.txn_signature {
            serializer.add_vl(sig);
        } else {
            serializer.add_vl(&[]);
        }

        // Serialize transaction hash
        serializer.add256(tx.hash);

        // Serialize transaction-specific fields based on type
        match tx.tx_type {
            protocol::TxType::Payment => {
                if let Some(dest) = tx.destination {
                    serializer.add_account(dest);
                }
                if let Some(amt) = tx.amount {
                    serializer.add_amount(amt);
                }
                if let Some(tag) = tx.destination_tag {
                    serializer.add32(tag);
                }
            }
            protocol::TxType::TrustSet => {
                if let Some(limit) = tx.limit_amount {
                    serializer.add_amount(limit);
                }
                if let Some(issuer) = tx.issuer {
                    serializer.add_account(issuer);
                }
            }
            protocol::TxType::OfferCreate => {
                if let Some(pays) = tx.taker_pays {
                    serializer.add_amount(pays);
                }
                if let Some(gets) = tx.taker_gets {
                    serializer.add_amount(gets);
                }
                serializer.add32(tx.offer_sequence);
            }
            protocol::TxType::OfferCancel => {
                serializer.add32(tx.offer_sequence);
            }
            _ => {
                // Other transaction types - basic serialization only
            }
        }

        let payload = serializer.finish();
        Self::new(MessageType::Transaction, payload)
    }

    /// Create a ping message
    pub fn ping() -> Self {
        Self::new(MessageType::Ping, vec![])
    }

    /// Create a get_ledger message
    pub fn get_ledger(ledger_index: LedgerIndex) -> Self {
        // Serialize get_ledger request
        // Format: ledger_index (4 bytes) + flags (1 byte)
        let mut payload = Vec::with_capacity(5);
        payload.extend_from_slice(&ledger_index.to_be_bytes());
        payload.push(0x00); // flags: 0 = full ledger data
        Self::new(MessageType::GetLedger, payload)
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
