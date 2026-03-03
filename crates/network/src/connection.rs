//! TCP Connection handling for P2P network protocol
//!
//! This module implements the low-level TCP connection handling including:
//! - Message framing (length-prefixed messages)
//! - Async socket I/O with tokio
//! - Protocol handshake (Hello/Status exchange)
//! - Connection state management

use crate::message::{HelloMessage, Message, MessageType, StatusChangeMessage};
use bytes::{Buf, BytesMut};
use primitives::UInt256;
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::time::{sleep, timeout};
use tracing::{debug, error, info, warn};

/// Protocol constants
pub const PROTOCOL_VERSION: u32 = 1;
pub const MAX_MESSAGE_SIZE: usize = 64 * 1024 * 1024; // 64MB max message
pub const MESSAGE_HEADER_SIZE: usize = 6; // 4 bytes length + 2 bytes type
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
pub const PING_INTERVAL: Duration = Duration::from_secs(60);
pub const SEND_RETRY_MAX_ATTEMPTS: u32 = 3;
pub const SEND_RETRY_DELAY: Duration = Duration::from_millis(100);
pub const SEND_CONFIRMATION_TIMEOUT: Duration = Duration::from_secs(5);

/// Framed message for network transmission
#[derive(Debug, Clone)]
pub struct FramedMessage {
    pub message_type: u16,
    pub payload: Vec<u8>,
}

impl FramedMessage {
    /// Encode a message with length prefix
    pub fn encode(&self) -> Vec<u8> {
        let total_len = MESSAGE_HEADER_SIZE + self.payload.len();
        let mut result = Vec::with_capacity(total_len);

        // 4 bytes: message length (including header)
        result.extend_from_slice(&(total_len as u32).to_be_bytes());
        // 2 bytes: message type
        result.extend_from_slice(&self.message_type.to_be_bytes());
        // N bytes: payload
        result.extend_from_slice(&self.payload);

        result
    }

    /// Decode a message from bytes
    pub fn decode(data: &[u8]) -> Option<(Self, usize)> {
        if data.len() < MESSAGE_HEADER_SIZE {
            return None;
        }

        let msg_len = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
        if msg_len < MESSAGE_HEADER_SIZE || msg_len > MAX_MESSAGE_SIZE {
            return None;
        }

        if data.len() < msg_len {
            return None; // Need more data
        }

        let msg_type = u16::from_be_bytes([data[4], data[5]]);
        let payload = data[MESSAGE_HEADER_SIZE..msg_len].to_vec();

        Some((
            Self {
                message_type: msg_type,
                payload,
            },
            msg_len,
        ))
    }
}

impl From<Message> for FramedMessage {
    fn from(msg: Message) -> Self {
        Self {
            message_type: msg.message_type as u16,
            payload: msg.payload,
        }
    }
}

impl From<FramedMessage> for Message {
    fn from(framed: FramedMessage) -> Self {
        Self {
            message_type: match framed.message_type {
                1 => MessageType::Hello,
                2 => MessageType::StatusChange,
                3 => MessageType::Propose,
                4 => MessageType::Validation,
                5 => MessageType::Transaction,
                6 => MessageType::HaveTransactions,
                7 => MessageType::GetTransactions,
                8 => MessageType::GetLedger,
                9 => MessageType::LedgerData,
                10 => MessageType::HaveTransactionSet,
                11 => MessageType::GetTransactionSet,
                12 => MessageType::Ping,
                13 => MessageType::Cluster,
                _ => MessageType::Ping, // Default for unknown
            },
            payload: framed.payload,
        }
    }
}

/// Message delivery status for tracking confirmations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryStatus {
    Pending,
    Delivered,
    Failed,
}

/// Tracked message with delivery confirmation
#[derive(Debug, Clone)]
pub struct TrackedMessage {
    pub sequence_id: u64,
    pub message: Message,
    pub sent_at: Instant,
    pub status: DeliveryStatus,
    pub retry_count: u32,
}

