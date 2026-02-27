//! WebSocket server for real-time subscriptions
//!
//! Provides streaming updates for:
//! - ledger: New validated ledgers
//! - transactions: New transactions
//! - validations: Consensus validations
//! - accounts: Account-specific updates
//! - consensus: Consensus phase updates
//! - peer: Peer connection events

use crate::application::ApplicationHandle;
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
use primitives::AccountID;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, RwLock};
use tracing::{debug, error, info, warn};

/// Type alias for WebSocket sender
type WsSender = Arc<Mutex<futures::stream::SplitSink<WebSocket, Message>>>;

/// WebSocket configuration
#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    pub enabled: bool,
    pub bind_address: String,
    pub port: u16,
    pub max_connections: u32,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            bind_address: "0.0.0.0".to_string(),
            port: 6005,
            max_connections: 1000,
        }
    }
}

/// WebSocket server
pub struct WebSocketServer {
    config: WebSocketConfig,
    app: ApplicationHandle,
    /// Broadcast channel for ledger updates
    ledger_tx: broadcast::Sender<serde_json::Value>,
    /// Broadcast channel for transaction updates
    transactions_tx: broadcast::Sender<serde_json::Value>,
    /// Broadcast channel for validation updates
    validations_tx: broadcast::Sender<serde_json::Value>,
    /// Broadcast channel for consensus updates
    consensus_tx: broadcast::Sender<serde_json::Value>,
    /// Broadcast channel for peer events
    peer_tx: broadcast::Sender<serde_json::Value>,
    /// Active connections
    connections: Arc<RwLock<ConnectionManager>>,
    /// Next connection ID
    next_conn_id: Arc<Mutex<u64>>,
}

/// Connection manager tracking all active connections
#[derive(Default)]
struct ConnectionManager {
    connections: Vec<ConnectionInfo>,
}

#[derive(Clone)]
struct ConnectionInfo {
    id: u64,
    ip: String,
    subscriptions: Arc<Mutex<Subscriptions>>,
    sender: WsSender,
}

#[derive(Debug, Default, Clone)]
struct Subscriptions {
    /// Subscribe to ledger closed events
    ledger: bool,
    /// Subscribe to all transactions
    transactions: bool,
    /// Subscribe to validations
    validations: bool,
    /// Subscribe to consensus events
    consensus: bool,
    /// Subscribe to peer events
    peer: bool,
    /// Subscribe to specific accounts
    accounts: HashSet<String>,
    /// Subscribe to specific streams
    streams: HashSet<String>,
}

/// Client request
#[derive(Debug, Deserialize)]
#[serde(tag = "command")]
#[serde(rename_all = "camelCase")]
enum WsRequest {
    #[serde(rename = "subscribe")]
    Subscribe {
        streams: Option<Vec<String>>,
        accounts: Option<Vec<String>>,
        id: Option<u64>,
    },
    #[serde(rename = "unsubscribe")]
    Unsubscribe {
        streams: Option<Vec<String>>,
        accounts: Option<Vec<String>>,
        id: Option<u64>,
    },
    #[serde(rename = "ping")]
    Ping { id: Option<u64> },
    #[serde(rename = "server_info")]
    ServerInfo { id: Option<u64> },
    #[serde(rename = "ledger")]
    Ledger { id: Option<u64> },
    #[serde(rename = "account_info")]
    AccountInfo {
        account: String,
        id: Option<u64>,
    },
}

/// Server response
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
enum WsResponse {
    #[serde(rename = "response")]
    Response {
        id: Option<u64>,
        status: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        result: Option<serde_json::Value>,
    },
    #[serde(rename = "ledgerClosed")]
    LedgerClosed {
        ledger_index: u32,
        ledger_hash: String,
        close_time: i64,
    },
    #[serde(rename = "transaction")]
    Transaction {
        transaction: serde_json::Value,
        ledger_index: Option<u32>,
        validated: bool,
    },
    #[serde(rename = "validationReceived")]
    ValidationReceived {
        validation: serde_json::Value,
    },
    #[serde(rename = "consensusPhase")]
    ConsensusPhase {
        phase: String,
        ledger_index: u32,
        round_time: u64,
    },
    #[serde(rename = "peerStatusChange")]
    PeerStatusChange {
        action: String,
        peer: serde_json::Value,
    },
    #[serde(rename = "pong")]
    Pong { id: Option<u64> },
    #[serde(rename = "error")]
    Error {
        id: Option<u64>,
        error: String,
        error_code: i32,
        error_message: String,
    },
}

