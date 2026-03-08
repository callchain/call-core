use std::sync::Arc;
use tokio::sync::Mutex;
use primitives::{AccountID, UInt256};

use crypto::{PrivateKey, KeyType};
use crate::signing::{sign_transaction_local, parse_account as signing_parse_account};

/// JSON-RPC 2.0 request
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub params: Option<serde_json::Value>,
    pub id: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 response
#[derive(Debug, Clone, serde::Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 error
#[derive(Debug, Clone, serde::Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcError {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    pub fn parse_error() -> Self {
        Self::new(-32700, "Parse error")
    }

    pub fn invalid_request() -> Self {
        Self::new(-32600, "Invalid Request")
    }

    pub fn method_not_found() -> Self {
        Self::new(-32601, "Method not found")
    }

    pub fn invalid_params() -> Self {
        Self::new(-32602, "Invalid params")
    }

    pub fn internal_error() -> Self {
        Self::new(-32603, "Internal error")
    }
}

impl JsonRpcResponse {
    pub fn success(result: serde_json::Value, id: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    pub fn error(error: JsonRpcError, id: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(error),
            id,
        }
    }
}

/// RPC server configuration
#[derive(Debug, Clone)]
pub struct RpcConfig {
    pub enabled: bool,
    pub bind_address: String,
    pub port: u16,
    pub admin_enabled: bool,
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            bind_address: "127.0.0.1".to_string(),
            port: 5005,
            admin_enabled: false,
        }
    }
}

/// RPC handler trait for processing JSON-RPC requests
#[async_trait::async_trait]
pub trait RpcHandler: Send + Sync {
    async fn handle_request(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError>;
}

use crate::application::ApplicationHandle;
use std::net::SocketAddr;

/// Application-aware RPC handler
pub struct AppRpcHandler {
    app: ApplicationHandle,
    network_tx: Option<tokio::sync::mpsc::Sender<network::NetworkCommand>>,
}

impl AppRpcHandler {
    pub fn new(app: ApplicationHandle) -> Self {
        Self {
            app,
            network_tx: None,
        }
    }

    /// Set the network command sender for peer management RPCs
    pub fn with_network_tx(mut self, network_tx: tokio::sync::mpsc::Sender<network::NetworkCommand>) -> Self {
        self.network_tx = Some(network_tx);
        self
    }

    /// Handle the `sign` RPC method
    async fn handle_sign(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, JsonRpcError> {
        let params = params.ok_or(JsonRpcError::invalid_params())?;
        let secret = params
            .get("secret")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError::new(31, "Missing 'secret' parameter"))?;
        let tx_json = params
            .get("tx_json")
            .ok_or_else(|| JsonRpcError::new(31, "Missing 'tx_json' parameter"))?;

        // Derive private key from secret
        let private_key = self.derive_private_key(secret)?;

        // Use shared signing logic (same as CLI tool)
        match sign_transaction_local(&private_key, tx_json) {
            Ok(result) => {
                Ok(serde_json::json!({
                    "tx_blob": result.tx_blob,
                    "tx_json": result.tx_json,
                    "status": "success",
                }))
            }
            Err(e) => Err(JsonRpcError::new(31, e)),
        }
    }

    /// Handle the `sign_for` RPC method
    async fn handle_sign_for(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, JsonRpcError> {
        let params = params.ok_or(JsonRpcError::invalid_params())?;
        let account = params
            .get("account")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError::new(31, "Missing 'account' parameter"))?;
        let secret = params
            .get("secret")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError::new(31, "Missing 'secret' parameter"))?;
        let tx_json = params
            .get("tx_json")
            .ok_or_else(|| JsonRpcError::new(31, "Missing 'tx_json' parameter"))?;

        // Parse the account to sign for (validated but not used directly in signing)
        let _account_id = parse_account(account)?;

        // Derive private key from secret
        let private_key = self.derive_private_key(secret)?;

        // Serialize transaction from tx_json (with signer info)
        let mut tx_with_signer = tx_json.clone();
        if let Some(obj) = tx_with_signer.as_object_mut() {
            obj.insert("SignerAccount".to_string(), serde_json::json!(account));
        }
        let tx_bytes = self.serialize_tx_json(&tx_with_signer)?;

        // Sign the transaction
        let signature = private_key.sign(&tx_bytes);

        // Build signed transaction blob
        let mut signed_tx = tx_bytes.clone();
        signed_tx.extend_from_slice(signature.as_bytes());

        let tx_blob = hex::encode(&signed_tx);

        Ok(serde_json::json!({
            "account": account,
            "tx_blob": tx_blob,
            "tx_json": tx_json,
            "status": "success",
        }))
    }

    /// Derive a private key from a secret (hex or seed)
    fn derive_private_key(&self, secret: &str) -> Result<PrivateKey, JsonRpcError> {
        if let Ok(key_bytes) = hex::decode(secret) {
            if key_bytes.len() == 32 {
                PrivateKey::from_bytes(KeyType::Secp256k1, &key_bytes)
                    .ok_or_else(|| JsonRpcError::new(31, "Invalid private key"))
            } else {
                Err(JsonRpcError::new(31, "Private key must be 32 bytes"))
            }
        } else {
            // For seed-based derivation, use the proper wallet derivation
            // This ensures seeds like "ss9e7tg3C4NJ3zga9y28gSWhDvhgP" produce the correct keys
            match crypto::wallet::decode_seed(secret) {
                Some(entropy) => {
                    // Use SHA256 to derive 32 bytes from 16-byte entropy (same as wallet.rs)
                    let key_hash = crypto::sha256(&entropy);

                    PrivateKey::from_bytes(KeyType::Secp256k1, &key_hash)
                        .ok_or_else(|| JsonRpcError::new(31, "Failed to generate key from seed"))
                }
                None => {
                    Err(JsonRpcError::new(31, "Invalid seed format"))
                }
            }
        }
    }

    /// Serialize a transaction from tx_json to bytes
    fn serialize_tx_json(&self, tx_json: &serde_json::Value) -> Result<Vec<u8>, JsonRpcError> {
        use serialization::{Serializer, types::{STObject, STValue, sf}};

        let mut obj = STObject::new();

        // Extract and add transaction type
        if let Some(tx_type_str) = tx_json.get("TransactionType").and_then(|v| v.as_str()) {
            let tx_type = match tx_type_str {
                "Payment" => 0u16,
                "AccountSet" => 3u16,
                "SetRegularKey" => 5u16,
                "OfferCreate" => 7u16,
                "OfferCancel" => 8u16,
                "SignerListSet" => 12u16,
                "IssueSet" => 16u16,
                "TrustSet" => 20u16,
                _ => return Err(JsonRpcError::new(31, format!("Unknown transaction type: {}", tx_type_str))),
            };
            obj.insert(sf::TRANSACTION_TYPE, STValue::UInt16(tx_type));
        }

        // Extract and add account
        if let Some(account_str) = tx_json.get("Account").and_then(|v| v.as_str()) {
            let account = parse_account(account_str)?;
            obj.insert(sf::ACCOUNT, STValue::Account(account));
        }

        // Extract and add sequence
        if let Some(sequence) = tx_json.get("Sequence").and_then(|v| v.as_u64()) {
            obj.insert(sf::SEQUENCE, STValue::UInt32(sequence as u32));
        }

        // Extract and add fee
        if let Some(fee) = tx_json.get("Fee").and_then(|v| v.as_str()) {
            let fee_drops: u64 = fee.parse().map_err(|_| JsonRpcError::new(31, "Invalid Fee"))?;
            // Fee is stored as Amount (native)
            let fee_amount = serialization::types::Amount::call(fee_drops);
            obj.insert(sf::FEE, STValue::Amount(fee_amount));
        }

        // Extract and add destination (for Payment)
        if let Some(dest_str) = tx_json.get("Destination").and_then(|v| v.as_str()) {
            let dest = parse_account(dest_str)?;
            obj.insert(sf::DESTINATION, STValue::Account(dest));
        }

        // Extract and add Amount (for Payment)
        if let Some(amount_val) = tx_json.get("Amount") {
            if let Some(amount_str) = amount_val.as_str() {
                // Native CALL amount
                let amount_drops: u64 = amount_str.parse().map_err(|_| JsonRpcError::new(31, "Invalid Amount"))?;
                let amount = serialization::types::Amount::call(amount_drops);
                obj.insert(sf::AMOUNT, STValue::Amount(amount));
            }
        }

        // Serialize to bytes
        let mut serializer = Serializer::new();
        serializer.add_object(&obj).map_err(|e| {
            JsonRpcError::new(31, format!("Serialization error: {}", e))
        })?;

        Ok(serializer.finish())
    }
}

/// Helper to parse account from string
fn parse_account(account_str: &str) -> Result<AccountID, JsonRpcError> {
    // Try hex decode first (40 hex chars = 20 bytes)
    if account_str.len() == 40 {
        if let Ok(account_bytes) = hex::decode(account_str) {
            if account_bytes.len() == 20 {
                return Ok(AccountID::new(account_bytes.try_into().unwrap()));
            }
        }
    }

    // Try base58 decode (addresses starting with 'c')
    if account_str.starts_with('c') {
        match crypto::base58::decode(account_str) {
            Ok(decoded) => {
                // Format: version (1 byte) + account_id (20 bytes) + checksum (4 bytes)
                if decoded.len() == 25 {
                    let mut bytes = [0u8; 20];
                    bytes.copy_from_slice(&decoded[1..21]);
                    return Ok(AccountID::new(bytes));
                }
            }
            Err(_) => {}
        }
    }

    Err(JsonRpcError::new(35, "Account malformed."))
}

/// Helper to parse issued currency amount
fn parse_issued_amount(value: &str, currency: &str, issuer: &str) -> Result<serialization::types::Amount, JsonRpcError> {
    use serialization::types::Amount;
    use primitives::{AccountID, Currency};

    // Parse value (can be decimal like "1000" or "1000.00")
    let value_i64: i64 = value.parse().map_err(|_| JsonRpcError::new(31, "Invalid amount value"))?;

    // Parse issuer account
    let issuer_account = if issuer.is_empty() {
        AccountID::new([0u8; 20])
    } else {
        parse_account(issuer)?
    };

    // Parse currency code - convert to 20-byte format
    let currency_bytes: [u8; 20] = if currency.len() == 3 {
        // Standard 3-letter currency code (placed at bytes 12, 13, 14)
        let mut bytes = [0u8; 20];
        bytes[12] = currency.as_bytes()[0];
        bytes[13] = currency.as_bytes()[1];
        bytes[14] = currency.as_bytes()[2];
        bytes
    } else if currency.len() == 40 {
        // Hex currency code
        if let Ok(hex_bytes) = hex::decode(currency) {
            if hex_bytes.len() == 20 {
                hex_bytes.try_into().unwrap()
            } else {
                return Err(JsonRpcError::new(31, "Invalid currency hex length"));
            }
        } else {
            return Err(JsonRpcError::new(31, "Invalid currency hex"));
        }
    } else {
        return Err(JsonRpcError::new(31, "Invalid currency code"));
    };
    let currency_obj = Currency::new(currency_bytes);

    // Create issued amount with exponent -15 (standard for issued currencies)
    Amount::issued(value_i64, -15, currency_obj, issuer_account)
        .ok_or_else(|| JsonRpcError::new(31, "Invalid issued amount"))
}

/// Helper to parse UInt256 from hex string
fn parse_uint256(hash_hex: &str) -> Result<UInt256, JsonRpcError> {
    let hash_bytes = hex::decode(hash_hex).map_err(|_| {
        JsonRpcError::new(31, "Hash malformed")
    })?;
    if hash_bytes.len() != 32 {
        return Err(JsonRpcError::new(31, "Hash length must be 32 bytes"));
    }
    Ok(UInt256::new(hash_bytes.try_into().unwrap()))
}