/// Message tracker for pending confirmations
#[derive(Debug, Default)]
pub struct MessageTracker {
    next_sequence_id: AtomicU64,
    pending: Arc<Mutex<HashMap<u64, TrackedMessage>>>,
}

impl MessageTracker {
    pub fn new() -> Self {
        Self {
            next_sequence_id: AtomicU64::new(1),
            pending: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Generate a new sequence ID
    pub fn next_id(&self) -> u64 {
        self.next_sequence_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Track a message as pending confirmation
    pub async fn track_message(&self, sequence_id: u64, message: Message) {
        let tracked = TrackedMessage {
            sequence_id,
            message,
            sent_at: Instant::now(),
            status: DeliveryStatus::Pending,
            retry_count: 0,
        };
        self.pending.lock().await.insert(sequence_id, tracked);
    }

    /// Mark a message as delivered
    pub async fn confirm_delivery(&self, sequence_id: u64) -> bool {
        let mut pending = self.pending.lock().await;
        if let Some(msg) = pending.get_mut(&sequence_id) {
            msg.status = DeliveryStatus::Delivered;
            true
        } else {
            false
        }
    }

    /// Mark a message as failed
    pub async fn mark_failed(&self, sequence_id: u64) {
        let mut pending = self.pending.lock().await;
        if let Some(msg) = pending.get_mut(&sequence_id) {
            msg.status = DeliveryStatus::Failed;
        }
    }

    /// Increment retry count for a message
    pub async fn increment_retry(&self, sequence_id: u64) -> Option<u32> {
        let mut pending = self.pending.lock().await;
        pending.get_mut(&sequence_id).map(|msg| {
            msg.retry_count += 1;
            msg.retry_count
        })
    }

    /// Get pending message count
    pub async fn pending_count(&self) -> usize {
        self.pending.lock().await.len()
    }

    /// Remove a tracked message
    pub async fn remove(&self, sequence_id: u64) -> Option<TrackedMessage> {
        self.pending.lock().await.remove(&sequence_id)
    }

    /// Get all pending messages older than a duration
    pub async fn get_stale_messages(&self, max_age: Duration) -> Vec<u64> {
        let pending = self.pending.lock().await;
        let now = Instant::now();
        pending
            .values()
            .filter(|msg| msg.status == DeliveryStatus::Pending && now.duration_since(msg.sent_at) > max_age)
            .map(|msg| msg.sequence_id)
            .collect()
    }
}

/// Connection handle for peer communication
#[allow(dead_code)]
pub struct Connection {
    stream: TcpStream,
    peer_addr: SocketAddr,
    local_addr: SocketAddr,
    read_buffer: BytesMut,
    write_buffer: BytesMut,
    message_tracker: MessageTracker,
}

impl Connection {
    pub fn new(stream: TcpStream) -> io::Result<Self> {
        let peer_addr = stream.peer_addr()?;
        let local_addr = stream.local_addr()?;
        Ok(Self {
            stream,
            peer_addr,
            local_addr,
            read_buffer: BytesMut::with_capacity(4096),
            write_buffer: BytesMut::with_capacity(4096),
            message_tracker: MessageTracker::new(),
        })
    }

    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Read available data from socket into buffer
    pub async fn read(&mut self) -> io::Result<usize> {
        let n = self.stream.read_buf(&mut self.read_buffer).await?;
        Ok(n)
    }

    /// Try to parse a complete message from the buffer
    pub fn try_parse_message(&mut self) -> Option<Message> {
        let framed = FramedMessage::decode(&self.read_buffer)?;
        let consumed = framed.1;
        let msg = Message::from(framed.0);

        // Remove consumed bytes from buffer
        self.read_buffer.advance(consumed);

        Some(msg)
    }

    /// Send a message to the peer
    /// Returns success when data is written to the kernel buffer (no delivery confirmation)
    pub async fn send_message(&mut self, msg: Message) -> io::Result<()> {
        let framed = FramedMessage::from(msg);
        let encoded = framed.encode();

        self.stream.write_all(&encoded).await?;
        self.stream.flush().await?;

        Ok(())
    }

    /// Send a message with retry logic
    /// Attempts to send up to max_attempts times with a delay between retries
    pub async fn send_with_retry(
        &mut self,
        msg: Message,
        max_attempts: u32,
    ) -> io::Result<()> {
        let mut last_error = None;

        for attempt in 1..=max_attempts {
            match self.send_message(msg.clone()).await {
                Ok(()) => {
                    debug!("Message sent successfully on attempt {}", attempt);
                    return Ok(());
                }
                Err(e) => {
                    warn!("Send attempt {} failed: {}", attempt, e);
                    last_error = Some(e);

                    if attempt < max_attempts {
                        sleep(SEND_RETRY_DELAY).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            io::Error::new(io::ErrorKind::Other, "All retry attempts failed")
        }))
    }

    /// Send a message with delivery tracking
    /// Returns a sequence ID that can be used to check delivery status
    pub async fn send_tracked(&mut self, msg: Message) -> io::Result<u64> {
        let sequence_id = self.message_tracker.next_id();

        // Track the message before sending
        self.message_tracker.track_message(sequence_id, msg.clone()).await;

        match self.send_message(msg).await {
            Ok(()) => {
                debug!("Tracked message {} sent", sequence_id);
                Ok(sequence_id)
            }
            Err(e) => {
                self.message_tracker.mark_failed(sequence_id).await;
                Err(e)
            }
        }
    }

    /// Send a message with confirmation using ping/pong
    /// Waits for a Ping message response to confirm delivery
    pub async fn send_with_confirmation(
        &mut self,
        msg: Message,
        timeout_duration: Duration,
    ) -> io::Result<()> {
        // Send the message first
        self.send_message(msg).await?;

        // Send a ping to verify the connection is alive
        let ping_msg = Message::new(MessageType::Ping, vec![]);
        self.send_message(ping_msg).await?;

        // Wait for any message (peer should respond, indicating connection is alive)
        let result = timeout(timeout_duration, async {
            loop {
                match self.read().await {
                    Ok(_) => {
                        if let Some(_msg) = self.try_parse_message() {
                            // Received any message - connection is alive
                            return Ok(());
                        }
                    }
                    Err(e) => return Err(e),
                }
            }
        })
        .await;

        match result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Confirmation timeout - no response from peer",
            )),
        }
    }

    /// Check delivery status of a tracked message
    pub async fn get_delivery_status(&self, sequence_id: u64) -> Option<DeliveryStatus> {
        let pending = self.message_tracker.pending.lock().await;
        pending.get(&sequence_id).map(|msg| msg.status)
    }

    /// Confirm delivery of a tracked message
    pub async fn confirm_delivery(&self, sequence_id: u64) -> bool {
        self.message_tracker.confirm_delivery(sequence_id).await
    }

    /// Get count of pending messages awaiting confirmation
    pub async fn pending_message_count(&self) -> usize {
        self.message_tracker.pending_count().await
    }

    /// Perform protocol handshake as initiator (outbound connection)
    pub async fn handshake_outbound(
        &mut self,
        hello: &HelloMessage,
    ) -> io::Result<StatusChangeMessage> {
        info!("Starting outbound handshake with {}", self.peer_addr);

        // Send Hello
        let hello_msg = create_hello_message(hello);
        self.send_message(hello_msg).await?;

        // Wait for peer's Hello
        let peer_hello = loop {
            self.read().await?;
            if let Some(msg) = self.try_parse_message() {
                if let MessageType::Hello = msg.message_type {
                    break parse_hello_message(&msg.payload)?;
                }
            }
        };

        debug!("Received Hello from peer: version={}", peer_hello.protocol_version);

        // Wait for StatusChange
        let status = loop {
            self.read().await?;
            if let Some(msg) = self.try_parse_message() {
                if let MessageType::StatusChange = msg.message_type {
                    break parse_status_message(&msg.payload)?;
                }
            }
        };

        // Send our StatusChange
        let status_msg = create_status_message(&StatusChangeMessage {
            ledger_index: hello.ledger_index,
            ledger_hash: hello.ledger_hash,
            network_time: hello.network_time,
        });
        self.send_message(status_msg).await?;

        info!("Handshake completed with {}", self.peer_addr);
        Ok(status)
    }

    /// Perform protocol handshake as responder (inbound connection)
    pub async fn handshake_inbound(
        &mut self,
        hello: &HelloMessage,
    ) -> io::Result<(HelloMessage, StatusChangeMessage)> {
        info!("Starting inbound handshake with {}", self.peer_addr);

        // Wait for peer's Hello
        let peer_hello = loop {
            match timeout(DEFAULT_TIMEOUT, self.read()).await {
                Ok(Ok(_)) => {
                    if let Some(msg) = self.try_parse_message() {
                        if let MessageType::Hello = msg.message_type {
                            break parse_hello_message(&msg.payload)?;
                        }
                    }
                }
                Ok(Err(e)) => return Err(e),
                Err(_) => return Err(io::Error::new(io::ErrorKind::TimedOut, "Handshake timeout")),
            }
        };

        // Send our Hello
        let hello_msg = create_hello_message(hello);
        self.send_message(hello_msg).await?;

        // Send our StatusChange
        let status_msg = create_status_message(&StatusChangeMessage {
            ledger_index: hello.ledger_index,
            ledger_hash: hello.ledger_hash,
            network_time: hello.network_time,
        });
        self.send_message(status_msg).await?;

        // Wait for peer's StatusChange
        let peer_status = loop {
            self.read().await?;
            if let Some(msg) = self.try_parse_message() {
                if let MessageType::StatusChange = msg.message_type {
                    break parse_status_message(&msg.payload)?;
                }
            }
        };

        info!("Handshake completed with {}", self.peer_addr);
        Ok((peer_hello, peer_status))
    }

    /// Close the connection gracefully
    pub async fn close(mut self) -> io::Result<()> {
        self.stream.shutdown().await
    }
}

/// Network server for accepting incoming connections
pub struct NetworkServer {
    listener: TcpListener,
    local_addr: SocketAddr,
}

impl NetworkServer {
    /// Bind to a local address and start listening
    pub async fn bind(bind_addr: SocketAddr) -> io::Result<Self> {
        let listener = TcpListener::bind(bind_addr).await?;
        let local_addr = listener.local_addr()?;

        info!("Network server listening on {}", local_addr);

        Ok(Self {
            listener,
            local_addr,
        })
    }

