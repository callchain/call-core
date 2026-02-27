use crate::config::Config;
use crate::rpc::{RpcConfig, RpcServer, SimpleRpcHandler};
use consensus::{Consensus, ConsensusParms, ConsensusMode, ConsensusPhase};
use network::Overlay;
use primitives::NodeID;
use storage::Database;
use std::sync::Arc;
use tokio::sync::{RwLock, watch};
use tokio::task::JoinHandle;
use tracing::{info, debug, warn};

/// Node state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeState {
    Starting,
    Syncing,
    Tracking,
    Full,
    Stopping,
    Stopped,
}

/// Core application that orchestrates all components
pub struct Application {
    pub config: Config,
    pub node_id: NodeID,
    pub state: NodeState,
    pub consensus: Consensus,
    pub overlay: Overlay,
    pub database: Database,
    pub rpc_config: RpcConfig,
    rpc_shutdown_tx: Option<watch::Sender<bool>>,
    rpc_handle: Option<JoinHandle<()>>,
    last_consensus_tick: std::time::Instant,
    /// Current ledger info
    current_ledger_hash: primitives::UInt256,
    current_ledger_seq: u32,
    /// Ledger state for account and object storage
    ledger_state: protocol::LedgerState,
    /// Transaction queue for pending transactions
    tx_queue: protocol::TransactionQueue,
}

impl Application {
    /// Create a new application instance
    pub fn new(config: Config) -> anyhow::Result<Self> {
        // Initialize storage
        let backend = Box::new(storage::RocksDBBackend::new(&config.data_dir)
            .map_err(|e| anyhow::anyhow!("Failed to open database: {}", e))?);
        let database = Database::new(backend);

        // Generate or load node identity
        let node_id = if let Some(_seed) = &config.validation_seed {
            // In production, derive node_id from seed
            NodeID::new([0u8; 32])
        } else {
            // Generate random node_id
            NodeID::new([0u8; 32])
        };

        // Initialize consensus
        let consensus = Consensus::new(node_id, ConsensusParms::default())
            .with_mode(if config.validation_seed.is_some() {
                ConsensusMode::Proposing
            } else {
                ConsensusMode::Observing
            });

        // Initialize overlay network
        let overlay = Overlay::with_config(
            config.max_peers,
            config.target_peers,
        );

        // Initialize RPC config
        let rpc_config = RpcConfig {
            enabled: config.rpc_enabled,
            bind_address: config.rpc_bind_address.clone(),
            port: config.rpc_port,
            admin_enabled: config.rpc_admin_enabled,
        };

        // Initialize ledger state
        let ledger_state = protocol::LedgerState::new();

        // Initialize transaction queue (max 10000 pending transactions)
        let tx_queue = protocol::TransactionQueue::new(10000);

        info!("Application initialized with node_id: {:?}", node_id);

        Ok(Self {
            config,
            node_id,
            state: NodeState::Starting,
            consensus,
            overlay,
            database,
            rpc_config,
            rpc_shutdown_tx: None,
            rpc_handle: None,
            last_consensus_tick: std::time::Instant::now(),
            current_ledger_hash: primitives::UInt256::zero(),
            current_ledger_seq: 0,
            ledger_state,
            tx_queue,
        })
    }

    /// Get the current node state
    pub fn get_state(&self) -> NodeState {
        self.state
    }

    /// Set the node state
    pub fn set_state(&mut self, state: NodeState) {
        info!("Node state transition: {:?} -> {:?}", self.state, state);
        self.state = state;
    }

    /// Get the current ledger hash
    pub fn get_current_ledger_hash(&self) -> primitives::UInt256 {
        self.current_ledger_hash
    }

    /// Get the current ledger sequence
    pub fn get_current_ledger_seq(&self) -> u32 {
        self.current_ledger_seq
    }

    /// Get the ledger state
    pub fn get_ledger_state(&self) -> &protocol::LedgerState {
        &self.ledger_state
    }

    /// Get the ledger state (mutable)
    pub fn get_ledger_state_mut(&mut self) -> &mut protocol::LedgerState {
        &mut self.ledger_state
    }