impl WsResponse {
    fn success(id: Option<u64>, result: Option<serde_json::Value>) -> Self {
        Self::Response {
            id,
            status: "success".to_string(),
            result,
        }
    }

    fn error(id: Option<u64>, error: String, error_code: i32, error_message: String) -> Self {
        Self::Error {
            id,
            error,
            error_code,
            error_message,
        }
    }
}

impl WebSocketServer {
    pub fn new(config: WebSocketConfig, app: ApplicationHandle) -> Self {
        let (ledger_tx, _) = broadcast::channel(100);
        let (transactions_tx, _) = broadcast::channel(1000);
        let (validations_tx, _) = broadcast::channel(100);
        let (consensus_tx, _) = broadcast::channel(50);
        let (peer_tx, _) = broadcast::channel(50);

        Self {
            config,
            app,
            ledger_tx,
            transactions_tx,
            validations_tx,
            consensus_tx,
            peer_tx,
            connections: Arc::new(RwLock::new(ConnectionManager::default())),
            next_conn_id: Arc::new(Mutex::new(0)),
        }
    }

    /// Get the number of active connections
    pub async fn connection_count(&self) -> usize {
        self.connections.read().await.connections.len()
    }

    /// Run the WebSocket server
    pub async fn run(&self, shutdown: tokio::sync::watch::Receiver<bool>) -> anyhow::Result<()> {
        if !self.config.enabled {
            info!("WebSocket server disabled");
            return Ok(());
        }

        let bind_addr = format!("{}:{}", self.config.bind_address, self.config.port);
        let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
        info!(
            "WebSocket server listening on ws://{} (max {} connections)",
            bind_addr, self.config.max_connections
        );

        // Spawn the broadcaster task
        let app = self.app.clone();
        let ledger_tx = self.ledger_tx.clone();
        let transactions_tx = self.transactions_tx.clone();
        let validations_tx = self.validations_tx.clone();
        let consensus_tx = self.consensus_tx.clone();
        let connections_clone = self.connections.clone();

        tokio::spawn(async move {
            broadcaster_task(
                app,
                ledger_tx,
                transactions_tx,
                validations_tx,
                consensus_tx,
                connections_clone,
            )
            .await;
        });

        let mut shutdown_rx = shutdown;
        let mut conn_counter = 0u64;

        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, addr)) => {
                            // Check connection limit
                            let current_conns = self.connections.read().await.connections.len();
                            if current_conns >= self.config.max_connections as usize {
                                warn!("Connection limit reached, rejecting connection from {}", addr);
                                continue;
                            }

                            conn_counter += 1;
                            let conn_id = conn_counter;

                            info!(
                                "New WebSocket connection {} from {} (total: {})",
                                conn_id, addr, current_conns + 1
                            );

                            // Spawn connection handler
                            let connections = self.connections.clone();
                            let ledger_tx = self.ledger_tx.clone();
                            let transactions_tx = self.transactions_tx.clone();
                            let validations_tx = self.validations_tx.clone();
                            let consensus_tx = self.consensus_tx.clone();
                            let peer_tx = self.peer_tx.clone();

                            tokio::spawn(async move {
                                handle_connection(
                                    stream,
                                    conn_id,
                                    addr.to_string(),
                                    connections,
                                    ledger_tx,
                                    transactions_tx,
                                    validations_tx,
                                    consensus_tx,
                                    peer_tx,
                                )
                                .await;
                            });
                        }
                        Err(e) => {
                            error!("Failed to accept connection: {}", e);
                        }
                    }
                }
                _ = shutdown_rx.changed() => {
                    info!("WebSocket server shutting down...");
                    // Close all connections
                    let mut manager = self.connections.write().await;
                    manager.connections.clear();
                    break;
                }
            }
        }

        Ok(())
    }

    /// Handle a WebSocket upgrade request from axum
    pub async fn handle_socket(
        self: Arc<Self>,
        socket: WebSocket,
        conn_id: u64,
    ) {
        let (sender, mut receiver) = socket.split();
        let sender: WsSender = Arc::new(Mutex::new(sender));

        let subscriptions = Arc::new(Mutex::new(Subscriptions::default()));

        let conn_info = ConnectionInfo {
            id: conn_id,
            ip: "unknown".to_string(),
            subscriptions: subscriptions.clone(),
            sender: sender.clone(),
        };

        // Add connection to manager
        self.connections.write().await.connections.push(conn_info);

        // Subscribe to broadcast channels
        let mut ledger_rx = self.ledger_tx.subscribe();
        let mut transactions_rx = self.transactions_tx.subscribe();
        let mut validations_rx = self.validations_tx.subscribe();
        let mut consensus_rx = self.consensus_tx.subscribe();
        let mut peer_rx = self.peer_tx.subscribe();

        // Spawn task to forward broadcasts to client
        let sender_clone = sender.clone();
        let subscriptions_clone = subscriptions.clone();
        let connections = self.connections.clone();
        let conn_id_clone = conn_id;

        let broadcast_task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    Ok(ledger) = ledger_rx.recv() => {
                        if subscriptions_clone.lock().await.ledger {
                            let msg = serde_json::to_string(&WsResponse::LedgerClosed {
                                ledger_index: ledger["ledger_index"].as_u64().unwrap_or(0) as u32,
                                ledger_hash: ledger["ledger_hash"].as_str().unwrap_or("").to_string(),
                                close_time: ledger["close_time"].as_i64().unwrap_or(0),
                            }).unwrap();
                            let _ = sender_clone.lock().await.send(Message::Text(msg)).await;
                        }
                    }
                    Ok(tx) = transactions_rx.recv() => {
                        if subscriptions_clone.lock().await.transactions {
                            let msg = serde_json::to_string(&WsResponse::Transaction {
                                transaction: tx,
                                ledger_index: None,
                                validated: false,
                            }).unwrap();
                            let _ = sender_clone.lock().await.send(Message::Text(msg)).await;
                        }
                    }
                    Ok(val) = validations_rx.recv() => {
                        if subscriptions_clone.lock().await.validations {
                            let msg = serde_json::to_string(&WsResponse::ValidationReceived {
                                validation: val,
                            }).unwrap();
                            let _ = sender_clone.lock().await.send(Message::Text(msg)).await;
                        }
                    }
                    Ok(consensus) = consensus_rx.recv() => {
                        if subscriptions_clone.lock().await.consensus {
                            let msg = serde_json::to_string(&WsResponse::ConsensusPhase {
                                phase: consensus["phase"].as_str().unwrap_or("unknown").to_string(),
                                ledger_index: consensus["ledger_index"].as_u64().unwrap_or(0) as u32,
                                round_time: consensus["round_time"].as_u64().unwrap_or(0),
                            }).unwrap();
                            let _ = sender_clone.lock().await.send(Message::Text(msg)).await;
                        }
                    }
                    Ok(peer) = peer_rx.recv() => {
                        if subscriptions_clone.lock().await.peer {
                            let msg = serde_json::to_string(&WsResponse::PeerStatusChange {
                                action: peer["action"].as_str().unwrap_or("unknown").to_string(),
                                peer: peer["peer"].clone(),
                            }).unwrap();
                            let _ = sender_clone.lock().await.send(Message::Text(msg)).await;
                        }
                    }
                    else => break,
                }
            }
            // Connection closed, remove from manager
            let mut manager = connections.write().await;
            manager.connections.retain(|c| c.id != conn_id_clone);
        });

        // Handle incoming messages
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    match serde_json::from_str::<WsRequest>(&text) {
                        Ok(request) => {
                            match request {
                                WsRequest::Subscribe { streams, accounts, id } => {
                                    let response = handle_subscribe(
                                        &subscriptions,
                                        streams,
                                        accounts,
                                        id,
                                    ).await;
                                    send_response(&sender, response).await;
                                }
                                WsRequest::Unsubscribe { streams, accounts, id } => {
                                    let response = handle_unsubscribe(
                                        &subscriptions,
                                        streams,
                                        accounts,
                                        id,
                                    ).await;
                                    send_response(&sender, response).await;
                                }
                                WsRequest::Ping { id } => {
                                    let response = WsResponse::Pong { id };
                                    send_response(&sender, response).await;
                                }
                                WsRequest::ServerInfo { id } => {
                                    let response = handle_server_info(&self.app, id).await;
                                    send_response(&sender, response).await;
                                }
                                WsRequest::Ledger { id } => {
                                    let response = handle_ledger(&self.app, id).await;
                                    send_response(&sender, response).await;
                                }
                                WsRequest::AccountInfo { account, id } => {
                                    let response = handle_account_info(&self.app, &account, id).await;
                                    send_response(&sender, response).await;
                                }
                            }
                        }
                        Err(e) => {
                            let response = WsResponse::error(
                                None,
                                "invalid_json".to_string(),
                                -32700,
                                format!("Parse error: {}", e),
                            );
                            send_response(&sender, response).await;
                        }
                    }
                }
                Message::Ping(data) => {
                    let _ = sender.lock().await.send(Message::Pong(data)).await;
                }
                Message::Close(_) => {
                    debug!("WebSocket connection {} closed", conn_id);
                    break;
                }
                _ => {}
            }
        }

        // Wait for broadcast task to complete
        let _ = broadcast_task.await;
    }

    /// Broadcast a ledger update to all subscribers
    pub fn broadcast_ledger(&self, ledger: serde_json::Value) {
        let _ = self.ledger_tx.send(ledger);
    }

    /// Broadcast a transaction to all subscribers
    pub fn broadcast_transaction(&self, tx: serde_json::Value) {
        let _ = self.transactions_tx.send(tx);
    }

    /// Broadcast a validation to all subscribers
    pub fn broadcast_validation(&self, validation: serde_json::Value) {
        let _ = self.validations_tx.send(validation);
    }

    /// Broadcast a consensus phase update
    pub fn broadcast_consensus(&self, consensus: serde_json::Value) {
        let _ = self.consensus_tx.send(consensus);
    }

    /// Broadcast a peer event
    pub fn broadcast_peer(&self, peer: serde_json::Value) {
        let _ = self.peer_tx.send(peer);
    }
}

