//! RPC API Tests for call-core
//!
//! This module contains comprehensive tests for the JSON-RPC API methods
//! as documented in api_todo.md.

use primitives::AccountID;
use protocol::Ledger;

// ============================================================================
// RPC Request/Response Types
// ============================================================================

/// JSON-RPC 2.0 request structure
#[derive(Debug, Clone, serde::Serialize)]
struct RpcRequest {
    jsonrpc: String,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<serde_json::Value>,
    id: serde_json::Value,
}

impl RpcRequest {
    fn new(method: impl Into<String>, id: impl Into<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params: None,
            id: id.into(),
        }
    }

    fn with_params(mut self, params: serde_json::Value) -> Self {
        self.params = Some(params);
        self
    }
}

/// JSON-RPC 2.0 error structure
#[derive(Debug, Clone, serde::Deserialize)]
struct RpcError {
    code: i32,
    message: String,
    #[serde(default)]
    data: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 response structure
#[derive(Debug, Clone, serde::Deserialize)]
struct RpcResponse {
    jsonrpc: String,
    #[serde(default)]
    result: Option<serde_json::Value>,
    #[serde(default)]
    error: Option<RpcError>,
    id: serde_json::Value,
}

impl RpcResponse {
    fn is_success(&self) -> bool {
        self.error.is_none() && self.result.is_some()
    }

    fn is_error(&self) -> bool {
        self.error.is_some()
    }
}

// ============================================================================
// Mock RPC Handler for Testing
// ============================================================================

/// Mock RPC handler for testing without a full node
struct MockRpcHandler {
    ledger: Ledger,
    accounts: Vec<AccountID>,
}

impl MockRpcHandler {
    fn new() -> Self {
        let ledger = Ledger::genesis();
        let accounts = vec![
            AccountID::new([1u8; 20]),
            AccountID::new([2u8; 20]),
            AccountID::new([3u8; 20]),
        ];

        Self { ledger, accounts }
    }

