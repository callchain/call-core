pub mod uint;

pub use uint::{uint128, uint160, uint256, UInt128, UInt160, UInt256};

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AccountID(pub UInt160);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Currency(pub UInt160);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeID(pub UInt256);

pub type LedgerIndex = u32;

impl AccountID {
    pub const fn new(bytes: [u8; 20]) -> Self {
        Self(UInt160::new(bytes))
    }

    pub fn as_bytes(&self) -> &[u8; 20] {
        self.0.as_bytes()
    }

    pub fn to_hex(&self) -> String {
        self.0.to_hex()
    }
}

impl Currency {
    pub const CALL: Currency = Currency(UInt160::new([
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
    ]));

    pub const fn new(bytes: [u8; 20]) -> Self {
        Self(UInt160::new(bytes))
    }

    pub fn as_bytes(&self) -> &[u8; 20] {
        self.0.as_bytes()
    }

    pub fn is_call(&self) -> bool {
        self.0.as_bytes().iter().all(|&b| b == 0)
    }
}

impl NodeID {
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(UInt256::new(bytes))
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        self.0.as_bytes()
    }
}

impl fmt::Display for AccountID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl FromStr for AccountID {
    type Err = hex::FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = hex::decode(s)?;
        if bytes.len() != 20 {
            return Err(hex::FromHexError::InvalidStringLength);
        }
        let mut arr = [0u8; 20];
        arr.copy_from_slice(&bytes);
        Ok(Self::new(arr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_currency_call() {
        assert!(Currency::CALL.is_call());
    }

    #[test]
    fn test_account_id_hex() {
        let id = AccountID::new([0xab; 20]);
        let hex = id.to_hex();
        assert_eq!(hex, "abababababababababababababababababababab");
    }
}