/// Handle subscribe command
async fn handle_subscribe(
    subscriptions: &Arc<Mutex<Subscriptions>>,
    streams: Option<Vec<String>>,
    accounts: Option<Vec<String>>,
    id: Option<u64>,
) -> WsResponse {
    let mut subs = subscriptions.lock().await;

    if let Some(streams) = streams {
        for stream in streams {
            match stream.as_str() {
                "ledger" => subs.ledger = true,
                "transactions" | "transaction" => subs.transactions = true,
                "validations" | "validation" => subs.validations = true,
                "consensus" => subs.consensus = true,
                "peer" => subs.peer = true,
                _ => {}
            }
            subs.streams.insert(stream);
        }
    }

    if let Some(accts) = accounts {
        for acct in accts {
            subs.accounts.insert(acct);
        }
    }

    WsResponse::success(id, None)
}

/// Handle unsubscribe command
async fn handle_unsubscribe(
    subscriptions: &Arc<Mutex<Subscriptions>>,
    streams: Option<Vec<String>>,
    accounts: Option<Vec<String>>,
    id: Option<u64>,
) -> WsResponse {
    let mut subs = subscriptions.lock().await;

    if let Some(streams) = streams {
        for stream in streams {
            match stream.as_str() {
                "ledger" => subs.ledger = false,
                "transactions" | "transaction" => subs.transactions = false,
                "validations" | "validation" => subs.validations = false,
                "consensus" => subs.consensus = false,
                "peer" => subs.peer = false,
                _ => {}
            }
            subs.streams.remove(&stream);
        }
    }

    if let Some(accts) = accounts {
        for acct in accts {
            subs.accounts.remove(&acct);
        }
    }

    WsResponse::success(id, None)
}

