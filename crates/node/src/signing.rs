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
            let total_supply_drops: u64 = total_supply_str.parse().map_err(|_| "Invalid TotalSupply")?;
            let total_supply = serialization::types::Amount::call(total_supply_drops);
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

    // Parse currency
    let currency_bytes: [u8; 20] = if currency.len() == 3 {
        let mut bytes = [0u8; 20];
        bytes[12] = currency.as_bytes()[0];
        bytes[13] = currency.as_bytes()[1];
        bytes[14] = currency.as_bytes()[2];
        bytes
    } else if currency.len() == 40 {
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