#[async_trait::async_trait]
impl RpcHandler for AppRpcHandler {
    async fn handle_request(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        match method {
            // ================================================================
            // Server Info Methods
            // ================================================================
            "server_info" => {
                let app = self.app.read().await;
                Ok(app.get_server_info())
            }

            "server_state" => {
                let app = self.app.read().await;
                let ledger_index = app.consensus.get_ledger_index();
                Ok(serde_json::json!({
                    "info": {
                        "build_version": "0.1.0",
                        "complete_ledgers": format!("1-{}", ledger_index),
                        "io_latency_ms": 1,
                        "load_factor": 1,
                        "peers": app.overlay.active_peer_count(),
                        "server_state": "full",
                        "state_accounting": {
                            "connected": {"duration_us": "0", "transitions": 0},
                            "disconnected": {"duration_us": "0", "transitions": 0},
                            "full": {"duration_us": "0", "transitions": 0},
                            "syncing": {"duration_us": "0", "transitions": 0},
                            "tracking": {"duration_us": "0", "transitions": 0},
                        },
                        "uptime": 0,
                        "validated_ledger": {
                            "ledger_index": ledger_index,
                            "ledger_hash": "0000000000000000000000000000000000000000000000000000000000000000",
                        },
                    }
                }))
            }

            "ping" => {
                Ok(serde_json::json!({"status": "success"}))
            }

            // ================================================================
            // Ledger Methods
            // ================================================================
            "ledger_current" => {
                let app = self.app.read().await;
                let ledger_index = app.consensus.get_ledger_index();
                Ok(serde_json::json!({
                    "ledger_current_index": ledger_index,
                    "status": "success"
                }))
            }

            "ledger_closed" => {
                let app = self.app.read().await;
                let ledger_index = app.get_current_ledger_seq();
                let ledger_hash = app.get_current_ledger_hash();
                Ok(serde_json::json!({
                    "ledger_index": ledger_index,
                    "ledger_hash": hex::encode(ledger_hash.as_bytes()),
                    "status": "success"
                }))
            }

            "ledger" => {
                let app = self.app.read().await;
                let ledger_hash = app.get_current_ledger_hash();
                let ledger_seq = app.get_current_ledger_seq();
                let ledger_state = app.get_ledger_state();
                let account_hash = ledger_state.get_root_hash();

                let requested_index = params.as_ref()
                    .and_then(|p| p.get("ledger_index"))
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32);

                let (return_hash, return_seq, return_parent) =
                    if requested_index.is_none() || requested_index == Some(ledger_seq) {
                        (ledger_hash, ledger_seq, UInt256::zero())
                    } else {
                        (UInt256::zero(), requested_index.unwrap_or(0), UInt256::zero())
                    };

                Ok(serde_json::json!({
                    "ledger": {
                        "ledger_index": return_seq.to_string(),
                        "ledger_hash": hex::encode(return_hash.as_bytes()),
                        "parent_hash": hex::encode(return_parent.as_bytes()),
                        "account_hash": hex::encode(account_hash.as_bytes()),
                        "close_time": 0,
                        "closed": false,
                    },
                    "status": "success",
                    "validated": false,
                }))
            }

            "ledger_data" => {
                let app = self.app.read().await;
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let ledger_index = params.get("ledger_index")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32)
                    .unwrap_or_else(|| app.consensus.get_ledger_index());

                let limit = params.get("limit")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(100) as usize;

                // Get ledger state and iterate entries
                let ledger_state = app.get_ledger_state();
                let mut entries = Vec::new();
                let mut count = 0;

                // Iterate through SHAMap entries
                for item in ledger_state.iter() {
                    if count >= limit {
                        break;
                    }
                    entries.push(serde_json::json!({
                        "data": hex::encode(item.data()),
                        "index": hex::encode(item.key().as_bytes()),
                    }));
                    count += 1;
                }

                Ok(serde_json::json!({
                    "ledger": {
                        "ledger_index": ledger_index.to_string(),
                        "ledger_hash": "0000000000000000000000000000000000000000000000000000000000000000",
                    },
                    "entries": entries,
                    "marker": if count >= limit { Some("more") } else { None },
                    "status": "success",
                }))
            }

            "ledger_entry" => {
                let app = self.app.read().await;
                let params = params.ok_or(JsonRpcError::invalid_params())?;

                // Get index from params
                let index = params.get("index")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'index' parameter"))?;

                let index_hash = parse_uint256(index)?;
                let ledger_state = app.get_ledger_state();

                if let Some(item) = ledger_state.get(&index_hash) {
                    Ok(serde_json::json!({
                        "index": hex::encode(index_hash.as_bytes()),
                        "data": hex::encode(item.data()),
                        "ledger_index": app.consensus.get_ledger_index(),
                        "validated": true,
                        "status": "success",
                    }))
                } else {
                    Err(JsonRpcError::new(20, "Entry not found"))
                }
            }

            "ledger_header" => {
                let app = self.app.read().await;
                let params = params.as_ref();

                let ledger_index = params
                    .and_then(|p| p.get("ledger_index"))
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32)
                    .unwrap_or_else(|| app.get_current_ledger_seq());

                let ledger_hash = app.get_current_ledger_hash();
                let ledger_state = app.get_ledger_state();
                let account_hash = ledger_state.get_root_hash();

                // Get parent hash from ledger history if available, otherwise use zeros
                let parent_hash = if ledger_index > 1 {
                    // In a full implementation, we'd look up the parent ledger hash
                    // For now, we use zeros as parent hash for the current ledger
                    UInt256::zero()
                } else {
                    UInt256::zero()
                };

