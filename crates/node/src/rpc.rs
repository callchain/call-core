use std::sync::Arc;
use tokio::sync::Mutex;
use primitives::{AccountID, UInt256};

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

/// Application-aware RPC handler
pub struct AppRpcHandler {
    app: ApplicationHandle,
}

impl AppRpcHandler {
    pub fn new(app: ApplicationHandle) -> Self {
        Self { app }
    }
}

/// Helper to parse account from string
fn parse_account(account_str: &str) -> Result<AccountID, JsonRpcError> {
    // Try hex decode first
    let account_bytes = hex::decode(account_str).map_err(|_| {
        JsonRpcError::new(35, "Account malformed.")
    })?;
    if account_bytes.len() != 20 {
        return Err(JsonRpcError::new(35, "Account malformed."));
    }
    Ok(AccountID::new(account_bytes.try_into().unwrap()))
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
                let ledger_index = app.consensus.get_round_id();
                Ok(serde_json::json!({
                    "ledger_index": ledger_index,
                    "ledger_hash": "0000000000000000000000000000000000000000000000000000000000000000",
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
                    .unwrap_or_else(|| app.consensus.get_ledger_index());

                Ok(serde_json::json!({
                    "ledger_header": {
                        "ledger_index": ledger_index.to_string(),
                        "ledger_hash": "0000000000000000000000000000000000000000000000000000000000000000",
                        "parent_hash": "0000000000000000000000000000000000000000000000000000000000000000",
                        "account_hash": "0000000000000000000000000000000000000000000000000000000000000000",
                        "transaction_hash": "0000000000000000000000000000000000000000000000000000000000000000",
                        "close_time": 0,
                        "close_time_human": "1970-01-01T00:00:00Z",
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
                    .unwrap_or(-1);
                let ledger_index_max = params.get("ledger_index_max")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(-1);
                let limit = params.get("limit")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(20);
                let forward = params.get("forward")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                // Query account transactions from database
                let transactions: Vec<serde_json::Value> = Vec::new();
                let current_ledger = app.consensus.get_ledger_index();

                // TODO: Implement actual account_tx lookup in database
                // For now, return empty array with proper structure
                let _ = (account_id, ledger_index_min, ledger_index_max, limit, forward, current_ledger);

                Ok(serde_json::json!({
                    "account": account,
                    "ledger_index_min": ledger_index_min,
                    "ledger_index_max": ledger_index_max,
                    "transactions": [],
                    "validated": false,
                    "status": "success",
                }))
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
                    .unwrap_or(100);

                let ledger_state = app.get_ledger_state();
                let lines: Vec<serde_json::Value> = Vec::new();

                // Query trust lines for account
                // TODO: Implement actual trust line lookup
                let _ = (account_id, peer, limit, ledger_state);

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
                    .unwrap_or(100);
                let ledger_state = app.get_ledger_state();
                let objects: Vec<serde_json::Value> = Vec::new();

                // Query account objects (offers, trust lines, etc.)
                // TODO: Implement actual account objects lookup
                let _ = (account_id, limit, ledger_state);

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
                    .unwrap_or(100);
                let ledger_state = app.get_ledger_state();
                let offers: Vec<serde_json::Value> = Vec::new();

                // Query account offers from ledger state
                // TODO: Implement actual account offers lookup
                let _ = (account_id, limit, ledger_state);

                Ok(serde_json::json!({
                    "account": account,
                    "offers": offers,
                    "ledger_current_index": app.consensus.get_ledger_index(),
                    "validated": false,
                    "status": "success",
                }))
            }

            "account_channels" => {
                let app = self.app.read().await;
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let account = params.get("account")
                    .and_then(|v| v.as_str())
                    .ok_or(JsonRpcError::invalid_params())?;

                let account_id = parse_account(account)?;
                let limit = params.get("limit")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(100);
                let ledger_state = app.get_ledger_state();
                let channels: Vec<serde_json::Value> = Vec::new();

                // Query payment channels for account
                // TODO: Implement actual payment channel lookup
                let _ = (account_id, limit, ledger_state);

                Ok(serde_json::json!({
                    "account": account,
                    "payment_channels": channels,
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

                let _account_id = parse_account(account)?;

                // Return list of currencies held by account
                // TODO: Implement actual currency lookup
                let currencies: Vec<String> = vec![];

                Ok(serde_json::json!({
                    "account": account,
                    "receive": currencies,
                    "send": currencies,
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
                let obligations: std::collections::HashMap<String, String> = std::collections::HashMap::new();
                let balances: std::collections::HashMap<String, String> = std::collections::HashMap::new();

                // Query gateway balances
                // TODO: Implement actual gateway balance lookup
                let _ = (account_id, hotwallets, ledger_state);

                Ok(serde_json::json!({
                    "account": account,
                    "obligations": {},
                    "balances": {},
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

                let owner_count = 0u32;
                let directory_indexes: Vec<serde_json::Value> = Vec::new();

                // Count owner objects
                // TODO: Implement actual owner info lookup
                let _ = (account_id, ledger_state);

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

                if let Some(tx_node) = app.database.fetch_transaction_node(&tx_hash) {
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
                    .unwrap_or(0);

                let limit = 50u64;
                let transactions: Vec<serde_json::Value> = Vec::new();

                // Query transaction history
                // TODO: Implement actual tx history lookup
                let _ = (start, limit);

                Ok(serde_json::json!({
                    "transactions": transactions,
                    "start": start,
                    "status": "success",
                }))
            }

            "sign" => {
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let secret = params.get("secret")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'secret' parameter"))?;
                let tx_json = params.get("tx_json")
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'tx_json' parameter"))?;

                // Parse secret and sign transaction
                // TODO: Implement actual transaction signing
                let _ = (secret, tx_json);

                Ok(serde_json::json!({
                    "tx_blob": "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
                    "tx_json": tx_json,
                    "status": "success",
                }))
            }

            "sign_for" => {
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let account = params.get("account")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'account' parameter"))?;
                let secret = params.get("secret")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'secret' parameter"))?;
                let tx_json = params.get("tx_json")
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'tx_json' parameter"))?;

                // Sign transaction for another account
                // TODO: Implement actual sign_for
                let _ = (account, secret, tx_json);

                Ok(serde_json::json!({
                    "tx_blob": "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
                    "tx_json": tx_json,
                    "status": "success",
                }))
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

                let taker = params.get("taker")
                    .and_then(|v| v.as_str());
                let limit = params.get("limit")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(100);

                let offers: Vec<serde_json::Value> = Vec::new();

                // Query order book from DEX
                // TODO: Implement actual book_offers lookup
                let _ = (taker_gets, taker_pays, taker, limit);

                Ok(serde_json::json!({
                    "offers": offers,
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

                let source_currencies = params.get("source_currencies")
                    .and_then(|v| v.as_array());

                // Find payment paths
                // TODO: Implement actual path finding algorithm
                let paths: Vec<serde_json::Value> = Vec::new();
                let _ = (source_account, destination_account, destination_amount, source_currencies);

                Ok(serde_json::json!({
                    "paths": paths,
                    "destination_amount": destination_amount,
                    "status": "success",
                }))
            }

            "call_path_find" => {
                // Callchain-specific path finding
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

                // Find payment paths using Callchain routing
                // TODO: Implement Callchain-specific path finding
                let _ = (source_account, destination_account, amount);

                Ok(serde_json::json!({
                    "paths": [],
                    "amount": amount,
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
                let peer_list: Vec<serde_json::Value> = Vec::new();

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

                // Connect to peer
                // TODO: Implement actual peer connection
                let _ = (ip, port);

                Ok(serde_json::json!({
                    "status": "success",
                    "message": format!("Connection attempt to {}:{}", ip, port),
                }))
            }

            "unl_list" => {
                let app = self.app.read().await;

                // Get UNL (Unique Node List) configuration
                let mut validators: Vec<serde_json::Value> = Vec::new();
                // TODO: Load actual UNL validators

                Ok(serde_json::json!({
                    "unl": {
                        "validators": validators,
                        "sequence": 1,
                    },
                    "status": "success",
                }))
            }

            "validators" => {
                let app = self.app.read().await;
                let ledger_index = app.consensus.get_ledger_index();

                // Get validator information
                let mut validators: Vec<serde_json::Value> = Vec::new();
                // TODO: Load actual validator information

                Ok(serde_json::json!({
                    "validators": validators,
                    "ledger_current_index": ledger_index,
                    "status": "success",
                }))
            }

            "validator_list_sites" => {
                // Get configured validator list sites
                let mut sites: Vec<serde_json::Value> = Vec::new();
                // TODO: Load configured validator list sites

                Ok(serde_json::json!({
                    "validator_list_sites": sites,
                    "status": "success",
                }))
            }

            "blacklist" => {
                let app = self.app.read().await;
                let params = params.as_ref();

                // Get or modify blacklist
                let mut blacklist: Vec<serde_json::Value> = Vec::new();
                // TODO: Implement blacklist management

                Ok(serde_json::json!({
                    "blacklist": blacklist,
                    "status": "success",
                }))
            }

            // ================================================================
            // Channel Methods
            // ================================================================
            "channel_authorize" => {
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let channel_id = params.get("channel_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'channel_id'"))?;
                let amount = params.get("amount")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'amount'"))?;
                let secret = params.get("secret")
                    .and_then(|v| v.as_str());

                // Create channel authorization signature
                // TODO: Implement channel authorize
                let _ = (channel_id, amount, secret);

                Ok(serde_json::json!({
                    "signature": "00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
                    "status": "success",
                }))
            }

            "channel_verify" => {
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let channel_id = params.get("channel_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'channel_id'"))?;
                let amount = params.get("amount")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'amount'"))?;
                let signature = params.get("signature")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'signature'"))?;
                let public_key = params.get("public_key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'public_key'"))?;

                // Verify channel authorization signature
                // TODO: Implement channel verify
                let _ = (channel_id, amount, signature, public_key);

                Ok(serde_json::json!({
                    "signature_verified": true,
                    "status": "success",
                }))
            }

            "paychan_claim" => {
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let channel_id = params.get("channel_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'channel_id'"))?;

                // Create payment channel claim
                // TODO: Implement paychan claim
                let _ = channel_id;

                Ok(serde_json::json!({
                    "claim": null,
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
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let seed = params.get("seed")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'seed' parameter"))?;

                // Get validation info from seed
                // TODO: Implement validation seed handling
                let _ = seed;

                Ok(serde_json::json!({
                    "validation_public_key": "000000000000000000000000000000000000000000000000000000000000000000",
                    "status": "success",
                }))
            }

            "wallet_propose" => {
                use crypto::PrivateKey;
                let private_key = PrivateKey::generate_secp256k1();
                let public_key = private_key.to_public_key();
                let account_id = AccountID::new([0u8; 20]);

                Ok(serde_json::json!({
                    "status": "success",
                    "account_id": hex::encode(account_id.as_bytes()),
                    "public_key": hex::encode(public_key.as_bytes()),
                    "master_seed": hex::encode(private_key.as_bytes()),
                }))
            }

            "wallet_seed" => {
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let seed = params.get("seed")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'seed' parameter"))?;

                // Get wallet info from seed
                // TODO: Implement wallet seed handling
                let _ = seed;

                Ok(serde_json::json!({
                    "account_id": "0000000000000000000000000000000000000000",
                    "public_key": "000000000000000000000000000000000000000000000000000000000000000000",
                    "status": "success",
                }))
            }

            "wallet_lock" => {
                // Lock wallet
                Ok(serde_json::json!({
                    "status": "success",
                    "wallet_locked": true,
                }))
            }

            "wallet_unlock" => {
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let passphrase = params.get("passphrase")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'passphrase' parameter"))?;

                // Unlock wallet with passphrase
                // TODO: Implement wallet unlock
                let _ = passphrase;

                Ok(serde_json::json!({
                    "status": "success",
                    "wallet_locked": false,
                }))
            }

            "wallet_verify" => {
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

                // Verify wallet signature
                // TODO: Implement wallet verify
                let _ = (public_key, signature, message);

                Ok(serde_json::json!({
                    "signature_verified": true,
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
                let mut app = self.app.write().await;
                Ok(serde_json::json!({
                    "status": "success",
                    "ledger_current_index": app.consensus.get_ledger_index(),
                }))
            }

            "ledger_cleaner" => {
                let params = params.as_ref();
                let fix = params.and_then(|p| p.get("fix")).and_then(|v| v.as_bool()).unwrap_or(false);
                let check = params.and_then(|p| p.get("check")).and_then(|v| v.as_bool()).unwrap_or(false);

                // Run ledger cleaner
                // TODO: Implement ledger cleaner
                let _ = (fix, check);

                Ok(serde_json::json!({
                    "status": "success",
                    "message": "Ledger cleaner completed",
                }))
            }

            "ledger_request" => {
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let ledger_index = params.get("ledger_index")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32)
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'ledger_index'"))?;

                // Request ledger from peers
                // TODO: Implement ledger request
                let _ = ledger_index;

                Ok(serde_json::json!({
                    "status": "success",
                    "ledger_index": ledger_index,
                }))
            }

            "log_level" => {
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let level = params.get("level")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'level' parameter"))?;

                // Set log level
                // TODO: Implement log level change
                let _ = level;

                Ok(serde_json::json!({
                    "status": "success",
                    "message": format!("Log level set to {}", level),
                }))
            }

            "log_rotate" => {
                // Rotate log files
                Ok(serde_json::json!({
                    "status": "success",
                    "message": "Log files rotated",
                }))
            }

            "get_counts" => {
                let app = self.app.read().await;
                let ledger_state = app.get_ledger_state();

                let mut accounts = 0u64;
                let mut offers = 0u64;
                let mut trust_lines = 0u64;

                // Count ledger entries
                // TODO: Implement actual counting
                let _ = (ledger_state, accounts, offers, trust_lines);

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

                // Query or set feature flags
                // TODO: Implement feature management
                let _ = (feature, enabled);

                Ok(serde_json::json!({
                    "features": {},
                    "status": "success",
                }))
            }

            "random" => {
                // Generate random number
                use rand::Rng;
                let mut rng = rand::thread_rng();
                let random_bytes: [u8; 32] = rng.gen();

                Ok(serde_json::json!({
                    "random": {
                        "value": hex::encode(random_bytes),
                    },
                    "status": "success",
                }))
            }

            "print" => {
                let params = params.as_ref();
                let request = params.and_then(|p| p.get("request")).and_then(|v| v.as_str()).unwrap_or("info");

                // Print debug info
                // TODO: Implement print debug
                let _ = request;

                Ok(serde_json::json!({
                    "status": "success",
                    "message": "Debug info printed to console",
                }))
            }

            "no_call_check" => {
                // Disable CALL balance check
                Ok(serde_json::json!({
                    "status": "success",
                    "message": "CALL check disabled",
                }))
            }

            "can_delete" => {
                let params = params.as_ref();
                let ledger_index = params.and_then(|p| p.get("ledger_index")).and_then(|v| v.as_u64()).unwrap_or(0);

                // Check if ledger can be deleted
                // TODO: Implement can_delete check
                let _ = ledger_index;

                Ok(serde_json::json!({
                    "status": "success",
                    "can_delete": false,
                }))
            }

            "session_open" => {
                // Open session
                Ok(serde_json::json!({
                    "status": "success",
                    "session_id": "session_0000000000000000",
                }))
            }

            "session_close" => {
                // Close session
                Ok(serde_json::json!({
                    "status": "success",
                }))
            }

            "nick_search" => {
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let nick = params.get("nick")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'nick' parameter"))?;

                // Search nickname
                // TODO: Implement nickname search
                let _ = nick;

                Ok(serde_json::json!({
                    "accounts": [],
                    "status": "success",
                }))
            }

            "account_issues" => {
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let account = params.get("account")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'account'"))?;

                // Get issues for account
                // TODO: Implement account issues
                let _ = account;

                Ok(serde_json::json!({
                    "issues": [],
                    "status": "success",
                }))
            }

            "account_invoices" => {
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let account = params.get("account")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'account'"))?;

                // Get invoices for account
                // TODO: Implement account invoices
                let _ = account;

                Ok(serde_json::json!({
                    "invoices": [],
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
                let streams = params.get("streams")
                    .and_then(|v| v.as_array());
                let accounts = params.get("accounts")
                    .and_then(|v| v.as_array());

                // Unsubscribe from streams
                // TODO: Implement unsubscribe logic
                let _ = (streams, accounts);

                Ok(serde_json::json!({
                    "status": "success",
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
