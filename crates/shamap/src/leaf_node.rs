use crypto::HashPrefix;
use primitives::UInt256;
use serialization::{Serializer, SerialIter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SHAMapTreeNodeType {
    TransactionNM = 1,
    TransactionMD = 2,
    AccountState = 3,
}

impl SHAMapTreeNodeType {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(Self::TransactionNM),
            2 => Some(Self::TransactionMD),
            3 => Some(Self::AccountState),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SHAMapItem {
    key: UInt256,
    data: Vec<u8>,
}

impl SHAMapItem {
    pub fn new(key: UInt256, data: Vec<u8>) -> Self {
        Self { key, data }
    }

    pub fn key(&self) -> UInt256 {
        self.key
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn peek_data(&self) -> &[u8] {
        &self.data
    }
}

#[derive(Debug, Clone)]
pub struct SHAMapTreeNode {
    node_type: SHAMapTreeNodeType,
    item: SHAMapItem,
    hash: UInt256,
}

impl SHAMapTreeNode {
    pub fn new(node_type: SHAMapTreeNodeType, item: SHAMapItem) -> Self {
        let mut node = Self {
            node_type,
            item,
            hash: UInt256::zero(),
        };
        node.compute_hash();
        node
    }

    pub fn node_type(&self) -> SHAMapTreeNodeType {
        self.node_type
    }

    pub fn item(&self) -> &SHAMapItem {
        &self.item
    }

    pub fn hash(&self) -> UInt256 {
        self.hash
    }

    pub fn compute_hash(&mut self) {
        self.hash = match self.node_type {
            SHAMapTreeNodeType::TransactionNM => {
                let mut serializer = Serializer::new();
                serializer.add32(HashPrefix::TransactionId.as_u32());
                serializer.add_vl(&self.item.data);
                crypto::sha512_half(serializer.as_slice())
            }
            SHAMapTreeNodeType::AccountState => {
                let mut serializer = Serializer::new();
                serializer.add32(HashPrefix::LeafNode.as_u32());
                serializer.add_vl(&self.item.data);
                serializer.add256(self.item.key);
                crypto::sha512_half(serializer.as_slice())
            }
            SHAMapTreeNodeType::TransactionMD => {
                let mut serializer = Serializer::new();
                serializer.add32(HashPrefix::TxNode.as_u32());
                serializer.add_vl(&self.item.data);
                serializer.add256(self.item.key);
                crypto::sha512_half(serializer.as_slice())
            }
        };
    }

    pub fn serialize(&self) -> Vec<u8> {
        match self.node_type {
            SHAMapTreeNodeType::TransactionNM => {
                let mut serializer = Serializer::new();
                serializer.add32(HashPrefix::TransactionId.as_u32());
                serializer.add_vl(&self.item.data);
                serializer.finish()
            }
            SHAMapTreeNodeType::AccountState => {
                let mut serializer = Serializer::new();
                serializer.add32(HashPrefix::LeafNode.as_u32());
                serializer.add_vl(&self.item.data);
                serializer.add256(self.item.key);
                serializer.finish()
            }
            SHAMapTreeNodeType::TransactionMD => {
                let mut serializer = Serializer::new();
                serializer.add32(HashPrefix::TxNode.as_u32());
                serializer.add_vl(&self.item.data);
                serializer.add256(self.item.key);
                serializer.finish()
            }
        }
    }

    pub fn deserialize(data: &[u8]) -> Option<Self> {
        if data.len() < 4 {
            return None;
        }

        let mut iter = SerialIter::new(data);
        let prefix = iter.get32().ok()?;

        let node_type = match prefix {
            0x54584E00 => SHAMapTreeNodeType::TransactionNM,
            0x4D4C4E00 => SHAMapTreeNodeType::AccountState,
            0x534E4400 => SHAMapTreeNodeType::TransactionMD,
            _ => return None,
        };

        let remaining = iter.remaining();
        let mut full_data = vec![0u8; remaining];
        full_data.copy_from_slice(&data[data.len() - remaining..]);

        let key = UInt256::zero();

        let mut node = Self {
            node_type,
            item: SHAMapItem::new(key, full_data),
            hash: UInt256::zero(),
        };
        node.compute_hash();
        Some(node)
    }
}
