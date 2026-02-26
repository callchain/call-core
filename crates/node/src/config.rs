use std::net::SocketAddr;

/// Node configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// Node name for identification
    pub node_name: String,

    /// P2P listen address for incoming connections
    pub listen_address: SocketAddr,

    /// Bootstrap peers to connect to
    pub peers: Vec<SocketAddr>,

    /// Data directory for database storage
    pub data_dir: String,

    /// Validation seed for validators (None for non-validating nodes)
    pub validation_seed: Option<String>,

    /// Maximum number of peer connections
    pub max_peers: usize,

    /// Target number of peer connections
    pub target_peers: usize,

    /// Whether RPC server is enabled
    pub rpc_enabled: bool,

    /// RPC bind address
    pub rpc_bind_address: String,

    /// RPC port
    pub rpc_port: u16,

    /// Whether RPC admin methods are enabled
    pub rpc_admin_enabled: bool,

    /// Log level
    pub log_level: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            node_name: "call-core-node".to_string(),
            listen_address: "0.0.0.0:51235".parse().unwrap(),
            peers: Vec::new(),
            data_dir: "./data".to_string(),
            validation_seed: None,
            max_peers: 50,
            target_peers: 10,
            rpc_enabled: true,
            rpc_bind_address: "127.0.0.1".to_string(),
            rpc_port: 5005,
            rpc_admin_enabled: false,
            log_level: "info".to_string(),
        }
    }
}

impl Config {
    /// Create a new config with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Load configuration from a file
    pub fn from_file(_path: &str) -> anyhow::Result<Self> {
        // TODO: Implement TOML/JSON config file parsing
        Ok(Self::default())
    }

    /// Add a bootstrap peer
    pub fn add_peer(&mut self, addr: SocketAddr) {
        self.peers.push(addr);
    }

    /// Set the node as a validator with the given seed
    pub fn set_validator(&mut self, seed: impl Into<String>) {
        self.validation_seed = Some(seed.into());
    }

    /// Check if this node is configured as a validator
    pub fn is_validator(&self) -> bool {
        self.validation_seed.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.node_name, "call-core-node");
        assert!(config.rpc_enabled);
        assert_eq!(config.rpc_port, 5005);
        assert!(!config.is_validator());
    }

    #[test]
    fn test_validator_config() {
        let mut config = Config::default();
        assert!(!config.is_validator());

        config.set_validator("sn3nxiW7v8KXzPzAqzyHXbSSKNuN");
        assert!(config.is_validator());
    }

    #[test]
    fn test_add_peer() {
        let mut config = Config::default();
        assert!(config.peers.is_empty());

        let addr: SocketAddr = "127.0.0.1:51235".parse().unwrap();
        config.add_peer(addr);
        assert_eq!(config.peers.len(), 1);
    }
}