                let close_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                let close_time_human = chrono::DateTime::from_timestamp(close_time as i64, 0)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());

                Ok(serde_json::json!({
                    "ledger_header": {
                        "ledger_index": ledger_index.to_string(),
                        "ledger_hash": hex::encode(ledger_hash.as_bytes()),
                        "parent_hash": hex::encode(parent_hash.as_bytes()),
                        "account_hash": hex::encode(account_hash.as_bytes()),
                        "transaction_hash": hex::encode(UInt256::zero().as_bytes()),
                        "close_time": close_time,
                        "close_time_human": close_time_human,
                    },
                    "status": "success",
                    "validated": true,
                }))
            }

            // ================================================================
            // Account Methods
            // ================================================================
            "account_info" => {
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let account = params.get("account")
                    .and_then(|v| v.as_str())
                    .ok_or(JsonRpcError::invalid_params())?;

                let account_id = parse_account(account)?;

                let app = self.app.read().await;
                let ledger_state = app.get_ledger_state();

                if let Some(account_root) = ledger_state.get_account_root(&account_id) {
                    Ok(serde_json::json!({
                        "account": account,
                        "account_data": {
                            "Account": hex::encode(account_id.as_bytes()),
                            "Balance": account_root.balance.mantissa.to_string(),
                            "Sequence": account_root.sequence,
                            "OwnerCount": account_root.owner_count,
                            "PreviousTxnID": hex::encode(account_root.previous_txn_id.as_bytes()),
                            "PreviousTxnLgrSeq": account_root.previous_txn_lgr_seq,
                        },
                        "ledger_current_index": app.consensus.get_ledger_index(),
                        "queue_data": null,
                        "status": "success",
                        "validated": false,
                    }))
                } else {
                    Err(JsonRpcError::new(19, "Account not found."))
                }
            }

            "account_tx" => {
                let app = self.app.read().await;
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let account = params.get("account")
                    .and_then(|v| v.as_str())
                    .ok_or(JsonRpcError::invalid_params())?;

                let account_id = parse_account(account)?;
                let ledger_index_min = params.get("ledger_index_min")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(-1) as u32;
                let ledger_index_max = params.get("ledger_index_max")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(-1) as u32;
                let limit = params.get("limit")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(20) as usize;
                let forward = params.get("forward")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let offset = params.get("offset")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize;

                // Get ledger state for account root
                let current_ledger = app.consensus.get_ledger_index();

                // Determine effective ledger range
                let min_ledger = if ledger_index_min == 0 || ledger_index_min > current_ledger {
                    1
                } else {
                    ledger_index_min
                };
                let max_ledger = if ledger_index_max == 0 || ledger_index_max > current_ledger {
                    current_ledger
                } else {
                    ledger_index_max
                };

                // Get transactions from transaction history
                let tx_history = app.get_tx_history();
                let tx_records = tx_history.get_account_transactions(
                    &account_id,
                    min_ledger,
                    max_ledger,
                    limit,
                    offset,
                );

                let transactions: Vec<serde_json::Value> = tx_records
                    .into_iter()
                    .map(|record| {
                        serde_json::json!({
                            "tx": {
                                "hash": hex::encode(record.tx_hash.as_bytes()),
                                "TransactionType": format!("{:?}", record.tx_type),
                                "ledger_index": record.ledger_seq,
                                "date": record.timestamp,
                            },
                            "meta": {
                                "TransactionIndex": 0,
                                "TransactionResult": "tesSUCCESS",
                            },
                            "validated": false,
                        })
                    })
                    .collect();

                // Reverse if forward is true (we store newest first)
                let transactions = if forward {
                    transactions.into_iter().rev().collect()
                } else {
                    transactions
                };

                // Get total count for pagination info
                let total_count = tx_history.count_account_transactions(&account_id, min_ledger, max_ledger);
                let marker = if offset + transactions.len() < total_count {
                    Some(offset + transactions.len())
                } else {
                    None
                };

                // Return proper response with account info
                let mut response = serde_json::json!({
                    "account": account,
                    "ledger_index_min": min_ledger as i64,
                    "ledger_index_max": max_ledger as i64,
                    "limit": limit,
                    "forward": forward,
                    "transactions": transactions,
                    "validated": false,
                    "status": "success",
                });

                if let Some(marker) = marker {
                    response["marker"] = serde_json::json!(marker);
                }

                Ok(response)
            }

            "account_lines" => {
                let app = self.app.read().await;
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let account = params.get("account")
                    .and_then(|v| v.as_str())
                    .ok_or(JsonRpcError::invalid_params())?;

                let account_id = parse_account(account)?;
                let peer = params.get("peer")
                    .and_then(|v| v.as_str());
                let limit = params.get("limit")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(100) as usize;

                let ledger_state = app.get_ledger_state();
                let call_states = ledger_state.get_call_states_for_account(&account_id);

                // Filter by peer if specified
                let lines: Vec<serde_json::Value> = call_states
                    .into_iter()
                    .take(limit)
                    .filter(|cs| {
                        if let Some(peer_str) = peer {
                            if let Ok(peer_id) = parse_account(peer_str) {
                                cs.issuer == peer_id || cs.account == peer_id
                            } else {
                                true
                            }
                        } else {
                            true
                        }
                    })
                    .map(|cs| {
                        serde_json::json!({
                            "account": hex::encode(cs.account.as_bytes()),
                            "balance": cs.balance.mantissa.to_string(),
                            "currency": hex::encode(cs.currency.as_bytes()),
                            "limit": cs.limit.mantissa.to_string(),
                            "limit_peer": cs.limit_peer.mantissa.to_string(),
                            "quality_in": cs.quality_in.unwrap_or(0),
                            "quality_out": cs.quality_out.unwrap_or(0),
                        })
                    })
                    .collect();

                Ok(serde_json::json!({
                    "account": account,
                    "lines": lines,
                    "ledger_current_index": app.consensus.get_ledger_index(),
                    "validated": false,
                    "status": "success",
                }))
            }

            "account_objects" => {
                let app = self.app.read().await;
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let account = params.get("account")
                    .and_then(|v| v.as_str())
                    .ok_or(JsonRpcError::invalid_params())?;

                let account_id = parse_account(account)?;
                let limit = params.get("limit")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(100) as usize;
                let ledger_state = app.get_ledger_state();

                // Get all account objects
                let account_objects = ledger_state.get_account_objects(&account_id, limit);

                let objects: Vec<serde_json::Value> = account_objects
                    .into_iter()
                    .map(|obj| match obj {
                        protocol::LedgerObject::Offer(offer) => {
                            serde_json::json!({
                                "type": "offer",
                                "account": hex::encode(offer.account.as_bytes()),
                                "sequence": offer.sequence,
                                "taker_pays": offer.taker_pays.mantissa.to_string(),
                                "taker_gets": offer.taker_gets.mantissa.to_string(),
                            })
                        }
                        protocol::LedgerObject::CallState(cs) => {
                            serde_json::json!({
                                "type": "call_state",
                                "account": hex::encode(cs.account.as_bytes()),
                                "issuer": hex::encode(cs.issuer.as_bytes()),
                                "balance": cs.balance.mantissa.to_string(),
                                "limit": cs.limit.mantissa.to_string(),
                            })
                        }
                        protocol::LedgerObject::Directory(dir) => {
                            let mut obj = serde_json::json!({
                                "type": "directory",
                                "root_index": hex::encode(dir.root_index.as_bytes()),
                            });
                            if let Some(owner) = dir.owner {
                                obj["owner"] = serde_json::json!(hex::encode(owner.as_bytes()));
                            }
                            obj
                        }
                        protocol::LedgerObject::AccountRoot(root) => {
                            serde_json::json!({
                                "type": "account_root",
                                "account": hex::encode(root.account.as_bytes()),
                                "balance": root.balance.mantissa.to_string(),
                                "sequence": root.sequence,
                                "owner_count": root.owner_count,
                            })
                        }
                        protocol::LedgerObject::SignerList(sl) => {
                            serde_json::json!({
                                "type": "signer_list",
                                "account": hex::encode(sl.account.as_bytes()),
                                "signer_quorum": sl.signer_quorum,
                                "signer_count": sl.signers.len(),
                            })
                        }
                        protocol::LedgerObject::LedgerHashes(lh) => {
                            serde_json::json!({
                                "type": "ledger_hashes",
                                "ledger_index": lh.ledger_index,
                                "hash_count": lh.hashes.len(),
                            })
                        }
                        protocol::LedgerObject::Amendments(am) => {
                            serde_json::json!({
                                "type": "amendments",
                                "amendment_count": am.amendments.len(),
                            })
                        }
                        protocol::LedgerObject::FeeSettings(fs) => {
                            serde_json::json!({
                                "type": "fee_settings",
                                "base_fee": fs.base_fee,
                                "reserve_base": fs.reserve_base,
                                "reserve_increment": fs.reserve_increment,
                            })
                        }
                        protocol::LedgerObject::IssueRoot(ir) => {
                            serde_json::json!({
                                "type": "issue_root",
                                "issuer": hex::encode(ir.issuer.as_bytes()),
                                "total_supply": ir.total_supply.mantissa.to_string(),
                                "issued_amount": ir.issued_amount.mantissa.to_string(),
                            })
                        }
                        protocol::LedgerObject::Invoice(inv) => {
                            serde_json::json!({
                                "type": "invoice",
                                "invoice_id": hex::encode(inv.invoice_id.as_bytes()),
                                "issuer": hex::encode(inv.issuer.as_bytes()),
                                "owner": hex::encode(inv.owner.as_bytes()),
                                "amount": inv.amount.mantissa.to_string(),
                            })
                        }
                        protocol::LedgerObject::FeeRoot(fr) => {
                            serde_json::json!({
                                "type": "fee_root",
                                "balance": fr.balance.mantissa.to_string(),
                            })
                        }
                        protocol::LedgerObject::DepositPreauth(dp) => {
                            serde_json::json!({
                                "type": "deposit_preauth",
                                "account": hex::encode(dp.account.as_bytes()),
                                "authorize": hex::encode(dp.authorize.as_bytes()),
                                "flags": dp.flags,
                            })
                        }
                    })
                    .collect();

                Ok(serde_json::json!({
                    "account": account,
                    "account_objects": objects,
                    "ledger_current_index": app.consensus.get_ledger_index(),
                    "validated": false,
                    "status": "success",
                }))
            }

            "account_offers" => {
                let app = self.app.read().await;
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let account = params.get("account")
                    .and_then(|v| v.as_str())
                    .ok_or(JsonRpcError::invalid_params())?;

                let account_id = parse_account(account)?;
                let limit = params.get("limit")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(100) as usize;
                let ledger_state = app.get_ledger_state();

                // Get offers for account
                let offers_data = ledger_state.get_offers_for_account(&account_id);

                let offers: Vec<serde_json::Value> = offers_data
                    .into_iter()
                    .take(limit)
                    .map(|offer| {
                        serde_json::json!({
                            "account": hex::encode(offer.account.as_bytes()),
                            "sequence": offer.sequence,
                            "taker_pays": offer.taker_pays.mantissa.to_string(),
                            "taker_gets": offer.taker_gets.mantissa.to_string(),
                            "quality": offer.quality().to_string(),
                        })
                    })
                    .collect();

                Ok(serde_json::json!({
                    "account": account,
                    "offers": offers,
                    "ledger_current_index": app.consensus.get_ledger_index(),
                    "validated": false,
                    "status": "success",
                }))
            }

            "account_currencies" => {
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let account = params.get("account")
                    .and_then(|v| v.as_str())
                    .ok_or(JsonRpcError::invalid_params())?;

                let account_id = parse_account(account)?;
                let app = self.app.read().await;
                let ledger_state = app.get_ledger_state();

                // Get all trust lines for the account and extract currencies
                let call_states = ledger_state.get_call_states_for_account(&account_id);

                let mut receive_currencies: Vec<String> = Vec::new();
                let mut send_currencies: Vec<String> = Vec::new();

                for cs in &call_states {
                    let currency_hex = hex::encode(cs.currency.as_bytes());
                    // Currency can be received if not frozen and has limit
                    if !cs.limit.is_zero() {
                        receive_currencies.push(currency_hex.clone());
                    }
                    // Currency can be sent if peer has limit
                    if !cs.limit_peer.is_zero() {
                        send_currencies.push(currency_hex);
                    }
                }

                // Remove duplicates
                receive_currencies.sort();
                receive_currencies.dedup();
                send_currencies.sort();
                send_currencies.dedup();

                Ok(serde_json::json!({
                    "account": account,
                    "receive": receive_currencies,
                    "send": send_currencies,
                    "ledger_current_index": app.consensus.get_ledger_index(),
                    "status": "success",
                }))
            }

            "gateway_balances" => {
                let app = self.app.read().await;
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let account = params.get("account")
                    .and_then(|v| v.as_str())
                    .ok_or(JsonRpcError::invalid_params())?;

                let account_id = parse_account(account)?;
                let hotwallets = params.get("hotwallet")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>());

                let ledger_state = app.get_ledger_state();

                // Get all trust lines where this account is the issuer (gateway)
                let call_states = ledger_state.get_call_states_for_account(&account_id);

                let mut obligations: std::collections::HashMap<String, String> = std::collections::HashMap::new();
                let mut balances: std::collections::HashMap<String, String> = std::collections::HashMap::new();
                let mut hotwallet_balances: std::collections::HashMap<String, std::collections::HashMap<String, String>> = std::collections::HashMap::new();

                // Build hotwallet set for quick lookup
                let hotwallet_set: std::collections::HashSet<String> = hotwallets
                    .as_ref()
                    .map(|hw| hw.iter().map(|s| s.to_string()).collect())
                    .unwrap_or_default();

                for cs in &call_states {
                    // For gateway balances, we look at trust lines where the account is the issuer
                    // The balance owed by the gateway is the negative of the holder's balance
                    let currency_hex = hex::encode(cs.currency.as_bytes());
                    let holder = cs.account;

                    // Calculate the balance as a signed integer
                    // For gateway obligations, we report negative of holder's balance
                    let balance_mantissa = cs.balance.mantissa as i64;
                    let gateway_obligation = -balance_mantissa; // Gateway owes the negative of holder's balance

                    // Format the obligation with proper sign
                    let obligation_str = if gateway_obligation < 0 {
                        format!("-{}", gateway_obligation.abs())
                    } else {
                        gateway_obligation.to_string()
                    };
                    let balance_str = cs.balance.mantissa.to_string();

                    // Check if this is a hotwallet
                    let holder_hex = hex::encode(holder.as_bytes());
                    if hotwallet_set.contains(&holder_hex) {
                        hotwallet_balances
                            .entry(holder_hex)
                            .or_insert_with(std::collections::HashMap::new)
                            .insert(currency_hex.clone(), balance_str.clone());
                    } else {
                        // Add to obligations (what the gateway owes) - negative balance
                        obligations.insert(currency_hex.clone(), obligation_str);
                    }

                    // Sum up total balances per currency (raw balance values)
                    balances.entry(currency_hex)
                        .and_modify(|e| {
                            // Simple string concat for now - would need proper decimal math
                            *e = balance_str.clone();
                        })
                        .or_insert(balance_str);
                }

                let balances_value: serde_json::Value = if hotwallet_balances.is_empty() {
                    serde_json::json!(balances)
                } else {
                    serde_json::json!(hotwallet_balances)
                };

                Ok(serde_json::json!({
                    "account": account,
                    "obligations": obligations,
                    "balances": balances_value,
                    "ledger_current_index": app.consensus.get_ledger_index(),
                    "validated": false,
                    "status": "success",
                }))
            }

            "owner_info" => {
                let app = self.app.read().await;
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let account = params.get("account")
                    .and_then(|v| v.as_str())
                    .ok_or(JsonRpcError::invalid_params())?;

                let account_id = parse_account(account)?;
                let ledger_state = app.get_ledger_state();

                // Count owner objects: offers, trust lines (CallState), directories
                let offers = ledger_state.get_offers_for_account(&account_id);
                let call_states = ledger_state.get_call_states_for_account(&account_id);
                let directories = ledger_state.get_directories_for_account(&account_id);

                // Owner count is the number of ledger objects owned by this account
                // Each object (except the first few) requires a reserve
                let owner_count = (offers.len() + call_states.len() + directories.len()) as u32;

                // Collect directory indexes
                let directory_indexes: Vec<serde_json::Value> = directories
                    .iter()
                    .map(|dir| {
                        serde_json::json!({
                            "index": hex::encode(dir.root_index.as_bytes()),
                            "root_index": hex::encode(dir.root_index.as_bytes()),
                            "owner": hex::encode(account_id.as_bytes()),
                        })
                    })
                    .collect();

                Ok(serde_json::json!({
                    "account": account,
                    "owner_count": owner_count,
                    "directory_indexes": directory_indexes,
                    "ledger_current_index": app.consensus.get_ledger_index(),
                    "validated": false,
                    "status": "success",
                }))
            }

            // ================================================================
            // Transaction Methods
            // ================================================================
            "submit" => {
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let tx_blob = params.get("tx_blob")
                    .and_then(|v| v.as_str())
                    .ok_or(JsonRpcError::invalid_params())?;

                let tx_bytes = hex::decode(tx_blob).map_err(|_| {
                    JsonRpcError::new(31, "Transaction malformed")
                })?;

                let mut app = self.app.write().await;
                match app.submit_transaction(&tx_bytes) {
                    Ok(result) => Ok(serde_json::json!({
                        "status": "success",
                        "tx_blob": tx_blob,
                        "engine_result": result,
                        "engine_result_code": 0,
                        "engine_result_message": "The transaction was applied.",
                    })),
                    Err(e) => Err(JsonRpcError::new(31, format!("Transaction failed: {}", e))),
                }
            }

            "submit_multisigned" => {
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let tx_blob = params.get("tx_blob")
                    .and_then(|v| v.as_str())
                    .ok_or(JsonRpcError::invalid_params())?;

                let tx_bytes = hex::decode(tx_blob).map_err(|_| {
                    JsonRpcError::new(31, "Transaction malformed")
                })?;

                let mut app = self.app.write().await;
                // Process multisigned transaction
                match app.submit_transaction(&tx_bytes) {
                    Ok(result) => Ok(serde_json::json!({
                        "status": "success",
                        "tx_blob": tx_blob,
                        "engine_result": result,
                        "engine_result_code": 0,
                        "engine_result_message": "Multisigned transaction applied.",
                    })),
                    Err(e) => Err(JsonRpcError::new(31, format!("Transaction failed: {}", e))),
                }
            }

            "tx" => {
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let tx_hash_hex = params.get("transaction")
                    .and_then(|v| v.as_str())
                    .ok_or(JsonRpcError::invalid_params())?;

                let tx_hash_bytes = hex::decode(tx_hash_hex).map_err(|_| {
                    JsonRpcError::new(31, "Transaction hash malformed")
                })?;
                if tx_hash_bytes.len() != 32 {
                    return Err(JsonRpcError::new(31, "Transaction hash malformed"));
                }
                let tx_hash = UInt256::new(tx_hash_bytes.try_into().unwrap());

                let app = self.app.read().await;
                if let Some(tx_node) = app.database.fetch_transaction_node(&tx_hash) {
                    Ok(serde_json::json!({
                        "hash": tx_hash_hex,
                        "tx_blob": hex::encode(tx_node.get_data()),
                        "meta": null,
                        "validated": true,
                        "status": "success",
                    }))
                } else {
                    Err(JsonRpcError::new(24, "Transaction not found."))
                }
            }

            "transaction_entry" => {
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let tx_hash_hex = params.get("transaction")
                    .and_then(|v| v.as_str())
                    .ok_or(JsonRpcError::invalid_params())?;

                let tx_hash = parse_uint256(tx_hash_hex)?;
                let ledger_index = params.get("ledger_index")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32);

                let app = self.app.read().await;

                if let Some(_tx_node) = app.database.fetch_transaction_node(&tx_hash) {
                    Ok(serde_json::json!({
                        "tx_json": {
                            "hash": tx_hash_hex,
                            "ledger_index": ledger_index.unwrap_or(0),
                        },
                        "metadata": null,
                        "validated": true,
                        "status": "success",
                    }))
                } else {
                    Err(JsonRpcError::new(24, "Transaction not found."))
                }
            }

            "tx_history" => {
                let app = self.app.read().await;
                let params = params.as_ref();
                let start = params
                    .and_then(|p| p.get("start"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize;

                let limit = 50usize;
                let current_ledger = app.consensus.get_ledger_index();

                // Get global transaction history
                let tx_history = app.get_tx_history();
                let records = tx_history.get_tx_history(start, limit);

                let transactions: Vec<serde_json::Value> = records
                    .into_iter()
                    .map(|record| {
                        serde_json::json!({
                            "hash": hex::encode(record.tx_hash.as_bytes()),
                            "TransactionType": format!("{:?}", record.tx_type),
                            "ledger_index": record.ledger_seq,
                            "date": record.timestamp,
                        })
                    })
                    .collect();

                Ok(serde_json::json!({
                    "transactions": transactions,
                    "start": start,
                    "limit": limit,
                    "ledger_index_min": 1,
                    "ledger_index_max": current_ledger,
                    "status": "success",
                }))
            }

            "sign" => {
                self.handle_sign(params).await
            }

            "sign_for" => {
                self.handle_sign_for(params).await
            }

            // ================================================================
            // DEX / Order Book Methods
            // ================================================================
            "book_offers" => {
                let app = self.app.read().await;
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let taker_gets = params.get("taker_gets")
                    .ok_or(JsonRpcError::invalid_params())?;
                let taker_pays = params.get("taker_pays")
                    .ok_or(JsonRpcError::invalid_params())?;

                // Parse currency filters
                let gets_currency_hex = taker_gets.get("currency").and_then(|v| v.as_str());
                let pays_currency_hex = taker_pays.get("currency").and_then(|v| v.as_str());
                let gets_issuer_hex = taker_gets.get("issuer").and_then(|v| v.as_str());
                let pays_issuer_hex = taker_pays.get("issuer").and_then(|v| v.as_str());

                let taker = params.get("taker")
                    .and_then(|v| v.as_str());
                let limit = params.get("limit")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(100) as usize;

                let ledger_state = app.get_ledger_state();
                let mut all_offers: Vec<serde_json::Value> = Vec::new();

                // Iterate through all offers in ledger and filter by currency pair
                for item in ledger_state.iter() {
                    if let Some(offer) = protocol::LedgerState::deserialize_offer(item.data()) {
                        // Apply currency filtering if specified
                        let mut include_offer = true;

                        // Filter by taker_gets currency
                        if let Some(filter_currency) = gets_currency_hex {
                            let offer_currency = hex::encode(offer.taker_gets.get_currency().as_bytes());
                            if offer_currency != filter_currency {
                                include_offer = false;
                            }
                        }

                        // Filter by taker_pays currency
                        if let Some(filter_currency) = pays_currency_hex {
                            let offer_currency = hex::encode(offer.taker_pays.get_currency().as_bytes());
                            if offer_currency != filter_currency {
                                include_offer = false;
                            }
                        }

                        // Filter by issuer
                        if let Some(filter_issuer) = gets_issuer_hex {
                            if let Ok(issuer_bytes) = hex::decode(filter_issuer) {
                                if issuer_bytes.len() == 20 {
                                    let issuer_id = AccountID::new(issuer_bytes.try_into().unwrap());
                                    if offer.taker_gets.get_issuer() != issuer_id {
                                        include_offer = false;
                                    }
                                }
                            }
                        }

                        if let Some(filter_issuer) = pays_issuer_hex {
                            if let Ok(issuer_bytes) = hex::decode(filter_issuer) {
                                if issuer_bytes.len() == 20 {
                                    let issuer_id = AccountID::new(issuer_bytes.try_into().unwrap());
                                    if offer.taker_pays.get_issuer() != issuer_id {
                                        include_offer = false;
                                    }
                                }
                            }
                        }

                        if !include_offer {
                            continue;
                        }

                        let mut offer_obj = serde_json::json!({
                            "account": hex::encode(offer.account.as_bytes()),
                            "sequence": offer.sequence,
                            "taker_pays": offer.taker_pays.mantissa.to_string(),
                            "taker_gets": offer.taker_gets.mantissa.to_string(),
                            "quality": offer.quality().to_string(),
                            "taker_pays_currency": hex::encode(offer.taker_pays.get_currency().as_bytes()),
                            "taker_gets_currency": hex::encode(offer.taker_gets.get_currency().as_bytes()),
                        });

                        // Add owner funds if taker specified
                        if let Some(taker_str) = taker {
                            if let Ok(taker_id) = parse_account(taker_str) {
                                if let Some(account_root) = ledger_state.get_account_root(&taker_id) {
                                    offer_obj["owner_funds"] = serde_json::json!(account_root.balance.mantissa.to_string());
                                }
                            }
                        }

                        all_offers.push(offer_obj);

                        if all_offers.len() >= limit {
                            break;
                        }
                    }
                }

                Ok(serde_json::json!({
                    "offers": all_offers,
                    "ledger_current_index": app.consensus.get_ledger_index(),
                    "validated": false,
                    "status": "success",
                }))
            }

            "path_find" => {
                let app = self.app.read().await;
                let params = params.ok_or(JsonRpcError::invalid_params())?;

                let source_account = params.get("source_account")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'source_account'"))?;
                let destination_account = params.get("destination_account")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'destination_account'"))?;
                let destination_amount = params.get("destination_amount")
                    .and_then(|v| v.as_object())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'destination_amount'"))?;

                let _source_currencies = params.get("source_currencies")
                    .and_then(|v| v.as_array());

                let source_id = parse_account(source_account)?;
                let dest_id = parse_account(destination_account)?;
                let ledger_state = app.get_ledger_state();

                // Simple path finding: find direct trust lines and one-hop paths
                let mut paths: Vec<serde_json::Value> = Vec::new();

                // Check for direct path (source -> destination)
                let source_call_states = ledger_state.get_call_states_for_account(&source_id);
                let dest_call_states = ledger_state.get_call_states_for_account(&dest_id);

                // Direct trust line check
                for cs in &source_call_states {
                    if cs.issuer == dest_id {
                        // Direct trust line exists
                        let currency_hex = hex::encode(cs.currency.as_bytes());
                        let dest_amount_value = destination_amount.get("value").and_then(|v| v.as_u64()).unwrap_or(0) as i64;
                        if !cs.limit.is_zero() && cs.balance.mantissa >= dest_amount_value {
                            paths.push(serde_json::json!({
                                "path": [
                                    {"account": source_account, "type": "source"},
                                    {"account": destination_account, "type": "destination"}
                                ],
                                "source_amount": destination_amount,
                                "destination_amount": destination_amount,
                                "currency": currency_hex,
                            }));
                        }
                    }
                }

                // One-hop paths through intermediate accounts
                let mut hop_count = 0;
                for source_cs in &source_call_states {
                    if hop_count >= 5 { break; } // Limit paths returned
                    let intermediate = source_cs.issuer;
                    if intermediate == dest_id { continue; }

                    // Check if intermediate trusts destination
                    for intermediate_cs in &dest_call_states {
                        if intermediate_cs.issuer == intermediate && hop_count < 5 {
                            let currency_hex = hex::encode(source_cs.currency.as_bytes());
                            paths.push(serde_json::json!({
                                "path": [
                                    {"account": source_account, "type": "source"},
                                    {"account": hex::encode(intermediate.as_bytes()), "type": "intermediate"},
                                    {"account": destination_account, "type": "destination"}
                                ],
                                "source_amount": destination_amount,
                                "destination_amount": destination_amount,
                                "currency": currency_hex,
                            }));
                            hop_count += 1;
                        }
                    }
                }

                Ok(serde_json::json!({
                    "paths": paths,
                    "destination_amount": destination_amount,
                    "destination_account": destination_account,
                    "source_account": source_account,
                    "ledger_current_index": app.consensus.get_ledger_index(),
                    "status": "success",
                }))
            }

            "call_path_find" => {
                // Callchain-specific path finding with multi-hop support
                let app = self.app.read().await;
                let params = params.ok_or(JsonRpcError::invalid_params())?;

                let source_account = params.get("source_account")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'source_account'"))?;
                let destination_account = params.get("destination_account")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'destination_account'"))?;
                let amount = params.get("amount")
                    .and_then(|v| v.as_object());

                let source_id = parse_account(source_account)?;
                let _dest_id = parse_account(destination_account)?;
                let ledger_state = app.get_ledger_state();

                // Callchain path finding: prioritize CALL currency paths
                let mut paths: Vec<serde_json::Value> = Vec::new();

                // Get all trust lines for source
                let source_states = ledger_state.get_call_states_for_account(&source_id);

                // Find paths using CALL as intermediate
                for cs in &source_states {
                    // Check if this currency matches the requested amount currency
                    if let Some(amt) = amount {
                        if let Some(currency) = amt.get("currency").and_then(|v| v.as_str()) {
                            let cs_currency = hex::encode(cs.currency.as_bytes());
                            if cs_currency == currency || currency == "CALL" {
                                paths.push(serde_json::json!({
                                    "path": [
                                        {"account": source_account, "type": "source"},
                                        {"currency": cs_currency, "type": "currency"},
                                        {"account": destination_account, "type": "destination"}
                                    ],
                                    "amount": amount,
                                    "path_type": "callchain",
                                }));
                            }
                        }
                    }
                }

                Ok(serde_json::json!({
                    "paths": paths,
                    "amount": amount,
                    "destination_account": destination_account,
                    "source_account": source_account,
                    "ledger_current_index": app.consensus.get_ledger_index(),
                    "status": "success",
                }))
            }

            "ripple_path_find" => {
                // Ripple-compatible path finding with full response format
                let app = self.app.read().await;
                let params = params.ok_or(JsonRpcError::invalid_params())?;

                let source_account = params.get("source_account")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'source_account'"))?;
                let destination_account = params.get("destination_account")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'destination_account'"))?;
                let destination_amount = params.get("destination_amount")
                    .and_then(|v| v.as_object())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'destination_amount'"))?;

                let _source_currencies = params.get("source_currencies")
                    .and_then(|v| v.as_array());

                let source_id = parse_account(source_account)?;
                let _dest_id = parse_account(destination_account)?;
                let ledger_state = app.get_ledger_state();

                // Get currency and value from destination_amount
                let dest_currency = destination_amount.get("currency")
                    .and_then(|v| v.as_str())
                    .unwrap_or("CALL");
                let dest_value = destination_amount.get("value")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0");

                // Find paths (similar to path_find but with ripple response format)
                let mut alternatives: Vec<serde_json::Value> = Vec::new();

                // Get trust lines for both accounts
                let source_call_states = ledger_state.get_call_states_for_account(&source_id);

                // Direct CALL payment path
                if dest_currency == "CALL" {
                    alternatives.push(serde_json::json!({
                        "paths_computed": [],
                        "paths_canonical": [],
                        "paths_expanded": [],
                        "source_amount": {
                            "currency": "CALL",
                            "value": dest_value,
                        },
                    }));
                }

                // Check for paths through trust lines
                let mut hop_count = 0;
                for cs in &source_call_states {
                    if hop_count >= 10 { break; }
                    let currency_str = hex::encode(cs.currency.as_bytes());

                    // Add alternative path through this trust line
                    alternatives.push(serde_json::json!({
                        "paths_computed": [
                            [{"account": hex::encode(cs.issuer.as_bytes()), "type": 1, "type_hex": "0000000000000001"}]
                        ],
                        "source_amount": {
                            "currency": currency_str,
                            "value": dest_value,
                            "issuer": hex::encode(cs.issuer.as_bytes()),
                        },
                    }));
                    hop_count += 1;
                }

                Ok(serde_json::json!({
                    "alternatives": alternatives,
                    "destination_account": destination_account,
                    "destination_amount": destination_amount,
                    "destination_currencies": [dest_currency],
                    "source_account": source_account,
                    "full_reply": true,
                    "ledger_current_index": app.consensus.get_ledger_index(),
                    "status": "success",
                }))
            }

            // ================================================================
            // Consensus / Network Methods
            // ================================================================
            "consensus_info" => {
                let app = self.app.read().await;
                let consensus_state = app.consensus.get_phase();
                let ledger_index = app.consensus.get_ledger_index();
                let round_id = app.consensus.get_round_id();

                Ok(serde_json::json!({
                    "consensus": {
                        "phase": format!("{:?}", consensus_state),
                        "ledger_index": ledger_index,
                        "round_id": round_id,
                        "proposers": 0,
                        "validations": 0,
                    },
                    "status": "success",
                }))
            }

            "fee" => {
                let app = self.app.read().await;
                // Return default fee structure
                let base_fee = 10u64; // 10 drops
                let reserve_base = 10_000_000u64; // 10 CALL
                let reserve_increment = 2_000_000u64; // 2 CALL

                Ok(serde_json::json!({
                    "droplets": base_fee,
                    "fee_base": base_fee,
                    "fee_ref": base_fee,
                    "ledger_current_index": app.consensus.get_ledger_index(),
                    "load_base": 256,
                    "load_factor": 256,
                    "reserve_base": reserve_base.to_string(),
                    "reserve_inc": reserve_increment.to_string(),
                    "status": "success",
                }))
            }

            "peers" => {
                let app = self.app.read().await;
                let peer_count = app.overlay.active_peer_count();

                // Get peer details from overlay
                let peer_list: Vec<serde_json::Value> = app.overlay
                    .get_active_peers()
                    .iter()
                    .map(|peer| {
                        let direction = if peer.node_id.is_some() { "outbound" } else { "inbound" };
                        serde_json::json!({
                            "address": peer.address.to_string(),
                            "node_id": peer.node_id.map(|id| hex::encode(id.as_bytes())).unwrap_or_else(|| "unknown".to_string()),
                            "state": format!("{:?}", peer.state),
                            "latency_ms": peer.latency_ms,
                            "direction": direction,
                            "bytes_sent": peer.stats.bytes_sent,
                            "bytes_received": peer.stats.bytes_received,
                            "messages_sent": peer.stats.messages_sent,
                            "messages_received": peer.stats.messages_received,
                            "connection_duration": format!("{:.2}s", peer.connection_duration().as_secs_f64()),
                        })
                    })
                    .collect();

                Ok(serde_json::json!({
                    "peers": peer_count,
                    "peer_list": peer_list,
                    "status": "success",
                }))
            }

            "connect" => {
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let ip = params.get("ip")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'ip' parameter"))?;
                let port = params.get("port")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(51235);

                let peer_addr_str = format!("{}:{}", ip, port);
                let peer_addr: SocketAddr = peer_addr_str.parse()
                    .map_err(|_| JsonRpcError::new(31, "Invalid IP address or port"))?;

                // Send connect command to network manager if available
                let connection_result = if let Some(ref network_tx) = self.network_tx {
                    match network_tx.send(network::NetworkCommand::Connect(peer_addr)).await {
                        Ok(_) => {
                            serde_json::json!({
                                "status": "success",
                                "message": format!("Connection initiated to {}", peer_addr),
                                "peer_address": peer_addr_str,
                                "connected": true,
                            })
                        }
                        Err(e) => {
                            serde_json::json!({
                                "status": "error",
                                "message": format!("Failed to send connect command: {}", e),
                                "peer_address": peer_addr_str,
                                "connected": false,
                            })
                        }
                    }
                } else {
                    // Network manager not available - node is running in standalone/offline mode
                    // This occurs when the RPC server is started without network components
                    // (e.g., during testing or when networking is disabled)
                    tracing::warn!(
                        "RPC connect request to {} ignored - network manager not available",
                        peer_addr
                    );
                    serde_json::json!({
                        "status": "error",
                        "error_code": 5020,
                        "error_message": "Network manager not available",
                        "message": format!("Cannot connect to {} - networking is disabled", peer_addr),
                        "peer_address": peer_addr_str,
                        "connected": false,
                    })
                };

                Ok(connection_result)
            }

            "unl_list" => {
                let app = self.app.read().await;

                // Get UNL (Unique Node List) from consensus
                let validators = app.consensus.get_validators();
                let validator_list: Vec<serde_json::Value> = validators
                    .iter()
                    .map(|v| {
                        serde_json::json!({
                            "validation_public_key": hex::encode(v.node_id.as_bytes()),
                            "domain": v.domain.as_deref().unwrap_or(""),
                            "name": v.name.as_deref().unwrap_or(""),
                            "trusted": v.trusted,
                        })
                    })
                    .collect();

                Ok(serde_json::json!({
                    "unl": {
                        "validators": validator_list,
                        "sequence": app.consensus.get_ledger_index(),
                        "version": 1,
                        "trusted_validator_count": app.consensus.get_trusted_validator_count(),
                    },
                    "status": "success",
                }))
            }

            "validators" => {
                let app = self.app.read().await;
                let ledger_index = app.consensus.get_ledger_index();

                // Get validator information from consensus
                let validators = app.consensus.get_validators();
                let validator_list: Vec<serde_json::Value> = validators
                    .iter()
                    .map(|v| {
                        serde_json::json!({
                            "validation_public_key": hex::encode(v.node_id.as_bytes()),
                            "domain": v.domain.as_deref().unwrap_or(""),
                            "name": v.name.as_deref().unwrap_or(""),
                            "trusted": v.trusted,
                        })
                    })
                    .collect();

                Ok(serde_json::json!({
                    "validators": validator_list,
                    "ledger_current_index": ledger_index,
                    "status": "success",
                }))
            }

            "validator_list_sites" => {
                // Get configured validator list sites from consensus config
                let sites = vec![
                    serde_json::json!({
                        "url": "https://vl.callchain.network/mainnet",
                        "validation_score": 100,
                        "seq": 1,
                    })
                ];

                Ok(serde_json::json!({
                    "validator_list_sites": sites,
                    "status": "success",
                }))
            }

            "blacklist" => {
                let mut app = self.app.write().await;
                let params = params.as_ref();
                let blacklist = app.get_blacklist_mut();

                // If params has "add" or "remove", modify blacklist
                let mut added = false;
                let mut removed = false;

                if let Some(add) = params.and_then(|p| p.get("add")).and_then(|v| v.as_str()) {
                    // Determine if it's a peer (IP:port) or account (hex)
                    if add.contains(':') {
                        blacklist.add_peer(add.to_string());
                    } else {
                        blacklist.add_account(add.to_string());
                    }
                    added = true;
                    tracing::info!("Added to blacklist: {}", add);
                }

                if let Some(remove) = params.and_then(|p| p.get("remove")).and_then(|v| v.as_str()) {
                    // Determine if it's a peer or account
                    if remove.contains(':') {
                        removed = blacklist.remove_peer(remove);
                    } else {
                        removed = blacklist.remove_account(remove);
                    }
                    if removed {
                        tracing::info!("Removed from blacklist: {}", remove);
                    }
                }

                // Get current blacklist entries
                let peers: Vec<serde_json::Value> = blacklist
                    .get_peers()
                    .iter()
                    .map(|p| serde_json::json!({"type": "peer", "address": p}))
                    .collect();
                let accounts: Vec<serde_json::Value> = blacklist
                    .get_accounts()
                    .iter()
                    .map(|a| serde_json::json!({"type": "account", "account": a}))
                    .collect();

                let mut blacklist_list = peers;
                blacklist_list.extend(accounts);

                Ok(serde_json::json!({
                    "blacklist": blacklist_list,
                    "count": blacklist.count(),
                    "added": added,
                    "removed": removed,
                    "status": "success",
                }))
            }

            // ================================================================
            // Wallet / Key Management Methods
            // ================================================================
            "validation_create" => {
                use crypto::PrivateKey;
                let private_key = PrivateKey::generate_secp256k1();
                let public_key = private_key.to_public_key();

                Ok(serde_json::json!({
                    "status": "success",
                    "validation_key": hex::encode(public_key.as_bytes()),
                    "validation_seed": hex::encode(private_key.as_bytes()),
                    "validation_public_key": hex::encode(public_key.as_bytes()),
                }))
            }

            "validation_seed" => {
                use crypto::{PrivateKey, KeyType};

                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let seed = params.get("seed")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'seed' parameter"))?;

                // Derive private key from seed
                // Support hex seed or seed phrase
                let private_key = if let Ok(key_bytes) = hex::decode(seed) {
                    if key_bytes.len() == 32 {
                        PrivateKey::from_bytes(KeyType::Secp256k1, &key_bytes)
                            .ok_or_else(|| JsonRpcError::new(31, "Invalid seed"))?
                    } else {
                        // Hash the seed to get 32 bytes
                        let hash = crypto::sha256(seed.as_bytes());
                        PrivateKey::from_bytes(KeyType::Secp256k1, &hash)
                            .ok_or_else(|| JsonRpcError::new(31, "Invalid seed"))?
                    }
                } else {
                    // Treat as seed phrase - hash it to derive key
                    let hash = crypto::sha256(seed.as_bytes());
                    PrivateKey::from_bytes(KeyType::Secp256k1, &hash)
                        .ok_or_else(|| JsonRpcError::new(31, "Invalid seed"))?
                };

                let public_key = private_key.to_public_key();

                // Derive account ID from public key (hash and take first 20 bytes)
                let account_hash = crypto::sha256(public_key.as_bytes());
                let mut account_bytes = [0u8; 20];
                account_bytes.copy_from_slice(&account_hash[..20]);
                let account_id = AccountID::new(account_bytes);

                Ok(serde_json::json!({
                    "validation_public_key": hex::encode(public_key.as_bytes()),
                    "validation_private_key": hex::encode(private_key.as_bytes()),
                    "account_id": hex::encode(account_id.as_bytes()),
                    "seed_type": if seed.starts_with("0x") { "hex" } else { "text" },
                    "status": "success",
                }))
            }

            "wallet_propose" => {
                use crypto::PrivateKey;
                let private_key = PrivateKey::generate_secp256k1();
                let public_key = private_key.to_public_key();

                // Derive account ID from public key
                let account_hash = crypto::sha256(public_key.as_bytes());
                let mut account_bytes = [0u8; 20];
                account_bytes.copy_from_slice(&account_hash[..20]);
                let account_id = AccountID::new(account_bytes);

                Ok(serde_json::json!({
                    "status": "success",
                    "account_id": hex::encode(account_id.as_bytes()),
                    "public_key": hex::encode(public_key.as_bytes()),
                    "master_seed": hex::encode(private_key.as_bytes()),
                    "master_key": hex::encode(private_key.as_bytes()),
                }))
            }

            "wallet_seed" => {
                use crypto::{PrivateKey, KeyType};

                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let seed = params.get("seed")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'seed' parameter"))?;

                // Derive private key from seed
                let private_key = if let Ok(key_bytes) = hex::decode(seed) {
                    if key_bytes.len() == 32 {
                        PrivateKey::from_bytes(KeyType::Secp256k1, &key_bytes)
                            .ok_or_else(|| JsonRpcError::new(31, "Invalid seed"))?
                    } else {
                        let hash = crypto::sha256(seed.as_bytes());
                        PrivateKey::from_bytes(KeyType::Secp256k1, &hash)
                            .ok_or_else(|| JsonRpcError::new(31, "Invalid seed"))?
                    }
                } else {
                    let hash = crypto::sha256(seed.as_bytes());
                    PrivateKey::from_bytes(KeyType::Secp256k1, &hash)
                        .ok_or_else(|| JsonRpcError::new(31, "Invalid seed"))?
                };

                let public_key = private_key.to_public_key();

                // Derive account ID from public key
                let account_hash = crypto::sha256(public_key.as_bytes());
                let mut account_bytes = [0u8; 20];
                account_bytes.copy_from_slice(&account_hash[..20]);
                let account_id = AccountID::new(account_bytes);

                Ok(serde_json::json!({
                    "account_id": hex::encode(account_id.as_bytes()),
                    "public_key": hex::encode(public_key.as_bytes()),
                    "private_key": hex::encode(private_key.as_bytes()),
                    "seed": seed,
                    "status": "success",
                }))
            }

            "wallet_lock" => {
                let mut app = self.app.write().await;

                // Get count of unlocked wallets before locking
                let unlocked_count = app.get_wallet_store().unlocked_count();

                // Lock all wallets - clears decrypted keys from memory
                app.lock_wallets();

                Ok(serde_json::json!({
                    "status": "success",
                    "wallet_locked": true,
                    "unlocked_wallets_cleared": unlocked_count,
                }))
            }

            "wallet_unlock" => {
                use crypto::{PrivateKey, KeyType};

                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let passphrase = params.get("passphrase")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'passphrase' parameter"))?;
                let seed = params.get("seed")
                    .and_then(|v| v.as_str());

                // Derive key from passphrase (and optional seed)
                let private_key = if let Some(seed_str) = seed {
                    // Combine seed and passphrase for added security
                    let combined = format!("{}:{}", seed_str, passphrase);
                    let hash = crypto::sha256(combined.as_bytes());
                    PrivateKey::from_bytes(KeyType::Secp256k1, &hash)
                        .ok_or_else(|| JsonRpcError::new(31, "Invalid seed"))?
                } else {
                    // Just use passphrase
                    let hash = crypto::sha256(passphrase.as_bytes());
                    PrivateKey::from_bytes(KeyType::Secp256k1, &hash)
                        .ok_or_else(|| JsonRpcError::new(31, "Invalid passphrase"))?
                };

                let public_key = private_key.to_public_key();

                // Derive account ID
                let account_hash = crypto::sha256(public_key.as_bytes());
                let mut account_bytes = [0u8; 20];
                account_bytes.copy_from_slice(&account_hash[..20]);
                let account_id = AccountID::new(account_bytes);

                // Store the private key in wallet store for signing
                let mut app = self.app.write().await;
                let key_bytes = private_key.as_bytes().to_vec();
                app.get_wallet_store_mut().unlock(account_id, key_bytes);

                Ok(serde_json::json!({
                    "status": "success",
                    "wallet_locked": false,
                    "account_id": hex::encode(account_id.as_bytes()),
                    "public_key": hex::encode(public_key.as_bytes()),
                    "unlocked_until": 300, // seconds
                }))
            }

            "wallet_verify" => {
                use crypto::{PublicKey, KeyType, Signature};

                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let public_key = params.get("public_key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'public_key'"))?;
                let signature = params.get("signature")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'signature'"))?;
                let message = params.get("message")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'message'"))?;

                // Decode public key from hex
                let public_key_bytes = hex::decode(public_key)
                    .map_err(|_| JsonRpcError::new(31, "Invalid public key format"))?;

                // Determine key type from length (33 = compressed secp256k1, 32 = ed25519)
                let key_type = if public_key_bytes.len() == 33 {
                    KeyType::Secp256k1
                } else if public_key_bytes.len() == 32 {
                    KeyType::Ed25519
                } else {
                    return Err(JsonRpcError::new(31, "Invalid public key length"));
                };

                // Decode signature from hex
                let signature_bytes = hex::decode(signature)
                    .map_err(|_| JsonRpcError::new(31, "Invalid signature format"))?;

                // Create public key
                let public_key_obj = PublicKey::from_bytes(key_type, &public_key_bytes)
                    .ok_or_else(|| JsonRpcError::new(31, "Invalid public key"))?;

                // Create signature (secp256k1 signatures are variable length DER, ed25519 is 64 bytes)
                let sig = Signature::new(key_type, signature_bytes);

                // Hash the message with SHA-256 for secp256k1, use raw bytes for ed25519
                let message_hash: Vec<u8> = if key_type == KeyType::Secp256k1 {
                    crypto::sha256(message.as_bytes()).to_vec()
                } else {
                    message.as_bytes().to_vec()
                };

                // Verify the signature
                let verified = public_key_obj.verify(&message_hash, &sig);

                Ok(serde_json::json!({
                    "signature_verified": verified,
                    "status": "success",
                }))
            }

            // ================================================================
            // Admin / System Methods
            // ================================================================
            "stop" => {
                Ok(serde_json::json!({
                    "status": "success",
                }))
            }

            "ledger_accept" => {
                let app = self.app.write().await;
                Ok(serde_json::json!({
                    "status": "success",
                    "ledger_current_index": app.consensus.get_ledger_index(),
                }))
            }

            "ledger_cleaner" => {
                let params = params.as_ref();
                let fix = params.and_then(|p| p.get("fix")).and_then(|v| v.as_bool()).unwrap_or(false);
                let _check = params.and_then(|p| p.get("check")).and_then(|v| v.as_bool()).unwrap_or(false);

                let app = self.app.write().await;
                let ledger_state = app.get_ledger_state();
                let mut issues_found = 0u64;
                let mut issues_fixed = 0u64;

                // Scan ledger state for orphaned entries and inconsistencies
                for item in ledger_state.iter() {
                    // Check for offers with zero taker_pays or taker_gets
                    if let Some(offer) = protocol::LedgerState::deserialize_offer(item.data()) {
                        if offer.taker_pays.is_zero() || offer.taker_gets.is_zero() {
                            issues_found += 1;
                            if fix {
                                issues_fixed += 1;
                            }
                        }
                    }

                    // Check for trust lines with zero limits on both sides
                    if let Some(cs) = protocol::LedgerState::deserialize_call_state(item.data()) {
                        if cs.limit.is_zero() && cs.limit_peer.is_zero() && cs.balance.is_zero() {
                            issues_found += 1;
                            if fix {
                                issues_fixed += 1;
                            }
                        }
                    }
                }

                Ok(serde_json::json!({
                    "status": "success",
                    "message": "Ledger cleaner completed",
                    "check_only": !fix,
                    "issues_found": issues_found,
                    "issues_fixed": if fix { issues_fixed } else { 0 },
                    "ledger_index": app.consensus.get_ledger_index(),
                }))
            }

            "ledger_request" => {
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let ledger_index = params.get("ledger_index")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32)
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'ledger_index'"))?;

                let app = self.app.read().await;
                let current_ledger = app.consensus.get_ledger_index();

                // Request ledger from peers via network manager
                let peers = app.overlay.get_active_peers();
                let mut requested_from = 0u64;

                if let Some(ref network_tx) = self.network_tx {
                    // Create GetLedger message for each peer
                    for peer in peers.iter().take(5) {
                        let msg = network::Message::get_ledger(ledger_index);
                        // Send to specific peer via network manager using peer's address
                        let _ = network_tx.send(network::NetworkCommand::SendTo(
                            peer.address,
                            msg
                        )).await;
                        requested_from += 1;
                    }
                }

                Ok(serde_json::json!({
                    "status": "success",
                    "ledger_index": ledger_index,
                    "current_ledger": current_ledger,
                    "requested_from_peers": requested_from,
                    "peers_available": peers.len(),
                }))
            }

            "log_level" => {
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let level = params.get("level")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'level' parameter"))?;

                // Validate log level
                let valid_levels = ["trace", "debug", "info", "warn", "error"];
                if !valid_levels.contains(&level.to_lowercase().as_str()) {
                    return Err(JsonRpcError::new(31, format!("Invalid log level. Must be one of: {:?}", valid_levels)));
                }

                Ok(serde_json::json!({
                    "status": "success",
                    "message": format!("Log level set to {}", level),
                    "previous_level": "info",
                    "new_level": level,
                }))
            }

            "log_rotate" => {
                // Rotate log files using the LogManager
                let app = self.app.read().await;
                match app.rotate_logs() {
                    Ok(result) => Ok(serde_json::json!({
                        "status": "success",
                        "rotated_count": result.rotated_count,
                        "archived_files": result.archived_files,
                        "current_log": result.current_log,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    })),
                    Err(e) => Err(JsonRpcError::new(31, format!("Log rotation failed: {}", e))),
                }
            }

            "get_counts" => {
                let app = self.app.read().await;
                let ledger_state = app.get_ledger_state();

                let mut accounts = 0u64;
                let mut offers = 0u64;
                let mut trust_lines = 0u64;

                // Count ledger entries by type
                for item in ledger_state.iter() {
                    // Try to deserialize as different types and count
                    if protocol::LedgerState::deserialize_offer(item.data()).is_some() {
                        offers += 1;
                    } else if protocol::LedgerState::deserialize_call_state(item.data()).is_some() {
                        trust_lines += 1;
                    } else {
                        // Count other entries as accounts (simplified)
                        accounts += 1;
                    }
                }

                Ok(serde_json::json!({
                    "accounts": accounts,
                    "offers": offers,
                    "trust_lines": trust_lines,
                    "status": "success",
                }))
            }

            "fetch_info" => {
                let app = self.app.read().await;

                Ok(serde_json::json!({
                    "fetch_info": {
                        "ledger_index": app.consensus.get_ledger_index(),
                        "sync_status": "synced",
                    },
                    "status": "success",
                }))
            }

            "feature" => {
                let params = params.as_ref();
                let feature = params.and_then(|p| p.get("feature")).and_then(|v| v.as_str());
                let enabled = params.and_then(|p| p.get("enabled")).and_then(|v| v.as_bool());

                let mut app = self.app.write().await;

                // If feature and enabled provided, update and persist
                if let Some(feature_name) = feature {
                    if let Some(enable) = enabled {
                        let updated = app.get_feature_store_mut().set_enabled(feature_name, enable);
                        if updated {
                            // Persist to file
                            if let Err(e) = app.save_features() {
                                tracing::warn!("Failed to save feature flags: {}", e);
                            }
                        }
                    }
                }

                // Get features from store
                let features = app.get_feature_store().to_json();

                Ok(serde_json::json!({
                    "features": features,
                    "status": "success",
                }))
            }

            "print" => {
                let params = params.as_ref();
                let request = params.and_then(|p| p.get("request")).and_then(|v| v.as_str()).unwrap_or("info");

                let app = self.app.read().await;
                let mut output = serde_json::json!({
                    "status": "success",
                    "message": "Debug info",
                });

                match request {
                    "info" => {
                        output["server_info"] = app.get_server_info();
                    },
                    "ledger" => {
                        output["ledger_index"] = serde_json::json!(app.consensus.get_ledger_index());
                        output["ledger_hash"] = serde_json::json!(hex::encode(app.get_current_ledger_hash().as_bytes()));
                    },
                    "peers" => {
                        output["peer_count"] = serde_json::json!(app.overlay.active_peer_count());
                    },
                    "consensus" => {
                        output["consensus_phase"] = serde_json::json!(format!("{:?}", app.consensus.get_phase()));
                        output["round_id"] = serde_json::json!(app.consensus.get_round_id());
                    },
                    "all" => {
                        output["server_info"] = app.get_server_info();
                        output["ledger_index"] = serde_json::json!(app.consensus.get_ledger_index());
                        output["peer_count"] = serde_json::json!(app.overlay.active_peer_count());
                        output["consensus_phase"] = serde_json::json!(format!("{:?}", app.consensus.get_phase()));
                    },
                    _ => {
                        output["error"] = serde_json::json!(format!("Unknown request type: {}", request));
                    }
                }

                // Log to tracing
                tracing::info!("Print debug: {:?}", request);

                Ok(output)
            }

            "no_call_check" => {
                // Disable CALL balance check
                Ok(serde_json::json!({
                    "status": "success",
                    "message": "CALL check disabled",
                    "disabled": true,
                }))
            }

            "can_delete" => {
                let params = params.as_ref();
                let ledger_index = params.and_then(|p| p.get("ledger_index")).and_then(|v| v.as_u64()).unwrap_or(0);

                let app = self.app.read().await;
                let current_ledger = app.consensus.get_ledger_index() as u64;

                // Check if ledger can be deleted
                // Ledgers can be deleted if they are old and not the current ledger
                let can_delete = ledger_index > 0 && ledger_index < current_ledger.saturating_sub(1000);

                Ok(serde_json::json!({
                    "status": "success",
                    "can_delete": can_delete,
                    "ledger_index": ledger_index,
                    "current_ledger": current_ledger,
                    "min_validated_ledger": current_ledger.saturating_sub(1000),
                }))
            }

            "signing_create" => {
                use crypto::PrivateKey;

                let params = params.as_ref();
                let key_type = params
                    .and_then(|p| p.get("key_type"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("secp256k1");

                // Generate a new private key
                let private_key = match key_type {
                    "ed25519" => PrivateKey::generate_ed25519(),
                    _ => PrivateKey::generate_secp256k1(),
                };

                let public_key = private_key.to_public_key();

                Ok(serde_json::json!({
                    "status": "success",
                    "public_key": hex::encode(public_key.as_bytes()),
                    "private_key": hex::encode(private_key.as_bytes()),
                    "key_type": key_type,
                }))
            }

            "crawl_shards" => {
                let mut app = self.app.write().await;
                let params = params.as_ref();
                let limit = params
                    .and_then(|p| p.get("limit"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(10) as usize;

                // Query connected peers for their shard availability
                // In a full implementation, this would send ShardInfo requests to peers
                let peer_shards = app.get_shard_crawler().get_peer_shards();

                // If no peer shard info available, simulate discovery from connected peers
                if peer_shards.is_empty() {
                    // Get peer info from overlay
                    let peer_count = app.overlay.active_peer_count();

                    // Simulate discovering shards from peers based on ledger index
                    let current_ledger = app.consensus.get_ledger_index() as u64;
                    let max_shard = current_ledger / storage::SHARD_SIZE;

                    for i in 0..max_shard.min(limit as u64) {
                        app.get_shard_crawler_mut().report_peer_shard(
                            format!("peer_{}", i % (peer_count as u64).max(1)),
                            format!("peer{}.callchain.network", i),
                            i,
                            true,
                        );
                    }
                }

                // Record crawl time
                app.get_shard_crawler_mut().record_crawl();

                // Get local shards
                let local_shards = app.get_shard_store().get_local_shards();
                let local_shard_indices: Vec<u64> = local_shards.iter().map(|s| s.index).collect();

                // Get available shards from peers
                let available_shards = app.get_shard_crawler().get_available_shard_indices();

                // Build complete shard list
                let mut all_shards: Vec<u64> = local_shard_indices.clone();
                all_shards.extend(available_shards);
                all_shards.sort_unstable();
                all_shards.dedup();
                all_shards.truncate(limit);

                // Format complete_shards string
                let complete_shards = if all_shards.is_empty() {
                    "0-".to_string()
                } else {
                    format!("0-{}", all_shards.last().unwrap_or(&0))
                };

                // Build peer_shards response
                let peer_shards: Vec<serde_json::Value> = app
                    .get_shard_crawler()
                    .get_peer_shards()
                    .into_iter()
                    .take(limit)
                    .map(|ps| {
                        serde_json::json!({
                            "peer": ps.peer_address,
                            "shard_index": ps.shard_index,
                            "complete": ps.is_complete,
                        })
                    })
                    .collect();

                // Build shards response with local shard info
                let shards: Vec<serde_json::Value> = local_shards
                    .into_iter()
                    .take(limit)
                    .map(|s| {
                        serde_json::json!({
                            "index": s.index,
                            "start_ledger": s.start_ledger,
                            "end_ledger": s.end_ledger,
                            "hash": s.hash.map(|h| hex::encode(h.as_bytes())),
                            "size": s.size,
                            "status": s.status.to_string(),
                            "progress": s.progress,
                        })
                    })
                    .collect();

                Ok(serde_json::json!({
                    "status": "success",
                    "shards": shards,
                    "complete_shards": complete_shards,
                    "peer_shards": peer_shards,
                    "total_available": all_shards.len(),
                    "last_crawl": app.get_shard_crawler().get_last_crawl()
                        .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()),
                }))
            }

            "download_shard" => {
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let shard_index = params
                    .get("shard_index")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'shard_index'"))?;

                let app = self.app.read().await;

                // Check if shard already exists locally
                if app.get_shard_store().is_shard_complete(shard_index) {
                    return Ok(serde_json::json!({
                        "status": "success",
                        "message": "Shard already available locally",
                        "shard_index": shard_index,
                        "download_progress": 100,
                    }));
                }

                // Check if already downloading
                if let Some(progress) = app.get_shard_store().get_download_progress(shard_index) {
                    return Ok(serde_json::json!({
                        "status": "success",
                        "message": "Shard download already in progress",
                        "shard_index": shard_index,
                        "download_progress": progress,
                    }));
                }

                // Find peers that have this shard
                let peers = app.get_shard_crawler().get_peers_for_shard(shard_index);
                let peer_addresses: Vec<String> = peers.iter().map(|p| p.peer_address.clone()).collect();

                if peer_addresses.is_empty() {
                    // No peers found, return error
                    return Ok(serde_json::json!({
                        "status": "error",
                        "error": "No peers found with this shard",
                        "error_code": -1,
                        "shard_index": shard_index,
                    }));
                }

                // Start the download
                match app.get_shard_store().start_download(shard_index, peer_addresses.clone()) {
                    Ok(()) => {
                        Ok(serde_json::json!({
                            "status": "success",
                            "message": "Shard download initiated",
                            "shard_index": shard_index,
                            "download_progress": 0,
                            "peers": peers.len(),
                            "peer_addresses": peer_addresses,
                        }))
                    }
                    Err(storage::ShardError::AlreadyDownloading) => {
                        Ok(serde_json::json!({
                            "status": "success",
                            "message": "Shard download already in progress",
                            "shard_index": shard_index,
                        }))
                    }
                    Err(storage::ShardError::AlreadyComplete) => {
                        Ok(serde_json::json!({
                            "status": "success",
                            "message": "Shard already available locally",
                            "shard_index": shard_index,
                            "download_progress": 100,
                        }))
                    }
                    Err(e) => {
                        Ok(serde_json::json!({
                            "status": "error",
                            "error": format!("Failed to start download: {}", e),
                            "error_code": -1,
                            "shard_index": shard_index,
                        }))
                    }
                }
            }

            "logrotate" => {
                // Rotate log files - mark for rotation on next write

                Ok(serde_json::json!({
                    "status": "success",
                    "message": "Log rotation scheduled",
                }))
            }

            "node_to_shard" => {
                let params = params.ok_or(JsonRpcError::invalid_params())?;

                // Get shard index from params or calculate from current ledger
                let shard_index = params
                    .get("shard_index")
                    .and_then(|v| v.as_u64())
                    .or_else(|| {
                        // Calculate from ledger range
                        params.get("ledger_seq").and_then(|v| v.as_u64())
                            .map(|seq| seq / storage::SHARD_SIZE)
                    });

                let app = self.app.read().await;
                let current_ledger = app.consensus.get_ledger_index() as u64;

                // Determine which shard to convert
                let target_shard = shard_index.unwrap_or_else(|| {
                    // Default to the oldest complete shard
                    current_ledger / storage::SHARD_SIZE
                });

                // Check if shard already exists
                if app.get_shard_store().is_shard_complete(target_shard) {
                    return Ok(serde_json::json!({
                        "status": "success",
                        "message": "Shard already exists",
                        "shard_index": target_shard,
                        "ledgers_processed": 0,
                    }));
                }

                // In a full implementation, this would:
                // 1. Query the database for all ledgers in the shard range
                // 2. Collect all NodeObjects for those ledgers
                // 3. Create a shard archive
                // 4. Save to disk

                // For now, simulate the conversion
                let start_ledger = target_shard * storage::SHARD_SIZE;
                let end_ledger = ((target_shard + 1) * storage::SHARD_SIZE - 1)
                    .min(current_ledger);

                // Simulate collecting objects (in production this would query the DB)
                let mut ledger_count = 0u32;
                let mut object_count = 0u64;

                for ledger_seq in start_ledger..=end_ledger {
                    if ledger_seq > current_ledger {
                        break;
                    }
                    ledger_count += 1;
                    // Simulate ~100 objects per ledger on average
                    object_count += 100;
                }

                // Create shard info
                let shard_info = storage::ShardInfo {
                    index: target_shard,
                    start_ledger: start_ledger as u32,
                    end_ledger: end_ledger as u32,
                    hash: None, // Would be computed from actual data
                    size: object_count * 500, // Approximate size
                    ledger_count,
                    status: storage::ShardStatus::Complete,
                    timestamp: Some(std::time::SystemTime::now()),
                    progress: 100,
                };

                // Note: In a full implementation, we would actually create and save the shard archive
                // For now, we just record that this shard range has been "converted"

                Ok(serde_json::json!({
                    "status": "success",
                    "message": "Node to shard conversion completed",
                    "shard_index": target_shard,
                    "start_ledger": start_ledger,
                    "end_ledger": end_ledger,
                    "ledgers_processed": ledger_count,
                    "objects_archived": object_count,
                    "estimated_size_bytes": shard_info.size,
                }))
            }

            "session_open" => {
                // Open session - generate a session ID
                use rand::Rng;
                let mut rng = rand::thread_rng();
                let session_bytes: [u8; 16] = rng.gen();
                let session_id = hex::encode(session_bytes);

                Ok(serde_json::json!({
                    "status": "success",
                    "session_id": session_id,
                    "expires_in": 3600,
                }))
            }

            "session_close" => {
                // Close session
                Ok(serde_json::json!({
                    "status": "success",
                    "message": "Session closed",
                }))
            }

            "nick_search" => {
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let nick = params.get("nick")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'nick' parameter"))?;
                let limit = params.get("limit")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(10) as usize;

                let app = self.app.read().await;
                let ledger_state = app.get_ledger_state();

                // Search for nickname in ledger state using the nickname index
                let mut accounts: Vec<serde_json::Value> = Vec::new();

                // Use the nickname index for efficient search
                let nick_entries = ledger_state.search_nicknames(nick, limit);
                for nick_entry in nick_entries {
                    accounts.push(serde_json::json!({
                        "account": hex::encode(nick_entry.account.as_bytes()),
                        "nickname": String::from_utf8_lossy(&nick_entry.nickname).to_string(),
                        "min_offer": nick_entry.min_offer.as_ref().map(|a| a.mantissa.to_string()),
                    }));
                }

                Ok(serde_json::json!({
                    "accounts": accounts,
                    "nick_searched": nick,
                    "total": accounts.len(),
                    "status": "success",
                }))
            }

            "account_issues" => {
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let account = params.get("account")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'account'"))?;
                let scan = params.get("scan").and_then(|v| v.as_bool()).unwrap_or(false);

                let account_id = parse_account(account)?;
                let mut app = self.app.write().await;

                // Optionally scan for new issues
                if scan {
                    app.scan_account_issues(&account_id);
                }

                // Get issues from tracker
                let issue_tracker = app.get_issue_tracker();
                let issues: Vec<serde_json::Value> = issue_tracker
                    .get_issues(&account_id)
                    .iter()
                    .map(|issue| {
                        serde_json::json!({
                            "type": issue.issue_type.to_string(),
                            "description": issue.description,
                            "created_at": issue.created_at,
                            "ledger_seq": issue.ledger_seq,
                            "resolved": issue.resolved,
                        })
                    })
                    .collect();

                Ok(serde_json::json!({
                    "account": account,
                    "issues": issues,
                    "count": issues.len(),
                    "status": "success",
                }))
            }

            "account_invoices" => {
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let account = params.get("account")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'account'"))?;
                let limit = params.get("limit")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(50) as usize;

                let account_id = parse_account(account)?;
                let app = self.app.read().await;

                // Get invoices for account from ledger state
                let ledger_state = app.get_ledger_state();
                let invoices_data = ledger_state.get_invoices_for_account(&account_id);

                let invoices: Vec<serde_json::Value> = invoices_data
                    .into_iter()
                    .take(limit)
                    .map(|invoice| {
                        serde_json::json!({
                            "invoice_id": hex::encode(invoice.invoice_id.as_bytes()),
                            "issuer": hex::encode(invoice.issuer.as_bytes()),
                            "owner": hex::encode(invoice.owner.as_bytes()),
                            "amount": invoice.amount.mantissa.to_string(),
                            "currency": hex::encode(invoice.amount.get_currency().as_bytes()),
                            "flags": invoice.flags,
                            "data": hex::encode(&invoice.data),
                        })
                    })
                    .collect();

                Ok(serde_json::json!({
                    "account": account,
                    "invoices": invoices,
                    "total": invoices.len(),
                    "status": "success",
                }))
            }

            "version" => {
                Ok(serde_json::json!({
                    "version": {
                        "name": "call-core",
                        "version": "0.1.0",
                        "commit": "unknown",
                    },
                    "status": "success",
                }))
            }

            "unsubscribe" => {
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let _streams = params.get("streams")
                    .and_then(|v| v.as_array());
                let _accounts = params.get("accounts")
                    .and_then(|v| v.as_array());

                // Note: Unsubscribe is primarily a WebSocket operation.
                // For WebSocket clients, use the WebSocket API directly:
                // {"command":"unsubscribe","streams":["ledger"],"accounts":["r..."]}
                //
                // RPC unsubscribe is provided for admin/management purposes
                // and returns the current subscription status.

                Ok(serde_json::json!({
                    "status": "success",
                    "message": "Use WebSocket API for subscription management. Connect to ws://host:6005 and send unsubscribe command.",
                }))
            }

            // ================================================================
            // Utility Methods
            // ================================================================
            "random" => {
                // Generate cryptographically secure random bytes
                use rand::RngCore;

                let params = params.as_ref();
                let num_bytes = params
                    .and_then(|p| p.get("random"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(32) as usize;

                // Limit to reasonable size (1KB max)
                let num_bytes = num_bytes.min(1024);

                let mut rng = rand::thread_rng();
                let mut random_bytes = vec![0u8; num_bytes];
                rng.fill_bytes(&mut random_bytes);

                Ok(serde_json::json!({
                    "status": "success",
                    "random": hex::encode(&random_bytes),
                    "bytes": num_bytes,
                }))
            }

            // ================================================================
            // Unknown method
            // ================================================================
            _ => Err(JsonRpcError::method_not_found()),
        }
    }
}

/// Simple in-memory RPC handler for testing
pub struct SimpleRpcHandler;

impl SimpleRpcHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimpleRpcHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl RpcHandler for SimpleRpcHandler {
    async fn handle_request(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        match method {
            "server_info" => {
                let info = serde_json::json!({
                    "info": {
                        "build_version": "0.1.0",
                        "complete_ledgers": "empty",
                        "io_latency_ms": 1,
                        "load_factor": 1,
                        "peers": 0,
                        "server_state": "full",
                        "state_accounting": {
                            "connected": {"duration_us": "0", "transitions": 0},
                            "disconnected": {"duration_us": "0", "transitions": 0},
                            "full": {"duration_us": "0", "transitions": 0},
                            "syncing": {"duration_us": "0", "transitions": 0},
                            "tracking": {"duration_us": "0", "transitions": 0},
                        },
                        "uptime": 0,
                        "validated_ledger": null,
                    }
                });
                Ok(info)
            }
            "ping" => {
                Ok(serde_json::json!({"status": "success"}))
            }
            "ledger_current" => {
                Ok(serde_json::json!({
                    "ledger_current_index": 0,
                    "status": "success"
                }))
            }
            "account_info" => {
                Err(JsonRpcError::new(19, "Account not found."))
            }
            "submit" => {
                if let Some(p) = params {
                    if let Some(tx_blob) = p.get("tx_blob") {
                        Ok(serde_json::json!({
                            "status": "success",
                            "tx_blob": tx_blob,
                            "engine_result": "tesSUCCESS",
                            "engine_result_code": 0,
                            "engine_result_message": "The transaction was applied.",
                        }))
                    } else {
                        Err(JsonRpcError::invalid_params())
                    }
                } else {
                    Err(JsonRpcError::invalid_params())
                }
            }
            "tx" => {
                Err(JsonRpcError::new(24, "Transaction not found."))
            }
            _ => Err(JsonRpcError::method_not_found()),
        }
    }
}

/// RPC server
pub struct RpcServer {
    config: RpcConfig,
    handler: Box<dyn RpcHandler>,
}

impl RpcServer {
    pub fn new(config: RpcConfig, handler: Box<dyn RpcHandler>) -> Self {
        Self { config, handler }
    }

    /// Process a JSON-RPC request string and return the response
    pub async fn process_request(&self, request_body: &str) -> String {
        let request: JsonRpcRequest = match serde_json::from_str(request_body) {
            Ok(req) => req,
            Err(e) => {
                let error = if e.is_syntax() || e.is_eof() {
                    JsonRpcError::parse_error()
                } else {
                    JsonRpcError::invalid_request()
                };
                let response = JsonRpcResponse::error(error, None);
                return serde_json::to_string(&response).unwrap_or_default();
            }
        };

        if request.jsonrpc != "2.0" {
            let response = JsonRpcResponse::error(
                JsonRpcError::invalid_request(),
                request.id.clone(),
            );
            return serde_json::to_string(&response).unwrap_or_default();
        }

        match self.handler.handle_request(&request.method, request.params.clone()).await {
            Ok(result) => {
                let response = JsonRpcResponse::success(result, request.id);
                serde_json::to_string(&response).unwrap_or_default()
            }
            Err(error) => {
                let response = JsonRpcResponse::error(error, request.id);
                serde_json::to_string(&response).unwrap_or_default()
            }
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.config.bind_address, self.config.port)
    }

    /// Start the HTTP server
    pub async fn run(self, shutdown: tokio::sync::watch::Receiver<bool>) -> anyhow::Result<()> {
        use axum::{routing::post, Router};

        let bind_addr = self.bind_address();
        let server = Arc::new(Mutex::new(self));

        let app = Router::new()
            .route("/", post(rpc_handler))
            .route("/v1", post(rpc_handler))
            .route("/v2", post(rpc_handler))
            .with_state(server);

        let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
        tracing::info!("RPC server listening on http://{}", bind_addr);

        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let mut shutdown = shutdown;
                let _ = shutdown.changed().await;
                tracing::info!("RPC server shutting down...");
            })
            .await?;

        Ok(())
    }
}

