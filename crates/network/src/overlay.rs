use crate::message::Message;
use crate::peer::Peer;
use std::collections::HashMap;
use std::net::SocketAddr;

pub struct Overlay {
    peers: HashMap<SocketAddr, Peer>,
}

impl Overlay {
    pub fn new() -> Self {
        Self {
            peers: HashMap::new(),
        }
    }

    pub fn add_peer(&mut self, peer: Peer) {
        self.peers.insert(peer.address, peer);
    }

    pub fn remove_peer(&mut self, address: &SocketAddr) {
        self.peers.remove(address);
    }

    pub fn get_peer(&self, address: &SocketAddr) -> Option<&Peer> {
        self.peers.get(address)
    }

    pub fn get_peer_mut(&mut self, address: &SocketAddr) -> Option<&mut Peer> {
        self.peers.get_mut(address)
    }

    pub fn broadcast(&mut self, message: &Message) {
        for peer in self.peers.values_mut() {
            if peer.is_active() {
                peer.send(message);
            }
        }
    }

    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }
}

impl Default for Overlay {
    fn default() -> Self {
        Self::new()
    }
}
