//! Genesis configuration loader for Callchain
//!
//! This module provides functionality to load genesis configuration from JSON files
//! similar to Ethereum's genesis.json format. It allows defining:
//! - Network parameters (chainId, genesis time, consensus settings)
//! - Initial account balances (alloc)
//! - Genesis validators
//! - Fee settings

use crate::ledger::{Ledger, LedgerInfo};
use crate::ledger_entries::{AccountRoot, account_flags};
use primitives::{AccountID, UInt256};
use serialization::Amount;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::{info, debug, warn};

/// Genesis configuration - similar to Ethereum's genesis.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisConfig {
    /// Network configuration
    pub config: NetworkConfig,
    /// Initial account allocations (address -> account info)
    #[serde(rename = "alloc")]
    pub allocations: HashMap<String, GenesisAccount>,
    /// Genesis validators
    #[serde(default)]
    pub validators: Vec<GenesisValidator>,
    /// Coinbase address (for rewards)
    #[serde(default)]
    pub coinbase: Option<String>,
    /// Extra data (for genesis hash uniqueness)
    #[serde(default)]
    pub extra_data: Option<String>,
}

/// Network configuration parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Chain ID for network identification
    #[serde(rename = "chainId")]
    pub chain_id: u32,
    /// Network name
    #[serde(rename = "networkName")]
    pub network_name: String,
    /// Genesis timestamp (ISO 8601 format)
    #[serde(rename = "genesisTime")]
    pub genesis_time: String,
    /// Consensus parameters
    #[serde(rename = "consensusParams")]
    pub consensus_params: ConsensusParams,
    /// Fee settings
    #[serde(rename = "feeSettings", default)]
    pub fee_settings: FeeSettings,
}

/// Consensus parameters for the network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusParams {
    /// Minimum time ledger remains open (seconds)
    #[serde(rename = "ledgerMinCloseTime")]
    pub ledger_min_close_time: u32,
    /// Maximum time ledger can remain open (seconds)
    #[serde(rename = "ledgerMaxCloseTime")]
    pub ledger_max_close_time: u32,
    /// Minimum validators needed for consensus
    #[serde(rename = "ledgerMinConsensus")]
    pub ledger_min_consensus: usize,
    /// Maximum validators for consensus
    #[serde(rename = "ledgerMaxConsensus")]
    pub ledger_max_consensus: usize,
    /// Validation quorum (minimum validations needed)
    #[serde(rename = "validationQuorum")]
    pub validation_quorum: usize,
    /// Minimum propose time
    #[serde(rename = "minProposeTime")]
    pub min_propose_time: u32,
    /// Maximum propose time
    #[serde(rename = "maxProposeTime")]
    pub max_propose_time: u32,
}

/// Fee settings for the network
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FeeSettings {
    /// Base fee in drops
    #[serde(rename = "baseFee")]
    pub base_fee: u64,
    /// Reserve base (minimum account balance)
    #[serde(rename = "reserveBase")]
    pub reserve_base: u64,
    /// Reserve increment per object
    #[serde(rename = "reserveIncrement")]
    pub reserve_increment: u64,
}

/// Genesis account allocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisAccount {
    /// Initial balance in drops (as string to handle large numbers)
    pub balance: String,
    /// Initial sequence number (default: 1)
    #[serde(default = "default_sequence")]
    pub sequence: u32,
    /// Account flags (default: 0)
    #[serde(default)]
    pub flags: u32,
    /// Optional regular key
    #[serde(rename = "regularKey")]
    pub regular_key: Option<String>,
}

fn default_sequence() -> u32 {
    1
}

/// Genesis validator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisValidator {
    /// Node ID
    #[serde(rename = "nodeId")]
    pub node_id: String,
    /// Public key (hex)
    #[serde(rename = "publicKey")]
    pub public_key: String,
    /// Domain (optional)
    pub domain: Option<String>,
}

/// Errors that can occur during genesis operations
#[derive(Debug, Clone)]
pub enum GenesisError {
    Io(String),
    Json(String),
    Validation(String),
    AddressParse(String),
}

impl std::fmt::Display for GenesisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GenesisError::Io(msg) => write!(f, "IO error: {}", msg),
            GenesisError::Json(msg) => write!(f, "JSON error: {}", msg),
            GenesisError::Validation(msg) => write!(f, "Validation error: {}", msg),
            GenesisError::AddressParse(msg) => write!(f, "Address parse error: {}", msg),
        }
    }
}

impl std::error::Error for GenesisError {}