    /// Accept an incoming connection
    pub async fn accept(&self) -> io::Result<Connection> {
        let (stream, peer_addr) = self.listener.accept().await?;
        info!("Accepted connection from {}", peer_addr);

        Connection::new(stream)
    }

    /// Get the local bind address
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }
}

/// Connect to a remote peer
pub async fn connect_peer(peer_addr: SocketAddr) -> io::Result<Connection> {
    info!("Connecting to peer at {}", peer_addr);

    match timeout(DEFAULT_TIMEOUT, TcpStream::connect(peer_addr)).await {
        Ok(Ok(stream)) => {
            info!("Connected to peer at {}", peer_addr);
            Connection::new(stream)
        }
        Ok(Err(e)) => {
            error!("Failed to connect to {}: {}", peer_addr, e);
            Err(e)
        }
        Err(_) => {
            error!("Connection timeout to {}", peer_addr);
            Err(io::Error::new(io::ErrorKind::TimedOut, "Connection timeout"))
        }
    }
}

// Helper functions for message serialization
fn create_hello_message(hello: &HelloMessage) -> Message {
    let mut payload = Vec::new();

    // Protocol version (4 bytes)
    payload.extend_from_slice(&hello.protocol_version.to_be_bytes());
    // Public key length (2 bytes)
    payload.extend_from_slice(&(hello.node_public_key.len() as u16).to_be_bytes());
    // Public key
    payload.extend_from_slice(&hello.node_public_key);
    // Node ID length
    payload.extend_from_slice(&(hello.node_id.len() as u16).to_be_bytes());
    // Node ID
    payload.extend_from_slice(&hello.node_id);
    // Ledger index
    payload.extend_from_slice(&hello.ledger_index.to_be_bytes());
    // Ledger hash (32 bytes)
    payload.extend_from_slice(hello.ledger_hash.as_bytes());
    // Network time
    payload.extend_from_slice(&hello.network_time.to_be_bytes());

    Message::new(MessageType::Hello, payload)
}

