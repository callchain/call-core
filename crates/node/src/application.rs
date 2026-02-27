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

    /// Start the application
    pub async fn start(&mut self) -> anyhow::Result<()> {
        info!("Starting Call Core node: {}", self.config.node_name);

        self.set_state(NodeState::Syncing);

        // TODO: Connect to bootstrap peers
        for peer_addr in &self.config.peers {
            info!("Connecting to bootstrap peer: {}", peer_addr);
        }

        // TODO: Load or create genesis ledger

        self.set_state(NodeState::Full);
        info!("Node is now operational");

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
            let genesis = primitives::UInt256::zero();
            self.consensus.start_round(genesis, 1);
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
                    // Create a transaction set hash (placeholder)
                    let tx_set_hash = primitives::UInt256::zero();
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

        // TODO: Deserialize and validate transaction
        // TODO: Add to open ledger
        // TODO: Broadcast to peers

        Ok("tesSUCCESS".to_string())
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
        // TODO: Persist ledger state, peer info, etc.

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
