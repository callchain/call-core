//! WebSocket API Tests for call-core
//!
//! This module contains comprehensive tests for the WebSocket API
//! as documented in api_todo.md.

// ============================================================================
// WebSocket Message Types
// ============================================================================

/// WebSocket request message
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct WsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streams: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accounts: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ledger_index: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
}

impl WsRequest {
    fn new(command: impl Into<String>) -> Self {
        Self {
            id: None,
            command: command.into(),
            streams: None,
            accounts: None,
            ledger_index: None,
            account: None,
        }
    }

    fn with_id(mut self, id: u64) -> Self {
        self.id = Some(id);
        self
    }

    fn with_streams(mut self, streams: Vec<String>) -> Self {
        self.streams = Some(streams);
        self
    }

    fn with_accounts(mut self, accounts: Vec<String>) -> Self {
        self.accounts = Some(accounts);
        self
    }

    fn with_ledger_index(mut self, index: u32) -> Self {
        self.ledger_index = Some(index);
        self
    }

    fn with_account(mut self, account: String) -> Self {
        self.account = Some(account);
        self
    }
}

/// WebSocket response message
#[derive(Debug, Clone, serde::Deserialize)]
struct WsResponse {
    #[serde(default)]
    pub id: Option<u64>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub error_code: Option<i32>,
    #[serde(default)]
    pub r#type: Option<String>,
}

impl WsResponse {
    fn is_success(&self) -> bool {
        self.status.as_ref().map(|s| s == "success").unwrap_or(false)
            && self.error.is_none()
    }

    fn is_error(&self) -> bool {
        self.error.is_some()
            || self.status.as_ref().map(|s| s == "error").unwrap_or(false)
    }
}

// ============================================================================
// Mock WebSocket Handler
// ============================================================================

/// # Mock WebSocket Handler
///
/// This mock provides a lightweight WebSocket handler for unit testing the WebSocket API
/// without requiring a full node instance. It simulates subscription management and
/// responds to WebSocket messages with predetermined responses.
///
/// ## Purpose
/// - Test WebSocket message routing and command handling
/// - Test subscription/unsubscription logic
/// - Validate request/response formatting and serialization
/// - Enable fast unit tests without network or database dependencies
///
/// ## Limitations
/// - Does NOT execute real transactions or consensus
/// - Does NOT send real-time ledger updates (subscriptions are tracked but not acted upon)
/// - Uses simulated account data, not computed from transaction history
/// - Not suitable for integration tests requiring actual subscription streaming
///
/// ## Usage
/// ```rust
/// let mut handler = MockWsHandler::new();
/// let response = handler.process_message(rpc_json);
/// ```
///
/// ## When to Use
/// - Unit testing individual WebSocket command handlers
/// - Testing subscription management logic
/// - Testing request validation and error responses
/// - Testing JSON serialization/deserialization for WebSocket format
///
/// ## When NOT to Use
/// - Integration tests requiring real subscription streaming
/// - Tests needing actual ledger update notifications
/// - Tests requiring persistent state or transaction processing
///
/// # TODO: Consider using `BasicLedgerView` from `protocol::views` for more realistic state
struct MockWsHandler {
    subscriptions: Vec<String>,
    account_subscriptions: Vec<String>,
}

impl MockWsHandler {
    fn new() -> Self {
        Self {
            subscriptions: Vec::new(),
            account_subscriptions: Vec::new(),
        }
    }

    fn process_message(&mut self, message: &str) -> Result<WsResponse, serde_json::Error> {
        let req: WsRequest = serde_json::from_str(message)?;

        match req.command.as_str() {
            "subscribe" => self.handle_subscribe(req),
            "unsubscribe" => self.handle_unsubscribe(req),
            "ping" => self.handle_ping(req),
            "server_info" => self.handle_server_info(req),
            "ledger" => self.handle_ledger(req),
            "account_info" => self.handle_account_info(req),
            _ => Ok(WsResponse {
                id: req.id,
                status: Some("error".to_string()),
                result: None,
                error: Some(format!("Unknown command: {}", req.command)),
                error_code: Some(-1),
                r#type: None,
            }),
        }
    }

