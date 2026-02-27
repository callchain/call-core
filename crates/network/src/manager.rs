//! Network Manager - bridges TCP connections with the overlay network
//!
//! Manages the lifecycle of peer connections:
//! - Outbound connections to bootstrap/configured peers
//! - Inbound connections from incoming peers
//! - Message routing between Connection and Overlay
//! - Connection maintenance (ping/pong, timeouts)

use crate::connection::{connect_peer, Connection, NetworkServer, PING_INTERVAL};
use crate::message::{HelloMessage, Message};
use crate::overlay::Overlay;
use crate::peer::Peer;
use primitives::{LedgerIndex, NodeID, UInt256};
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, timeout};
use tracing::{debug, error, info, warn};

/// Commands for the network manager
#[derive(Debug)]
pub enum NetworkCommand {
    /// Connect to a peer
    Connect(SocketAddr),
    /// Disconnect from a peer
    Disconnect(SocketAddr),
    /// Broadcast a message to all peers
    Broadcast(Message),
    /// Send a message to a specific peer
    SendTo(SocketAddr, Message),
    /// Shutdown the network manager
    Shutdown,
}

/// Network manager events
#[derive(Debug)]
pub enum NetworkEvent {
    /// New peer connected
    PeerConnected(SocketAddr, NodeID),
    /// Peer disconnected
    PeerDisconnected(SocketAddr),
    /// Message received from peer
    MessageReceived(SocketAddr, Message),
    /// Peer status updated
    StatusUpdated(SocketAddr, LedgerIndex, UInt256),
}

/// Manages all network connections and bridges with overlay
#[allow(dead_code)]
pub struct NetworkManager {
    overlay: Arc<RwLock<Overlay>>,
    node_id: NodeID,
    local_addr: SocketAddr,
    server: Option<NetworkServer>,
    command_rx: mpsc::Receiver<NetworkCommand>,
    event_tx: mpsc::Sender<NetworkEvent>,
    peers: HashMap<SocketAddr, PeerConnection>,
    hello_message: HelloMessage,
}

/// Active peer connection state
#[allow(dead_code)]
struct PeerConnection {
    address: SocketAddr,
    state: PeerConnectionState,
    last_activity: tokio::time::Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum PeerConnectionState {
    Connecting,
    Handshaking,
    Active,
    Closing,
}

impl NetworkManager {
    /// Create a new network manager
    pub async fn new(
        bind_addr: SocketAddr,
        node_id: NodeID,
        overlay: Arc<RwLock<Overlay>>,
    ) -> io::Result<(Self, mpsc::Sender<NetworkCommand>, mpsc::Receiver<NetworkEvent>)> {
        let server = NetworkServer::bind(bind_addr).await?;
        let local_addr = server.local_addr();

        let (command_tx, command_rx) = mpsc::channel(100);
        let (event_tx, event_rx) = mpsc::channel(100);

        let hello_message = HelloMessage {
            protocol_version: crate::connection::PROTOCOL_VERSION,
            node_public_key: vec![], // TODO: Add actual public key
            node_id: node_id.as_bytes().to_vec(),
            ledger_index: 1,
            ledger_hash: UInt256::zero(),
            network_time: 0,
        };

        Ok((
            Self {
                overlay,
                node_id,
                local_addr,
                server: Some(server),
                command_rx,
                event_tx,
                peers: HashMap::new(),
                hello_message,
            },
            command_tx,
            event_rx,
        ))
    }

