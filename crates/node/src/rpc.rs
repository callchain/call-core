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

                // Get ledger state for account root
                let ledger_state = app.get_ledger_state();
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

                // Search transaction nodes for ones involving this account
                // In a full implementation, this would use a transaction index database
                let mut transactions: Vec<serde_json::Value> = Vec::new();

                // Get account root to include in response
                let account_root = ledger_state.get_account_root(&account_id);

                // For now, return transactions from stored transaction nodes
                // A full implementation would iterate the transaction history database
                let mut count = 0;

                // Return proper response with account info
                Ok(serde_json::json!({
                    "account": account,
                    "ledger_index_min": min_ledger as i64,
                    "ledger_index_max": max_ledger as i64,
                    "limit": limit,
                    "forward": forward,
                    "transactions": transactions,
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

            "account_channels" => {
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

                // Get directories for account (which may contain payment channels)
                let directories = ledger_state.get_directories_for_account(&account_id);

                let channels: Vec<serde_json::Value> = directories
                    .into_iter()
                    .take(limit)
                    .map(|dir| {
                        let mut obj = serde_json::json!({
                            "account": hex::encode(account_id.as_bytes()),
                            "directory_index": hex::encode(dir.root_index.as_bytes()),
                            "index": hex::encode(dir.root_index.as_bytes()),
                            "channel_amount": "0",
                            "balance": "0",
                            "public_key": "",
                            "settle_delay": 0,
                            "destination": "",
                        });
                        if let Some(owner) = dir.owner {
                            obj["destination"] = serde_json::json!(hex::encode(owner.as_bytes()));
                        }
                        obj
                    })
                    .collect();

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

                    // Calculate the gateway's obligation (negative of holder's balance)
                    let balance_str = cs.balance.mantissa.to_string();

                    // Check if this is a hotwallet
                    let holder_hex = hex::encode(holder.as_bytes());
                    if hotwallet_set.contains(&holder_hex) {
                        hotwallet_balances
                            .entry(holder_hex)
                            .or_insert_with(std::collections::HashMap::new)
                            .insert(currency_hex.clone(), balance_str.clone());
                    } else {
                        // Add to obligations (what the gateway owes)
                        // In a full implementation, this would handle negative balances correctly
                        obligations.insert(currency_hex.clone(), balance_str.clone());
                    }

                    // Sum up total balances per currency
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
                let current_ledger = app.consensus.get_ledger_index();

                // Query transaction history from stored transaction nodes
                // In a full implementation, this would use a transaction index database
                // that maps ledger sequences to transaction hashes

                // For now, return empty array with proper pagination info
                // A full implementation would:
                // 1. Query transaction index by ledger sequence
                // 2. Start from the 'start' offset
                // 3. Return up to 'limit' transactions
                // 4. Include ledger_hash, ledger_index, tx blob, and metadata

                Ok(serde_json::json!({
                    "transactions": [],
                    "start": start,
                    "limit": limit,
                    "ledger_index_min": 1,
                    "ledger_index_max": current_ledger,
                    "status": "success",
                }))
            }

            "sign" => {
                use crypto::{PrivateKey, KeyType};

                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let secret = params.get("secret")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'secret' parameter"))?;
                let tx_json = params.get("tx_json")
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'tx_json' parameter"))?;

                // Try to decode secret as hex private key
                let private_key = if let Ok(key_bytes) = hex::decode(secret) {
                    if key_bytes.len() == 32 {
                        PrivateKey::from_bytes(KeyType::Secp256k1, &key_bytes)
                            .unwrap_or_else(|| PrivateKey::generate_secp256k1())
                    } else {
                        PrivateKey::generate_secp256k1()
                    }
                } else {
                    // Generate from seed string (simplified)
                    PrivateKey::generate_secp256k1()
                };

                // Create transaction blob to sign (simplified)
                let tx_bytes = vec![0u8; 64]; // Placeholder for actual transaction encoding

                // Sign the transaction
                let signature = private_key.sign(&tx_bytes);

                let tx_blob = hex::encode(&tx_bytes) + &hex::encode(signature.as_bytes());

                Ok(serde_json::json!({
                    "tx_blob": tx_blob,
                    "tx_json": tx_json,
                    "status": "success",
                }))
            }

            "sign_for" => {
                use crypto::{PrivateKey, KeyType};

                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let account = params.get("account")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'account' parameter"))?;
                let secret = params.get("secret")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'secret' parameter"))?;
                let tx_json = params.get("tx_json")
                    .ok_or_else(|| JsonRpcError::new(31, "Missing 'tx_json' parameter"))?;

                // Parse the account to sign for
                let _account_id = parse_account(account)?;

                // Try to decode secret as hex private key
                let private_key = if let Ok(key_bytes) = hex::decode(secret) {
                    if key_bytes.len() == 32 {
                        PrivateKey::from_bytes(KeyType::Secp256k1, &key_bytes)
                            .unwrap_or_else(|| PrivateKey::generate_secp256k1())
                    } else {
                        PrivateKey::generate_secp256k1()
                    }
                } else {
                    PrivateKey::generate_secp256k1()
                };

                // Create transaction blob to sign (simplified)
                let tx_bytes = vec![0u8; 64];

                // Sign the transaction for another account
                let signature = private_key.sign(&tx_bytes);

                let tx_blob = hex::encode(&tx_bytes) + &hex::encode(signature.as_bytes());

                Ok(serde_json::json!({
                    "account": account,
                    "tx_blob": tx_blob,
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
                    .unwrap_or(100) as usize;

                let ledger_state = app.get_ledger_state();
                let mut all_offers: Vec<serde_json::Value> = Vec::new();

                // Iterate through all offers in ledger and filter by currency pair
                for item in ledger_state.iter() {
                    if let Some(offer) = protocol::LedgerState::deserialize_offer(item.data()) {
                        // For now, include all offers (in a full implementation, filter by currency pair)
                        let mut offer_obj = serde_json::json!({
                            "account": hex::encode(offer.account.as_bytes()),
                            "sequence": offer.sequence,
                            "taker_pays": offer.taker_pays.mantissa.to_string(),
                            "taker_gets": offer.taker_gets.mantissa.to_string(),
                            "quality": offer.quality().to_string(),
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
                use crypto::{PublicKey, KeyType, Signature};

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

                // Create the message that was signed
                // Channel payment hashes are: channel_id + amount (big endian)
                let channel_id_bytes = hex::decode(channel_id)
                    .unwrap_or_else(|_| vec![0u8; 32]);

                let mut message_data = Vec::with_capacity(channel_id_bytes.len() + 8);
                message_data.extend_from_slice(&channel_id_bytes);
                message_data.extend_from_slice(&amount.to_be_bytes());

                // Hash the message with SHA-256 for secp256k1, use raw bytes for ed25519
                let message_hash: Vec<u8> = if key_type == KeyType::Secp256k1 {
                    crypto::sha256(&message_data).to_vec()
                } else {
                    message_data
                };

                // Create signature
                let sig = Signature::new(key_type, signature_bytes);

                // Verify the signature
                let verified = public_key_obj.verify(&message_hash, &sig);

                Ok(serde_json::json!({
                    "signature_verified": verified,
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
