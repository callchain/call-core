//! Shared transaction signing logic
//!
//! This module provides a standalone function for signing transactions
//! that can be used by both the RPC handler and CLI tools.

use crypto::{PrivateKey, HashPrefix};
use primitives::{AccountID, UInt256, Currency};
use serialization::{Serializer, STObject, STValue};
use serialization::types::sf;

/// Result of signing a transaction
#[derive(Debug, Clone)]
pub struct SignResult {
    pub tx_blob: String,
    pub tx_json: serde_json::Value,
    pub hash: String,
}

/// Sign a transaction using the provided private key
///
/// This function contains the exact same logic as the RPC `sign` command
/// to ensure consistent signing across all interfaces.
pub fn sign_transaction_local(
    private_key: &PrivateKey,
    tx_json: &serde_json::Value,
) -> Result<SignResult, String> {
    // Get public key for SigningPubKey
    let public_key = private_key.to_public_key();
    let public_key_bytes = public_key.as_bytes();

    // Build STObject
    let mut obj = STObject::new();

    // TransactionType (required)
    let tx_type_str = tx_json.get("TransactionType")
        .and_then(|v| v.as_str())
        .ok_or("Missing TransactionType")?;

    let tx_type = match tx_type_str {
        "Payment" => 0u16,
        "AccountSet" => 3u16,
        "SetRegularKey" => 5u16,
        "NicknameSet" => 6u16,
        "OfferCreate" => 7u16,
        "OfferCancel" => 8u16,
        "SignerListSet" => 12u16,
        "IssueSet" => 16u16,
        "DepositPreauth" => 19u16,
        "TrustSet" => 20u16,
        _ => return Err(format!("Unknown transaction type: {}", tx_type_str)),
    };
    obj.insert(sf::TRANSACTION_TYPE, STValue::UInt16(tx_type));

    // Account (required)
    let account_str = tx_json.get("Account")
        .and_then(|v| v.as_str())
        .ok_or("Missing Account")?;
    let account = parse_account(account_str)?;
    obj.insert(sf::ACCOUNT, STValue::Account(account));

    // Sequence (required)
    let sequence = tx_json.get("Sequence")
        .and_then(|v| v.as_u64())
        .ok_or("Missing Sequence")?;
    obj.insert(sf::SEQUENCE, STValue::UInt32(sequence as u32));

    // Fee (required)
    let fee_str = tx_json.get("Fee")
        .and_then(|v| v.as_str())
        .ok_or("Missing Fee")?;
    let fee_drops: u64 = fee_str.parse().map_err(|_| "Invalid Fee")?;
    obj.insert(sf::FEE, STValue::Amount(serialization::types::Amount::call(fee_drops)));

    // Optional: Flags
    if let Some(flags) = tx_json.get("Flags").and_then(|v| v.as_u64()) {
        obj.insert(sf::FLAGS, STValue::UInt32(flags as u32));
    }

    // Optional: SourceTag
    if let Some(tag) = tx_json.get("SourceTag").and_then(|v| v.as_u64()) {
        obj.insert(sf::SOURCE_TAG, STValue::UInt32(tag as u32));
    }

    // Transaction-specific fields (same order as RPC handler)
    if let Some(dest_str) = tx_json.get("Destination").and_then(|v| v.as_str()) {
        let dest = parse_account(dest_str)?;
        obj.insert(sf::DESTINATION, STValue::Account(dest));
    }

    // Amount - handle both string (drops) and object (issued currency)
    if let Some(amt_val) = tx_json.get("Amount") {
        if let Some(amt_str) = amt_val.as_str() {
            let amount_drops: u64 = amt_str.parse().map_err(|_| "Invalid Amount")?;
            let amount = serialization::types::Amount::call(amount_drops);
            obj.insert(sf::AMOUNT, STValue::Amount(amount));
        }
    }

    // NicknameSet fields
    if let Some(nickname_str) = tx_json.get("Nickname").and_then(|v| v.as_str()) {
        let nickname_bytes = hex::decode(nickname_str).map_err(|_| "Invalid Nickname")?;
        if nickname_bytes.len() != 32 {
            return Err("Invalid Nickname: must be 32 bytes".to_string());
        }
        let nickname_hash = UInt256::new(nickname_bytes.try_into().unwrap());
        obj.insert(sf::NICKNAME, STValue::Hash256(nickname_hash));
    }

    // DepositPreauth fields
    if let Some(authorize_str) = tx_json.get("Authorize").and_then(|v| v.as_str()) {
        let authorize = parse_account(authorize_str)?;
        obj.insert(sf::AUTHORIZE, STValue::Account(authorize));
    }

    // SetRegularKey fields
    if let Some(regular_key_str) = tx_json.get("RegularKey").and_then(|v| v.as_str()) {
        let regular_key = parse_account(regular_key_str)?;
        obj.insert(sf::REGULAR_KEY, STValue::Account(regular_key));
    }

    // IssueSet fields
    if let Some(total_supply_val) = tx_json.get("TotalSupply") {
        if let Some(total_supply_str) = total_supply_val.as_str() {
            // Native CALL amount
            let total_supply_drops: u64 = total_supply_str.parse().map_err(|_| "Invalid TotalSupply")?;
            let total_supply = serialization::types::Amount::call(total_supply_drops);
            obj.insert(sf::TOTAL_SUPPLY, STValue::Amount(total_supply));
        } else if let Some(total_supply_obj) = total_supply_val.as_object() {
            // Issued currency
            let value = total_supply_obj.get("value").and_then(|v| v.as_str()).unwrap_or("0");
            let currency = total_supply_obj.get("currency").and_then(|v| v.as_str()).unwrap_or("USD");
            let issuer = total_supply_obj.get("issuer").and_then(|v| v.as_str()).unwrap_or("");
            let total_supply = parse_issued_amount(value, currency, issuer)?;
            obj.insert(sf::TOTAL_SUPPLY, STValue::Amount(total_supply));
        }
    }

    // OfferCreate fields - TakerPays (native)
    if let Some(taker_pays_val) = tx_json.get("TakerPays") {
        if let Some(taker_pays_str) = taker_pays_val.as_str() {
            let taker_pays_drops: u64 = taker_pays_str.parse().map_err(|_| "Invalid TakerPays")?;
            let taker_pays = serialization::types::Amount::call(taker_pays_drops);
            obj.insert(sf::TAKER_PAYS, STValue::Amount(taker_pays));
        }
    }

    // OfferCreate fields - TakerGets (native)
    if let Some(taker_gets_val) = tx_json.get("TakerGets") {
        if let Some(taker_gets_str) = taker_gets_val.as_str() {
            let taker_gets_drops: u64 = taker_gets_str.parse().map_err(|_| "Invalid TakerGets")?;
            let taker_gets = serialization::types::Amount::call(taker_gets_drops);
            obj.insert(sf::TAKER_GETS, STValue::Amount(taker_gets));
        }
    }

    // TrustSet fields - LimitAmount (native)
    if let Some(limit_amount_val) = tx_json.get("LimitAmount") {
        if let Some(limit_amount_str) = limit_amount_val.as_str() {
            let limit_amount_drops: u64 = limit_amount_str.parse().map_err(|_| "Invalid LimitAmount")?;
            let limit_amount = serialization::types::Amount::call(limit_amount_drops);
            obj.insert(sf::LIMIT_AMOUNT, STValue::Amount(limit_amount));
        }
    }

    // OfferCancel fields
    if let Some(offer_sequence) = tx_json.get("OfferSequence").and_then(|v| v.as_u64()) {
        obj.insert(sf::OFFER_SEQUENCE, STValue::UInt32(offer_sequence as u32));
    }

    // AccountSet fields
    if let Some(domain_str) = tx_json.get("Domain").and_then(|v| v.as_str()) {
        let domain_bytes = hex::decode(domain_str).map_err(|_| "Invalid Domain")?;
        obj.insert(sf::DOMAIN, STValue::VL(domain_bytes));
    }

    // Handle issued currency amounts (LimitAmount object format)
    if let Some(limit_amount_val) = tx_json.get("LimitAmount") {
        if let Some(limit_obj) = limit_amount_val.as_object() {
            let currency = limit_obj.get("currency").and_then(|v| v.as_str()).unwrap_or("USD");
            let issuer = limit_obj.get("issuer").and_then(|v| v.as_str()).unwrap_or("");
            let value = limit_obj.get("value").and_then(|v| v.as_str()).unwrap_or("0");

            let limit = parse_issued_amount(value, currency, issuer)?;
            obj.insert(sf::LIMIT_AMOUNT, STValue::Amount(limit));
        }
    }

    // TakerGets object format (issued currency)
    if let Some(taker_gets_val) = tx_json.get("TakerGets") {
        if let Some(gets_obj) = taker_gets_val.as_object() {
            let currency = gets_obj.get("currency").and_then(|v| v.as_str()).unwrap_or("USD");
            let issuer = gets_obj.get("issuer").and_then(|v| v.as_str()).unwrap_or("");
            let value = gets_obj.get("value").and_then(|v| v.as_str()).unwrap_or("0");

            let gets = parse_issued_amount(value, currency, issuer)?;
            obj.insert(sf::TAKER_GETS, STValue::Amount(gets));
        }
    }

    // Add SigningPubKey BEFORE signing
    obj.insert(sf::SIGNING_PUB_KEY, STValue::VL(public_key_bytes.to_vec()));

    // Serialize for signing
    let mut serializer = Serializer::new();
    serializer.add_object(&obj).map_err(|e| format!("Serialization error: {}", e))?;
    let serialized_tx = serializer.finish();

    // Create signing payload with HashPrefix
    let prefix = HashPrefix::TxSign.as_bytes();
    let mut sign_data = Vec::with_capacity(prefix.len() + serialized_tx.len());
    sign_data.extend_from_slice(prefix);
    sign_data.extend_from_slice(&serialized_tx);

    // Sign
    let signature = private_key.sign(&sign_data);
    let signature_bytes = signature.as_bytes();

    // Add TxnSignature
    obj.insert(sf::TXN_SIGNATURE, STValue::VL(signature_bytes.to_vec()));

    // Serialize final transaction
    let mut final_serializer = Serializer::new();
    final_serializer.add_object(&obj).map_err(|e| format!("Serialization error: {}", e))?;
    let signed_tx_bytes = final_serializer.finish();
    let tx_blob = hex::encode(&signed_tx_bytes);

    // Compute hash
    let hash = compute_tx_hash(&signed_tx_bytes);

    // Build result JSON
    let mut result_json = tx_json.clone();
    if let Some(obj) = result_json.as_object_mut() {
        obj.insert("TxnSignature".to_string(), serde_json::json!(hex::encode(signature_bytes)));
        obj.insert("SigningPubKey".to_string(), serde_json::json!(hex::encode(public_key_bytes)));
    }

    Ok(SignResult {
        tx_blob,
        tx_json: result_json,
        hash,
    })
}

