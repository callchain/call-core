use ed25519_dalek::{Signer, Verifier};
use primitives::AccountID;
use rand::rngs::OsRng;
use ripemd::Ripemd160;
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyType {
    Secp256k1,
    Ed25519,
}

impl KeyType {
    pub fn as_u8(&self) -> u8 {
        match self {
            Self::Secp256k1 => 0,
            Self::Ed25519 => 1,
        }
    }

    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Secp256k1),
            1 => Some(Self::Ed25519),
            _ => None,
        }
    }
}

#[derive(Clone, ZeroizeOnDrop)]
pub struct PrivateKey {
    #[zeroize(skip)]
    key_type: KeyType,
    #[zeroize(skip)]
    secret: Vec<u8>,
}

impl std::fmt::Debug for PrivateKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PrivateKey")
            .field("key_type", &self.key_type)
            .finish()
    }
}

impl PrivateKey {
    pub fn generate_secp256k1() -> Self {
        let secret_key = secp256k1::SecretKey::new(&mut OsRng);
        Self {
            key_type: KeyType::Secp256k1,
            secret: secret_key.secret_bytes().to_vec(),
        }
    }

    pub fn generate_ed25519() -> Self {
        let signing_key = ed25519_dalek::SigningKey::generate(&mut OsRng);
        Self {
            key_type: KeyType::Ed25519,
            secret: signing_key.to_bytes().to_vec(),
        }
    }

    pub fn from_bytes(key_type: KeyType, bytes: &[u8]) -> Option<Self> {
        match key_type {
            KeyType::Secp256k1 => {
                if bytes.len() != 32 {
                    return None;
                }
            }
            KeyType::Ed25519 => {
                if bytes.len() != 32 {
                    return None;
                }
            }
        }
        Some(Self {
            key_type,
            secret: bytes.to_vec(),
        })
    }

    pub fn key_type(&self) -> KeyType {
        self.key_type
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.secret
    }

    pub fn to_public_key(&self) -> PublicKey {
        match self.key_type {
            KeyType::Secp256k1 => {
                let secret_key =
                    secp256k1::SecretKey::from_slice(&self.secret).expect("valid secret key");
                let public_key = secret_key.public_key(&secp256k1::Secp256k1::new());
                let serialized = public_key.serialize();
                PublicKey {
                    key_type: KeyType::Secp256k1,
                    data: serialized.to_vec(),
                }
            }
            KeyType::Ed25519 => {
                let signing_key =
                    ed25519_dalek::SigningKey::from_bytes(&self.secret.clone().try_into().unwrap());
                let verifying_key = signing_key.verifying_key();
                PublicKey {
                    key_type: KeyType::Ed25519,
                    data: verifying_key.to_bytes().to_vec(),
                }
            }
        }
    }

    pub fn sign(&self, message: &[u8]) -> crate::signature::Signature {
        match self.key_type {
            KeyType::Secp256k1 => {
                let secp = secp256k1::Secp256k1::new();
                let secret_key =
                    secp256k1::SecretKey::from_slice(&self.secret).expect("valid secret key");
                let message_hash = Sha256::digest(message);
                let msg = secp256k1::Message::from_digest(message_hash.into());
                let sig = secp.sign_ecdsa(&msg, &secret_key);
                crate::signature::Signature {
                    key_type: KeyType::Secp256k1,
                    data: sig.serialize_der().to_vec(),
                }
            }
            KeyType::Ed25519 => {
                let signing_key =
                    ed25519_dalek::SigningKey::from_bytes(&self.secret.clone().try_into().unwrap());
                let sig = signing_key.sign(message);
                crate::signature::Signature {
                    key_type: KeyType::Ed25519,
                    data: sig.to_bytes().to_vec(),
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicKey {
    key_type: KeyType,
    data: Vec<u8>,
}

impl PublicKey {
    pub fn key_type(&self) -> KeyType {
        self.key_type
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    pub fn verify(&self, message: &[u8], signature: &crate::signature::Signature) -> bool {
        if self.key_type != signature.key_type {
            return false;
        }
        match self.key_type {
            KeyType::Secp256k1 => {
                let secp = secp256k1::Secp256k1::new();
                let public_key = match secp256k1::PublicKey::from_slice(&self.data) {
                    Ok(k) => k,
                    Err(_) => return false,
                };
                let signature = match secp256k1::ecdsa::Signature::from_der(&signature.data) {
                    Ok(s) => s,
                    Err(_) => return false,
                };
                let message_hash = Sha256::digest(message);
                let msg = secp256k1::Message::from_digest(message_hash.into());
                secp.verify_ecdsa(&msg, &signature, &public_key).is_ok()
            }
            KeyType::Ed25519 => {
                let verifying_key = match ed25519_dalek::VerifyingKey::from_bytes(
                    &self.data.clone().try_into().unwrap(),
                ) {
                    Ok(k) => k,
                    Err(_) => return false,
                };
                let signature_arr: [u8; 64] = match signature.data.clone().try_into() {
                    Ok(s) => s,
                    Err(_) => return false,
                };
                let sig = ed25519_dalek::Signature::from_bytes(&signature_arr);
                verifying_key.verify(message, &sig).is_ok()
            }
        }
    }
}

pub fn generate_account_id(public_key: &PublicKey) -> AccountID {
    let sha256_hash = Sha256::digest(public_key.as_bytes());
    let ripemd_hash = Ripemd160::digest(sha256_hash);
    AccountID::new(ripemd_hash.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secp256k1_keypair() {
        let private = PrivateKey::generate_secp256k1();
        let public = private.to_public_key();
        assert_eq!(public.key_type(), KeyType::Secp256k1);

        let message = b"test message";
        let sig = private.sign(message);
        assert!(public.verify(message, &sig));
    }

    #[test]
    fn test_ed25519_keypair() {
        let private = PrivateKey::generate_ed25519();
        let public = private.to_public_key();
        assert_eq!(public.key_type(), KeyType::Ed25519);

        let message = b"test message";
        let sig = private.sign(message);
        assert!(public.verify(message, &sig));
    }

    #[test]
    fn test_account_id_generation() {
        let private = PrivateKey::generate_secp256k1();
        let public = private.to_public_key();
        let account_id = generate_account_id(&public);
        assert!(!account_id.as_bytes().iter().all(|&b| b == 0));
    }
}