impl From<std::io::Error> for GenesisError {
    fn from(e: std::io::Error) -> Self {
        GenesisError::Io(e.to_string())
    }
}

impl From<serde_json::Error> for GenesisError {
    fn from(e: serde_json::Error) -> Self {
        GenesisError::Json(e.to_string())
    }
}

impl GenesisConfig {
    /// Load genesis configuration from a JSON file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, GenesisError> {
        let content = std::fs::read_to_string(path)?;
        Self::from_json(&content)
    }

    /// Load genesis configuration from JSON string
    pub fn from_json(json: &str) -> Result<Self, GenesisError> {
        let config: GenesisConfig = serde_json::from_str(json)?;
        config.validate()?;
        Ok(config)
    }

    /// Parse a Callchain address (c... format or hex)
    fn parse_address(address: &str) -> Result<AccountID, GenesisError> {
        // Check if it's a hex address (40 characters)
        if address.len() == 40 {
            // Parse hex manually
            let mut bytes = [0u8; 20];
            for i in 0..20 {
                let byte_str = &address[i*2..i*2+2];
                bytes[i] = u8::from_str_radix(byte_str, 16)
                    .map_err(|_| GenesisError::AddressParse(format!("Invalid hex in address: {}", address)))?;
            }
            return Ok(AccountID::new(bytes));
        }

        // Try to decode base58 address (c... format)
        if address.starts_with('c') {
            // Use crypto crate's base58 decode
            match crypto::base58::decode(address) {
                Ok(decoded) => {
                    // Format: version (1 byte) + account_id (20 bytes) + checksum (4 bytes)
                    if decoded.len() == 25 {
                        let mut bytes = [0u8; 20];
                        bytes.copy_from_slice(&decoded[1..21]);
                        return Ok(AccountID::new(bytes));
                    } else {
                        return Err(GenesisError::AddressParse(format!("Invalid decoded address length: {}", decoded.len())));
                    }
                }
                Err(e) => {
                    return Err(GenesisError::AddressParse(format!("Failed to decode base58 address: {:?}", e)));
                }
            }
        }

        Err(GenesisError::AddressParse(format!("Invalid address format: {}", address)))
    }

    /// Validate the genesis configuration
    fn validate(&self) -> Result<(), GenesisError> {
        // Validate chain ID
        if self.config.chain_id == 0 {
            return Err(GenesisError::Validation("Chain ID cannot be 0".to_string()));
        }

        // Validate at least one allocation
        if self.allocations.is_empty() {
            warn!("No genesis allocations specified");
        }

        // Validate fee settings
        if self.config.fee_settings.base_fee == 0 {
            return Err(GenesisError::Validation("Base fee cannot be 0".to_string()));
        }

        // Validate consensus parameters
        if self.config.consensus_params.ledger_min_close_time == 0 {
            return Err(GenesisError::Validation("ledgerMinCloseTime must be greater than 0".to_string()));
        }

        if self.config.consensus_params.ledger_max_close_time <= self.config.consensus_params.ledger_min_close_time {
            return Err(GenesisError::Validation("ledgerMaxCloseTime must be greater than ledgerMinCloseTime".to_string()));
        }

        // Validate network name
        if self.config.network_name.is_empty() {
            return Err(GenesisError::Validation("Network name cannot be empty".to_string()));
        }

        // Validate reserve settings
        if self.config.fee_settings.reserve_base == 0 {
            return Err(GenesisError::Validation("Reserve base cannot be 0".to_string()));
        }

        if self.config.fee_settings.reserve_increment == 0 {
            return Err(GenesisError::Validation("Reserve increment cannot be 0".to_string()));
        }

        // Validate all allocation addresses and balances
        for (address, account) in &self.allocations {
            // Validate address format
            Self::parse_address(address)?;

            // Validate balance is a valid number and not zero
            let balance = account.balance.parse::<u64>()
                .map_err(|_| GenesisError::Validation(format!("Invalid balance for {}: {}", address, account.balance)))?;

            if balance == 0 {
                return Err(GenesisError::Validation(format!("Balance for {} cannot be 0", address)));
            }

            // Validate sequence is valid
            if account.sequence == 0 {
                return Err(GenesisError::Validation(format!("Sequence for {} must be greater than 0", address)));
            }
        }

        // Validate validators if present
        for (i, validator) in self.validators.iter().enumerate() {
            if validator.node_id.is_empty() {
                return Err(GenesisError::Validation(format!("Validator {} has empty node_id", i)));
            }
            if validator.public_key.is_empty() {
                return Err(GenesisError::Validation(format!("Validator {} has empty public_key", i)));
            }
        }

        Ok(())
    }

    /// Create a genesis ledger from this configuration
    pub fn create_genesis_ledger(&self) -> Result<Ledger, GenesisError> {
        info!("Creating genesis ledger from configuration");
        info!("Chain ID: {}", self.config.chain_id);
        info!("Network: {}", self.config.network_name);
        info!("Allocations: {}", self.allocations.len());

        // Create genesis ledger info
        let mut ledger_info = LedgerInfo::genesis();

        // Set total drops based on allocations
        let total_drops: u64 = self.allocations
            .values()
            .map(|acc| acc.balance.parse::<u64>().unwrap_or(0))
            .sum();
        ledger_info.drops = total_drops;

        // Create the ledger
        let mut ledger = Ledger::new(ledger_info);

        // Add all allocations as AccountRoot entries
        for (address, account) in &self.allocations {
            match Self::create_account_root(address, account) {
                Ok((key, data)) => {
                    let success = ledger.add_state_entry(key, data.clone());
                    info!("Added genesis account: {} with key {} -> success={}, data_len={}",
                        address, key.to_hex(), success, data.len());
                }
                Err(e) => {
                    warn!("Failed to create genesis account {}: {}", address, e);
                }
            }
        }
        info!("Genesis ledger state_tree has {} items", ledger.state_tree.iter().count());

        // Update ledger hashes
        ledger.update_hashes();

        info!("Genesis ledger created with hash: {}", ledger.get_hash().to_hex());
        info!("Total allocated: {} drops", total_drops);

        Ok(ledger)
    }

    /// Create an AccountRoot entry for a genesis account
    fn create_account_root(address: &str, account: &GenesisAccount) -> Result<(UInt256, Vec<u8>), GenesisError> {
        // Parse the address
        let account_id = Self::parse_address(address)?;

        // Parse balance
        let balance_drops = account.balance.parse::<u64>()
            .map_err(|_| GenesisError::Validation(format!("Invalid balance: {}", account.balance)))?;

        // Create AccountRoot
        let mut account_root = AccountRoot::new(account_id);
        account_root.balance = Amount::call(balance_drops);
        account_root.sequence = account.sequence;
        account_root.flags = account.flags | account_flags::LSF_DEFAULT_CALL;

        // Set regular key if provided
        if let Some(ref regular_key_str) = account.regular_key {
            account_root.regular_key = Some(Self::parse_address(regular_key_str)?);
        }

        // Compute ledger index (key) for this account
        let ledger_index = AccountRoot::compute_ledger_index(&account_id);

        // Serialize account to bytes using raw format (consistent with LedgerState)
        use serialization::Serializer;
        use primitives::UInt160;
        let mut serializer = Serializer::new();
        // Account is written as raw 20 bytes (not VL-encoded)
        serializer.add160(UInt160::new(*account_root.account.as_bytes()));
        serializer.add_amount(account_root.balance);
        serializer.add32(account_root.sequence);
        serializer.add32(account_root.owner_count);
        serializer.add256(account_root.previous_txn_id);
        serializer.add32(account_root.previous_txn_lgr_seq);
        let data = serializer.finish();

        Ok((ledger_index, data))
    }

    /// Get the default genesis config (for testing)
    /// These accounts have known seeds for transaction signing in stress tests
    pub fn default_devnet() -> Self {
        let mut allocations = HashMap::new();

        // Genesis Account 1: ssyB7KxAvfRwQ6mseEjt3iY1qeqMC
        allocations.insert(
            "cGmJBrEfFssWuas4kCoHTX5r6aMEf6QHhy".to_string(),
            GenesisAccount {
                balance: "100000000000".to_string(), // 100,000 CALL
                sequence: 1,
                flags: 0,
                regular_key: None,
            },
        );

        // Genesis Account 2: snTHcoLdd2vwrCmNunkMZRhbfLuLs
        allocations.insert(
            "c3K3xXhvsWBnP3TitQfeg2ihAuaYybvtc7".to_string(),
            GenesisAccount {
                balance: "50000000000".to_string(), // 50,000 CALL
                sequence: 1,
                flags: 0,
                regular_key: None,
            },
        );

        // Genesis Account 3: shCQseDN6PMeeBK31bCjhQqYY4LrG
        allocations.insert(
            "cHSFoKcGXFZdbB7EKmWQMTUJbr66dwfMR1".to_string(),
            GenesisAccount {
                balance: "25000000000".to_string(), // 25,000 CALL
                sequence: 1,
                flags: 0,
                regular_key: None,
            },
        );

        // Genesis Account 4: shiWcE5Y2DJqZ2X2oLCrqYmovqz45
        allocations.insert(
            "cKKeufyrSZymFeGmtF1Vhi11eCSf2i6MhR".to_string(),
            GenesisAccount {
                balance: "10000000000".to_string(), // 10,000 CALL
                sequence: 1,
                flags: 0,
                regular_key: None,
            },
        );

        // Genesis Account 5: ss1nurfhbpEnnZkDai6GTYdzT7Jqb
        allocations.insert(
            "cUUsn5u9qPq7MiMiEDwdjMPsHHKyaesHPH".to_string(),
            GenesisAccount {
                balance: "5000000000".to_string(), // 5,000 CALL
                sequence: 1,
                flags: 0,
                regular_key: None,
            },
        );

        Self {
            config: NetworkConfig {
                chain_id: 1337, // Devnet chain ID
                network_name: "callchain-devnet".to_string(),
                genesis_time: "2025-01-01T00:00:00Z".to_string(),
                consensus_params: ConsensusParams {
                    ledger_min_close_time: 2,
                    ledger_max_close_time: 20,
                    ledger_min_consensus: 1,
                    ledger_max_consensus: 50,
                    validation_quorum: 1,
                    min_propose_time: 3,
                    max_propose_time: 30,
                },
                fee_settings: FeeSettings {
                    base_fee: 10,
                    reserve_base: 10_000_000, // 10 CALL
                    reserve_increment: 2_000_000, // 2 CALL
                },
            },
            allocations,
            validators: Vec::new(),
            coinbase: None,
            extra_data: None,
        }
    }
}