/// Convert AccountID to Callchain address (base58 with checksum)
fn account_to_address(account: &AccountID) -> String {
    use sha2::{Sha256, Digest};

    // Add version byte
    let mut data = vec![0x00u8]; // ADDRESS_VERSION for Callchain
    data.extend_from_slice(account.as_bytes());

    // Calculate checksum (first 4 bytes of double SHA256)
    let hash1 = Sha256::digest(&data);
    let hash2 = Sha256::digest(&hash1);
    let checksum = &hash2[..4];

    // Append checksum
    data.extend_from_slice(checksum);

    // Base58 encode
    crypto::base58::encode(&data)
}

/// Parse account from string (base58 or hex)
pub fn parse_account(account_str: &str) -> Result<AccountID, String> {
    // Try hex (40 chars)
    if account_str.len() == 40 {
        if let Ok(bytes) = hex::decode(account_str) {
            if bytes.len() == 20 {
                return Ok(AccountID::new(bytes.try_into().unwrap()));
            }
        }
    }

    // Try base58 (starts with 'c')
    if account_str.starts_with('c') {
        if let Ok(decoded) = crypto::base58::decode(account_str) {
            if decoded.len() == 25 {
                let mut bytes = [0u8; 20];
                bytes.copy_from_slice(&decoded[1..21]);
                return Ok(AccountID::new(bytes));
            }
        }
    }

    Err(format!("Invalid account: {}", account_str))
}

