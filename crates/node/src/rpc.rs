use std::sync::Arc;
use tokio::sync::Mutex;
use primitives::AccountID;

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

#[async_trait::async_trait]
impl RpcHandler for AppRpcHandler {
    async fn handle_request(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        match method {
            "server_info" => {
                let app = self.app.read().await;
                Ok(app.get_server_info())
            }
            "ping" => {
                Ok(serde_json::json!({"status": "success"}))
            }
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
            "account_info" => {
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let account = params.get("account")
                    .and_then(|v| v.as_str())
                    .ok_or(JsonRpcError::invalid_params())?;

                // Parse account ID
                let account_bytes = hex::decode(account).map_err(|_| {
                    JsonRpcError::new(35, "Account malformed.")
                })?;
                if account_bytes.len() != 20 {
                    return Err(JsonRpcError::new(35, "Account malformed."));
                }
                let account_id = AccountID::new(account_bytes.try_into().unwrap());

                // Query ledger state through application
                let app = self.app.read().await;
                let ledger_state = app.get_ledger_state();

                // Try to fetch account root from ledger state
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
                    // Account not found in ledger
                    Err(JsonRpcError::new(
                        19,
                        "Account not found."
                    ))
                }
            }
            "ledger" => {
                // Get ledger info from application
                let app = self.app.read().await;
                let ledger_hash = app.get_current_ledger_hash();
                let ledger_seq = app.get_current_ledger_seq();
                let ledger_state = app.get_ledger_state();
                let account_hash = ledger_state.get_root_hash();

                // Get requested ledger index (if specified)
                let requested_index = params.as_ref()
                    .and_then(|p| p.get("ledger_index"))
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32);

                // If requesting current ledger or no specific ledger, return current
                let (return_hash, return_seq, return_parent) = if requested_index.is_none() || requested_index == Some(ledger_seq) {
                    (ledger_hash, ledger_seq, primitives::UInt256::zero())
                } else {
                    // Requesting a different ledger - we only have current
                    // Return placeholder for non-current ledgers
                    (primitives::UInt256::zero(), requested_index.unwrap_or(0), primitives::UInt256::zero())
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
            "submit" => {
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let tx_blob = params.get("tx_blob")
                    .and_then(|v| v.as_str())
                    .ok_or(JsonRpcError::invalid_params())?;

                // Decode transaction blob
                let tx_bytes = hex::decode(tx_blob).map_err(|_| {
                    JsonRpcError::new(31, "Transaction malformed")
                })?;

                // Submit to application
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
            "tx" => {
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let tx_hash_hex = params.get("transaction")
                    .and_then(|v| v.as_str())
                    .ok_or(JsonRpcError::invalid_params())?;

                // Parse transaction hash
                let tx_hash_bytes = hex::decode(tx_hash_hex).map_err(|_| {
                    JsonRpcError::new(31, "Transaction hash malformed")
                })?;
                if tx_hash_bytes.len() != 32 {
                    return Err(JsonRpcError::new(31, "Transaction hash malformed"));
                }
                let tx_hash = primitives::UInt256::new(tx_hash_bytes.try_into().unwrap());

                // Query database for transaction
                let app = self.app.read().await;
                if let Some(tx_node) = app.database.fetch_transaction_node(&tx_hash) {
                    // Transaction found - return with metadata
                    Ok(serde_json::json!({
                        "hash": tx_hash_hex,
                        "tx_blob": hex::encode(tx_node.get_data()),
                        "meta": null, // Would include metadata if available
                        "validated": true,
                        "status": "success",
                    }))
                } else {
                    // Transaction not found in database
                    Err(JsonRpcError::new(
                        24,
                        "Transaction not found."
                    ))
                }
            }
            "account_tx" => {
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let account = params.get("account")
                    .and_then(|v| v.as_str())
                    .ok_or(JsonRpcError::invalid_params())?;

                // Return empty list for now
                Ok(serde_json::json!({
                    "account": account,
                    "transactions": [],
                    "status": "success"
                }))
            }
            "book_offers" => {
                let params = params.ok_or(JsonRpcError::invalid_params())?;
                let _taker_gets = params.get("taker_gets")
                    .ok_or(JsonRpcError::invalid_params())?;
                let _taker_pays = params.get("taker_pays")
                    .ok_or(JsonRpcError::invalid_params())?;

                // Return empty offers for now
                Ok(serde_json::json!({
                    "offers": [],
                    "status": "success"
                }))
            }
            // Admin methods
            "validation_create" => {
                // Generate a new validation seed
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
            "wallet_propose" => {
                // Generate a new wallet
                use crypto::PrivateKey;
                let private_key = PrivateKey::generate_secp256k1();
                let public_key = private_key.to_public_key();

                // Generate account ID from public key (simplified)
                let account_id = primitives::AccountID::new([0u8; 20]);

                Ok(serde_json::json!({
                    "status": "success",
                    "account_id": hex::encode(account_id.as_bytes()),
                    "public_key": hex::encode(public_key.as_bytes()),
                    "master_seed": hex::encode(private_key.as_bytes()),
                }))
            }
            "peers" => {
                let app = self.app.read().await;
                let peer_count = app.overlay.active_peer_count();
                Ok(serde_json::json!({
                    "status": "success",
                    "peers": peer_count,
                    "peer_list": [],
                }))
            }
            "stop" => {
                // Return success - actual shutdown handled by caller
                Ok(serde_json::json!({
                    "status": "success",
                }))
            }
            "ledger_accept" => {
                // Force ledger close (admin/testing method)
                let app = self.app.write().await;
                // Trigger consensus acceptance
                Ok(serde_json::json!({
                    "status": "success",
                    "ledger_current_index": app.consensus.get_ledger_index(),
                }))
            }
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
                // In a real implementation, this would query the ledger
                Err(JsonRpcError::new(
                    19,
                    "Account not found."
                ))
            }
            "submit" => {
                // In a real implementation, this would submit a transaction
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
                // Transaction lookup - would query ledger
                Err(JsonRpcError::new(
                    24,
                    "Transaction not found."
                ))
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
    pub async fn process_request(&self,
        request_body: &str,
    ) -> String {
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

        // Validate JSON-RPC version
        if request.jsonrpc != "2.0" {
            let response = JsonRpcResponse::error(
                JsonRpcError::invalid_request(),
                request.id.clone(),
            );
            return serde_json::to_string(&response).unwrap_or_default();
        }

        // Handle the request
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

    /// Check if RPC is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get the bind address
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.config.bind_address, self.config.port)
    }

    /// Start the HTTP server
    pub async fn run(self, shutdown: tokio::sync::watch::Receiver<bool>) -> anyhow::Result<()> {
        use axum::{
            routing::post,
            Router,
        };

        let bind_addr = self.bind_address();
        let server = Arc::new(Mutex::new(self));

        let app = Router::new()
            .route("/", post(rpc_handler))
            .route("/v1", post(rpc_handler))
            .route("/v2", post(rpc_handler))
            .with_state(server);

        let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
        tracing::info!("RPC server listening on http://{}", bind_addr);

        // Run server with graceful shutdown
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

    // Try to parse as JSON to determine status code
    let status = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response) {
        if json.get("error").is_some() {
            axum::http::StatusCode::OK // JSON-RPC errors return 200 with error in body
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
        assert!(response.contains("-32601")); // Method not found code
    }

    #[tokio::test]
    async fn test_rpc_parse_error() {
        let config = RpcConfig::default();
        let handler = Box::new(SimpleRpcHandler::new());
        let server = RpcServer::new(config, handler);

        let request = r#"{invalid json"#;
        let response = server.process_request(request).await;

        assert!(response.contains("error"));
        assert!(response.contains("-32700")); // Parse error code
    }
}
