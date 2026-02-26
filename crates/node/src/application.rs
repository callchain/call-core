use crate::config::Config;
use crate::rpc::{RpcConfig, RpcServer, SimpleRpcHandler};
use consensus::{Consensus, ConsensusParms, ConsensusMode};
use network::Overlay;
use primitives::NodeID;
use protocol::Ledger;
use storage::Database;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

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
}

impl Application {
    /// Create a new application instance
    pub fn new(config: Config) -> anyhow::Result<Self> {
        // Initialize storage
        let backend = Box::new(storage::RocksDBBackend::new(&config.data_dir));
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
        // TODO: Implement consensus tick
        Ok(())
    }

    /// Process network messages
    async fn process_network(&mut self) -> anyhow::Result<()> {
        // TODO: Process incoming messages from overlay
        Ok(())
    }

    /// Start the RPC server
    async fn start_rpc_server(&self) -> anyhow::Result<()> {
        if !self.rpc_config.enabled {
            return Ok(());
        }

        let bind_addr = format!("{}:{}", self.rpc_config.bind_address, self.rpc_config.port);
        info!("Starting RPC server on {}", bind_addr);

        // TODO: Implement HTTP server with axum
        // This is a placeholder for the actual server implementation

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

        // TODO: Close all peer connections
        // TODO: Save state to database
        // TODO: Stop RPC server

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

    #[tokio::test]
    async fn test_application_creation() {
        let config = Config::new();
        let app = Application::new(config).unwrap();

        assert_eq!(app.get_state(), NodeState::Starting);
        assert_eq!(app.config.node_name, "call-core-node");
    }

    #[test]
    fn test_node_state_transitions() {
        let mut app = Application::new(Config::new()).unwrap();

        assert_eq!(app.get_state(), NodeState::Starting);

        app.set_state(NodeState::Syncing);
        assert_eq!(app.get_state(), NodeState::Syncing);

        app.set_state(NodeState::Full);
        assert_eq!(app.get_state(), NodeState::Full);
    }
}