/// Parse issued currency amount
fn parse_issued_amount(value: &str, currency: &str, issuer: &str) -> Result<serialization::types::Amount, String> {
    use serialization::types::Amount;

    let value_i64: i64 = value.parse().map_err(|_| "Invalid amount value")?;

    let issuer_account = if issuer.is_empty() {
        AccountID::new([0u8; 20])
    } else {
        parse_account(issuer)?
    };

    // Parse currency - supports 3-letter codes (USD), 4-letter codes (GOLD), or 40-char hex
    let currency_bytes: [u8; 20] = if currency.len() <= 20 {
        // Standard ASCII currency code (USD, EUR, GOLD, etc.)
        // Store at offset 12 to match Ripple's currency encoding
        let mut bytes = [0u8; 20];
        let start = 12;
        for (i, c) in currency.bytes().enumerate() {
            if start + i < 20 {
                bytes[start + i] = c;
            }
        }
        bytes
    } else if currency.len() == 40 {
        // Full 20-byte hex-encoded currency
        let hex_bytes = hex::decode(currency).map_err(|_| "Invalid currency hex")?;
        if hex_bytes.len() == 20 {
            hex_bytes.try_into().map_err(|_| "Invalid currency length")?
        } else {
            return Err("Invalid currency hex length".to_string());
        }
    } else {
        return Err("Invalid currency code".to_string());
    };

    let currency_obj = Currency::new(currency_bytes);

    Amount::issued(value_i64, -15, currency_obj, issuer_account)
        .ok_or_else(|| "Invalid issued amount".to_string())
}