    /// Run the network manager main loop
    pub async fn run(mut self) -> io::Result<()> {
        info!("Starting network manager on {}", self.local_addr);

        // Start the server accept task
        let server = self.server.take().unwrap();
        let (accept_tx, mut accept_rx) = mpsc::channel(10);

        tokio::spawn(async move {
            loop {
                match server.accept().await {
                    Ok(connection) => {
                        if accept_tx.send(connection).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Accept error: {}", e);
                    }
                }
            }
        });

        // Main event loop
        let mut ping_interval = interval(PING_INTERVAL);
        let mut cleanup_interval = interval(Duration::from_secs(60));

        loop {
            tokio::select! {
                // Handle incoming connections
                Ok(connection) = async { accept_rx.recv().await.ok_or(()) },
                    if !matches!(self.peers.len() >= 50, true) =>
                {
                    if let Err(e) = self.handle_incoming(connection).await {
                        warn!("Failed to handle incoming connection: {}", e);
                    }
                }

                // Handle commands
                Some(cmd) = self.command_rx.recv() => {
                    match cmd {
                        NetworkCommand::Connect(addr) => {
                            if let Err(e) = self.connect_to_peer(addr).await {
                                warn!("Failed to connect to {}: {}", addr, e);
                            }
                        }
                        NetworkCommand::Disconnect(addr) => {
                            self.disconnect_peer(&addr).await;
                        }
                        NetworkCommand::Broadcast(msg) => {
                            self.broadcast_message(msg).await;
                        }
                        NetworkCommand::SendTo(addr, msg) => {
                            self.send_to_peer(&addr, msg).await;
                        }
                        NetworkCommand::Shutdown => {
                            info!("Shutting down network manager");
                            break;
                        }
                    }
                }

                // Periodic ping
                _ = ping_interval.tick() => {
                    self.send_pings().await;
                }

                // Periodic cleanup
                _ = cleanup_interval.tick() => {
                    self.cleanup_connections().await;
                }
            }
        }

        // Cleanup all connections
        let addrs: Vec<_> = self.peers.keys().copied().collect();
        for addr in addrs {
            self.disconnect_peer(&addr).await;
        }

        Ok(())
    }

    /// Handle an incoming connection
    async fn handle_incoming(&mut self, mut connection: Connection) -> io::Result<()> {
        let addr = connection.peer_addr();
        info!("Handling incoming connection from {}", addr);

        // Check if we can accept more peers
        {
            let overlay = self.overlay.read().await;
            if !overlay.can_accept_peer() {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Max peers reached",
                ));
            }
        }

        // Perform handshake
        let (peer_hello, peer_status) = match connection.handshake_inbound(&self.hello_message).await {
            Ok(result) => result,
            Err(e) => {
                warn!("Handshake failed with {}: {}", addr, e);
                return Err(e);
            }
        };

        // Create peer record
        let node_id = NodeID::new(
            peer_hello.node_id.try_into().unwrap_or([0u8; 32])
        );

        let peer = Peer::with_node_id(addr, node_id);

        // Add to overlay
        {
            let mut overlay = self.overlay.write().await;
            overlay.add_peer(peer);
        }

        // Track connection
        self.peers.insert(addr, PeerConnection {
            address: addr,
            state: PeerConnectionState::Active,
            last_activity: tokio::time::Instant::now(),
        });

        // Notify about new peer
        let _ = self.event_tx.send(NetworkEvent::PeerConnected(addr, node_id)).await;
        let _ = self.event_tx.send(NetworkEvent::StatusUpdated(
            addr,
            peer_status.ledger_index,
            peer_status.ledger_hash,
        )).await;