    fn handle_subscribe(&mut self, req: WsRequest) -> Result<WsResponse, serde_json::Error> {
        let mut subscribed_streams = Vec::new();

        // Subscribe to streams
        if let Some(streams) = req.streams {
            for stream in &streams {
                if !self.subscriptions.contains(stream) {
                    self.subscriptions.push(stream.clone());
                }
                subscribed_streams.push(stream.clone());
            }
        }

        // Subscribe to accounts
        if let Some(accounts) = req.accounts {
            for account in &accounts {
                if !self.account_subscriptions.contains(account) {
                    self.account_subscriptions.push(account.clone());
                }
            }
        }

        Ok(WsResponse {
            id: req.id,
            status: Some("success".to_string()),
            result: Some(serde_json::json!({
                "subscribed": subscribed_streams,
                "accounts_proposed": [],
            })),
            error: None,
            error_code: None,
            r#type: None,
        })
    }

    fn handle_unsubscribe(&mut self, req: WsRequest) -> Result<WsResponse, serde_json::Error> {
        let mut unsubscribed_streams = Vec::new();

        // Unsubscribe from streams
        if let Some(streams) = req.streams {
            for stream in &streams {
                if let Some(pos) = self.subscriptions.iter().position(|s| s == stream) {
                    self.subscriptions.remove(pos);
                    unsubscribed_streams.push(stream.clone());
                }
            }
        }

        // Unsubscribe from accounts
        if let Some(accounts) = req.accounts {
            for account in &accounts {
                if let Some(pos) = self.account_subscriptions.iter().position(|a| a == account) {
                    self.account_subscriptions.remove(pos);
                }
            }
        }

        Ok(WsResponse {
            id: req.id,
            status: Some("success".to_string()),
            result: Some(serde_json::json!({
                "unsubscribed": unsubscribed_streams,
            })),
            error: None,
            error_code: None,
            r#type: None,
        })
    }

    fn handle_ping(&self, req: WsRequest) -> Result<WsResponse, serde_json::Error> {
        Ok(WsResponse {
            id: req.id,
            status: Some("success".to_string()),
            result: Some(serde_json::json!({})),
            error: None,
            error_code: None,
            r#type: None,
        })
    }

    fn handle_server_info(&self, req: WsRequest) -> Result<WsResponse, serde_json::Error> {
        Ok(WsResponse {
            id: req.id,
            status: Some("success".to_string()),
            result: Some(serde_json::json!({
                "info": {
                    "build_version": "0.1.0",
                    "complete_ledgers": "1-100",
                    "hostid": "test-node",
                    "io_latency_ms": 1,
                    "last_close": {
                        "converge_time_s": 2.0,
                        "proposers": 5,
                    },
                    "load_factor": 1,
                    "peers": 8,
                    "server_state": "full",
                    "validated_ledger": {
                        "age": 3,
                        "base_fee": 10,
                        "hash": "0000000000000000000000000000000000000000000000000000000000000000",
                        "reserve_base": 10_000_000,
                        "reserve_inc": 2_000_000,
                        "seq": 100,
                    },
                }
            })),
            error: None,
            error_code: None,
            r#type: None,
        })
    }

    fn handle_ledger(&self, req: WsRequest) -> Result<WsResponse, serde_json::Error> {
        let ledger_index = req.ledger_index.unwrap_or(100);

        Ok(WsResponse {
            id: req.id,
            status: Some("success".to_string()),
            result: Some(serde_json::json!({
                "ledger": {
                    "ledger_index": ledger_index.to_string(),
                    "ledger_hash": "0000000000000000000000000000000000000000000000000000000000000000",
                    "parent_hash": "0000000000000000000000000000000000000000000000000000000000000000",
                    "account_hash": "0000000000000000000000000000000000000000000000000000000000000000",
                    "close_time": 0,
                },
                "validated": false,
            })),
            error: None,
            error_code: None,
            r#type: None,
        })
    }