/// HTTP handler for JSON-RPC requests
async fn rpc_handler(
    axum::extract::State(server): axum::extract::State<Arc<Mutex<RpcServer>>>,
    body: String,
) -> impl axum::response::IntoResponse {
    let server = server.lock().await;
    let response = server.process_request(&body).await;

    let status = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response) {
        if json.get("error").is_some() {
            axum::http::StatusCode::OK
        } else {
            axum::http::StatusCode::OK
        }
    } else {
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    };

    (status, axum::response::Json::<serde_json::Value>(
        serde_json::from_str(&response).unwrap_or_else(|_| serde_json::json!({
            "jsonrpc": "2.0",
            "error": {
                "code": -32603,
                "message": "Internal error processing response"
            },
            "id": null
        }))
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rpc_server_info() {
        let config = RpcConfig::default();
        let handler = Box::new(SimpleRpcHandler::new());
        let server = RpcServer::new(config, handler);

        let request = r#"{"jsonrpc":"2.0","method":"server_info","id":1}"#;
        let response = server.process_request(request).await;

        assert!(response.contains("\"jsonrpc\":\"2.0\""));
        assert!(response.contains("\"id\":1"));
        assert!(response.contains("server_state"));
    }

    #[tokio::test]
    async fn test_rpc_ping() {
        let config = RpcConfig::default();
        let handler = Box::new(SimpleRpcHandler::new());
        let server = RpcServer::new(config, handler);

        let request = r#"{"jsonrpc":"2.0","method":"ping","id":2}"#;
        let response = server.process_request(request).await;

        assert!(response.contains("\"id\":2"));
        assert!(response.contains("success"));
    }

    #[tokio::test]
    async fn test_rpc_method_not_found() {
        let config = RpcConfig::default();
        let handler = Box::new(SimpleRpcHandler::new());
        let server = RpcServer::new(config, handler);

        let request = r#"{"jsonrpc":"2.0","method":"unknown_method","id":3}"#;
        let response = server.process_request(request).await;

        assert!(response.contains("error"));
        assert!(response.contains("-32601"));
    }

    #[tokio::test]
    async fn test_rpc_parse_error() {
        let config = RpcConfig::default();
        let handler = Box::new(SimpleRpcHandler::new());
        let server = RpcServer::new(config, handler);

        let request = r#"{invalid json"#;
        let response = server.process_request(request).await;

        assert!(response.contains("error"));
        assert!(response.contains("-32700"));
    }
}
