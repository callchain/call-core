use crate::config::Config;
use crate::rpc::{RpcConfig, RpcServer, SimpleRpcHandler};
use consensus::{Consensus, ConsensusParms, ConsensusMode, ConsensusPhase};
use network::Overlay;
use primitives::{AccountID, NodeID, UInt256};
use storage::Database;
use std::sync::Arc;
use std::collections::HashMap;
use std::path::Path;
use tokio::sync::{RwLock, watch};
use tokio::task::JoinHandle;
use tracing::{info, debug, warn};

/// Transaction record for account history
#[derive(Debug, Clone)]
pub struct AccountTxRecord {
    pub tx_hash: UInt256,
    pub ledger_seq: u32,
    pub timestamp: u64,
    pub tx_type: protocol::TxType,
}

/// Node state data for persistence
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodeStateData {
    pub ledger_hash: primitives::UInt256,
    pub ledger_seq: u32,
}

/// Transaction history manager for account_tx and tx_history
#[derive(Debug, Default)]
pub struct TransactionHistory {
    /// Account -> List of transactions (sorted by ledger seq descending)
    by_account: HashMap<AccountID, Vec<AccountTxRecord>>,
    /// Global transaction history (for tx_history)
    global: Vec<AccountTxRecord>,
}

impl TransactionHistory {
    pub fn new() -> Self {
        Self {
            by_account: HashMap::new(),
            global: Vec::new(),
        }
    }

    /// Index a transaction for an account
    pub fn index_transaction(&mut self, account: AccountID, record: AccountTxRecord) {
        let entries = self.by_account.entry(account).or_default();
        // Insert in sorted order (newest first)
        let pos = entries.binary_search_by(|e| e.ledger_seq.cmp(&record.ledger_seq).reverse())
            .unwrap_or_else(|e| e);
        entries.insert(pos, record.clone());

        // Also add to global history
        let pos = self.global.binary_search_by(|e| e.ledger_seq.cmp(&record.ledger_seq).reverse())
            .unwrap_or_else(|e| e);
        self.global.insert(pos, record);
    }

    /// Get transactions for an account with pagination
    pub fn get_account_transactions(
        &self,
        account: &AccountID,
        ledger_min: u32,
        ledger_max: u32,
        limit: usize,
        offset: usize,
    ) -> Vec<&AccountTxRecord> {
        let entries = self.by_account.get(account);
        let filtered: Vec<_> = entries
            .into_iter()
            .flat_map(|v| v.iter())
            .filter(|e| e.ledger_seq >= ledger_min && e.ledger_seq <= ledger_max)
            .skip(offset)
            .take(limit)
            .collect();
        filtered
    }

    /// Get global transaction history
    pub fn get_tx_history(&self, start: usize, limit: usize) -> Vec<&AccountTxRecord> {
        self.global.iter().skip(start).take(limit).collect()
    }

    /// Count transactions for an account in ledger range
    pub fn count_account_transactions(&self, account: &AccountID, ledger_min: u32, ledger_max: u32) -> usize {
        self.by_account
            .get(account)
            .map(|entries| entries.iter().filter(|e| e.ledger_seq >= ledger_min && e.ledger_seq <= ledger_max).count())
            .unwrap_or(0)
    }
}

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
    /// Transaction history for account_tx and tx_history
    tx_history: TransactionHistory,
    /// Blacklist store for banned peers/accounts
    blacklist: BlacklistStore,
    /// Issue tracker for account issues/disputes
    issue_tracker: IssueTracker,
    /// Wallet store for secure key management
    wallet_store: WalletStore,
    /// Wallet password hash (SHA-256 of password)
    pub wallet_password_hash: Vec<u8>,
    /// Whether the wallet is currently locked
    pub wallet_locked: bool,
    /// When the wallet unlock expires
    pub wallet_unlock_time: Option<std::time::Instant>,
    /// Log manager for log rotation
    log_manager: LogManager,
    /// Feature store for protocol feature flags
    feature_store: FeatureStore,
    /// Network command sender for peer connections
    network_command_tx: Option<tokio::sync::mpsc::Sender<network::NetworkCommand>>,
}

/// Log manager for handling log file rotation
#[derive(Debug)]
pub struct LogManager {
    log_dir: String,
    current_log_file: String,
    max_files: usize,
}

impl Default for LogManager {
    fn default() -> Self {
        Self::new()
    }
}

