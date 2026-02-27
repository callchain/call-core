use primitives::UInt256;

/// NodeObjectType represents the type of data stored in the NodeStore
/// Compatible with calld's NodeObjectType encoding (byte 8 of the value)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NodeObjectType {
    /// Unknown/invalid type
    Unknown = 0,
    /// Ledger header
    Ledger = 1,
    /// Account state tree node (SHAMap inner node or leaf)
    AccountNode = 3,
    /// Transaction tree node
    TransactionNode = 4,
    /// Node metadata (peer info, config, etc.)
    Metadata = 5,
}

impl NodeObjectType {
    /// Convert from u8 value
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Unknown),
            1 => Some(Self::Ledger),
            3 => Some(Self::AccountNode),
            4 => Some(Self::TransactionNode),
            5 => Some(Self::Metadata),
            _ => None,
        }
    }

    /// Get the u8 representation
    pub fn as_u8(&self) -> u8 {
        *self as u8
    }
}

/// NodeObject represents a serialized SHAMap node stored in the database
/// Format: 9-byte header + data
/// - Bytes 0-7: zeros (reserved)
/// - Byte 8: NodeObjectType
/// - Bytes 9+: serialized data
#[derive(Debug, Clone)]
pub struct NodeObject {
    pub object_type: NodeObjectType,
    pub hash: UInt256,
    pub data: Vec<u8>,
}

impl NodeObject {
    /// Create a new NodeObject
    pub fn new(object_type: NodeObjectType, hash: UInt256, data: Vec<u8>) -> Self {
        Self {
            object_type,
            hash,
            data,
        }
    }

    /// Get the serialized data (without header)
    pub fn get_data(&self) -> &[u8] {
        &self.data
    }

    /// Get the hash (key)
    pub fn get_hash(&self) -> UInt256 {
        self.hash
    }

    /// Get the object type
    pub fn get_type(&self) -> NodeObjectType {
        self.object_type
    }

    /// Encode to database format: 9-byte header + data
    pub fn encode(&self) -> Vec<u8> {
        let mut result = Vec::with_capacity(9 + self.data.len());
        // Bytes 0-7: zeros
        result.extend_from_slice(&[0u8; 8]);
        // Byte 8: type
        result.push(self.object_type.as_u8());
        // Bytes 9+: data
        result.extend_from_slice(&self.data);
        result
    }

    /// Decode from database format
    /// Takes the hash (key) and the encoded value
    pub fn decode(hash: UInt256, encoded: &[u8]) -> Option<Self> {
        if encoded.len() < 9 {
            return None;
        }

        // Bytes 0-7 must be zeros
        if &encoded[0..8] != &[0u8; 8] {
            return None;
        }

        // Byte 8: type
        let object_type = NodeObjectType::from_u8(encoded[8])?;

        // Bytes 9+: data
        let data = encoded[9..].to_vec();

        Some(Self {
            object_type,
            hash,
            data,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_object_type_roundtrip() {
        assert_eq!(NodeObjectType::from_u8(0), Some(NodeObjectType::Unknown));
        assert_eq!(NodeObjectType::from_u8(1), Some(NodeObjectType::Ledger));
        assert_eq!(NodeObjectType::from_u8(3), Some(NodeObjectType::AccountNode));
        assert_eq!(NodeObjectType::from_u8(4), Some(NodeObjectType::TransactionNode));
        assert_eq!(NodeObjectType::from_u8(255), None);
    }

    #[test]
    fn test_node_object_encode_decode() {
        let hash = UInt256::new([1u8; 32]);
        let data = vec![1, 2, 3, 4, 5];
        let obj = NodeObject::new(NodeObjectType::AccountNode, hash, data.clone());

        let encoded = obj.encode();
        assert_eq!(encoded.len(), 9 + data.len());
        assert_eq!(&encoded[0..8], &[0u8; 8]);
        assert_eq!(encoded[8], 3); // AccountNode
        assert_eq!(&encoded[9..], &data[..]);

        let decoded = NodeObject::decode(hash, &encoded).unwrap();
        assert_eq!(decoded.get_type(), NodeObjectType::AccountNode);
        assert_eq!(decoded.get_hash(), hash);
        assert_eq!(decoded.get_data(), &data[..]);
    }

    #[test]
    fn test_node_object_decode_invalid() {
        // Too short
        assert!(NodeObject::decode(UInt256::zero(), &[0u8; 5]).is_none());

        // Invalid header (non-zero in bytes 0-7)
        let invalid = [1u8, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3];
        assert!(NodeObject::decode(UInt256::zero(), &invalid).is_none());

        // Unknown type
        let unknown_type = [0u8; 8].iter().copied().chain(std::iter::once(99u8)).collect::<Vec<_>>();
        assert!(NodeObject::decode(UInt256::zero(), &unknown_type).is_none());
    }
}