    /// Start the application
    pub async fn start(&mut self) -> anyhow::Result<()> {
        info!("Starting Call Core node: {}", self.config.node_name);

        self.set_state(NodeState::Syncing);

        // Initialize peer connections
        self.initialize_peers().await;

        // Load or create genesis ledger
        self.initialize_ledger().await?;

        // Initialize consensus
        self.initialize_consensus().await?;

        self.set_state(NodeState::Full);
        info!("Node is now operational");

        Ok(())
    }

    /// Initialize peer connections from config
    async fn initialize_peers(&mut self) {
        info!("Initializing {} bootstrap peers", self.config.peers.len());

        for peer_addr in &self.config.peers {
            info!("Adding bootstrap peer: {}", peer_addr);
            // Add peer to overlay - connection will be established lazily
            let peer = network::Peer::new(*peer_addr);
            self.overlay.add_peer(peer);
        }
    }

    /// Initialize the ledger - load from database or create genesis
    async fn initialize_ledger(&mut self) -> anyhow::Result<()> {
        info!("Initializing ledger...");

        // Try to load the latest ledger from database
        // For now, create genesis ledger
        let genesis = protocol::Ledger::genesis();
        let genesis_hash = genesis.get_hash();
        let genesis_seq = genesis.get_seq();

        info!(
            "Created genesis ledger: seq={}, hash={}",
            genesis_seq,
            hex::encode(genesis_hash.as_bytes())
        );

        // Store the current ledger info
        self.current_ledger_hash = genesis_hash;
        self.current_ledger_seq = genesis_seq;

        // Store genesis ledger in database
        // Serialize ledger info and store it
        let ledger_data = serde_json::to_vec(&serde_json::json!({
            "hash": hex::encode(genesis_hash.as_bytes()),
            "seq": genesis_seq,
            "parent_hash": hex::encode(genesis.info.parent_hash.as_bytes()),
            "close_time": genesis.info.close_time,
        }))?;
        self.database.store_ledger(genesis_hash, ledger_data);

        info!("Stored genesis ledger in database");

        Ok(())
    }

    /// Initialize consensus module
    async fn initialize_consensus(&mut self) -> anyhow::Result<()> {
        info!("Initializing consensus...");

        // Set consensus to tracking mode with the current ledger hash
        let ledger_hash = if self.current_ledger_hash == primitives::UInt256::zero() {
            // Fallback to genesis if ledger not initialized
            protocol::Ledger::genesis().get_hash()
        } else {
            self.current_ledger_hash
        };
        let ledger_seq = if self.current_ledger_seq == 0 { 1 } else { self.current_ledger_seq };

        self.consensus.start_round(ledger_hash, ledger_seq);

        info!("Consensus initialized in {:?} mode with ledger seq={}",
            self.consensus.get_mode(), ledger_seq);

        Ok(())
    }