    fn handle_account_info(&self, req: WsRequest) -> Result<WsResponse, serde_json::Error> {
        let account = match req.account {
            Some(acc) => acc,
            None => return Err(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Missing account parameter"
            ))),
        };

        Ok(WsResponse {
            id: req.id,
            status: Some("success".to_string()),
            result: Some(serde_json::json!({
                "account_data": {
                    "Account": account,
                    "Balance": "10000000",
                    "Sequence": 1,
                    "OwnerCount": 0,
                },
                "ledger_current_index": 100,
                "validated": false,
            })),
            error: None,
            error_code: None,
            r#type: None,
        })
    }

    fn get_subscriptions(&self) -> &[String] {
        &self.subscriptions
    }

    fn get_account_subscriptions(&self) -> &[String] {
        &self.account_subscriptions
    }
}

// ============================================================================
// WebSocket Command Tests
// ============================================================================

#[test]
fn test_ws_ping() {
    let mut handler = MockWsHandler::new();

    let request = WsRequest::new("ping").with_id(1);
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_message(&request_str).unwrap();

    assert!(response.is_success());
    assert_eq!(response.id, Some(1));
}

#[test]
fn test_ws_server_info() {
    let mut handler = MockWsHandler::new();

    let request = WsRequest::new("server_info").with_id(1);
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_message(&request_str).unwrap();

    assert!(response.is_success());
    assert_eq!(response.id, Some(1));

    let result = response.result.unwrap();
    assert!(result.get("info").is_some());

    let info = result.get("info").unwrap();
    assert!(info.get("build_version").is_some());
    assert!(info.get("server_state").is_some());
}

#[test]
fn test_ws_ledger() {
    let mut handler = MockWsHandler::new();

    let request = WsRequest::new("ledger")
        .with_id(1)
        .with_ledger_index(50);
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_message(&request_str).unwrap();

    assert!(response.is_success());

    let result = response.result.unwrap();
    assert!(result.get("ledger").is_some());

    let ledger = result.get("ledger").unwrap();
    assert_eq!(ledger.get("ledger_index").unwrap().as_str(), Some("50"));
}

#[test]
fn test_ws_account_info() {
    let mut handler = MockWsHandler::new();

    let request = WsRequest::new("account_info")
        .with_id(1)
        .with_account("000000000000000000000000000000000000000001".to_string());
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_message(&request_str).unwrap();

    assert!(response.is_success());

    let result = response.result.unwrap();
    assert!(result.get("account_data").is_some());
}

// ============================================================================
// WebSocket Subscription Tests
// ============================================================================

#[test]
fn test_subscribe_single_stream() {
    let mut handler = MockWsHandler::new();

    let request = WsRequest::new("subscribe")
        .with_id(1)
        .with_streams(vec!["ledger".to_string()]);
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_message(&request_str).unwrap();

    assert!(response.is_success());
    assert_eq!(handler.get_subscriptions(), &["ledger"]);

    let result = response.result.unwrap();
    let subscribed = result.get("subscribed").unwrap().as_array().unwrap();
    assert!(subscribed.contains(&serde_json::json!("ledger")));
}

#[test]
fn test_subscribe_multiple_streams() {
    let mut handler = MockWsHandler::new();

    let request = WsRequest::new("subscribe")
        .with_id(1)
        .with_streams(vec![
            "ledger".to_string(),
            "transactions".to_string(),
            "validations".to_string(),
        ]);
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_message(&request_str).unwrap();

    assert!(response.is_success());
    assert_eq!(handler.get_subscriptions().len(), 3);
    assert!(handler.get_subscriptions().contains(&"ledger".to_string()));
    assert!(handler.get_subscriptions().contains(&"transactions".to_string()));
    assert!(handler.get_subscriptions().contains(&"validations".to_string()));
}