impl LogManager {
    pub fn new() -> Self {
        Self {
            log_dir: "./logs".to_string(),
            current_log_file: "call-core.log".to_string(),
            max_files: 10,
        }
    }

    /// Rotate log files
    pub fn rotate_logs(&self) -> anyhow::Result<LogRotationResult> {
        use std::fs;
        use chrono::Local;

        let log_path = Path::new(&self.log_dir).join(&self.current_log_file);

        // Create logs directory if it doesn't exist
        if !Path::new(&self.log_dir).exists() {
            fs::create_dir_all(&self.log_dir)?;
        }

        // If current log file exists, rotate it
        let rotated_count = if log_path.exists() {
            // Generate timestamp for archived log
            let timestamp = Local::now().format("%Y%m%d_%H%M%S");
            let archived_name = format!("call-core_{}.log", timestamp);
            let archived_path = Path::new(&self.log_dir).join(&archived_name);

            // Rename current log to archived name
            fs::rename(&log_path, &archived_path)?;

            // Clean up old log files if exceeding max_files
            self.cleanup_old_logs()?;

            1
        } else {
            0
        };

        // Create new log file (tracing will reopen it automatically when needed)
        fs::File::create(&log_path)?;

        Ok(LogRotationResult {
            rotated_count,
            archived_files: self.list_archived_logs()?,
            current_log: log_path.to_string_lossy().to_string(),
        })
    }

    /// Clean up old log files, keeping only the most recent max_files
    fn cleanup_old_logs(&self) -> anyhow::Result<()> {
        use std::fs;

        let archived_logs = self.list_archived_logs()?;

        if archived_logs.len() > self.max_files {
            // Remove oldest files
            for file in &archived_logs[self.max_files..] {
                let path = Path::new(&self.log_dir).join(file);
                let _ = fs::remove_file(path);
            }
        }

        Ok(())
    }

    /// List archived log files sorted by modification time (newest first)
    fn list_archived_logs(&self) -> anyhow::Result<Vec<String>> {
        use std::fs;
        use std::time::SystemTime;

        let mut files: Vec<(String, SystemTime)> = Vec::new();

        if let Ok(entries) = fs::read_dir(&self.log_dir) {
            for entry in entries.flatten() {
                let file_name = entry.file_name();
                let file_name_str = file_name.to_string_lossy();

                if file_name_str.starts_with("call-core_") && file_name_str.ends_with(".log") {
                    if let Ok(metadata) = entry.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            files.push((file_name_str.to_string(), modified));
                        }
                    }
                }
            }
        }

        // Sort by modification time, newest first
        files.sort_by(|a, b| b.1.cmp(&a.1));

        Ok(files.into_iter().map(|(name, _)| name).collect())
    }

    /// Get current log file path
    pub fn current_log_path(&self) -> String {
        Path::new(&self.log_dir)
            .join(&self.current_log_file)
            .to_string_lossy()
            .to_string()
    }
}

/// Result of log rotation operation
#[derive(Debug, serde::Serialize)]
pub struct LogRotationResult {
    pub rotated_count: usize,
    pub archived_files: Vec<String>,
    pub current_log: String,
}

/// Secure wallet store for managing decrypted keys in memory
#[derive(Debug, Default)]
pub struct WalletStore {
    /// Unlocked wallets (account -> private key bytes)
    unlocked: HashMap<AccountID, Vec<u8>>,
    /// Wallet lock status
    locked: bool,
}

impl WalletStore {
    pub fn new() -> Self {
        Self {
            unlocked: HashMap::new(),
            locked: false,
        }
    }

    /// Unlock a wallet with private key
    pub fn unlock(&mut self, account: AccountID, private_key: Vec<u8>) {
        self.unlocked.insert(account, private_key);
    }

    /// Lock (clear) all stored keys
    pub fn lock(&mut self) {
        // Clear all keys from memory
        for (_, key) in self.unlocked.iter_mut() {
            key.zeroize();
        }
        self.unlocked.clear();
        self.locked = true;
    }

    /// Check if wallet is locked
    pub fn is_locked(&self) -> bool {
        self.locked || self.unlocked.is_empty()
    }

    /// Get number of unlocked wallets
    pub fn unlocked_count(&self) -> usize {
        self.unlocked.len()
    }

    /// Get a private key for an account (if unlocked)
    pub fn get_key(&self, account: &AccountID) -> Option<&Vec<u8>> {
        self.unlocked.get(account)
    }
}

