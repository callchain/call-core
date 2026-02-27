use crate::message::Message;
use primitives::NodeID;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

/// Peer connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerState {
    Connecting,
    Handshake,
    Active,
    Closing,
    Closed,
}

/// Peer statistics
#[derive(Debug, Clone)]
pub struct PeerStats {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub connected_at: Instant,
    pub last_activity: Instant,
}

impl Default for PeerStats {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            bytes_sent: 0,
            bytes_received: 0,
            messages_sent: 0,
            messages_received: 0,
            connected_at: now,
            last_activity: now,
        }
    }
}

/// A peer connection
#[derive(Debug)]
pub struct Peer {
    pub address: SocketAddr,
    pub node_id: Option<NodeID>,
    pub public_key: Vec<u8>,
    pub state: PeerState,
    pub stats: PeerStats,
    pub latency_ms: u32,
    outbound_queue: Vec<Message>,
}

impl Peer {
    pub fn new(address: SocketAddr) -> Self {
        Self {
            address,
            node_id: None,
            public_key: Vec::new(),
            state: PeerState::Connecting,
            stats: PeerStats::default(),
            latency_ms: 0,
            outbound_queue: Vec::new(),
        }
    }

    pub fn with_node_id(address: SocketAddr, node_id: NodeID) -> Self {
        Self {
            address,
            node_id: Some(node_id),
            public_key: Vec::new(),
            state: PeerState::Active,
            stats: PeerStats::default(),
            latency_ms: 0,
            outbound_queue: Vec::new(),
        }
    }

    pub fn is_active(&self) -> bool {
        self.state == PeerState::Active
    }

    pub fn get_state(&self) -> PeerState {
        self.state
    }

    pub fn set_state(&mut self, state: PeerState) {
        self.state = state;
    }

    pub fn set_node_id(&mut self, node_id: NodeID) {
        self.node_id = Some(node_id);
    }

    pub fn get_node_id(&self) -> Option<NodeID> {
        self.node_id
    }

    /// Queue a message for sending
    pub fn send(&mut self, message: Message) {
        self.outbound_queue.push(message);
        self.stats.messages_sent += 1;
    }

    /// Get and clear the outbound queue
    pub fn drain_outbound(&mut self) -> Vec<Message> {
        std::mem::take(&mut self.outbound_queue)
    }

    /// Record a received message
    pub fn record_received(&mut self, _message: &Message) {
        self.stats.messages_received += 1;
        self.stats.last_activity = Instant::now();
    }

    /// Check if the peer has timed out
    pub fn is_timed_out(&self, timeout: Duration) -> bool {
        self.stats.last_activity.elapsed() > timeout
    }

    /// Get connection duration
    pub fn connection_duration(&self) -> Duration {
        self.stats.connected_at.elapsed()
    }

    /// Update latency measurement
    pub fn update_latency(&mut self, latency_ms: u32) {
        // Simple moving average
        if self.latency_ms == 0 {
            self.latency_ms = latency_ms;
        } else {
            self.latency_ms = (self.latency_ms * 7 + latency_ms) / 8;
        }
    }

    /// Activate the peer
    pub fn activate(&mut self) {
        self.state = PeerState::Active;
    }

    /// Close the peer connection
    pub fn close(&mut self) {
        self.state = PeerState::Closing;
    }
}

/// Peer identification
#[derive(Debug, Clone)]
pub struct PeerIdentity {
    pub node_id: NodeID,
    pub public_key: Vec<u8>,
    pub address: SocketAddr,
}

/// Filter for peer connections
#[derive(Debug, Clone)]
pub struct PeerFilter {
    pub min_protocol_version: Option<u32>,
    pub max_protocol_version: Option<u32>,
    pub require_validated: bool,
}

impl Default for PeerFilter {
    fn default() -> Self {
        Self {
            min_protocol_version: None,
            max_protocol_version: None,
            require_validated: false,
        }
    }
}