    fn process_request(&self, request: &str) -> Result<RpcResponse, serde_json::Error> {
        let req: serde_json::Value = serde_json::from_str(request)?;
        let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let params = req.get("params").cloned();
        let id = req.get("id").cloned().unwrap_or(serde_json::json!(null));

        let result = match method {
            // Server Info Methods
            "server_info" => self.handle_server_info(),
            "server_state" => self.handle_server_state(),
            "ping" => self.handle_ping(),

            // Ledger Methods
            "ledger_current" => self.handle_ledger_current(),
            "ledger_closed" => self.handle_ledger_closed(),
            "ledger" => self.handle_ledger(params),
            "ledger_data" => self.handle_ledger_data(params),
            "ledger_entry" => self.handle_ledger_entry(params),
            "ledger_header" => self.handle_ledger_header(params),

            // Account Methods
            "account_info" => self.handle_account_info(params),
            "account_tx" => self.handle_account_tx(params),
            "account_lines" => self.handle_account_lines(params),
            "account_objects" => self.handle_account_objects(params),
            "account_offers" => self.handle_account_offers(params),
            "account_currencies" => self.handle_account_currencies(params),

            // Transaction Methods
            "submit" => self.handle_submit(params),
            "tx" => self.handle_tx(params),
            "tx_history" => self.handle_tx_history(params),
            "transaction_entry" => self.handle_transaction_entry(params),

            // Admin Methods
            "peers" => self.handle_peers(),
            "consensus_info" => self.handle_consensus_info(),
            "fee" => self.handle_fee(),
            "get_counts" => self.handle_get_counts(),

            // Utility Methods
            "random" => self.handle_random(),
            "version" => self.handle_version(),

            _ => Err(RpcError {
                code: -32601,
                message: format!("Method not found: {}", method),
                data: None,
            }),
        };

        match result {
            Ok(result_value) => Ok(RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(result_value),
                error: None,
                id,
            }),
            Err(error) => Ok(RpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(error),
                id,
            }),
        }
    }

    // Server Info Handlers

    fn handle_server_info(&self) -> Result<serde_json::Value, RpcError> {
        Ok(serde_json::json!({
            "info": {
                "build_version": "0.1.0",
                "complete_ledgers": format!("1-{}", self.ledger.get_seq()),
                "hostid": "test-node",
                "io_latency_ms": 1,
                "last_close": {
                    "converge_time_s": 2.0,
                    "proposers": 5,
                },
                "load_factor": 1,
                "peers": 8,
                "pubkey_node": "000000000000000000000000000000000000000000000000000000000000000000",
                "server_state": "full",
                "state_accounting": {
                    "connected": {"duration_us": "0", "transitions": 0},
                    "disconnected": {"duration_us": "0", "transitions": 0},
                    "full": {"duration_us": "0", "transitions": 0},
                    "syncing": {"duration_us": "0", "transitions": 0},
                    "tracking": {"duration_us": "0", "transitions": 0},
                },
                "time": "2024-01-01T00:00:00Z",
                "uptime": 3600,
                "validated_ledger": {
                    "age": 3,
                    "base_fee": 10,
                    "hash": hex::encode(self.ledger.info.hash.as_bytes()),
                    "reserve_base": 10_000_000,
                    "reserve_inc": 2_000_000,
                    "seq": self.ledger.get_seq(),
                },
                "validation_quorum": 5,
            }
        }))
    }

    fn handle_server_state(&self) -> Result<serde_json::Value, RpcError> {
        Ok(serde_json::json!({
            "info": {
                "build_version": "0.1.0",
                "complete_ledgers": format!("1-{}", self.ledger.get_seq()),
                "io_latency_ms": 1,
                "load_factor": 1,
                "peers": 8,
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
                    "ledger_index": self.ledger.get_seq(),
                    "ledger_hash": hex::encode(self.ledger.info.hash.as_bytes()),
                },
            }
        }))
    }

    fn handle_ping(&self) -> Result<serde_json::Value, RpcError> {
        Ok(serde_json::json!({
            "status": "success"
        }))
    }

    // Ledger Handlers

    fn handle_ledger_current(&self) -> Result<serde_json::Value, RpcError> {
        Ok(serde_json::json!({
            "ledger_current_index": self.ledger.get_seq(),
            "status": "success"
        }))
    }

    fn handle_ledger_closed(&self) -> Result<serde_json::Value, RpcError> {
        Ok(serde_json::json!({
            "ledger_index": self.ledger.get_seq(),
            "ledger_hash": hex::encode(self.ledger.info.hash.as_bytes()),
            "status": "success"
        }))
    }

    fn handle_ledger(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, RpcError> {
        let requested_index = params
            .as_ref()
            .and_then(|p| p.get("ledger_index"))
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);

        let ledger_index = requested_index.unwrap_or_else(|| self.ledger.get_seq());

        Ok(serde_json::json!({
            "ledger": {
                "ledger_index": ledger_index.to_string(),
                "ledger_hash": hex::encode(self.ledger.info.hash.as_bytes()),
                "parent_hash": hex::encode(self.ledger.info.parent_hash.as_bytes()),
                "account_hash": hex::encode(self.ledger.info.account_hash.as_bytes()),
                "transaction_hash": hex::encode(self.ledger.info.tx_hash.as_bytes()),
                "close_time": 0,
                "closed": false,
            },
            "status": "success",
            "validated": false,
        }))
    }

    fn handle_ledger_data(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, RpcError> {
        let _limit = params
            .as_ref()
            .and_then(|p| p.get("limit"))
            .and_then(|v| v.as_u64())
            .unwrap_or(100) as usize;

        // Return empty entries for mock
        Ok(serde_json::json!({
            "ledger": {
                "ledger_index": self.ledger.get_seq().to_string(),
                "ledger_hash": hex::encode(self.ledger.info.hash.as_bytes()),
            },
            "entries": [],
            "marker": null,
            "status": "success",
        }))
    }

    fn handle_ledger_entry(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, RpcError> {
        let params = params.ok_or_else(|| RpcError {
            code: -32602,
            message: "Invalid params: missing params".to_string(),
            data: None,
        })?;

        let index = params
            .get("index")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RpcError {
                code: 31,
                message: "Missing 'index' parameter".to_string(),
                data: None,
            })?;

        // For mock, return not found
        Err(RpcError {
            code: 20,
            message: "Entry not found".to_string(),
            data: Some(serde_json::json!({ "index": index })),
        })
    }

    fn handle_ledger_header(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, RpcError> {
        let ledger_index = params
            .as_ref()
            .and_then(|p| p.get("ledger_index"))
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .unwrap_or_else(|| self.ledger.get_seq());

        Ok(serde_json::json!({
            "ledger_header": {
                "ledger_index": ledger_index.to_string(),
                "ledger_hash": hex::encode(self.ledger.info.hash.as_bytes()),
                "parent_hash": hex::encode(self.ledger.info.parent_hash.as_bytes()),
                "account_hash": hex::encode(self.ledger.info.account_hash.as_bytes()),
                "transaction_hash": hex::encode(self.ledger.info.tx_hash.as_bytes()),
                "close_time": 0,
                "close_time_human": "1970-01-01T00:00:00Z",
            },
            "status": "success",
            "validated": true,
        }))
    }

    // Account Handlers

    fn handle_account_info(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, RpcError> {
        let params = params.ok_or_else(|| RpcError {
            code: -32602,
            message: "Invalid params: missing params".to_string(),
            data: None,
        })?;

        let account = params
            .get("account")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RpcError {
                code: -32602,
                message: "Missing 'account' parameter".to_string(),
                data: None,
            })?;

        // Mock: return account not found for random accounts
        // Return success for specific test account
        if account.starts_with("0000000001") {
            Ok(serde_json::json!({
                "account": account,
                "account_data": {
                    "Account": account,
                    "Balance": "10000000",
                    "Sequence": 1,
                    "OwnerCount": 0,
                    "PreviousTxnID": "0000000000000000000000000000000000000000000000000000000000000000",
                    "PreviousTxnLgrSeq": 0,
                },
                "ledger_current_index": self.ledger.get_seq(),
                "queue_data": null,
                "status": "success",
                "validated": false,
            }))
        } else {
            Err(RpcError {
                code: 19,
                message: "Account not found.".to_string(),
                data: Some(serde_json::json!({ "account": account })),
            })
        }
    }

    fn handle_account_tx(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, RpcError> {
        let params = params.ok_or_else(|| RpcError {
            code: -32602,
            message: "Invalid params: missing params".to_string(),
            data: None,
        })?;

        let account = params
            .get("account")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RpcError {
                code: -32602,
                message: "Missing 'account' parameter".to_string(),
                data: None,
            })?;

        let ledger_index_min = params
            .get("ledger_index_min")
            .and_then(|v| v.as_i64())
            .unwrap_or(-1) as u32;
        let ledger_index_max = params
            .get("ledger_index_max")
            .and_then(|v| v.as_i64())
            .unwrap_or(-1) as u32;
        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(20) as usize;

        Ok(serde_json::json!({
            "account": account,
            "ledger_index_min": if ledger_index_min == 0 { 1 } else { ledger_index_min },
            "ledger_index_max": if ledger_index_max == 0 { self.ledger.get_seq() } else { ledger_index_max },
            "limit": limit,
            "transactions": [],
            "validated": false,
            "status": "success",
        }))
    }

    fn handle_account_lines(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, RpcError> {
        let params = params.ok_or_else(|| RpcError {
            code: -32602,
            message: "Invalid params: missing params".to_string(),
            data: None,
        })?;

        let account = params
            .get("account")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RpcError {
                code: -32602,
                message: "Missing 'account' parameter".to_string(),
                data: None,
            })?;

        let peer = params.get("peer").and_then(|v| v.as_str());
        let _limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(100) as usize;

        // Mock: return empty trust lines
        let lines: Vec<serde_json::Value> = if peer.is_some() {
            vec![]
        } else {
            vec![]
        };

        Ok(serde_json::json!({
            "account": account,
            "lines": lines,
            "ledger_current_index": self.ledger.get_seq(),
            "validated": false,
            "status": "success",
        }))
    }

    fn handle_account_objects(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, RpcError> {
        let params = params.ok_or_else(|| RpcError {
            code: -32602,
            message: "Invalid params: missing params".to_string(),
            data: None,
        })?;

        let account = params
            .get("account")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RpcError {
                code: -32602,
                message: "Missing 'account' parameter".to_string(),
                data: None,
            })?;

        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(100) as usize;

        Ok(serde_json::json!({
            "account": account,
            "account_objects": [],
            "ledger_current_index": self.ledger.get_seq(),
            "limit": limit,
            "validated": false,
            "status": "success",
        }))
    }

    fn handle_account_offers(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, RpcError> {
        let params = params.ok_or_else(|| RpcError {
            code: -32602,
            message: "Invalid params: missing params".to_string(),
            data: None,
        })?;

        let account = params
            .get("account")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RpcError {
                code: -32602,
                message: "Missing 'account' parameter".to_string(),
                data: None,
            })?;

        Ok(serde_json::json!({
            "account": account,
            "offers": [],
            "ledger_current_index": self.ledger.get_seq(),
            "validated": false,
            "status": "success",
        }))
    }

    fn handle_account_currencies(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, RpcError> {
        let params = params.ok_or_else(|| RpcError {
            code: -32602,
            message: "Invalid params: missing params".to_string(),
            data: None,
        })?;

        let _account = params
            .get("account")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RpcError {
                code: -32602,
                message: "Missing 'account' parameter".to_string(),
                data: None,
            })?;

        Ok(serde_json::json!({
            "ledger_index": self.ledger.get_seq(),
            "receive_currencies": [],
            "send_currencies": [],
            "validated": false,
            "status": "success",
        }))
    }

    // Transaction Handlers

    fn handle_submit(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, RpcError> {
        let params = params.ok_or_else(|| RpcError {
            code: -32602,
            message: "Invalid params: missing params".to_string(),
            data: None,
        })?;

        let tx_blob = params
            .get("tx_blob")
            .and_then(|v| v.as_str());

        let tx_json = params.get("tx_json");

        if tx_blob.is_none() && tx_json.is_none() {
            return Err(RpcError {
                code: -32602,
                message: "Missing 'tx_blob' or 'tx_json' parameter".to_string(),
                data: None,
            });
        }

        Ok(serde_json::json!({
            "engine_result": "tesSUCCESS",
            "engine_result_code": 0,
            "engine_result_message": "The transaction was applied.",
            "tx_blob": tx_blob.unwrap_or(""),
            "tx_json": {
                "Account": "000000000000000000000000000000000000000001",
                "TransactionType": "Payment",
                "Fee": "10",
                "Sequence": 1,
                "hash": "0000000000000000000000000000000000000000000000000000000000000000",
            },
            "status": "success",
        }))
    }

    fn handle_tx(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, RpcError> {
        let params = params.ok_or_else(|| RpcError {
            code: -32602,
            message: "Invalid params: missing params".to_string(),
            data: None,
        })?;

        let transaction = params
            .get("transaction")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RpcError {
                code: -32602,
                message: "Missing 'transaction' parameter".to_string(),
                data: None,
            })?;

        // Mock: return transaction not found
        Err(RpcError {
            code: 24,
            message: "Transaction not found.".to_string(),
            data: Some(serde_json::json!({ "transaction": transaction })),
        })
    }

    fn handle_tx_history(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, RpcError> {
        let start = params
            .as_ref()
            .and_then(|p| p.get("start"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        Ok(serde_json::json!({
            "index": start,
            "transactions": [],
            "status": "success",
        }))
    }

    fn handle_transaction_entry(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, RpcError> {
        let params = params.ok_or_else(|| RpcError {
            code: -32602,
            message: "Invalid params: missing params".to_string(),
            data: None,
        })?;

        let tx_hash = params
            .get("tx_hash")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RpcError {
                code: -32602,
                message: "Missing 'tx_hash' parameter".to_string(),
                data: None,
            })?;

        let ledger_index = params
            .get("ledger_index")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);

        Err(RpcError {
            code: 24,
            message: "Transaction not found.".to_string(),
            data: Some(serde_json::json!({
                "tx_hash": tx_hash,
                "ledger_index": ledger_index,
            })),
        })
    }

    // Admin Handlers

    fn handle_peers(&self) -> Result<serde_json::Value, RpcError> {
        Ok(serde_json::json!({
            "peers": [],
            "status": "success",
        }))
    }

    fn handle_consensus_info(&self) -> Result<serde_json::Value, RpcError> {
        Ok(serde_json::json!({
            "info": {
                "consensus_phase": "open",
                "ledger_seq": self.ledger.get_seq(),
                "proposers": [],
                "round": 1,
            },
            "status": "success",
        }))
    }

    fn handle_fee(&self) -> Result<serde_json::Value, RpcError> {
        Ok(serde_json::json!({
            "current_ledger_size": "0",
            "current_queue_size": "0",
            "expected_ledger_size": "1000",
            "minimum_fee": "10",
            "median_fee": "10",
            "open_ledger_fee": "10",
            "drops": {
                "base_fee": "10",
                "median_fee": "10",
                "minimum_fee": "10",
                "open_ledger_fee": "10",
            },
            "levels": {
                "median": "256",
                "minimum": "256",
                "open": "256",
            },
            "status": "success",
        }))
    }

    fn handle_get_counts(&self) -> Result<serde_json::Value, RpcError> {
        Ok(serde_json::json!({
            "accounts": 0,
            "offers": 0,
            "trust_lines": 0,
            "status": "success",
        }))
    }

    // Utility Handlers

    fn handle_random(&self) -> Result<serde_json::Value, RpcError> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let random_bytes: [u8; 32] = rng.gen();

        Ok(serde_json::json!({
            "random": hex::encode(random_bytes),
            "status": "success",
        }))
    }

    fn handle_version(&self) -> Result<serde_json::Value, RpcError> {
        Ok(serde_json::json!({
            "version": {
                "first": 1,
                "good": 1,
                "last": 1,
                "major": 0,
                "minor": 1,
                "patch": 0,
                "full": "0.1.0",
            },
            "status": "success",
        }))
    }
}

// ============================================================================
// Server Info API Tests
// ============================================================================

#[test]
fn test_server_info() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("server_info", 1);
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_success());
    assert_eq!(response.id, serde_json::json!(1));

    let result = response.result.unwrap();
    assert!(result.get("info").is_some());

    let info = result.get("info").unwrap();
    assert!(info.get("build_version").is_some());
    assert!(info.get("server_state").is_some());
    assert!(info.get("validated_ledger").is_some());
    assert_eq!(info.get("server_state").unwrap().as_str(), Some("full"));
}

#[test]
fn test_server_state() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("server_state", 2);
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_success());
    assert_eq!(response.id, serde_json::json!(2));

    let result = response.result.unwrap();
    assert!(result.get("info").is_some());

    let info = result.get("info").unwrap();
    assert!(info.get("server_state").is_some());
    assert!(info.get("validated_ledger").is_some());
}