/// Trait for zeroizing sensitive data
trait Zeroize {
    fn zeroize(&mut self);
}

impl Zeroize for Vec<u8> {
    fn zeroize(&mut self) {
        for byte in self.iter_mut() {
            *byte = 0;
        }
    }
}

/// Issue tracker for managing account issues and disputes
#[derive(Debug, Default)]
pub struct IssueTracker {
    /// Issues by account
    issues: HashMap<AccountID, Vec<AccountIssue>>,
}

/// Account issue types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssueType {
    /// Account is frozen
    Frozen,
    /// Trust line is frozen
    FrozenLine,
    /// No trust line for currency
    NoTrustLine,
    /// Negative balance
    NegativeBalance,
    /// Offer expired
    ExpiredOffer,
    /// General dispute
    Dispute,
}

impl std::fmt::Display for IssueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IssueType::Frozen => write!(f, "frozen"),
            IssueType::FrozenLine => write!(f, "frozen_line"),
            IssueType::NoTrustLine => write!(f, "no_trust_line"),
            IssueType::NegativeBalance => write!(f, "negative_balance"),
            IssueType::ExpiredOffer => write!(f, "expired_offer"),
            IssueType::Dispute => write!(f, "dispute"),
        }
    }
}

/// Account issue record
#[derive(Debug, Clone)]
pub struct AccountIssue {
    pub issue_type: IssueType,
    pub description: String,
    pub created_at: u64,
    pub ledger_seq: u32,
    pub resolved: bool,
}

impl IssueTracker {
    pub fn new() -> Self {
        Self {
            issues: HashMap::new(),
        }
    }

    /// Add an issue for an account
    pub fn add_issue(&mut self, account: AccountID, issue: AccountIssue) {
        let account_issues = self.issues.entry(account).or_default();
        account_issues.push(issue);
    }

    /// Get all issues for an account
    pub fn get_issues(&self, account: &AccountID) -> Vec<&AccountIssue> {
        self.issues.get(account).map(|v| v.iter().collect()).unwrap_or_default()
    }

    /// Mark an issue as resolved
    pub fn resolve_issue(&mut self, account: &AccountID, index: usize) -> bool {
        if let Some(account_issues) = self.issues.get_mut(account) {
            if let Some(issue) = account_issues.get_mut(index) {
                issue.resolved = true;
                return true;
            }
        }
        false
    }

    /// Clear resolved issues for an account
    pub fn clear_resolved(&mut self, account: &AccountID) {
        if let Some(account_issues) = self.issues.get_mut(account) {
            account_issues.retain(|issue| !issue.resolved);
        }
    }

    /// Scan ledger state and detect issues for an account
    pub fn scan_account_issues(
        &mut self,
        account: &AccountID,
        ledger_state: &protocol::LedgerState,
        current_ledger: u32,
    ) -> Vec<AccountIssue> {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut detected = Vec::new();

        // Check for expired offers
        let offers = ledger_state.get_offers_for_account(account);
        for offer in offers {
            if let Some(expiration) = offer.expiration {
                if expiration < current_ledger {
                    detected.push(AccountIssue {
                        issue_type: IssueType::ExpiredOffer,
                        description: format!("Offer expired at ledger {}", expiration),
                        created_at: now,
                        ledger_seq: current_ledger,
                        resolved: false,
                    });
                }
            }
        }

        // Check for negative balance
        if let Some(account_root) = ledger_state.get_account_root(account) {
            if account_root.balance.mantissa < 0 {
                detected.push(AccountIssue {
                    issue_type: IssueType::NegativeBalance,
                    description: format!("Negative balance: {}", account_root.balance.mantissa),
                    created_at: now,
                    ledger_seq: current_ledger,
                    resolved: false,
                });
            }
        }

        // Add detected issues to tracker
        for issue in &detected {
            self.add_issue(*account, issue.clone());
        }

        detected
    }
}

/// Blacklist store for managing banned peers and accounts
#[derive(Debug, Default)]
pub struct BlacklistStore {
    /// Banned peer addresses
    peers: std::collections::HashSet<String>,
    /// Banned account IDs (hex)
    accounts: std::collections::HashSet<String>,
}

impl BlacklistStore {
    pub fn new() -> Self {
        Self {
            peers: std::collections::HashSet::new(),
            accounts: std::collections::HashSet::new(),
        }
    }