/// Compute transaction hash
fn compute_tx_hash(tx_blob: &[u8]) -> String {
    use crypto::sha512_half;
    let hash = sha512_half(tx_blob);
    hex::encode(hash.as_bytes())
}

/// Result of verifying a transaction signature
#[derive(Debug, Clone)]
pub struct VerifyResult {
    pub valid: bool,
    pub tx_hash: String,
    pub account: String,
    pub sequence: u32,
    pub tx_type: String,
    pub error: Option<String>,
}

/// Verify a transaction signature from tx_blob
///
/// This function extracts the transaction from the blob, reconstructs
/// the signing hash, and verifies the signature.
pub fn verify_transaction_blob(tx_blob_hex: &str) -> Result<VerifyResult, String> {
    // Decode tx_blob
    let tx_blob = hex::decode(tx_blob_hex).map_err(|_| "Invalid tx_blob hex")?;

    // Parse the transaction blob to extract fields
    // The blob is a serialized STObject - we need to deserialize it
    let (account, sequence, tx_type, signing_pub_key, txn_signature) =
        parse_tx_blob_fields(&tx_blob)?;

    // Get the signing public key
    let pk_bytes = match signing_pub_key {
        Some(pk) => pk,
        None => return Ok(VerifyResult {
            valid: false,
            tx_hash: hex::encode(&crypto::sha512_half(&tx_blob).as_bytes()),
            account: account.unwrap_or_else(|| "unknown".to_string()),
            sequence: sequence.unwrap_or(0),
            tx_type: tx_type.unwrap_or_else(|| "unknown".to_string()),
            error: Some("Missing SigningPubKey".to_string()),
        }),
    };

    // Get the signature
    let sig_bytes = match txn_signature {
        Some(sig) => sig,
        None => return Ok(VerifyResult {
            valid: false,
            tx_hash: hex::encode(&crypto::sha512_half(&tx_blob).as_bytes()),
            account: account.unwrap_or_else(|| "unknown".to_string()),
            sequence: sequence.unwrap_or(0),
            tx_type: tx_type.unwrap_or_else(|| "unknown".to_string()),
            error: Some("Missing TxnSignature".to_string()),
        }),
    };

    // Determine key type from public key length
    let key_type = if pk_bytes.len() == 33 {
        crypto::KeyType::Secp256k1
    } else if pk_bytes.len() == 32 {
        crypto::KeyType::Ed25519
    } else {
        return Ok(VerifyResult {
            valid: false,
            tx_hash: hex::encode(&crypto::sha512_half(&tx_blob).as_bytes()),
            account: account.clone().unwrap_or_else(|| "unknown".to_string()),
            sequence: sequence.unwrap_or(0),
            tx_type: tx_type.clone().unwrap_or_else(|| "unknown".to_string()),
            error: Some(format!("Invalid public key length: {}", pk_bytes.len())),
        });
    };

    // Create public key
    let public_key = match crypto::PublicKey::from_bytes(key_type, &pk_bytes) {
        Some(pk) => pk,
        None => {
            return Ok(VerifyResult {
                valid: false,
                tx_hash: hex::encode(&crypto::sha512_half(&tx_blob).as_bytes()),
                account: account.clone().unwrap_or_else(|| "unknown".to_string()),
                sequence: sequence.unwrap_or(0),
                tx_type: tx_type.clone().unwrap_or_else(|| "unknown".to_string()),
                error: Some("Invalid public key".to_string()),
            })
        }
    };

    // Create signature
    let signature = crypto::Signature::new(key_type, sig_bytes);

    // Build the signing data from the tx_blob (reconstructs what was signed)
    // This returns the raw sign_data (prefix + serialized_tx_without_TxnSignature)
    let sign_data = match compute_signing_data_from_blob(&tx_blob) {
        Ok(data) => data,
        Err(e) => {
            return Ok(VerifyResult {
                valid: false,
                tx_hash: hex::encode(&crypto::sha512_half(&tx_blob).as_bytes()),
                account: account.clone().unwrap_or_else(|| "unknown".to_string()),
                sequence: sequence.unwrap_or(0),
                tx_type: tx_type.clone().unwrap_or_else(|| "unknown".to_string()),
                error: Some(format!("Failed to compute signing data: {}", e)),
            })
        }
    };

    // Verify the signature using the raw sign_data
    // The verify function will hash the data internally with SHA256
    let valid = public_key.verify(&sign_data, &signature);

    Ok(VerifyResult {
        valid,
        tx_hash: hex::encode(&crypto::sha512_half(&tx_blob).as_bytes()),
        account: account.unwrap_or_else(|| "unknown".to_string()),
        sequence: sequence.unwrap_or(0),
        tx_type: tx_type.unwrap_or_else(|| "unknown".to_string()),
        error: if valid { None } else { Some("Signature verification failed".to_string()) },
    })
}