#[test]
fn test_subscribe_all_stream_types() {
    let mut handler = MockWsHandler::new();

    let streams = vec![
        "ledger".to_string(),
        "transactions".to_string(),
        "validations".to_string(),
        "consensus".to_string(),
        "peer".to_string(),
    ];

    let request = WsRequest::new("subscribe")
        .with_id(1)
        .with_streams(streams.clone());
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_message(&request_str).unwrap();

    assert!(response.is_success());
    assert_eq!(handler.get_subscriptions().len(), 5);

    for stream in &streams {
        assert!(handler.get_subscriptions().contains(stream));
    }
}

#[test]
fn test_subscribe_accounts() {
    let mut handler = MockWsHandler::new();

    let accounts = vec![
        "000000000000000000000000000000000000000001".to_string(),
        "000000000000000000000000000000000000000002".to_string(),
    ];

    let request = WsRequest::new("subscribe")
        .with_id(1)
        .with_accounts(accounts.clone());
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_message(&request_str).unwrap();

    assert!(response.is_success());
    assert_eq!(handler.get_account_subscriptions().len(), 2);
    assert!(handler.get_account_subscriptions().contains(&accounts[0]));
    assert!(handler.get_account_subscriptions().contains(&accounts[1]));
}

#[test]
fn test_subscribe_streams_and_accounts() {
    let mut handler = MockWsHandler::new();

    let request = WsRequest::new("subscribe")
        .with_id(1)
        .with_streams(vec!["ledger".to_string(), "transactions".to_string()])
        .with_accounts(vec!["000000000000000000000000000000000000000001".to_string()]);
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_message(&request_str).unwrap();

    assert!(response.is_success());
    assert_eq!(handler.get_subscriptions().len(), 2);
    assert_eq!(handler.get_account_subscriptions().len(), 1);
}

#[test]
fn test_unsubscribe_single_stream() {
    let mut handler = MockWsHandler::new();

    // First subscribe
    let request = WsRequest::new("subscribe")
        .with_id(1)
        .with_streams(vec!["ledger".to_string(), "transactions".to_string()]);
    let request_str = serde_json::to_string(&request).unwrap();
    handler.process_message(&request_str).unwrap();

    assert_eq!(handler.get_subscriptions().len(), 2);

    // Then unsubscribe from one
    let request = WsRequest::new("unsubscribe")
        .with_id(2)
        .with_streams(vec!["ledger".to_string()]);
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_message(&request_str).unwrap();

    assert!(response.is_success());
    assert_eq!(handler.get_subscriptions().len(), 1);
    assert!(handler.get_subscriptions().contains(&"transactions".to_string()));
    assert!(!handler.get_subscriptions().contains(&"ledger".to_string()));
}

#[test]
fn test_unsubscribe_all_streams() {
    let mut handler = MockWsHandler::new();

    // First subscribe
    let request = WsRequest::new("subscribe")
        .with_id(1)
        .with_streams(vec![
            "ledger".to_string(),
            "transactions".to_string(),
            "validations".to_string(),
        ]);
    let request_str = serde_json::to_string(&request).unwrap();
    handler.process_message(&request_str).unwrap();

    assert_eq!(handler.get_subscriptions().len(), 3);

    // Then unsubscribe from all
    let request = WsRequest::new("unsubscribe")
        .with_id(2)
        .with_streams(vec![
            "ledger".to_string(),
            "transactions".to_string(),
            "validations".to_string(),
        ]);
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_message(&request_str).unwrap();

    assert!(response.is_success());
    assert!(handler.get_subscriptions().is_empty());
}

#[test]
fn test_unsubscribe_accounts() {
    let mut handler = MockWsHandler::new();

    // First subscribe to accounts
    let accounts = vec![
        "000000000000000000000000000000000000000001".to_string(),
        "000000000000000000000000000000000000000002".to_string(),
    ];

    let request = WsRequest::new("subscribe")
        .with_id(1)
        .with_accounts(accounts.clone());
    let request_str = serde_json::to_string(&request).unwrap();
    handler.process_message(&request_str).unwrap();

    assert_eq!(handler.get_account_subscriptions().len(), 2);

    // Then unsubscribe from one account
    let request = WsRequest::new("unsubscribe")
        .with_id(2)
        .with_accounts(vec![accounts[0].clone()]);
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_message(&request_str).unwrap();

    assert!(response.is_success());
    assert_eq!(handler.get_account_subscriptions().len(), 1);
    assert!(!handler.get_account_subscriptions().contains(&accounts[0]));
    assert!(handler.get_account_subscriptions().contains(&accounts[1]));
}