/// Handle server_info command
async fn handle_server_info(app: &ApplicationHandle, id: Option<u64>) -> WsResponse {
    let app_guard = app.read().await;
    let info = serde_json::json!({
        "info": {
            "build_version": "0.1.0",
            "complete_ledgers": format!("1-{}", app_guard.consensus.get_ledger_index()),
            "io_latency_ms": 1,
            "load_factor": 1,
            "peers": app_guard.overlay.active_peer_count(),
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
                "ledger_index": app_guard.consensus.get_ledger_index(),
                "ledger_hash": "0000000000000000000000000000000000000000000000000000000000000000",
            },
        }
    });
    WsResponse::success(id, Some(info))
}

/// Handle ledger command
async fn handle_ledger(app: &ApplicationHandle, id: Option<u64>) -> WsResponse {
    let app_guard = app.read().await;
    let ledger_index = app_guard.consensus.get_ledger_index();
    let ledger = serde_json::json!({
        "ledger": {
            "ledger_index": ledger_index,
            "ledger_hash": "0000000000000000000000000000000000000000000000000000000000000000",
            "close_time": chrono::Utc::now().timestamp(),
            "closed": false,
            "validated": false,
        }
    });
    WsResponse::success(id, Some(ledger))
}

/// Handle account_info command
async fn handle_account_info(app: &ApplicationHandle, account: &str, id: Option<u64>) -> WsResponse {
    // Parse account from hex
    let account_bytes = match hex::decode(account) {
        Ok(bytes) if bytes.len() == 20 => bytes,
        _ => {
            return WsResponse::error(
                id,
                "act_not_found".to_string(),
                -32000,
                "Account not found.".to_string(),
            );
        }
    };

    let account_id = AccountID::new(account_bytes.try_into().unwrap());
    let app_guard = app.read().await;
    let ledger_state = app_guard.get_ledger_state();

    if let Some(account_root) = ledger_state.get_account_root(&account_id) {
        let info = serde_json::json!({
            "account": account,
            "account_data": {
                "Account": hex::encode(account_id.as_bytes()),
                "Balance": account_root.balance.mantissa.to_string(),
                "Sequence": account_root.sequence,
                "OwnerCount": account_root.owner_count,
                "PreviousTxnID": hex::encode(account_root.previous_txn_id.as_bytes()),
                "PreviousTxnLgrSeq": account_root.previous_txn_lgr_seq,
            },
            "ledger_current_index": app_guard.consensus.get_ledger_index(),
            "validated": false,
        });
        WsResponse::success(id, Some(info))
    } else {
        WsResponse::error(
            id,
            "act_not_found".to_string(),
            -32000,
            "Account not found.".to_string(),
        )
    }
}