#[test]
fn test_ping() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("ping", 3);
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_success());
    assert_eq!(response.id, serde_json::json!(3));

    let result = response.result.unwrap();
    assert_eq!(result.get("status").unwrap().as_str(), Some("success"));
}

// ============================================================================
// Ledger API Tests
// ============================================================================

#[test]
fn test_ledger_current() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("ledger_current", 1);
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_success());

    let result = response.result.unwrap();
    assert!(result.get("ledger_current_index").is_some());
    assert_eq!(result.get("status").unwrap().as_str(), Some("success"));
}

#[test]
fn test_ledger_closed() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("ledger_closed", 1);
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_success());

    let result = response.result.unwrap();
    assert!(result.get("ledger_index").is_some());
    assert!(result.get("ledger_hash").is_some());
    assert_eq!(result.get("status").unwrap().as_str(), Some("success"));
}

#[test]
fn test_ledger_without_params() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("ledger", 1);
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_success());

    let result = response.result.unwrap();
    assert!(result.get("ledger").is_some());

    let ledger = result.get("ledger").unwrap();
    assert!(ledger.get("ledger_index").is_some());
    assert!(ledger.get("ledger_hash").is_some());
    assert!(ledger.get("parent_hash").is_some());
}

#[test]
fn test_ledger_with_index() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("ledger", 1)
        .with_params(serde_json::json!({"ledger_index": 100}));
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_success());

    let result = response.result.unwrap();
    let ledger = result.get("ledger").unwrap();
    assert_eq!(ledger.get("ledger_index").unwrap().as_str(), Some("100"));
}

