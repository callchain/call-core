use crate::message::{Message, MessageType};
use crate::peer::{Peer, PeerFilter, PeerState};
use consensus::{Consensus, Proposal, Validation};
use primitives::NodeID;
use protocol::Transaction;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;

/// Overlay network manager
pub struct Overlay {
    peers: HashMap<SocketAddr, Peer>,
    peer_by_node_id: HashMap<NodeID, SocketAddr>,
    pending_peers: Vec<SocketAddr>,
    max_peers: usize,
    target_peers: usize,
}

impl Overlay {
    pub fn new() -> Self {
        Self {
            peers: HashMap::new(),
            peer_by_node_id: HashMap::new(),
            pending_peers: Vec::new(),
            max_peers: 50,
            target_peers: 10,
        }
    }

    pub fn with_config(max_peers: usize, target_peers: usize) -> Self {
        Self {
            peers: HashMap::new(),
            peer_by_node_id: HashMap::new(),
            pending_peers: Vec::new(),
            max_peers,
            target_peers,
        }
    }

    /// Add a new peer connection
    pub fn add_peer(&mut self, peer: Peer) {
        let address = peer.address;
        if let Some(node_id) = peer.node_id {
            self.peer_by_node_id.insert(node_id, address);
        }
        self.peers.insert(address, peer);
    }

    /// Remove a peer by address
    pub fn remove_peer(&mut self, address: &SocketAddr) {
        if let Some(peer) = self.peers.remove(address) {
            if let Some(node_id) = peer.node_id {
                self.peer_by_node_id.remove(&node_id);
            }
        }
    }

    /// Get a peer by address
    pub fn get_peer(&self, address: &SocketAddr) -> Option<&Peer> {
        self.peers.get(address)
    }

    /// Get a mutable peer by address
    pub fn get_peer_mut(&mut self, address: &SocketAddr) -> Option<&mut Peer> {
        self.peers.get_mut(address)
    }

    /// Get a peer by node ID
    pub fn get_peer_by_node_id(&self, node_id: &NodeID) -> Option<&Peer> {
        self.peer_by_node_id
            .get(node_id)
            .and_then(|addr| self.peers.get(addr))
    }

    /// Get total peer count
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    /// Get count of active peers
    pub fn active_peer_count(&self) -> usize {
        self.peers.values().filter(|p| p.is_active()).count()
    }

    /// Broadcast a message to all active peers
    pub fn broadcast(&mut self, message: Message) {
        for peer in self.peers.values_mut() {
            if peer.is_active() {
                peer.send(message.clone());
            }
        }
    }

    /// Broadcast to all active peers except one
    pub fn relay(&mut self, exclude: &SocketAddr, message: Message) {
        for (addr, peer) in self.peers.iter_mut() {
            if addr != exclude && peer.is_active() {
                peer.send(message.clone());
            }
        }
    }

    /// Send a message to a specific peer
    pub fn send_to(&mut self, address: &SocketAddr, message: Message) -> bool {
        if let Some(peer) = self.peers.get_mut(address) {
            peer.send(message);
            true
        } else {
            false
        }
    }

    /// Send to a peer by node ID
    pub fn send_to_node(&mut self, node_id: &NodeID, message: Message) -> bool {
        if let Some(addr) = self.peer_by_node_id.get(node_id).copied() {
            self.send_to(&addr, message)
        } else {
            false
        }
    }

    /// Process a received message from a peer
    pub fn process_message(&mut self, from: &SocketAddr, message: Message) {
        if let Some(peer) = self.peers.get_mut(from) {
            peer.record_received(&message);

            match message.message_type {
                MessageType::Ping => {
                    // Send pong (ping back)
                    peer.send(Message::ping());
                }
                _ => {
                    // Other message types processed elsewhere
                }
            }
        }
    }

    /// Relay a proposal to all peers
    pub fn relay_proposal(&mut self, proposal: &Proposal, exclude: Option<&NodeID>) {
        let message = Message::propose(proposal);
        if let Some(exclude_id) = exclude {
            for (_addr, peer) in self.peers.iter_mut() {
                if peer.is_active() && peer.node_id.as_ref() != Some(exclude_id) {
                    peer.send(message.clone());
                }
            }
        } else {
            self.broadcast(message);
        }
    }

    /// Relay a validation to all peers
    pub fn relay_validation(&mut self, validation: &Validation, exclude: Option<&NodeID>) {
        let message = Message::validation(validation);
        if let Some(exclude_id) = exclude {
            for (_addr, peer) in self.peers.iter_mut() {
                if peer.is_active() && peer.node_id.as_ref() != Some(exclude_id) {
                    peer.send(message.clone());
                }
            }
        } else {
            self.broadcast(message);
        }
    }

    /// Broadcast a transaction to all peers
    pub fn broadcast_transaction(&mut self, tx: &Transaction) {
        let message = Message::transaction(tx);
        self.broadcast(message);
    }

    /// Check if we need more peers
    pub fn needs_more_peers(&self) -> bool {
        self.active_peer_count() < self.target_peers
    }

    /// Check if we can accept more connections
    pub fn can_accept_peer(&self) -> bool {
        self.peers.len() < self.max_peers
    }

    /// Clean up timed out peers
    pub fn cleanup_timed_out(&mut self, timeout: Duration) {
        let to_remove: Vec<_> = self
            .peers
            .values()
            .filter(|p| p.is_timed_out(timeout))
            .map(|p| p.address)
            .collect();

        for addr in to_remove {
            self.remove_peer(&addr);
        }
    }

    /// Get all active peers
    pub fn get_active_peers(&self) -> Vec<&Peer> {
        self.peers.values().filter(|p| p.is_active()).collect()
    }

    /// Get addresses of all active peers
    pub fn get_active_peer_addresses(&self) -> Vec<SocketAddr> {
        self.peers
            .values()
            .filter(|p| p.is_active())
            .map(|p| p.address)
            .collect()
    }

    /// Apply a filter to find matching peers
    pub fn filter_peers(&self, filter: &PeerFilter) -> Vec<&Peer> {
        self.peers
            .values()
            .filter(|p| {
                if filter.require_validated && !p.is_active() {
                    return false;
                }
                true
            })
            .collect()
    }
}

impl Default for Overlay {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overlay_peer_management() {
        let mut overlay = Overlay::new();
        assert_eq!(overlay.peer_count(), 0);

        let addr: SocketAddr = "127.0.0.1:1234".parse().unwrap();
        let peer = Peer::new(addr);
        overlay.add_peer(peer);

        assert_eq!(overlay.peer_count(), 1);
        assert!(overlay.get_peer(&addr).is_some());

        overlay.remove_peer(&addr);
        assert_eq!(overlay.peer_count(), 0);
    }

    #[test]
    fn test_overlay_can_accept() {
        let overlay = Overlay::with_config(2, 1);
        assert!(overlay.can_accept_peer());

        let mut overlay = overlay;
        let addr1: SocketAddr = "127.0.0.1:1234".parse().unwrap();
        let addr2: SocketAddr = "127.0.0.1:1235".parse().unwrap();
        overlay.add_peer(Peer::new(addr1));
        overlay.add_peer(Peer::new(addr2));

        assert!(!overlay.can_accept_peer());
    }
}
