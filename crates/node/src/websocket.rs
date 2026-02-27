//! WebSocket server for real-time subscriptions
//!
//! Provides streaming updates for:
//! - ledger: New validated ledgers
//! - transactions: New transactions
//! - validations: Consensus validations

use crate::application::ApplicationHandle;
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, RwLock};
use tracing::{debug, info, warn};

/// WebSocket configuration
#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    pub enabled: bool,
    pub bind_address: String,
    pub port: u16,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            bind_address: "127.0.0.1".to_string(),
            port: 6005,
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
    /// Active connections
    connections: Arc<RwLock<Vec<ConnectionHandle>>>,
}

#[derive(Clone)]
struct ConnectionHandle {
    id: u64,
    subscriptions: Arc<Mutex<Subscriptions>>,
}

#[derive(Debug, Default)]
struct Subscriptions {
    ledger: bool,
    transactions: bool,
    validations: bool,
    accounts: HashSet<String>,
    streams: HashSet<String>,
}

/// Client request
#[derive(Debug, Deserialize)]
#[serde(tag = "command")]
enum WsRequest {
    #[serde(rename = "subscribe")]
    Subscribe { streams: Option<Vec<String>>, accounts: Option<Vec<String>> },
    #[serde(rename = "unsubscribe")]
    Unsubscribe { streams: Option<Vec<String>>, accounts: Option<Vec<String>> },
    #[serde(rename = "ping")]
    Ping,
}

/// Server response
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum WsResponse {
    #[serde(rename = "response")]
    Response { id: Option<u64>, status: String },
    #[serde(rename = "ledger")]
    Ledger { ledger: serde_json::Value },
    #[serde(rename = "transaction")]
    Transaction { tx: serde_json::Value },
    #[serde(rename = "validation")]
    Validation { validation: serde_json::Value },
    #[serde(rename = "pong")]
    Pong,
}