    /// Add a peer to the blacklist
    pub fn add_peer(&mut self, peer: String) {
        self.peers.insert(peer);
    }

    /// Remove a peer from the blacklist
    pub fn remove_peer(&mut self, peer: &str) -> bool {
        self.peers.remove(peer)
    }

    /// Check if a peer is blacklisted
    pub fn is_peer_blacklisted(&self, peer: &str) -> bool {
        self.peers.contains(peer)
    }

    /// Add an account to the blacklist
    pub fn add_account(&mut self, account: String) {
        self.accounts.insert(account);
    }

    /// Remove an account from the blacklist
    pub fn remove_account(&mut self, account: &str) -> bool {
        self.accounts.remove(account)
    }

    /// Check if an account is blacklisted
    pub fn is_account_blacklisted(&self, account: &str) -> bool {
        self.accounts.contains(account)
    }

    /// Get all blacklisted peers
    pub fn get_peers(&self) -> Vec<&String> {
        self.peers.iter().collect()
    }

    /// Get all blacklisted accounts
    pub fn get_accounts(&self) -> Vec<&String> {
        self.accounts.iter().collect()
    }

    /// Get total count of blacklisted entries
    pub fn count(&self) -> usize {
        self.peers.len() + self.accounts.len()
    }
}

/// Feature store for managing protocol feature flags
#[derive(Debug, Default)]
pub struct FeatureStore {
    /// Feature flags by name
    features: HashMap<String, FeatureFlag>,
}

/// Individual feature flag
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FeatureFlag {
    pub enabled: bool,
    pub supported: bool,
    pub description: String,
    pub updated_at: u64,
}

impl FeatureStore {
    pub fn new() -> Self {
        let mut store = Self {
            features: HashMap::new(),
        };
        // Initialize with default features
        store.init_defaults();
        store
    }

    /// Initialize default feature flags
    fn init_defaults(&mut self) {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let defaults = vec![
            ("FeatureDepositAuth", false, "Deposit authorization feature"),
            ("FeatureChecksFix", true, "Checks fix feature"),
            ("FeatureFix1513", true, "Fix for issue 1513"),
            ("FeatureFix1543", true, "Fix for issue 1543"),
            ("FeatureFlowSort", true, "Flow sort feature"),
        ];

        for (name, enabled, desc) in defaults {
            self.features.insert(name.to_string(), FeatureFlag {
                enabled,
                supported: true,
                description: desc.to_string(),
                updated_at: now,
            });
        }
    }

    /// Get a feature flag
    pub fn get(&self, name: &str) -> Option<&FeatureFlag> {
        self.features.get(name)
    }

    /// Set a feature flag's enabled status
    pub fn set_enabled(&mut self, name: &str, enabled: bool) -> bool {
        use std::time::{SystemTime, UNIX_EPOCH};

        if let Some(flag) = self.features.get_mut(name) {
            flag.enabled = enabled;
            flag.updated_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            true
        } else {
            false
        }
    }

    /// Get all features as JSON
    pub fn to_json(&self) -> serde_json::Value {
        let mut obj = serde_json::Map::new();
        for (name, flag) in &self.features {
            obj.insert(name.clone(), serde_json::json!({
                "enabled": flag.enabled,
                "supported": flag.supported,
            }));
        }
        serde_json::Value::Object(obj)
    }

    /// Check if a feature is enabled
    pub fn is_enabled(&self, name: &str) -> bool {
        self.features.get(name).map(|f| f.enabled).unwrap_or(false)
    }

    /// Load features from file
    pub fn load_from_file(&mut self, path: &str) -> anyhow::Result<()> {
        use std::fs;

        if !Path::new(path).exists() {
            // No file yet, use defaults
            return Ok(());
        }

        let content = fs::read_to_string(path)?;
        let saved: HashMap<String, bool> = serde_json::from_str(&content)?;

        for (name, enabled) in saved {
            if let Some(flag) = self.features.get_mut(&name) {
                flag.enabled = enabled;
            }
        }

        Ok(())
    }