/// Parse transaction blob to extract key fields
fn parse_tx_blob_fields(tx_blob: &[u8]) -> Result<
    (Option<String>, Option<u32>, Option<String>, Option<Vec<u8>>, Option<Vec<u8>>),
    String
> {
    use serialization::SerialIter;

    let mut iter = SerialIter::new(tx_blob);

    let mut account = None;
    let mut sequence = None;
    let mut tx_type = None;
    let mut signing_pub_key = None;
    let mut txn_signature = None;

    while !iter.eof() {
        let field_id = iter.get_field_id()
            .map_err(|e| format!("Failed to read field ID: {}", e))?;

        // Check for object end marker (type 14, field 1)
        if field_id.0 == 14 && field_id.1 == 1 {
            break;
        }

        match (field_id.0, field_id.1) {
            // TransactionType (type=1, field=2)
            (1, 2) => {
                let val = iter.get16()
                    .map_err(|e| format!("Failed to read tx type: {}", e))?;
                tx_type = Some(match val {
                    0 => "Payment",
                    3 => "AccountSet",
                    5 => "SetRegularKey",
                    6 => "NicknameSet",
                    7 => "OfferCreate",
                    8 => "OfferCancel",
                    12 => "SignerListSet",
                    16 => "IssueSet",
                    19 => "DepositPreauth",
                    20 => "TrustSet",
                    _ => "Unknown",
                }.to_string());
            }
            // Account (type=8, field=1)
            (8, 1) => {
                let acc = iter.get_account()
                    .map_err(|e| format!("Failed to read account: {}", e))?;
                // Convert AccountID to Callchain address with version and checksum
                let addr = account_to_address(&acc);
                account = Some(addr);
            }
            // Sequence (type=2, field=4)
            (2, 4) => {
                sequence = Some(iter.get32()
                    .map_err(|e| format!("Failed to read sequence: {}", e))?);
            }
            // SigningPubKey (type=7, field=3)
            (7, 3) => {
                signing_pub_key = Some(iter.get_vl()
                    .map_err(|e| format!("Failed to read signing pub key: {}", e))?);
            }
            // TxnSignature (type=7, field=4)
            (7, 4) => {
                txn_signature = Some(iter.get_vl()
                    .map_err(|e| format!("Failed to read txn signature: {}", e))?);
            }
            // Skip other fields
            _ => {
                iter.skip_field(field_id.0)
                    .map_err(|e| format!("Failed to skip field: {}", e))?;
            }
        }
    }

    Ok((account, sequence, tx_type, signing_pub_key, txn_signature))
}