/// Genesis loader utility
pub struct GenesisLoader;

impl GenesisLoader {
    /// Load or create genesis ledger
    /// If genesis file exists, load from it. Otherwise, use default devnet config.
    pub fn load_or_create<P: AsRef<Path>>(genesis_path: Option<P>) -> Result<(Ledger, GenesisConfig), GenesisError> {
        let config = if let Some(path) = genesis_path {
            info!("Loading genesis configuration from: {}", path.as_ref().display());
            GenesisConfig::from_file(path)?
        } else {
            info!("No genesis file specified, using default devnet configuration");
            GenesisConfig::default_devnet()
        };

        let ledger = config.create_genesis_ledger()?;

        Ok((ledger, config))
    }

    /// Check if a genesis file exists at the given path
    pub fn genesis_exists<P: AsRef<Path>>(path: P) -> bool {
        path.as_ref().exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genesis_config_from_json() {
        let json = r#"{
            "config": {
                "chainId": 1337,
                "networkName": "testnet",
                "genesisTime": "2025-01-01T00:00:00Z",
                "consensusParams": {
                    "ledgerMinCloseTime": 2,
                    "ledgerMaxCloseTime": 20,
                    "ledgerMinConsensus": 1,
                    "ledgerMaxConsensus": 50,
                    "validationQuorum": 1,
                    "minProposeTime": 3,
                    "maxProposeTime": 30
                },
                "feeSettings": {
                    "baseFee": 10,
                    "reserveBase": 10000000,
                    "reserveIncrement": 2000000
                }
            },
            "alloc": {
                "cw6htsw3GuePRosi6viQfUrCiMcqn5L2R2": {
                    "balance": "1000000000",
                    "sequence": 1,
                    "flags": 0
                }
            }
        }"#;

        let config = GenesisConfig::from_json(json).unwrap();
        assert_eq!(config.config.chain_id, 1337);
        assert_eq!(config.config.network_name, "testnet");
        assert_eq!(config.allocations.len(), 1);
    }

    #[test]
    fn test_default_devnet() {
        let config = GenesisConfig::default_devnet();
        assert_eq!(config.config.chain_id, 1337);
        assert!(!config.allocations.is_empty());

        let ledger = config.create_genesis_ledger().unwrap();
        assert_eq!(ledger.get_seq(), 1);
    }

    #[test]
    fn test_invalid_chain_id() {
        let json = r#"{
            "config": {
                "chainId": 0,
                "networkName": "invalid",
                "genesisTime": "2025-01-01T00:00:00Z",
                "consensusParams": {
                    "ledgerMinCloseTime": 2,
                    "ledgerMaxCloseTime": 20,
                    "ledgerMinConsensus": 1,
                    "ledgerMaxConsensus": 50,
                    "validationQuorum": 1,
                    "minProposeTime": 3,
                    "maxProposeTime": 30
                }
            },
            "alloc": {}
        }"#;

        assert!(GenesisConfig::from_json(json).is_err());
    }
}