/// Send response to client
async fn send_response(
    sender: &WsSender,
    response: WsResponse,
) {
    match serde_json::to_string(&response) {
        Ok(msg) => {
            let _ = sender.lock().await.send(Message::Text(msg)).await;
        }
        Err(e) => {
            error!("Failed to serialize WebSocket response: {}", e);
        }
    }
}

/// Connection handler
async fn handle_connection(
    stream: tokio::net::TcpStream,
    conn_id: u64,
    addr: String,
    connections: Arc<RwLock<ConnectionManager>>,
    ledger_tx: broadcast::Sender<serde_json::Value>,
    _transactions_tx: broadcast::Sender<serde_json::Value>,
    _validations_tx: broadcast::Sender<serde_json::Value>,
    consensus_tx: broadcast::Sender<serde_json::Value>,
    peer_tx: broadcast::Sender<serde_json::Value>,
) {
    // For a proper implementation, we'd use axum's WebSocketUpgrade
    // This is a simplified version that assumes the stream is already upgraded
    // In practice, this would be handled by the HTTP router

    // Skip actual WebSocket handling here - it's done in handle_socket
    // This function is a placeholder for the TCP stream handling
    let _ = (stream, conn_id, addr, connections, ledger_tx, consensus_tx, peer_tx);
}