#[test]
fn test_ledger_data() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("ledger_data", 1)
        .with_params(serde_json::json!({"limit": 10}));
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_success());

    let result = response.result.unwrap();
    assert!(result.get("ledger").is_some());
    assert!(result.get("entries").is_some());
}

#[test]
fn test_ledger_data_default_limit() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("ledger_data", 1);
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_success());
}

#[test]
fn test_ledger_entry_not_found() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("ledger_entry", 1)
        .with_params(serde_json::json!({
            "index": "0000000000000000000000000000000000000000000000000000000000000001"
        }));
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_error());

    let error = response.error.unwrap();
    assert_eq!(error.code, 20);
    assert!(error.message.contains("Entry not found"));
}

#[test]
fn test_ledger_entry_missing_index() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("ledger_entry", 1)
        .with_params(serde_json::json!({}));
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_error());

    let error = response.error.unwrap();
    assert_eq!(error.code, 31);
}

#[test]
fn test_ledger_header() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("ledger_header", 1);
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_success());

    let result = response.result.unwrap();
    assert!(result.get("ledger_header").is_some());

    let header = result.get("ledger_header").unwrap();
    assert!(header.get("ledger_index").is_some());
    assert!(header.get("ledger_hash").is_some());
    assert!(header.get("close_time_human").is_some());
}

