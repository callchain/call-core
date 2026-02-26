use primitives::UInt256;
use sha2::{Digest, Sha512};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashPrefix {
    TransactionId,
    TxNode,
    LeafNode,
    InnerNode,
    InnerNodeV2,
    LedgerMaster,
    TxSign,
    TxMultiSign,
    Validation,
    Proposal,
    Manifest,
}

impl HashPrefix {
    pub fn as_bytes(&self) -> &'static [u8; 4] {
        match self {
            Self::TransactionId => b"TXN\0",
            Self::TxNode => b"SND\0",
            Self::LeafNode => b"MLN\0",
            Self::InnerNode => b"MIN\0",
            Self::InnerNodeV2 => b"INR\0",
            Self::LedgerMaster => b"LWR\0",
            Self::TxSign => b"STX\0",
            Self::TxMultiSign => b"SMT\0",
            Self::Validation => b"VAL\0",
            Self::Proposal => b"PRP\0",
            Self::Manifest => b"MAN\0",
        }
    }

    pub fn as_u32(&self) -> u32 {
        u32::from_be_bytes(*self.as_bytes())
    }
}

pub fn sha512_half(data: &[u8]) -> UInt256 {
    let mut hasher = Sha512::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&result[..32]);
    UInt256::new(bytes)
}

pub fn sha512_half_with_prefix(prefix: HashPrefix, data: &[u8]) -> UInt256 {
    let mut hasher = Sha512::new();
    hasher.update(prefix.as_bytes());
    hasher.update(data);
    let result = hasher.finalize();
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&result[..32]);
    UInt256::new(bytes)
}

pub fn sha256(data: &[u8]) -> [u8; 32] {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&result);
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha512_half() {
        let data = b"hello world";
        let hash = sha512_half(data);
        assert!(!hash.is_zero());
    }

    #[test]
    fn test_hash_prefix_bytes() {
        assert_eq!(HashPrefix::TransactionId.as_bytes(), b"TXN\0");
        assert_eq!(HashPrefix::InnerNode.as_bytes(), b"MIN\0");
    }
}
