use crate::message::Message;
use std::net::SocketAddr;

#[derive(Debug)]
pub struct Peer {
    pub address: SocketAddr,
    pub public_key: Vec<u8>,
    pub active: bool,
}

impl Peer {
    pub fn new(address: SocketAddr) -> Self {
        Self {
            address,
            public_key: Vec::new(),
            active: false,
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn activate(&mut self) {
        self.active = true;
    }

    pub fn send(&mut self, _message: &Message) {
        // Placeholder for actual sending
    }
}
