use crate::keys::KeyType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    pub(crate) key_type: KeyType,
    pub(crate) data: Vec<u8>,
}

impl Signature {
    pub fn new(key_type: KeyType, data: Vec<u8>) -> Self {
        Self { key_type, data }
    }

    pub fn key_type(&self) -> KeyType {
        self.key_type
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    pub fn to_hex(&self) -> String {
        hex::encode(&self.data)
    }
}