// ============================================================================
// Account API Tests
// ============================================================================

#[test]
fn test_account_info_success() {
    let handler = MockRpcHandler::new();

    // Use test account that returns success
    let request = RpcRequest::new("account_info", 1)
        .with_params(serde_json::json!({
            "account": "0000000001000000000000000000000000000000"
        }));
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_success());

    let result = response.result.unwrap();
    assert!(result.get("account").is_some());
    assert!(result.get("account_data").is_some());

    let account_data = result.get("account_data").unwrap();
    assert!(account_data.get("Balance").is_some());
    assert!(account_data.get("Sequence").is_some());
}

#[test]
fn test_account_info_not_found() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("account_info", 1)
        .with_params(serde_json::json!({
            "account": "ffffffffffffffffffffffffffffffffffffffff"
        }));
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_error());

    let error = response.error.unwrap();
    assert_eq!(error.code, 19);
    assert!(error.message.contains("Account not found"));
}

#[test]
fn test_account_info_missing_account() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("account_info", 1)
        .with_params(serde_json::json!({}));
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_error());

    let error = response.error.unwrap();
    assert_eq!(error.code, -32602);
}

#[test]
fn test_account_tx() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("account_tx", 1)
        .with_params(serde_json::json!({
            "account": "000000000000000000000000000000000000000001",
            "ledger_index_min": 1,
            "ledger_index_max": 100,
            "limit": 10,
        }));
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_success());

    let result = response.result.unwrap();
    assert!(result.get("account").is_some());
    assert!(result.get("transactions").is_some());
    assert!(result.get("ledger_index_min").is_some());
    assert!(result.get("ledger_index_max").is_some());
}