    /// Save features to file
    pub fn save_to_file(&self, path: &str) -> anyhow::Result<()> {
        use std::fs;

        // Create directory if needed
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent)?;
        }

        let mut to_save = HashMap::new();
        for (name, flag) in &self.features {
            to_save.insert(name.clone(), flag.enabled);
        }

        let content = serde_json::to_string_pretty(&to_save)?;
        fs::write(path, content)?;

        Ok(())
    }
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

        // Initialize transaction history
        let tx_history = TransactionHistory::new();

        // Initialize blacklist store
        let blacklist = BlacklistStore::new();

        // Initialize issue tracker
        let issue_tracker = IssueTracker::new();

        // Initialize wallet store
        let wallet_store = WalletStore::new();

        // Initialize log manager
        let log_manager = LogManager::new();

        // Initialize feature store and load saved features
        let mut feature_store = FeatureStore::new();
        let feature_file = format!("{}/features.json", config.data_dir);
        if let Err(e) = feature_store.load_from_file(&feature_file) {
            warn!("Failed to load feature flags from file: {}", e);
        }

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
            tx_history,
            blacklist,
            issue_tracker,
            wallet_store,
            wallet_password_hash: Vec::new(),
            wallet_locked: true,
            wallet_unlock_time: None,
            log_manager,
            feature_store,
            network_command_tx: None,
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

    /// Get the transaction history
    pub fn get_tx_history(&self) -> &TransactionHistory {
        &self.tx_history
    }

    /// Get the transaction history (mutable)
    pub fn get_tx_history_mut(&mut self) -> &mut TransactionHistory {
        &mut self.tx_history
    }

    /// Get the blacklist store
    pub fn get_blacklist(&self) -> &BlacklistStore {
        &self.blacklist
    }

    /// Get the blacklist store (mutable)
    pub fn get_blacklist_mut(&mut self) -> &mut BlacklistStore {
        &mut self.blacklist
    }

    /// Get the issue tracker
    pub fn get_issue_tracker(&self) -> &IssueTracker {
        &self.issue_tracker
    }

    /// Get the issue tracker (mutable)
    pub fn get_issue_tracker_mut(&mut self) -> &mut IssueTracker {
        &mut self.issue_tracker
    }

    /// Scan for account issues and update tracker
    pub fn scan_account_issues(&mut self, account: &AccountID) -> Vec<AccountIssue> {
        let ledger_state = &self.ledger_state;
        let current_ledger = self.current_ledger_seq;
        self.issue_tracker.scan_account_issues(account, ledger_state, current_ledger)
    }

    /// Get the wallet store
    pub fn get_wallet_store(&self) -> &WalletStore {
        &self.wallet_store
    }

    /// Get the wallet store (mutable)
    pub fn get_wallet_store_mut(&mut self) -> &mut WalletStore {
        &mut self.wallet_store
    }

    /// Lock all wallets (clear decrypted keys from memory)
    pub fn lock_wallets(&mut self) {
        self.wallet_store.lock();
        info!("All wallets locked - keys cleared from memory");
    }

    /// Get the log manager
    pub fn get_log_manager(&self) -> &LogManager {
        &self.log_manager
    }

    /// Rotate log files
    pub fn rotate_logs(&self) -> anyhow::Result<LogRotationResult> {
        self.log_manager.rotate_logs()
    }

    /// Get the feature store
    pub fn get_feature_store(&self) -> &FeatureStore {
        &self.feature_store
    }

    /// Get the feature store (mutable)
    pub fn get_feature_store_mut(&mut self) -> &mut FeatureStore {
        &mut self.feature_store
    }

    /// Save feature flags to file
    pub fn save_features(&self) -> anyhow::Result<()> {
        let feature_file = format!("{}/features.json", self.config.data_dir);
        self.feature_store.save_to_file(&feature_file)
    }

    /// Index a transaction for account history
    pub fn index_transaction(
        &mut self,
        account: AccountID,
        tx_hash: UInt256,
        tx_type: protocol::TxType,
    ) {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let record = AccountTxRecord {
            tx_hash,
            ledger_seq: self.current_ledger_seq,
            timestamp,
            tx_type,
        };
        self.tx_history.index_transaction(account, record);
    }

    /// Save node state to disk
    fn save_node_state(&self) {
        use std::fs;

        let state_file = format!("{}/node_state.json", self.config.data_dir);
        let state = NodeStateData {
            ledger_hash: self.current_ledger_hash,
            ledger_seq: self.current_ledger_seq,
        };

        match serde_json::to_string_pretty(&state) {
            Ok(json) => {
                if let Err(e) = fs::write(&state_file, json) {
                    warn!("Failed to save node state to {}: {}", state_file, e);
                }
            }
            Err(e) => {
                warn!("Failed to serialize node state: {}", e);
            }
        }
    }

    /// Load node state from disk
    fn load_node_state(&self) -> Option<(primitives::UInt256, u32)> {
        use std::fs;

        let state_file = format!("{}/node_state.json", self.config.data_dir);

        match fs::read_to_string(&state_file) {
            Ok(content) => {
                match serde_json::from_str::<NodeStateData>(&content) {
                    Ok(state) => {
                        info!("Loaded node state: ledger {} with hash {}",
                              state.ledger_seq, state.ledger_hash.to_hex());
                        Some((state.ledger_hash, state.ledger_seq))
                    }
                    Err(e) => {
                        warn!("Failed to parse node state: {}", e);
                        None
                    }
                }
            }
            Err(_) => {
                debug!("No saved node state found");
                None
            }
        }
    }

    /// Load ledger state from database
    fn load_ledger_state(&mut self) -> bool {
        // Try to load the ledger state from the database
        // This loads the last ledger hash and reconstructs the SHAMap from stored nodes

        if let Some((ledger_hash, ledger_seq)) = self.load_node_state() {
            info!("Attempting to load ledger {} from database", ledger_seq);

            // Try to load ledger state from database
            let loaded = self.ledger_state.load_from_database(&self.database, ledger_hash);

            if loaded {
                self.current_ledger_hash = ledger_hash;
                self.current_ledger_seq = ledger_seq;
                info!("Successfully loaded ledger {} from database", ledger_seq);
                return true;
            } else {
                warn!("Failed to load ledger from database, will use genesis");
            }
        }

        false
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
        if self.load_ledger_state() {
            info!("Loaded existing ledger state from disk");
            return Ok(());
        }

        // No saved state, create genesis ledger
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

        // Save node state
        self.save_node_state();

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
                // Process accepted transactions - apply queued transactions to ledger
                self.process_ledger_transactions()?;

                // Move consensus forward after processing
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

    /// Process transactions from the queue and apply them to the ledger
    fn process_ledger_transactions(&mut self) -> anyhow::Result<()> {
        use protocol::views::MutableLedgerView;
        use protocol::ledger::LedgerInfo;
        use protocol::tx_engine::{TransactionEngine, ApplyContext, ApplyRules};

        info!("Processing {} queued transactions", self.tx_queue.len());

        // Create transaction engine
        let engine = TransactionEngine::new();

        // Track transaction hashes for computing the new ledger hash
        let mut tx_hashes: Vec<primitives::UInt256> = Vec::new();

        // Process transactions from queue directly
        let mut applied_count = 0;

        while let Some(queued) = self.tx_queue.pop_highest_fee() {
            // Create a mutable view of the ledger state for each transaction
            let ledger_info = LedgerInfo {
                hash: self.current_ledger_hash,
                seq: self.current_ledger_seq,
                ..Default::default()
            };

            // Create the mutable ledger view
            let mut view = MutableLedgerView::new(&mut self.ledger_state, ledger_info);

            // Create apply context
            let mut ctx = ApplyContext {
                ledger: &mut view,
                rules: ApplyRules::default(),
                ledger_seq: self.current_ledger_seq + 1,
                parent_ledger_hash: self.current_ledger_hash,
            };

            // Apply transaction
            let result = engine.process(&mut ctx, &queued.transaction);

            // Count successful applications and track the transaction hash
            if result.ter.is_success() {
                applied_count += 1;
                tx_hashes.push(queued.transaction.get_hash());
            }
        }

        info!("Applied {} transactions to ledger", applied_count);

        // Update ledger sequence
        self.current_ledger_seq += 1;

        // Compute the new ledger hash
        // This includes the parent hash, transaction hash, account hash, and close time
        let account_hash = self.ledger_state.get_root_hash();
        let tx_hash = Self::compute_transaction_hash(&tx_hashes);

        // Create new ledger info and compute its hash
        let mut new_ledger_info = protocol::LedgerInfo {
            seq: self.current_ledger_seq,
            parent_hash: self.current_ledger_hash,
            account_hash,
            tx_hash,
            close_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as u32,
            ..Default::default()
        };
        new_ledger_info.hash = Self::compute_ledger_header_hash(&new_ledger_info);

        self.current_ledger_hash = new_ledger_info.hash;

        // Persist the ledger state to the database
        self.ledger_state.persist_to_database(&self.database);

        // Persist node state (ledger hash and sequence)
        self.save_node_state();

        info!(
            "Closed ledger {} with hash {} and {} transactions",
            self.current_ledger_seq,
            self.current_ledger_hash.to_hex(),
            applied_count
        );

        Ok(())
    }

    /// Compute the transaction hash from a list of transaction hashes
    fn compute_transaction_hash(tx_hashes: &[primitives::UInt256]) -> primitives::UInt256 {
        use crypto::sha512_half;

        if tx_hashes.is_empty() {
            return sha512_half(b"");
        }

        // Build a simple Merkle tree of transactions
        let mut hashes: Vec<primitives::UInt256> = tx_hashes.to_vec();

        while hashes.len() > 1 {
            let mut next_level = Vec::new();
            for chunk in hashes.chunks(2) {
                let mut data = Vec::new();
                data.extend_from_slice(chunk[0].as_bytes());
                if chunk.len() > 1 {
                    data.extend_from_slice(chunk[1].as_bytes());
                } else {
                    // Duplicate the last hash if odd number
                    data.extend_from_slice(chunk[0].as_bytes());
                }
                next_level.push(sha512_half(&data));
            }
            hashes = next_level;
        }

        hashes[0]
    }

    /// Compute the ledger header hash from ledger info
    fn compute_ledger_header_hash(info: &protocol::LedgerInfo) -> primitives::UInt256 {
        use serialization::Serializer;
        use crypto::sha512_half;

        // Serialize ledger header data
        let mut serializer = Serializer::with_capacity(256);
        serializer.add32(0x524C3344); // 'RL3D' - ledger master prefix
        serializer.add32(info.seq);
        serializer.add32(info.close_time);
        serializer.add32(info.parent_close_time);
        serializer.add32(info.close_time_resolution as u32);
        serializer.add8(info.close_flags);
        serializer.add64(info.drops);
        serializer.add256(info.parent_hash);
        serializer.add256(info.tx_hash);
        serializer.add256(info.account_hash);

        // Compute the hash
        sha512_half(serializer.as_slice())
    }

    /// Process network messages and manage peer connections
    async fn process_network(&mut self) -> anyhow::Result<()> {
        // Clean up timed out peers periodically
        self.overlay.cleanup_timed_out(std::time::Duration::from_secs(300));

        // Check if we need more peers and try to connect
        if self.overlay.needs_more_peers() {
            for peer_addr in &self.config.peers {
                debug!("Initiating connection to bootstrap peer: {}", peer_addr);
                // Send connect command to network manager
                if let Some(ref network_tx) = self.network_command_tx {
                    if let Err(e) = network_tx
                        .send(network::NetworkCommand::Connect(*peer_addr))
                        .await
                    {
                        warn!("Failed to send connect command: {}", e);
                    }
                } else {
                    debug!("Network command channel not available");
                }
            }
        }

        Ok(())
    }

    /// Set the network command sender
    pub fn set_network_command_sender(
        &mut self,
        sender: tokio::sync::mpsc::Sender<network::NetworkCommand>,
    ) {
        self.network_command_tx = Some(sender);
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
        let message = Message::new(MessageType::Transaction, tx_blob.to_vec());

        // Broadcast to all active peers
        let peer_count = self.overlay.active_peer_count();
        if peer_count == 0 {
            debug!("No peers to broadcast to");
            return Ok(());
        }

        debug!("Broadcasting transaction to {} peers", peer_count);

        // Broadcast via the overlay network
        self.overlay.broadcast(message);

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

        // 3. Persist current ledger info
        let ledger_info = serde_json::json!({
            "ledger_hash": hex::encode(self.current_ledger_hash.as_bytes()),
            "ledger_seq": self.current_ledger_seq,
            "persist_time": chrono::Utc::now().to_rfc3339(),
        });

        let ledger_data = serde_json::to_vec(&ledger_info)
            .map_err(|e| anyhow::anyhow!("Failed to serialize ledger info: {}", e))?;

        let ledger_hash = crypto::sha512_half(&ledger_data);
        let ledger_object = NodeObject::new(NodeObjectType::Ledger, ledger_hash, ledger_data);
        self.database.store_node(ledger_object);

        // 4. Persist ledger state (SHAMap contents)
        self.ledger_state.persist_to_database(&self.database);

        info!("Persisted ledger {} to database", self.current_ledger_seq);

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