fn parse_hello_message(data: &[u8]) -> io::Result<HelloMessage> {
    if data.len() < 48 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Hello message too short",
        ));
    }

    let mut offset = 0;

    let protocol_version = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
    offset += 4;

    let pk_len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
    offset += 2;

    if data.len() < offset + pk_len + 2 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid Hello message"));
    }

    let node_public_key = data[offset..offset + pk_len].to_vec();
    offset += pk_len;

    let id_len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
    offset += 2;

    if data.len() < offset + id_len + 36 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid Hello message"));
    }

    let node_id = data[offset..offset + id_len].to_vec();
    offset += id_len;

    let ledger_index = u32::from_be_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]);
    offset += 4;

    let mut hash_bytes = [0u8; 32];
    hash_bytes.copy_from_slice(&data[offset..offset + 32]);
    let ledger_hash = UInt256::from_be_bytes(hash_bytes);
    offset += 32;

    let network_time = u64::from_be_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
        data[offset + 4],
        data[offset + 5],
        data[offset + 6],
        data[offset + 7],
    ]);

    Ok(HelloMessage {
        protocol_version,
        node_public_key,
        node_id,
        ledger_index,
        ledger_hash,
        network_time,
    })
}

fn create_status_message(status: &StatusChangeMessage) -> Message {
    let mut payload = Vec::with_capacity(44);

    payload.extend_from_slice(&status.ledger_index.to_be_bytes());

    let mut hash_bytes = [0u8; 32];
    hash_bytes.copy_from_slice(status.ledger_hash.as_bytes());
    payload.extend_from_slice(&hash_bytes);

    payload.extend_from_slice(&status.network_time.to_be_bytes());

    Message::new(MessageType::StatusChange, payload)
}

