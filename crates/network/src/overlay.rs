use crate::message::{Message, MessageType};
use crate::peer::{Peer, PeerFilter};
use consensus::{Proposal, Validation};
use primitives::NodeID;
use protocol::Transaction;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;

/// Peer slot type for reservation system
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerSlotType {
    /// Regular peer slot
    Regular,
    /// Reserved for cluster node
    Cluster,
    /// Reserved for specific peer
    Reserved,
}

/// Cluster node configuration
#[derive(Debug, Clone)]
pub struct ClusterNode {
    /// Node ID of the cluster peer
    pub node_id: NodeID,
    /// Fixed address (optional - can use discovery)
    pub fixed_address: Option<SocketAddr>,
    /// Whether this node is a hub in the cluster
    pub is_hub: bool,
}

impl ClusterNode {
    pub fn new(node_id: NodeID) -> Self {
        Self {
            node_id,
            fixed_address: None,
            is_hub: false,
        }
    }

    pub fn with_address(node_id: NodeID, address: SocketAddr) -> Self {
        Self {
            node_id,
            fixed_address: Some(address),
            is_hub: false,
        }
    }

    pub fn as_hub(mut self) -> Self {
        self.is_hub = true;
        self
    }
}

/// Reserved peer slot
#[derive(Debug, Clone)]
pub struct ReservedSlot {
    /// The slot type
    pub slot_type: PeerSlotType,
    /// Expected node ID (if known)
    pub expected_node_id: Option<NodeID>,
    /// Expected address (if fixed)
    pub expected_address: Option<SocketAddr>,
    /// Whether this slot is currently filled
    pub is_filled: bool,
}

/// Overlay network manager
#[allow(dead_code)]
pub struct Overlay {
    peers: HashMap<SocketAddr, Peer>,
    peer_by_node_id: HashMap<NodeID, SocketAddr>,
    pending_peers: Vec<SocketAddr>,
    max_peers: usize,
    target_peers: usize,
    /// Cluster nodes configuration
    cluster_nodes: HashMap<NodeID, ClusterNode>,
    /// Reserved peer slots
    reserved_slots: Vec<ReservedSlot>,
    /// Number of slots reserved for cluster nodes
    cluster_slot_count: usize,
    /// Enable proof of work for incoming connections
    pub require_pow: bool,
    /// PoW validator for incoming connections
    pow_validator: Option<crate::proof_of_work::PowValidator>,
}

impl Overlay {
    pub fn new() -> Self {
        Self {
            peers: HashMap::new(),
            peer_by_node_id: HashMap::new(),
            pending_peers: Vec::new(),
            max_peers: 50,
            target_peers: 10,
            cluster_nodes: HashMap::new(),
            reserved_slots: Vec::new(),
            cluster_slot_count: 0,
            require_pow: false,
            pow_validator: None,
        }
    }

    pub fn with_config(max_peers: usize, target_peers: usize) -> Self {
        Self {
            peers: HashMap::new(),
            peer_by_node_id: HashMap::new(),
            pending_peers: Vec::new(),
            max_peers,
            target_peers,
            cluster_nodes: HashMap::new(),
            reserved_slots: Vec::new(),
            cluster_slot_count: 0,
            require_pow: false,
            pow_validator: None,
        }
    }

    /// Enable proof of work for incoming connections
    pub fn enable_pow(mut self, difficulty: u8) -> Self {
        self.require_pow = true;
        self.pow_validator = Some(crate::proof_of_work::PowValidator::with_difficulty(difficulty));
        self
    }

    /// Check if proof of work is required
    pub fn is_pow_required(&self) -> bool {
        self.require_pow
    }

    /// Get PoW validator if enabled
    pub fn pow_validator(&self) -> Option<&crate::proof_of_work::PowValidator> {
        self.pow_validator.as_ref()
    }

    /// Add a cluster node
    pub fn add_cluster_node(&mut self, node: ClusterNode) {
        // Reserve a slot if not already reserved
        if !self.cluster_nodes.contains_key(&node.node_id) {
            self.cluster_slot_count += 1;
            self.reserved_slots.push(ReservedSlot {
                slot_type: PeerSlotType::Cluster,
                expected_node_id: Some(node.node_id),
                expected_address: node.fixed_address,
                is_filled: false,
            });
        }
        self.cluster_nodes.insert(node.node_id, node);
    }

    /// Remove a cluster node
    pub fn remove_cluster_node(&mut self, node_id: &NodeID) {
        if self.cluster_nodes.remove(node_id).is_some() {
            // Free the reserved slot
            if let Some(pos) = self.reserved_slots.iter().position(|s| {
                s.slot_type == PeerSlotType::Cluster && s.expected_node_id == Some(*node_id)
            }) {
                self.reserved_slots.remove(pos);
                self.cluster_slot_count = self.cluster_slot_count.saturating_sub(1);
            }
        }
    }

    /// Check if a node ID is a configured cluster node
    pub fn is_cluster_node(&self, node_id: &NodeID) -> bool {
        self.cluster_nodes.contains_key(node_id)
    }

    /// Get all cluster nodes
    pub fn cluster_nodes(&self) -> &HashMap<NodeID, ClusterNode> {
        &self.cluster_nodes
    }

    /// Reserve a peer slot
    pub fn reserve_slot(&mut self, node_id: Option<NodeID>, address: Option<SocketAddr>) {
        self.reserved_slots.push(ReservedSlot {
            slot_type: PeerSlotType::Reserved,
            expected_node_id: node_id,
            expected_address: address,
            is_filled: false,
        });
    }

    /// Get number of reserved slots
    pub fn reserved_slot_count(&self) -> usize {
        self.reserved_slots.len()
    }

    /// Get number of available regular slots
    pub fn available_regular_slots(&self) -> usize {
        let used_regular = self.peers.values().filter(|p| {
            !p.node_id.map_or(false, |id| self.is_cluster_node(&id))
        }).count();
        self.max_peers.saturating_sub(self.cluster_slot_count).saturating_sub(used_regular)
    }

    /// Check if an incoming connection matches a reserved slot
    pub fn matches_reserved_slot(&self, addr: SocketAddr, node_id: Option<NodeID>) -> bool {
        self.reserved_slots.iter().any(|slot| {
            !slot.is_filled && (
                (slot.expected_address == Some(addr)) ||
                (node_id.is_some() && slot.expected_node_id == node_id)
            )
        })
    }

    /// Mark a reserved slot as filled
    pub fn fill_reserved_slot(&mut self, addr: SocketAddr, node_id: NodeID) {
        if let Some(slot) = self.reserved_slots.iter_mut().find(|s| {
            !s.is_filled && (
                (s.expected_address == Some(addr)) ||
                (s.expected_node_id == Some(node_id))
            )
        }) {
            slot.is_filled = true;
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