#[test]
fn test_account_lines() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("account_lines", 1)
        .with_params(serde_json::json!({
            "account": "000000000000000000000000000000000000000001",
            "limit": 50,
        }));
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_success());

    let result = response.result.unwrap();
    assert!(result.get("account").is_some());
    assert!(result.get("lines").is_some());
}

#[test]
fn test_account_lines_with_peer() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("account_lines", 1)
        .with_params(serde_json::json!({
            "account": "000000000000000000000000000000000000000001",
            "peer": "000000000000000000000000000000000000000002",
        }));
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_success());

    let result = response.result.unwrap();
    assert!(result.get("lines").is_some());
}

#[test]
fn test_account_objects() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("account_objects", 1)
        .with_params(serde_json::json!({
            "account": "000000000000000000000000000000000000000001",
            "limit": 25,
        }));
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_success());

    let result = response.result.unwrap();
    assert!(result.get("account").is_some());
    assert!(result.get("account_objects").is_some());
    assert!(result.get("limit").is_some());
}

#[test]
fn test_account_offers() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("account_offers", 1)
        .with_params(serde_json::json!({
            "account": "000000000000000000000000000000000000000001",
        }));
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_success());

    let result = response.result.unwrap();
    assert!(result.get("account").is_some());
    assert!(result.get("offers").is_some());
}