/// Background task that polls the application and broadcasts updates
async fn broadcaster_task(
    app: ApplicationHandle,
    ledger_tx: broadcast::Sender<serde_json::Value>,
    transactions_tx: broadcast::Sender<serde_json::Value>,
    validations_tx: broadcast::Sender<serde_json::Value>,
    consensus_tx: broadcast::Sender<serde_json::Value>,
    connections: Arc<RwLock<ConnectionManager>>,
) {
    let mut ledger_interval = tokio::time::interval(tokio::time::Duration::from_secs(4));
    let mut consensus_interval = tokio::time::interval(tokio::time::Duration::from_millis(500));
    let mut last_ledger_index = 0u32;

    loop {
        tokio::select! {
            _ = ledger_interval.tick() => {
                // Get current ledger info from app
                let app_guard = app.read().await;
                let ledger_index = app_guard.consensus.get_ledger_index();

                // Broadcast ledger update if changed
                if ledger_index > last_ledger_index {
                    let ledger_info = serde_json::json!({
                        "ledger_index": ledger_index,
                        "ledger_hash": "0000000000000000000000000000000000000000000000000000000000000000",
                        "close_time": chrono::Utc::now().timestamp(),
                        "close_time_human": chrono::Utc::now().to_rfc3339(),
                        "closed": true,
                        "validated": true,
                    });

                    let _ = ledger_tx.send(ledger_info);
                    last_ledger_index = ledger_index;
                }
                drop(app_guard);
            }
            _ = consensus_interval.tick() => {
                // Broadcast consensus phase updates
                let app_guard = app.read().await;
                let phase = app_guard.consensus.get_phase();
                let ledger_index = app_guard.consensus.get_ledger_index();
                let round_id = app_guard.consensus.get_round_id();

                let phase_str = format!("{:?}", phase);

                // Only broadcast if there are subscribers
                let has_consensus_subscribers = {
                    let manager = connections.read().await;
                    manager.connections.iter().any(|c| {
                        let subs = c.subscriptions.try_lock();
                        subs.map_or(false, |s| s.consensus)
                    })
                };

                if has_consensus_subscribers {
                    let consensus_info = serde_json::json!({
                        "phase": phase_str,
                        "ledger_index": ledger_index,
                        "round_id": round_id,
                        "round_time": chrono::Utc::now().timestamp_millis() as u64,
                    });

                    let _ = consensus_tx.send(consensus_info);
                }
                drop(app_guard);
            }
        }
    }
}