        // Spawn connection handler
        let event_tx = self.event_tx.clone();
        let overlay = self.overlay.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(connection, addr, event_tx, overlay).await {
                debug!("Connection handler error for {}: {}", addr, e);
            }
        });

        Ok(())
    }

    /// Connect to a peer
    async fn connect_to_peer(&mut self, addr: SocketAddr) -> io::Result<()> {
        if self.peers.contains_key(&addr) {
            return Ok(()); // Already connected
        }

        info!("Connecting to peer at {}", addr);

        // Check if we can add more peers
        {
            let overlay = self.overlay.read().await;
            if !overlay.can_accept_peer() {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Max peers reached",
                ));
            }
        }

        // Track connection attempt
        self.peers.insert(addr, PeerConnection {
            address: addr,
            state: PeerConnectionState::Connecting,
            last_activity: tokio::time::Instant::now(),
        });

        // Perform connection
        let mut connection = connect_peer(addr).await?;

        // Perform handshake
        let peer_status = match connection.handshake_outbound(&self.hello_message).await {
            Ok(status) => status,
            Err(e) => {
                self.peers.remove(&addr);
                return Err(e);
            }
        };

        // Create peer record
        let peer = Peer::new(addr);

        // Add to overlay
        {
            let mut overlay = self.overlay.write().await;
            overlay.add_peer(peer);
        }

        // Update connection state
        if let Some(conn) = self.peers.get_mut(&addr) {
            conn.state = PeerConnectionState::Active;
        }

        // Notify about new peer
        let _ = self.event_tx.send(NetworkEvent::PeerConnected(addr, NodeID::new([0u8; 32]))).await;
        let _ = self.event_tx.send(NetworkEvent::StatusUpdated(
            addr,
            peer_status.ledger_index,
            peer_status.ledger_hash,
        )).await;

        // Spawn connection handler
        let event_tx = self.event_tx.clone();
        let overlay = self.overlay.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(connection, addr, event_tx, overlay).await {
                debug!("Connection handler error for {}: {}", addr, e);
            }
        });

        Ok(())
    }

    /// Disconnect from a peer
    async fn disconnect_peer(&mut self, addr: &SocketAddr) {
        info!("Disconnecting from peer at {}", addr);

        if let Some(_conn) = self.peers.remove(addr) {
            // Remove from overlay
            let mut overlay = self.overlay.write().await;
            overlay.remove_peer(addr);
        }

        let _ = self.event_tx.send(NetworkEvent::PeerDisconnected(*addr)).await;
    }

    /// Broadcast a message to all peers
    async fn broadcast_message(&self, message: Message) {
        let mut overlay = self.overlay.write().await;
        overlay.broadcast(message);
    }

    /// Send a message to a specific peer
    async fn send_to_peer(&self, addr: &SocketAddr, message: Message) {
        let mut overlay = self.overlay.write().await;
        overlay.send_to(addr, message);
    }

    /// Send ping messages to all active peers
    async fn send_pings(&self) {
        let mut overlay = self.overlay.write().await;
        let ping = Message::ping();
        overlay.broadcast(ping);
    }

    /// Cleanup stale connections
    async fn cleanup_connections(&mut self) {
        let timeout = Duration::from_secs(300);
        let to_remove: Vec<_> = self.peers
            .iter()
            .filter(|(_, conn)| {
                conn.last_activity.elapsed() > timeout
            })
            .map(|(addr, _)| *addr)
            .collect();

        for addr in to_remove {
            warn!("Peer {} timed out, disconnecting", addr);
            self.disconnect_peer(&addr).await;
        }
    }
}

/// Handle a single connection - read messages and forward to overlay
async fn handle_connection(
    mut connection: Connection,
    addr: SocketAddr,
    event_tx: mpsc::Sender<NetworkEvent>,
    overlay: Arc<RwLock<Overlay>>,
) -> io::Result<()> {
    let timeout_duration = Duration::from_secs(30);

    loop {
        // Read with timeout
        match timeout(timeout_duration, connection.read()).await {
            Ok(Ok(0)) => {
                debug!("Connection closed by peer {}", addr);
                break;
            }
            Ok(Ok(_)) => {
                // Process any available messages
                while let Some(message) = connection.try_parse_message() {
                    // Update peer activity in overlay
                    {
                        let mut overlay = overlay.write().await;
                        overlay.process_message(&addr, message.clone());
                    }

                    // Forward event
                    if event_tx.send(NetworkEvent::MessageReceived(addr, message)).await.is_err() {
                        return Err(io::Error::new(io::ErrorKind::Other, "Event channel closed"));
                    }
                }
            }
            Ok(Err(e)) => {
                warn!("Read error from {}: {}", addr, e);
                break;
            }
            Err(_) => {
                warn!("Read timeout from {}", addr);
                break;
            }
        }
    }

    // Notify about disconnection
    let _ = event_tx.send(NetworkEvent::PeerDisconnected(addr)).await;

    // Close connection
    connection.close().await.ok();

    // Remove from overlay
    let mut overlay = overlay.write().await;
    overlay.remove_peer(&addr);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_manager_creation() {
        let bind_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let node_id = NodeID::new([0u8; 32]);
        let overlay = Arc::new(RwLock::new(Overlay::new()));

        let (manager, _cmd_tx, _event_rx) = NetworkManager::new(
            bind_addr,
            node_id,
            overlay,
        ).await.unwrap();

        assert_eq!(manager.node_id, node_id);
        assert!(manager.peers.is_empty());
    }
}