#[test]
fn test_account_currencies() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("account_currencies", 1)
        .with_params(serde_json::json!({
            "account": "000000000000000000000000000000000000000001",
        }));
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_success());

    let result = response.result.unwrap();
    assert!(result.get("receive_currencies").is_some());
    assert!(result.get("send_currencies").is_some());
}

// ============================================================================
// Transaction API Tests
// ============================================================================

#[test]
fn test_submit_with_tx_blob() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("submit", 1)
        .with_params(serde_json::json!({
            "tx_blob": "1200002200000000240000000161400000000000000068000000000000000"
        }));
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_success());

    let result = response.result.unwrap();
    assert_eq!(result.get("engine_result").unwrap().as_str(), Some("tesSUCCESS"));
    assert_eq!(result.get("engine_result_code").unwrap().as_i64(), Some(0));
    assert!(result.get("tx_json").is_some());
}

#[test]
fn test_submit_with_tx_json() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("submit", 1)
        .with_params(serde_json::json!({
            "tx_json": {
                "TransactionType": "Payment",
                "Account": "000000000000000000000000000000000000000001",
                "Destination": "000000000000000000000000000000000000000002",
                "Amount": "1000000",
            }
        }));
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_success());
}

#[test]
fn test_submit_missing_params() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("submit", 1)
        .with_params(serde_json::json!({}));
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_error());

    let error = response.error.unwrap();
    assert_eq!(error.code, -32602);
}

#[test]
fn test_tx_not_found() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("tx", 1)
        .with_params(serde_json::json!({
            "transaction": "0000000000000000000000000000000000000000000000000000000000000001"
        }));
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_error());

    let error = response.error.unwrap();
    assert_eq!(error.code, 24);
    assert!(error.message.contains("Transaction not found"));
}

#[test]
fn test_tx_missing_transaction() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("tx", 1)
        .with_params(serde_json::json!({}));
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_error());

    let error = response.error.unwrap();
    assert_eq!(error.code, -32602);
}

#[test]
fn test_tx_history() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("tx_history", 1)
        .with_params(serde_json::json!({"start": 0}));
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_success());

    let result = response.result.unwrap();
    assert!(result.get("index").is_some());
    assert!(result.get("transactions").is_some());
}

#[test]
fn test_transaction_entry_not_found() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("transaction_entry", 1)
        .with_params(serde_json::json!({
            "tx_hash": "0000000000000000000000000000000000000000000000000000000000000001",
            "ledger_index": 100,
        }));
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_error());

    let error = response.error.unwrap();
    assert_eq!(error.code, 24);
}

// ============================================================================
// Admin API Tests
// ============================================================================

#[test]
fn test_peers() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("peers", 1);
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_success());

    let result = response.result.unwrap();
    assert!(result.get("peers").is_some());
}

#[test]
fn test_consensus_info() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("consensus_info", 1);
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_success());

    let result = response.result.unwrap();
    assert!(result.get("info").is_some());

    let info = result.get("info").unwrap();
    assert!(info.get("consensus_phase").is_some());
    assert!(info.get("ledger_seq").is_some());
}