// ============================================================================
// WebSocket Error Handling Tests
// ============================================================================

#[test]
fn test_unknown_command() {
    let mut handler = MockWsHandler::new();

    let request = WsRequest::new("unknown_command").with_id(1);
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_message(&request_str).unwrap();

    assert!(response.is_error());
    assert!(response.error.is_some());
    assert!(response.error.unwrap().contains("Unknown command"));
}

#[test]
fn test_malformed_json() {
    let mut handler = MockWsHandler::new();

    let result = handler.process_message("{invalid json");

    assert!(result.is_err());
}

// ============================================================================
// WebSocket Subscription Lifecycle Tests
// ============================================================================

#[test]
fn test_subscription_persistence() {
    let mut handler = MockWsHandler::new();

    // Subscribe to multiple streams
    let streams = vec![
        "ledger".to_string(),
        "transactions".to_string(),
        "validations".to_string(),
    ];

    let request = WsRequest::new("subscribe")
        .with_id(1)
        .with_streams(streams.clone());
    let request_str = serde_json::to_string(&request).unwrap();
    handler.process_message(&request_str).unwrap();

    // Verify subscriptions persist
    assert_eq!(handler.get_subscriptions().len(), 3);

    // Send another request (ping) - subscriptions should remain
    let request = WsRequest::new("ping").with_id(2);
    let request_str = serde_json::to_string(&request).unwrap();
    handler.process_message(&request_str).unwrap();

    // Verify subscriptions still there
    assert_eq!(handler.get_subscriptions().len(), 3);
}

#[test]
fn test_duplicate_subscription() {
    let mut handler = MockWsHandler::new();

    // Subscribe to the same stream twice
    let request = WsRequest::new("subscribe")
        .with_id(1)
        .with_streams(vec!["ledger".to_string()]);
    let request_str = serde_json::to_string(&request).unwrap();
    handler.process_message(&request_str).unwrap();

    let request = WsRequest::new("subscribe")
        .with_id(2)
        .with_streams(vec!["ledger".to_string()]);
    let request_str = serde_json::to_string(&request).unwrap();
    handler.process_message(&request_str).unwrap();

    // Should only have one subscription
    assert_eq!(handler.get_subscriptions().len(), 1);
}

#[test]
fn test_unsubscribe_not_subscribed() {
    let mut handler = MockWsHandler::new();

    // Try to unsubscribe from a stream we never subscribed to
    let request = WsRequest::new("unsubscribe")
        .with_id(1)
        .with_streams(vec!["ledger".to_string()]);
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_message(&request_str).unwrap();

    // Should succeed but not change anything
    assert!(response.is_success());
    assert!(handler.get_subscriptions().is_empty());
}

// ============================================================================
// WebSocket Response ID Echo Tests
// ============================================================================

#[test]
fn test_response_id_echo() {
    let mut handler = MockWsHandler::new();

    // Test with different IDs
    for id in [1, 42, 999, 0] {
        let request = WsRequest::new("ping").with_id(id);
        let request_str = serde_json::to_string(&request).unwrap();
        let response = handler.process_message(&request_str).unwrap();

        assert_eq!(response.id, Some(id));
    }
}

#[test]
fn test_no_id_provided() {
    let mut handler = MockWsHandler::new();

    let request = WsRequest::new("ping"); // No ID
    let request_str = serde_json::to_string(&request).unwrap();
    let response = handler.process_message(&request_str).unwrap();

    assert!(response.is_success());
    // ID should be None when not provided
    assert_eq!(response.id, None);
}

// ============================================================================
// WebSocket Stream Types Tests
// ============================================================================

