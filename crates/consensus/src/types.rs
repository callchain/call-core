use crypto::{KeyType, PrivateKey, PublicKey};
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
    Accepted,
}

/// A proposal from a validator during consensus
#[derive(Debug, Clone)]
pub struct Proposal {
    pub node_id: NodeID,
    pub previous_ledger: UInt256,
    pub position: UInt256,
    pub propose_seq: u32,
    pub close_time: u32,
    pub signature: Option<Vec<u8>>,
    pub signing_pub_key: Option<Vec<u8>>,
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
            signature: None,
            signing_pub_key: None,
        }
    }

    pub fn with_signature(
        node_id: NodeID,
        previous_ledger: UInt256,
        position: UInt256,
        propose_seq: u32,
        close_time: u32,
        signing_key: &PrivateKey,
    ) -> Self {
        let mut proposal = Self::new(node_id, previous_ledger, position, propose_seq, close_time);
        proposal.sign(signing_key);
        proposal
    }

    /// Sign the proposal with a private key
    pub fn sign(&mut self, signing_key: &PrivateKey) {
        let message = self.get_signing_hash();
        let signature = signing_key.sign(&message);
        self.signature = Some(signature.as_bytes().to_vec());
        self.signing_pub_key = Some(signing_key.to_public_key().as_bytes().to_vec());
    }

    /// Verify the proposal signature
    pub fn verify_signature(&self) -> bool {
        let signature = match &self.signature {
            Some(sig) => sig,
            None => return false,
        };

        let pub_key_bytes = match &self.signing_pub_key {
            Some(pk) => pk,
            None => return false,
        };

        // Determine key type from length
        let key_type = if pub_key_bytes.len() == 33 {
            KeyType::Secp256k1
        } else if pub_key_bytes.len() == 32 {
            KeyType::Ed25519
        } else {
            return false;
        };

        let public_key = match PublicKey::from_bytes(key_type, pub_key_bytes) {
            Some(pk) => pk,
            None => return false,
        };

        let sig = crypto::Signature::new(key_type, signature.clone());
        let message = self.get_signing_hash();
        public_key.verify(&message, &sig)
    }

    /// Get the hash of proposal data for signing
    fn get_signing_hash(&self) -> Vec<u8> {
        use crypto::sha512_half;

        let mut data = Vec::new();
        data.extend_from_slice(self.node_id.as_bytes());
        data.extend_from_slice(self.previous_ledger.as_bytes());
        data.extend_from_slice(self.position.as_bytes());
        data.extend_from_slice(&self.propose_seq.to_be_bytes());
        data.extend_from_slice(&self.close_time.to_be_bytes());

        sha512_half(&data).to_vec()
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
    pub signature: Option<Vec<u8>>,
    pub signing_pub_key: Option<Vec<u8>>,
}

impl Validation {
    pub fn new(
        node_id: NodeID,
        ledger_index: LedgerIndex,
        ledger_hash: UInt256,
        close_time: u32,
    ) -> Self {
        Self {
            node_id,
            ledger_hash,
            ledger_index,
            sign_time: 0,
            close_time,
            signature: None,
            signing_pub_key: None,
        }
    }

    pub fn with_signature(
        node_id: NodeID,
        ledger_index: LedgerIndex,
        ledger_hash: UInt256,
        close_time: u32,
        signing_key: &PrivateKey,
    ) -> Self {
        let mut validation = Self::new(node_id, ledger_index, ledger_hash, close_time);
        validation.sign(signing_key);
        validation
    }

    /// Sign the validation with a private key
    pub fn sign(&mut self, signing_key: &PrivateKey) {
        let message = self.get_signing_hash();
        let signature = signing_key.sign(&message);
        self.signature = Some(signature.as_bytes().to_vec());
        self.signing_pub_key = Some(signing_key.to_public_key().as_bytes().to_vec());
    }

    /// Verify the validation signature
    pub fn verify_signature(&self) -> bool {
        let signature = match &self.signature {
            Some(sig) => sig,
            None => return false,
        };

        let pub_key_bytes = match &self.signing_pub_key {
            Some(pk) => pk,
            None => return false,
        };

        // Determine key type from length
        let key_type = if pub_key_bytes.len() == 33 {
            KeyType::Secp256k1
        } else if pub_key_bytes.len() == 32 {
            KeyType::Ed25519
        } else {
            return false;
        };

        let public_key = match PublicKey::from_bytes(key_type, pub_key_bytes) {
            Some(pk) => pk,
            None => return false,
        };

        let sig = crypto::Signature::new(key_type, signature.clone());
        let message = self.get_signing_hash();
        public_key.verify(&message, &sig)
    }

    /// Get the hash of validation data for signing
    fn get_signing_hash(&self) -> Vec<u8> {
        use crypto::sha512_half;

        let mut data = Vec::new();
        data.extend_from_slice(self.node_id.as_bytes());
        data.extend_from_slice(self.ledger_hash.as_bytes());
        data.extend_from_slice(&self.ledger_index.to_be_bytes());
        data.extend_from_slice(&self.close_time.to_be_bytes());

        sha512_half(&data).to_vec()
    }
}

/// Position of a peer during consensus
#[derive(Debug, Clone)]
pub struct PeerPosition {
    pub node_id: NodeID,
    pub proposal: Proposal,
    pub last_update: u64,
}