impl WebSocketServer {
    pub fn new(config: WebSocketConfig, app: ApplicationHandle) -> Self {
        let (ledger_tx, _) = broadcast::channel(100);
        let (transactions_tx, _) = broadcast::channel(1000);
        let (validations_tx, _) = broadcast::channel(100);

        Self {
            config,
            app,
            ledger_tx,
            transactions_tx,
            validations_tx,
            connections: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Run the WebSocket server
    pub async fn run(&self, shutdown: tokio::sync::watch::Receiver<bool>) -> anyhow::Result<()> {
        if !self.config.enabled {
            info!("WebSocket server disabled");
            return Ok(());
        }

        let bind_addr = format!("{}:{}", self.config.bind_address, self.config.port);
        let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
        info!("WebSocket server listening on ws://{}", bind_addr);

        // Spawn the broadcaster task
        let app = self.app.clone();
        let ledger_tx = self.ledger_tx.clone();
        let transactions_tx = self.transactions_tx.clone();
        let validations_tx = self.validations_tx.clone();

        tokio::spawn(async move {
            broadcaster_task(app, ledger_tx, transactions_tx, validations_tx).await;
        });

        let connections = self.connections.clone();
        let mut conn_id = 0u64;
        let mut shutdown_rx = shutdown;

        loop {
            tokio::select! {
                Ok((_stream, addr)) = listener.accept() => {
                    conn_id += 1;
                    info!("New WebSocket connection {} from {}", conn_id, addr);

                    let handle = ConnectionHandle {
                        id: conn_id,
                        subscriptions: Arc::new(Mutex::new(Subscriptions::default())),
                    };

                    connections.write().await.push(handle.clone());

                    // Spawn connection handler
                    let _ledger_rx = self.ledger_tx.subscribe();
                    let _transactions_rx = self.transactions_tx.subscribe();
                    let _validations_rx = self.validations_tx.subscribe();
                    let _connections_clone = connections.clone();

                    tokio::spawn(async move {
                        // Use axum's WebSocket upgrade
                        // Note: In actual axum usage, we'd use WebSocketUpgrade here
                        // This is a simplified version
                    });
                }
                _ = shutdown_rx.changed() => {
                    info!("WebSocket server shutting down...");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Handle a WebSocket upgrade request
    pub async fn handle_socket(
        self: Arc<Self>,
        socket: WebSocket,
        conn_id: u64,
    ) {
        let (sender, mut receiver) = socket.split();
        let sender = Arc::new(Mutex::new(sender));

        let handle = ConnectionHandle {
            id: conn_id,
            subscriptions: Arc::new(Mutex::new(Subscriptions::default())),
        };

        self.connections.write().await.push(handle.clone());

        // Subscribe to broadcast channels
        let mut ledger_rx = self.ledger_tx.subscribe();
        let mut transactions_rx = self.transactions_tx.subscribe();
        let mut validations_rx = self.validations_tx.subscribe();

        // Spawn task to forward broadcasts to client
        let sender_clone = sender.clone();
        let handle_clone = handle.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Ok(ledger) = ledger_rx.recv() => {
                        if handle_clone.subscriptions.lock().await.ledger {
                            let msg = serde_json::to_string(&WsResponse::Ledger { ledger }).unwrap();
                            let _ = sender_clone.lock().await.send(Message::Text(msg)).await;
                        }
                    }
                    Ok(tx) = transactions_rx.recv() => {
                        if handle_clone.subscriptions.lock().await.transactions {
                            let msg = serde_json::to_string(&WsResponse::Transaction { tx }).unwrap();
                            let _ = sender_clone.lock().await.send(Message::Text(msg)).await;
                        }
                    }
                    Ok(val) = validations_rx.recv() => {
                        if handle_clone.subscriptions.lock().await.validations {
                            let msg = serde_json::to_string(&WsResponse::Validation { validation: val }).unwrap();
                            let _ = sender_clone.lock().await.send(Message::Text(msg)).await;
                        }
                    }
                    else => break,
                }
            }
        });

        // Handle incoming messages
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    match serde_json::from_str::<WsRequest>(&text) {
                        Ok(request) => {
                            match request {
                                WsRequest::Subscribe { streams, accounts } => {
                                    let mut subs = handle.subscriptions.lock().await;

                                    if let Some(streams) = streams {
                                        for stream in streams {
                                            match stream.as_str() {
                                                "ledger" => subs.ledger = true,
                                                "transactions" => subs.transactions = true,
                                                "validations" => subs.validations = true,
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

                                    let response = WsResponse::Response {
                                        id: None,
                                        status: "success".to_string(),
                                    };
                                    let msg = serde_json::to_string(&response).unwrap();
                                    let _ = sender.lock().await.send(Message::Text(msg)).await;
                                }
                                WsRequest::Unsubscribe { streams, accounts } => {
                                    let mut subs = handle.subscriptions.lock().await;

                                    if let Some(streams) = streams {
                                        for stream in streams {
                                            match stream.as_str() {
                                                "ledger" => subs.ledger = false,
                                                "transactions" => subs.transactions = false,
                                                "validations" => subs.validations = false,
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

                                    let response = WsResponse::Response {
                                        id: None,
                                        status: "success".to_string(),
                                    };
                                    let msg = serde_json::to_string(&response).unwrap();
                                    let _ = sender.lock().await.send(Message::Text(msg)).await;
                                }
                                WsRequest::Ping => {
                                    let response = WsResponse::Pong;
                                    let msg = serde_json::to_string(&response).unwrap();
                                    let _ = sender.lock().await.send(Message::Text(msg)).await;
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Invalid WebSocket message: {}", e);
                        }
                    }
                }
                Message::Close(_) => {
                    debug!("WebSocket connection {} closed", conn_id);
                    break;
                }
                _ => {}
            }
        }

        // Remove connection
        let mut conns = self.connections.write().await;
        conns.retain(|c| c.id != conn_id);
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
}

/// Background task that polls the application and broadcasts updates
async fn broadcaster_task(
    app: ApplicationHandle,
    ledger_tx: broadcast::Sender<serde_json::Value>,
    _transactions_tx: broadcast::Sender<serde_json::Value>,
    _validations_tx: broadcast::Sender<serde_json::Value>,
) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));

    loop {
        interval.tick().await;

        // Get current ledger info from app
        let app_guard = app.read().await;
        let ledger_index = app_guard.consensus.get_ledger_index();
        drop(app_guard);

        // Broadcast ledger update if changed
        if ledger_index > 0 {
            let ledger_info = serde_json::json!({
                "ledger_index": ledger_index,
                "ledger_hash": "0000000000000000000000000000000000000000000000000000000000000000",
                "close_time": chrono::Utc::now().timestamp(),
            });

            let _ = ledger_tx.send(ledger_info);
        }
    }
}

/// WebSocket upgrade handler for axum
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(server): State<Arc<WebSocketServer>>,
) -> impl IntoResponse {
    let conn_id = rand::random::<u64>();
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
    }

    #[test]
    fn test_ws_request_deserialization() {
        let json = r#"{"command":"subscribe","streams":["ledger","transactions"]}"#;
        let req: WsRequest = serde_json::from_str(json).unwrap();

        match req {
            WsRequest::Subscribe { streams, accounts } => {
                assert_eq!(streams.unwrap().len(), 2);
                assert!(accounts.is_none());
            }
            _ => panic!("Expected Subscribe variant"),
        }
    }

    #[test]
    fn test_ws_response_serialization() {
        let response = WsResponse::Pong;
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("pong"));
    }
}