    /// Run the main application loop
    pub async fn run(&mut self) -> anyhow::Result<()> {
        self.start().await?;

        // Start RPC server if enabled
        if self.rpc_config.enabled {
            self.start_rpc_server().await?;
        }

        // Main event loop
        loop {
            if self.state == NodeState::Stopping {
                break;
            }

            // Process consensus
            self.process_consensus().await?;

            // Process network messages
            self.process_network().await?;

            // Small delay to prevent tight loop
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        Ok(())
    }

    /// Process consensus logic
    async fn process_consensus(&mut self) -> anyhow::Result<()> {
        use std::time::{SystemTime, UNIX_EPOCH};

        // Only run consensus tick periodically (every 1 second)
        if self.last_consensus_tick.elapsed().as_secs() < 1 {
            return Ok(());
        }
        self.last_consensus_tick = std::time::Instant::now();

        // Start round if not started
        if self.consensus.get_state().is_none() {
            let ledger_hash = if self.current_ledger_hash == primitives::UInt256::zero() {
                protocol::Ledger::genesis().get_hash()
            } else {
                self.current_ledger_hash
            };
            self.consensus.start_round(ledger_hash, self.current_ledger_seq.max(1));
        }

        // Process consensus based on current phase
        let phase = self.consensus.get_phase();
        match phase {
            ConsensusPhase::Open => {
                // Check if we should close the ledger
                if self.consensus.should_close_ledger() {
                    debug!("Closing ledger for consensus");
                    let close_time = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as u32;
                    // Create a transaction set hash from pending transactions
                    // For now, use the current ledger hash as the transaction set hash
                    let tx_set_hash = self.current_ledger_hash;
                    self.consensus.close_ledger(tx_set_hash, close_time);
                }
            }
            ConsensusPhase::Establish => {
                // Check if we should accept the current position
                if self.consensus.should_accept() {
                    info!("Consensus achieved, accepting position");
                    self.consensus.accept_position();
                }
            }
            ConsensusPhase::Processing => {
                // Process accepted transactions and create validation
                self.consensus.process_ledger()?;
            }
            ConsensusPhase::Accepted => {
                // Start new round if needed
                if self.consensus.is_round_complete() {
                    debug!("Starting new consensus round");
                    self.consensus.start_new_round()?;
                }
            }
        }

        Ok(())
    }

    /// Process network messages
    async fn process_network(&mut self) -> anyhow::Result<()> {
        // Get pending messages from overlay and process them
        // In a real implementation, this would receive messages from peer connections

        // Clean up timed out peers periodically
        self.overlay.cleanup_timed_out(std::time::Duration::from_secs(300));

        // Check if we need more peers and try to connect
        if self.overlay.needs_more_peers() {
            for peer_addr in &self.config.peers {
                debug!("Attempting to connect to bootstrap peer: {}", peer_addr);
                // In a full implementation, this would initiate TCP connections
            }
        }

        Ok(())
    }

    /// Start the RPC server
    async fn start_rpc_server(&mut self) -> anyhow::Result<()> {
        if !self.rpc_config.enabled {
            return Ok(());
        }

        let bind_addr = format!("{}:{}", self.rpc_config.bind_address, self.rpc_config.port);
        info!("Starting RPC server on {}", bind_addr);

        // Create RPC server with handler
        let handler = Box::new(SimpleRpcHandler::new());
        let rpc_server = RpcServer::new(self.rpc_config.clone(), handler);

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        self.rpc_shutdown_tx = Some(shutdown_tx);

        // Start RPC server in background task
        let handle = tokio::spawn(async move {
            if let Err(e) = rpc_server.run(shutdown_rx).await {
                warn!("RPC server error: {}", e);
            }
        });
        self.rpc_handle = Some(handle);

        info!("RPC server started on http://{}", bind_addr);
        Ok(())
    }

    /// Submit a transaction to the network
    pub fn submit_transaction(&mut self, tx_blob: &[u8]) -> anyhow::Result<String> {
        info!("Submitting transaction, size: {} bytes", tx_blob.len());

        // Step 1: Deserialize the transaction
        let tx = self.deserialize_transaction(tx_blob)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize transaction: {}", e))?;

        info!("Deserialized transaction: type={:?}, account={}",
            tx.get_tx_type(),
            hex::encode(tx.get_account().as_bytes())
        );

        // Step 2: Validate the transaction
        self.validate_transaction(&tx)
            .map_err(|e| anyhow::anyhow!("Transaction validation failed: {}", e))?;

        info!("Transaction validated successfully");

        // Step 3: Add to open ledger (transaction queue)
        self.add_to_open_ledger(&tx)
            .map_err(|e| anyhow::anyhow!("Failed to add to open ledger: {}", e))?;

        info!("Transaction added to open ledger");

        // Step 4: Broadcast to peers
        self.broadcast_transaction(tx_blob)
            .map_err(|e| anyhow::anyhow!("Failed to broadcast transaction: {}", e))?;

        info!("Transaction broadcast to peers");

        Ok("tesSUCCESS".to_string())
    }

    /// Deserialize a transaction from bytes
    fn deserialize_transaction(&self, tx_blob: &[u8]) -> anyhow::Result<protocol::Transaction> {
        if tx_blob.len() < 10 {
            return Err(anyhow::anyhow!("Transaction blob too short"));
        }

        // Parse basic transaction fields from the blob
        // This is a simplified parser - full implementation would parse all fields
        use serialization::SerialIter;

        let mut iter = SerialIter::new(tx_blob);

        // Read transaction type (2 bytes)
        let tx_type_val = iter.get16()
            .map_err(|e| anyhow::anyhow!("Failed to read tx type: {}", e))?;
        let tx_type = protocol::TxType::from_i16(tx_type_val as i16)
            .ok_or_else(|| anyhow::anyhow!("Invalid transaction type: {}", tx_type_val))?;

        // Read account (variable length, AccountID is 20 bytes)
        let account = iter.get_account()
            .map_err(|e| anyhow::anyhow!("Failed to read account: {}", e))?;

        // Read sequence (4 bytes)
        let sequence = iter.get32()
            .map_err(|e| anyhow::anyhow!("Failed to read sequence: {}", e))?;

        // Read fee (8 bytes)
        let fee = iter.get64()
            .map_err(|e| anyhow::anyhow!("Failed to read fee: {}", e))?;

        // Create transaction with parsed fields
        let mut tx = protocol::Transaction::new(tx_type, account, sequence);
        tx.set_fee(fee);

        // Parse remaining fields based on transaction type
        // For now, skip the rest as this is a simplified implementation

        Ok(tx)
    }

    /// Validate a transaction
    fn validate_transaction(&self, tx: &protocol::Transaction) -> anyhow::Result<()> {
        // Check transaction type is valid
        if tx.get_tx_type() == protocol::TxType::Invalid {
            return Err(anyhow::anyhow!("Invalid transaction type"));
        }

        // Check fee is reasonable (at least base fee)
        if tx.get_fee() < 10 {
            return Err(anyhow::anyhow!("Fee too low"));
        }

        // Check sequence is valid (must be > 0)
        if tx.get_sequence() == 0 {
            return Err(anyhow::anyhow!("Invalid sequence number"));
        }

        // Additional validation would include:
        // - Signature verification
        // - Account exists in ledger
        // - Sufficient balance for fee
        // - Sequence number matches expected

        Ok(())
    }

    /// Add transaction to open ledger
    fn add_to_open_ledger(&mut self, tx: &protocol::Transaction) -> anyhow::Result<()> {
        // Add the transaction to the queue for inclusion in the next consensus round
        debug!("Adding transaction to open ledger");

        // Clone the transaction and add to queue
        let tx_clone = tx.clone();
        match self.tx_queue.insert(tx_clone) {
            Ok(_) => {
                debug!("Transaction added to queue successfully");
                Ok(())
            }
            Err(ter) => {
                Err(anyhow::anyhow!("Failed to queue transaction: {:?}", ter))
            }
        }
    }

    /// Broadcast transaction to connected peers
    fn broadcast_transaction(&mut self, tx_blob: &[u8]) -> anyhow::Result<()> {
        use network::{Message, MessageType};

        // Create transaction message
        let _message = Message::new(MessageType::Transaction, tx_blob.to_vec());

        // Broadcast to all active peers
        let peer_count = self.overlay.active_peer_count();
        if peer_count == 0 {
            debug!("No peers to broadcast to");
            return Ok(());
        }

        debug!("Broadcasting transaction to {} peers", peer_count);

        // Send to each peer
        for peer_addr in self.overlay.get_active_peer_addresses() {
            debug!("Sending transaction to peer: {}", peer_addr);
            // In a full implementation, this would queue the message for sending
            // through the peer's connection
        }

        Ok(())
    }

    /// Get server info for RPC
    pub fn get_server_info(&self) -> serde_json::Value {
        let consensus_state = self.consensus.get_phase();
        let peer_count = self.overlay.active_peer_count();

        serde_json::json!({
            "info": {
                "build_version": env!("CARGO_PKG_VERSION"),
                "complete_ledgers": "empty",
                "io_latency_ms": 1,
                "load_factor": 1,
                "peers": peer_count,
                "server_state": format!("{:?}", self.state),
                "state_accounting": {
                    "connected": {"duration_us": "0", "transitions": 0},
                    "disconnected": {"duration_us": "0", "transitions": 0},
                    "full": {"duration_us": "0", "transitions": 0},
                    "syncing": {"duration_us": "0", "transitions": 0},
                    "tracking": {"duration_us": "0", "transitions": 0},
                },
                "uptime": 0,
                "validated_ledger": {
                    "hash": "0000000000000000000000000000000000000000000000000000000000000000",
                    "seq": 0,
                },
                "node_id": format!("{:?}", self.node_id),
                "consensus_phase": format!("{:?}", consensus_state),
            }
        })
    }

    /// Persist application state to database
    async fn persist_state(&self) -> anyhow::Result<()> {
        use storage::node_object::{NodeObject, NodeObjectType};

        info!("Persisting application state...");

        // 1. Persist peer information
        let peer_addresses: Vec<String> = self
            .overlay
            .get_active_peer_addresses()
            .iter()
            .map(|addr| addr.to_string())
            .collect();

        let peer_data = serde_json::to_vec(&peer_addresses)
            .map_err(|e| anyhow::anyhow!("Failed to serialize peers: {}", e))?;

        let peer_hash = crypto::sha512_half(&peer_data);
        let peer_object = NodeObject::new(NodeObjectType::Metadata, peer_hash, peer_data);
        self.database.store_node(peer_object);

        info!("Persisted {} peer addresses", peer_addresses.len());

        // 2. Persist node configuration/state
        let node_state = serde_json::json!({
            "node_id": hex::encode(self.node_id.as_bytes()),
            "shutdown_time": chrono::Utc::now().to_rfc3339(),
            "consensus_phase": format!("{:?}", self.consensus.get_phase()),
            "consensus_ledger_index": self.consensus.get_ledger_index(),
        });

        let state_data = serde_json::to_vec(&node_state)
            .map_err(|e| anyhow::anyhow!("Failed to serialize node state: {}", e))?;

        let state_hash = crypto::sha512_half(&state_data);
        let state_object = NodeObject::new(NodeObjectType::Metadata, state_hash, state_data);
        self.database.store_node(state_object);

        info!("Persisted node state");

        // 3. Persist last ledger info (placeholder - would store actual ledger)
        // In a full implementation, this would serialize and store the current ledger

        Ok(())
    }

    /// Shutdown the application gracefully
    pub async fn shutdown(&mut self) {
        info!("Shutting down application...");
        self.set_state(NodeState::Stopping);

        // Close all peer connections
        for peer_addr in self.overlay.get_active_peer_addresses() {
            debug!("Closing connection to peer: {}", peer_addr);
            if let Some(peer) = self.overlay.get_peer_mut(&peer_addr) {
                peer.close();
            }
        }

        // Save state to database
        debug!("Saving state to database...");
        if let Err(e) = self.persist_state().await {
            warn!("Failed to persist state: {}", e);
        }

        // Stop RPC server
        if let Some(shutdown_tx) = self.rpc_shutdown_tx.take() {
            info!("Stopping RPC server...");
            let _ = shutdown_tx.send(true);
        }

        // Wait for RPC server to stop
        if let Some(handle) = self.rpc_handle.take() {
            let _ = tokio::time::timeout(tokio::time::Duration::from_secs(5), handle).await;
        }

        self.set_state(NodeState::Stopped);
        info!("Application shutdown complete");
    }

    /// Request graceful shutdown
    pub fn request_shutdown(&mut self) {
        self.set_state(NodeState::Stopping);
    }
}

/// Application handle for sharing between tasks
pub type ApplicationHandle = Arc<RwLock<Application>>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;

    fn test_config() -> Config {
        let temp_dir = tempfile::tempdir().unwrap();
        Config {
            node_name: "test-node".to_string(),
            listen_address: SocketAddr::from(([127, 0, 0, 1], 0)),
            peers: vec![],
            data_dir: temp_dir.path().to_str().unwrap().to_string(),
            validation_seed: None,
            max_peers: 10,
            target_peers: 5,
            rpc_enabled: false,
            rpc_bind_address: "127.0.0.1".to_string(),
            rpc_port: 0,
            rpc_admin_enabled: false,
            log_level: "info".to_string(),
        }
    }

    #[tokio::test]
    async fn test_application_creation() {
        let config = test_config();
        let app = Application::new(config).unwrap();

        assert_eq!(app.get_state(), NodeState::Starting);
        assert_eq!(app.config.node_name, "test-node");
    }

    #[test]
    fn test_node_state_transitions() {
        let mut app = Application::new(test_config()).unwrap();

        assert_eq!(app.get_state(), NodeState::Starting);

        app.set_state(NodeState::Syncing);
        assert_eq!(app.get_state(), NodeState::Syncing);

        app.set_state(NodeState::Full);
        assert_eq!(app.get_state(), NodeState::Full);
    }
}
