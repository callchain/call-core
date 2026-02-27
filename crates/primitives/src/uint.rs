use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

macro_rules! define_uint {
    ($name:ident, $n_bytes:expr) => {
        #[derive(Clone, Copy, Default, Serialize, Deserialize)]
        #[serde(transparent)]
        #[repr(transparent)]
        pub struct $name {
            bytes: [u8; $n_bytes],
        }

        impl $name {
            pub const fn new(bytes: [u8; $n_bytes]) -> Self {
                Self { bytes }
            }

            pub const fn from_be_bytes(bytes: [u8; $n_bytes]) -> Self {
                Self { bytes }
            }

            pub const fn from_le_bytes(bytes: [u8; $n_bytes]) -> Self {
                let mut be = [0u8; $n_bytes];
                let mut i = 0;
                while i < $n_bytes {
                    be[$n_bytes - 1 - i] = bytes[i];
                    i += 1;
                }
                Self { bytes: be }
            }

            pub const fn as_bytes(&self) -> &[u8; $n_bytes] {
                &self.bytes
            }

            pub fn to_hex(&self) -> String {
                hex::encode(self.bytes)
            }

            pub const fn zero() -> Self {
                Self { bytes: [0u8; $n_bytes] }
            }

            pub fn is_zero(&self) -> bool {
                self.bytes.iter().all(|&b| b == 0)
            }

            pub const fn size() -> usize {
                $n_bytes
            }
        }

        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                self.bytes == other.bytes
            }
        }

        impl Eq for $name {}

        impl PartialOrd for $name {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        impl Ord for $name {
            fn cmp(&self, other: &Self) -> Ordering {
                self.bytes.cmp(&other.bytes)
            }
        }

        impl Hash for $name {
            fn hash<H: Hasher>(&self, state: &mut H) {
                self.bytes.hash(state);
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self.to_hex())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.to_hex())
            }
        }

        impl FromStr for $name {
            type Err = hex::FromHexError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let bytes = hex::decode(s)?;
                if bytes.len() != $n_bytes {
                    return Err(hex::FromHexError::InvalidStringLength);
                }
                let mut arr = [0u8; $n_bytes];
                arr.copy_from_slice(&bytes);
                Ok(Self::new(arr))
            }
        }

        impl From<[u8; $n_bytes]> for $name {
            fn from(bytes: [u8; $n_bytes]) -> Self {
                Self::new(bytes)
            }
        }

        impl Deref for $name {
            type Target = [u8; $n_bytes];

            fn deref(&self) -> &Self::Target {
                &self.bytes
            }
        }

        impl DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.bytes
            }
        }
    };
}

define_uint!(UInt128, 16);
define_uint!(UInt160, 20);
define_uint!(UInt256, 32);

#[allow(non_camel_case_types)]
pub type uint128 = UInt128;
#[allow(non_camel_case_types)]
pub type uint160 = UInt160;
#[allow(non_camel_case_types)]
pub type uint256 = UInt256;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uint256_hex_roundtrip() {
        let original = UInt256::new([0xab; 32]);
        let hex = original.to_hex();
        let parsed: UInt256 = hex.parse().unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn test_uint256_ordering() {
        let a = UInt256::new([0x00; 32]);
        let b = UInt256::new([0xff; 32]);
        assert!(a < b);
    }

    #[test]
    fn test_uint160_zero() {
        let zero = UInt160::zero();
        assert!(zero.is_zero());
    }
}