#[test]
fn test_subscribe_ledger_stream() {
    let mut handler = MockWsHandler::new();

    let request = WsRequest::new("subscribe")
        .with_id(1)
        .with_streams(vec!["ledger".to_string()]);
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_message(&request_str).unwrap();

    assert!(response.is_success());
    assert!(handler.get_subscriptions().contains(&"ledger".to_string()));
}

#[test]
fn test_subscribe_transactions_stream() {
    let mut handler = MockWsHandler::new();

    let request = WsRequest::new("subscribe")
        .with_id(1)
        .with_streams(vec!["transactions".to_string()]);
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_message(&request_str).unwrap();

    assert!(response.is_success());
    assert!(handler.get_subscriptions().contains(&"transactions".to_string()));
}

#[test]
fn test_subscribe_validations_stream() {
    let mut handler = MockWsHandler::new();

    let request = WsRequest::new("subscribe")
        .with_id(1)
        .with_streams(vec!["validations".to_string()]);
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_message(&request_str).unwrap();

    assert!(response.is_success());
    assert!(handler.get_subscriptions().contains(&"validations".to_string()));
}

#[test]
fn test_subscribe_consensus_stream() {
    let mut handler = MockWsHandler::new();

    let request = WsRequest::new("subscribe")
        .with_id(1)
        .with_streams(vec!["consensus".to_string()]);
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_message(&request_str).unwrap();

    assert!(response.is_success());
    assert!(handler.get_subscriptions().contains(&"consensus".to_string()));
}

#[test]
fn test_subscribe_peer_stream() {
    let mut handler = MockWsHandler::new();

    let request = WsRequest::new("subscribe")
        .with_id(1)
        .with_streams(vec!["peer".to_string()]);
    let request_str = serde_json::to_string(&request).unwrap();

    let response = handler.process_message(&request_str).unwrap();

    assert!(response.is_success());
    assert!(handler.get_subscriptions().contains(&"peer".to_string()));
}

// ============================================================================
// WebSocket Complex Scenario Tests
// ============================================================================

#[test]
fn test_full_subscription_lifecycle() {
    let mut handler = MockWsHandler::new();

    // 1. Subscribe to multiple streams and accounts
    let request = WsRequest::new("subscribe")
        .with_id(1)
        .with_streams(vec![
            "ledger".to_string(),
            "transactions".to_string(),
            "validations".to_string(),
        ])
        .with_accounts(vec![
            "000000000000000000000000000000000000000001".to_string(),
            "000000000000000000000000000000000000000002".to_string(),
        ]);
    let request_str = serde_json::to_string(&request).unwrap();
    let response = handler.process_message(&request_str).unwrap();

    assert!(response.is_success());
    assert_eq!(handler.get_subscriptions().len(), 3);
    assert_eq!(handler.get_account_subscriptions().len(), 2);

    // 2. Get server info while subscribed
    let request = WsRequest::new("server_info").with_id(2);
    let request_str = serde_json::to_string(&request).unwrap();
    let response = handler.process_message(&request_str).unwrap();

    assert!(response.is_success());
    assert_eq!(handler.get_subscriptions().len(), 3); // Subscriptions still active

    // 3. Unsubscribe from some streams
    let request = WsRequest::new("unsubscribe")
        .with_id(3)
        .with_streams(vec!["validations".to_string()])
        .with_accounts(vec!["000000000000000000000000000000000000000001".to_string()]);
    let request_str = serde_json::to_string(&request).unwrap();
    let response = handler.process_message(&request_str).unwrap();

    assert!(response.is_success());
    assert_eq!(handler.get_subscriptions().len(), 2);
    assert_eq!(handler.get_account_subscriptions().len(), 1);

    // 4. Unsubscribe from remaining
    let request = WsRequest::new("unsubscribe")
        .with_id(4)
        .with_streams(vec!["ledger".to_string(), "transactions".to_string()])
        .with_accounts(vec!["000000000000000000000000000000000000000002".to_string()]);
    let request_str = serde_json::to_string(&request).unwrap();
    let response = handler.process_message(&request_str).unwrap();

    assert!(response.is_success());
    assert!(handler.get_subscriptions().is_empty());
    assert!(handler.get_account_subscriptions().is_empty());
}