/// Compute the signing data from a transaction blob
/// This parses the blob, finds and removes TxnSignature, then returns the data to be signed
/// Note: SigningPubKey IS included (only TxnSignature is excluded)
fn compute_signing_data_from_blob(tx_blob: &[u8]) -> Result<Vec<u8>, String> {
    use crypto::HashPrefix;
    use serialization::SerialIter;

    // Parse to find TxnSignature position
    let mut iter = SerialIter::new(tx_blob);
    let mut txn_signature_start: Option<usize> = None;
    let mut txn_signature_end: Option<usize> = None;

    while !iter.eof() {
        let pos = iter.position() as usize;

        let field_id = iter.get_field_id()
            .map_err(|e| format!("Failed to read field ID: {}", e))?;

        // Check for object end marker (type 14, field 1)
        if field_id.0 == 14 && field_id.1 == 1 {
            break;
        }

        // TxnSignature (type=7, field=4)
        if field_id.0 == 7 && field_id.1 == 4 {
            txn_signature_start = Some(pos);
            iter.get_vl().map_err(|e| format!("Failed to read TxnSignature: {}", e))?;
            txn_signature_end = Some(iter.position() as usize);
        } else {
            // Skip other fields (including SigningPubKey which IS part of the hash)
            iter.skip_field(field_id.0)
                .map_err(|e| format!("Failed to skip field: {}", e))?;
        }
    }

    // Build the serialized data without TxnSignature only
    let serialized = if let (Some(start), Some(end)) = (txn_signature_start, txn_signature_end) {
        // Concatenate: before TxnSignature + after TxnSignature
        let mut result = Vec::with_capacity(tx_blob.len() - (end - start));
        result.extend_from_slice(&tx_blob[..start]);
        result.extend_from_slice(&tx_blob[end..]);
        result
    } else {
        // No TxnSignature found - use whole blob
        tx_blob.to_vec()
    };

    // Create signing payload: HashPrefix + serialized tx
    let prefix = HashPrefix::TxSign.as_bytes();
    let mut sign_data = Vec::with_capacity(prefix.len() + serialized.len());
    sign_data.extend_from_slice(prefix);
    sign_data.extend_from_slice(&serialized);

    // Return the raw sign_data (not the hash)
    Ok(sign_data)
}