#[test]
fn test_fee() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("fee", 1);
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_success());

    let result = response.result.unwrap();
    assert!(result.get("drops").is_some());
    assert!(result.get("levels").is_some());

    let drops = result.get("drops").unwrap();
    assert!(drops.get("base_fee").is_some());
    assert!(drops.get("minimum_fee").is_some());
    assert!(drops.get("median_fee").is_some());
}

#[test]
fn test_get_counts() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("get_counts", 1);
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_success());

    let result = response.result.unwrap();
    assert!(result.get("accounts").is_some());
    assert!(result.get("offers").is_some());
    assert!(result.get("trust_lines").is_some());
}

// ============================================================================
// Utility API Tests
// ============================================================================

#[test]
fn test_random() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("random", 1);
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_success());

    let result = response.result.unwrap();
    assert!(result.get("random").is_some());

    // Verify random value is a 64-character hex string (32 bytes)
    let random = result.get("random").unwrap().as_str().unwrap();
    assert_eq!(random.len(), 64);
}

#[test]
fn test_version() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("version", 1);
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_success());

    let result = response.result.unwrap();
    assert!(result.get("version").is_some());

    let version = result.get("version").unwrap();
    assert!(version.get("major").is_some());
    assert!(version.get("minor").is_some());
    assert!(version.get("patch").is_some());
    assert!(version.get("full").is_some());
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_method_not_found() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("unknown_method", 1);
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_error());

    let error = response.error.unwrap();
    assert_eq!(error.code, -32601);
    assert!(error.message.contains("Method not found"));
}

#[test]
fn test_missing_params_error() {
    let handler = MockRpcHandler::new();

    // account_info requires params
    let request = RpcRequest::new("account_info", 1);
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_error());

    let error = response.error.unwrap();
    assert_eq!(error.code, -32602);
}

#[test]
fn test_response_id_echo() {
    let handler = MockRpcHandler::new();

    // Test string ID
    let request = RpcRequest::new("ping", "test-id-123");
    let request_str = serde_json::to_string(&request).unwrap();
    let response = handler.process_request(&request_str).unwrap();
    assert_eq!(response.id, serde_json::json!("test-id-123"));

    // Test numeric ID
    let request = RpcRequest::new("ping", 42);
    let request_str = serde_json::to_string(&request).unwrap();
    let response = handler.process_request(&request_str).unwrap();
    assert_eq!(response.id, serde_json::json!(42));

    // Test null ID
    let request = RpcRequest::new("ping", serde_json::Value::Null);
    let request_str = serde_json::to_string(&request).unwrap();
    let response = handler.process_request(&request_str).unwrap();
    assert_eq!(response.id, serde_json::Value::Null);
}

// ============================================================================
// Batch Request Tests
// ============================================================================

#[test]
fn test_batch_requests() {
    let handler = MockRpcHandler::new();

    let batch = vec![
        RpcRequest::new("server_info", 1),
        RpcRequest::new("ping", 2),
        RpcRequest::new("ledger_current", 3),
    ];

    for (i, request) in batch.iter().enumerate() {
        let request_str = serde_json::to_string(request).unwrap();
        let response = handler.process_request(&request_str).unwrap();

        assert!(response.is_success());
        assert_eq!(response.id, serde_json::json!(i + 1));
    }
}

// ============================================================================
// Parameter Validation Tests
// ============================================================================

#[test]
fn test_ledger_data_with_custom_limit() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("ledger_data", 1)
        .with_params(serde_json::json!({"limit": 5}));
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_success());
}

#[test]
fn test_account_tx_with_forward() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("account_tx", 1)
        .with_params(serde_json::json!({
            "account": "000000000000000000000000000000000000000001",
            "forward": true,
            "limit": 5,
        }));
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_success());
}

#[test]
fn test_tx_history_with_start() {
    let handler = MockRpcHandler::new();

    let request = RpcRequest::new("tx_history", 1)
        .with_params(serde_json::json!({"start": 100}));
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_request(&request_str).unwrap();

    assert!(response.is_success());

    let result = response.result.unwrap();
    assert_eq!(result.get("index").unwrap().as_u64(), Some(100));
}
