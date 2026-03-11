use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tracing::{info, warn};

/// Transaction pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionPoolConfig {
    /// Enable PRE_SEQ caching
    pub pre_seq_cache_enabled: bool,
    /// How many ledger rounds to cache PRE_SEQ transactions
    pub pre_seq_cache_rounds: u64,
    /// Maximum total cached transactions
    pub pre_seq_cache_max_size: usize,
    /// Maximum cached per account (anti-spam)
    pub pre_seq_per_account_limit: usize,
    /// Maximum sequence gap to cache (0 = disable gap checking)
    pub pre_seq_max_sequence_gap: u32,
    /// Maximum transactions per account per ledger (fairness)
    pub max_tx_per_account_per_ledger: usize,
}

impl Default for TransactionPoolConfig {
    fn default() -> Self {
        Self {
            pre_seq_cache_enabled: true,
            pre_seq_cache_rounds: 10,
            pre_seq_cache_max_size: 10000,
            pre_seq_per_account_limit: 1000,
            pre_seq_max_sequence_gap: 100,
            max_tx_per_account_per_ledger: 50,
        }
    }
}

/// Node configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Node name for identification
    pub node_name: String,

    /// P2P listen address for incoming connections
    #[serde(with = "serde_socket_addr")]
    pub listen_address: SocketAddr,

    /// Bootstrap peers to connect to
    #[serde(with = "serde_vec_socket_addr")]
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

    /// Path to genesis configuration file (optional)
    pub genesis_file: Option<String>,

    /// Transaction pool configuration
    #[serde(default)]
    pub transaction_pool: TransactionPoolConfig,
}

mod serde_socket_addr {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::net::SocketAddr;

    pub fn serialize<S>(addr: &SocketAddr, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&addr.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SocketAddr, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

mod serde_vec_socket_addr {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::net::SocketAddr;

    pub fn serialize<S>(addrs: &[SocketAddr], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let strings: Vec<String> = addrs.iter().map(|a| a.to_string()).collect();
        strings.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<SocketAddr>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let strings: Vec<String> = Vec::deserialize(deserializer)?;
        strings
            .into_iter()
            .map(|s| s.parse().map_err(serde::de::Error::custom))
            .collect()
    }
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
            genesis_file: None,
            transaction_pool: TransactionPoolConfig::default(),
        }
    }
}

impl Config {
    /// Create a new config with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Load configuration from a TOML file
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        info!("Loading configuration from: {}", path);

        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read config file: {}", e))?;

        let config: Config = toml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse TOML config: {}", e))?;

        info!("Configuration loaded successfully");
        Ok(config)
    }

    /// Save configuration to a TOML file
    pub fn save_to_file(&self, path: &str) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| anyhow::anyhow!("Failed to serialize config: {}", e))?;

        std::fs::write(path, content)
            .map_err(|e| anyhow::anyhow!("Failed to write config file: {}", e))?;

        Ok(())
    }

    /// Load from file or create with defaults if not exists
    pub fn load_or_create(path: &str) -> anyhow::Result<Self> {
        if std::path::Path::new(path).exists() {
            Self::from_file(path)
        } else {
            warn!("Config file not found, using defaults: {}", path);
            let config = Self::default();
            // Try to save default config for user reference
            if let Err(e) = config.save_to_file(path) {
                warn!("Failed to save default config: {}", e);
            }
            Ok(config)
        }
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