/// WebSocket upgrade handler for axum
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(server): State<Arc<WebSocketServer>>,
) -> impl IntoResponse {
    let conn_id = {
        let mut counter = server.next_conn_id.lock().await;
        *counter += 1;
        *counter
    };
    ws.on_upgrade(move |socket| async move {
        server.handle_socket(socket, conn_id).await;
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_config_default() {
        let config = WebSocketConfig::default();
        assert!(config.enabled);
        assert_eq!(config.port, 6005);
        assert_eq!(config.max_connections, 1000);
    }

    #[test]
    fn test_subscribe_request_deserialization() {
        let json = r#"{"command":"subscribe","streams":["ledger","transactions","consensus"]}"#;
        let req: WsRequest = serde_json::from_str(json).unwrap();

        match req {
            WsRequest::Subscribe { streams, accounts, id } => {
                assert_eq!(streams.unwrap().len(), 3);
                assert!(accounts.is_none());
                assert!(id.is_none());
            }
            _ => panic!("Expected Subscribe variant"),
        }
    }

    #[test]
    fn test_subscribe_with_accounts() {
        let json = r#"{"command":"subscribe","accounts":["account1","account2"]}"#;
        let req: WsRequest = serde_json::from_str(json).unwrap();

        match req {
            WsRequest::Subscribe { streams, accounts, id } => {
                assert!(streams.is_none());
                assert_eq!(accounts.unwrap().len(), 2);
                assert!(id.is_none());
            }
            _ => panic!("Expected Subscribe variant"),
        }
    }

    #[test]
    fn test_unsubscribe_request_deserialization() {
        let json = r#"{"command":"unsubscribe","streams":["ledger"]}"#;
        let req: WsRequest = serde_json::from_str(json).unwrap();

        match req {
            WsRequest::Unsubscribe { streams, .. } => {
                assert_eq!(streams.unwrap().len(), 1);
            }
            _ => panic!("Expected Unsubscribe variant"),
        }
    }

    #[test]
    fn test_ping_request_deserialization() {
        let json = r#"{"command":"ping","id":123}"#;
        let req: WsRequest = serde_json::from_str(json).unwrap();

        match req {
            WsRequest::Ping { id } => {
                assert_eq!(id, Some(123));
            }
            _ => panic!("Expected Ping variant"),
        }
    }

    #[test]
    fn test_server_info_request_deserialization() {
        let json = r#"{"command":"server_info","id":1}"#;
        let req: WsRequest = serde_json::from_str(json).unwrap();

        match req {
            WsRequest::ServerInfo { id } => {
                assert_eq!(id, Some(1));
            }
            _ => panic!("Expected ServerInfo variant"),
        }
    }

    #[test]
    fn test_account_info_request_deserialization() {
        let json = r#"{"command":"account_info","account":"abc123","id":5}"#;
        let req: WsRequest = serde_json::from_str(json).unwrap();

        match req {
            WsRequest::AccountInfo { account, id } => {
                assert_eq!(account, "abc123");
                assert_eq!(id, Some(5));
            }
            _ => panic!("Expected AccountInfo variant"),
        }
    }

    #[test]
    fn test_response_serialization() {
        let response = WsResponse::Pong { id: Some(1) };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("pong"));
        assert!(json.contains("1"));
    }

    #[test]
    fn test_ledger_closed_response() {
        let response = WsResponse::LedgerClosed {
            ledger_index: 100,
            ledger_hash: "abc123".to_string(),
            close_time: 1234567890,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("ledgerClosed"));
        assert!(json.contains("100"));
    }

    #[test]
    fn test_consensus_phase_response() {
        let response = WsResponse::ConsensusPhase {
            phase: "establish".to_string(),
            ledger_index: 50,
            round_time: 1000,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("consensusPhase"));
        assert!(json.contains("establish"));
    }

    #[test]
    fn test_error_response() {
        let response = WsResponse::error(
            Some(1),
            "not_found".to_string(),
            -32000,
            "Resource not found".to_string(),
        );
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("error"));
        assert!(json.contains("-32000"));
    }

    #[test]
    fn test_subscriptions_default() {
        let subs = Subscriptions::default();
        assert!(!subs.ledger);
        assert!(!subs.transactions);
        assert!(!subs.validations);
        assert!(!subs.consensus);
        assert!(!subs.peer);
        assert!(subs.accounts.is_empty());
    }

    #[tokio::test]
    async fn test_subscription_management() {
        let subs = Arc::new(Mutex::new(Subscriptions::default()));

        // Subscribe to ledger
        let subs_clone = subs.clone();
        tokio::spawn(async move {
            let _ = handle_subscribe(&subs_clone, Some(vec!["ledger".to_string()]), None, None).await;
        });

        // Small delay to let the spawn complete
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Verify subscription
        let subs_guard = subs.try_lock().unwrap();
        assert!(subs_guard.ledger);
    }
}