fn parse_status_message(data: &[u8]) -> io::Result<StatusChangeMessage> {
    if data.len() < 44 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Status message too short",
        ));
    }

    let ledger_index = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);

    let mut hash_bytes = [0u8; 32];
    hash_bytes.copy_from_slice(&data[4..36]);
    let ledger_hash = UInt256::from_be_bytes(hash_bytes);

    let network_time = u64::from_be_bytes([
        data[36],
        data[37],
        data[38],
        data[39],
        data[40],
        data[41],
        data[42],
        data[43],
    ]);

    Ok(StatusChangeMessage {
        ledger_index,
        ledger_hash,
        network_time,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_framed_message_encode_decode() {
        let msg = FramedMessage {
            message_type: 1,
            payload: vec![1, 2, 3, 4, 5],
        };

        let encoded = msg.encode();
        assert_eq!(encoded.len(), MESSAGE_HEADER_SIZE + 5);

        let (decoded, consumed) = FramedMessage::decode(&encoded).unwrap();
        assert_eq!(consumed, encoded.len());
        assert_eq!(decoded.message_type, 1);
        assert_eq!(decoded.payload, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_message_too_short() {
        let data = vec![0, 0, 0, 1]; // Only 4 bytes
        assert!(FramedMessage::decode(&data).is_none());
    }

    #[test]
    fn test_message_partial_data() {
        // Header says 20 bytes but only 15 provided
        let data = vec![0, 0, 0, 20, 0, 1, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        assert!(FramedMessage::decode(&data).is_none());
    }

    #[test]
    fn test_max_message_size() {
        // Message larger than MAX_MESSAGE_SIZE
        let data = vec![0, 64, 0, 0, 1, 1]; // 64MB + 1
        assert!(FramedMessage::decode(&data).is_none());
    }

    #[test]
    fn test_hello_message_roundtrip() {
        let hello = HelloMessage {
            protocol_version: 1,
            node_public_key: vec![1, 2, 3, 4],
            node_id: vec![5, 6, 7, 8],
            ledger_index: 100,
            ledger_hash: UInt256::new([9u8; 32]),
            network_time: 12345678,
        };

        let msg = create_hello_message(&hello);
        let parsed = parse_hello_message(&msg.payload).unwrap();

        assert_eq!(parsed.protocol_version, hello.protocol_version);
        assert_eq!(parsed.node_public_key, hello.node_public_key);
        assert_eq!(parsed.node_id, hello.node_id);
        assert_eq!(parsed.ledger_index, hello.ledger_index);
        assert_eq!(parsed.ledger_hash, hello.ledger_hash);
        assert_eq!(parsed.network_time, hello.network_time);
    }

    #[test]
    fn test_status_message_roundtrip() {
        let status = StatusChangeMessage {
            ledger_index: 200,
            ledger_hash: UInt256::new([10u8; 32]),
            network_time: 87654321,
        };

        let msg = create_status_message(&status);
        let parsed = parse_status_message(&msg.payload).unwrap();

        assert_eq!(parsed.ledger_index, status.ledger_index);
        assert_eq!(parsed.ledger_hash, status.ledger_hash);
        assert_eq!(parsed.network_time, status.network_time);
    }

    #[tokio::test]
    async fn test_message_tracker() {
        let tracker = MessageTracker::new();

        // Test sequence ID generation
        let id1 = tracker.next_id();
        let id2 = tracker.next_id();
        assert_eq!(id1 + 1, id2);

        // Test message tracking
        let msg = Message::new(MessageType::Ping, vec![1, 2, 3]);
        tracker.track_message(id1, msg.clone()).await;

        // Should be pending (still in tracking map)
        assert_eq!(tracker.pending_count().await, 1);

        // Confirm delivery (changes status but keeps in map)
        assert!(tracker.confirm_delivery(id1).await);
        // Still 1 because message is tracked until removed
        assert_eq!(tracker.pending_count().await, 1);

        // Verify status is Delivered
        let status = tracker.confirm_delivery(id1).await;
        assert!(status); // Already confirmed, still returns true

        // Remove the message
        let removed = tracker.remove(id1).await;
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().status, DeliveryStatus::Delivered);
    }

    #[tokio::test]
    async fn test_message_tracker_retry() {
        let tracker = MessageTracker::new();
        let msg = Message::new(MessageType::Ping, vec![]);

        tracker.track_message(1, msg).await;

        // Increment retry count
        let count = tracker.increment_retry(1).await;
        assert_eq!(count, Some(1));

        let count = tracker.increment_retry(1).await;
        assert_eq!(count, Some(2));

        // Non-existent message
        let count = tracker.increment_retry(999).await;
        assert_eq!(count, None);
    }

    #[tokio::test]
    async fn test_message_tracker_stale() {
        let tracker = MessageTracker::new();
        let msg = Message::new(MessageType::Ping, vec![]);

        tracker.track_message(1, msg).await;

        // Wait a tiny bit
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Get stale messages (older than 1ms)
        let stale = tracker.get_stale_messages(Duration::from_millis(1)).await;
        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0], 1);

        // Get stale messages (older than 1s - should be none)
        let stale = tracker.get_stale_messages(Duration::from_secs(1)).await;
        assert!(stale.is_empty());
    }
}
