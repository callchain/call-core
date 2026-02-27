//! Bootstrap and Ledger Synchronization
//!
//! This module handles the process of joining the network and synchronizing
//! the ledger state with peers. It includes:
//!
//! - Genesis ledger loading
//! - Peer discovery and connection
//! - Ledger catch-up (backfilling)
//! - Transaction set synchronization
//! - Validation of synced data

use crate::ledger::{Ledger, LedgerIndex, LedgerInfo};
use crate::transactions::Transaction;
use primitives::{AccountID, UInt256};
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Bootstrap configuration
#[derive(Debug, Clone)]
pub struct BootstrapConfig {
    /// Genesis ledger hash (for verification)
    pub genesis_hash: UInt256,
    /// Number of validations needed to confirm a ledger
    pub validation_threshold: usize,
    /// Maximum ledgers to fetch in parallel
    pub max_parallel_fetch: usize,
    /// Timeout for ledger fetch
    pub fetch_timeout: Duration,
    /// Bootstrap peers
    pub bootstrap_peers: Vec<String>,
    /// Whether to fetch full history or just recent
    pub full_history: bool,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            genesis_hash: UInt256::zero(),
            validation_threshold: 5,
            max_parallel_fetch: 10,
            fetch_timeout: Duration::from_secs(30),
            bootstrap_peers: Vec::new(),
            full_history: false,
        }
    }
}

/// Ledger sync status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncStatus {
    /// Not started
    Idle,
    /// Connecting to peers
    Connecting,
    /// Fetching validations for recent ledgers
    FetchingValidations,
    /// Backfilling ledger history
    Backfilling,
    /// Syncing current ledger
    SyncingCurrent,
    /// Fully synced
    Synced,
    /// Failed to sync
    Failed,
}

/// Ledger sync statistics
#[derive(Debug, Clone)]
pub struct SyncStats {
    pub status: SyncStatus,
    pub ledgers_fetched: usize,
    pub ledgers_to_fetch: usize,
    pub transactions_fetched: usize,
    pub start_time: Instant,
    pub last_progress: Instant,
    pub current_ledger: LedgerIndex,
    pub target_ledger: LedgerIndex,
}

impl Default for SyncStats {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            status: SyncStatus::Idle,
            ledgers_fetched: 0,
            ledgers_to_fetch: 0,
            transactions_fetched: 0,
            start_time: now,
            last_progress: now,
            current_ledger: 0,
            target_ledger: 0,
        }
    }
}

/// Ledger fetch request
#[derive(Debug, Clone)]
pub struct LedgerFetchRequest {
    pub ledger_hash: UInt256,
    pub ledger_index: LedgerIndex,
    pub requested_from: Vec<AccountID>,
    pub request_time: Instant,
}

/// Pending ledger data
#[derive(Debug, Clone)]
pub struct PendingLedger {
    pub info: LedgerInfo,
    pub transactions: Vec<Transaction>,
    pub received_from: AccountID,
    pub received_time: Instant,
}

/// Ledger synchronizer
pub struct LedgerSynchronizer {
    config: BootstrapConfig,
    stats: SyncStats,
    /// Ledgers we're trying to fetch
    pending_fetch: HashMap<UInt256, LedgerFetchRequest>,
    /// Ledgers we've received but not yet validated
    pending_validation: HashMap<UInt256, Vec<PendingLedger>>,
    /// Validated ledgers waiting to be applied
    validated_queue: VecDeque<Ledger>,
    /// Ledgers we've fully processed
    processed_ledgers: HashSet<UInt256>,
    /// Current ledger sequence we have
    current_ledger_seq: LedgerIndex,
}

impl LedgerSynchronizer {
    pub fn new(config: BootstrapConfig) -> Self {
        Self {
            config,
            stats: SyncStats::default(),
            pending_fetch: HashMap::new(),
            pending_validation: HashMap::new(),
            validated_queue: VecDeque::new(),
            processed_ledgers: HashSet::new(),
            current_ledger_seq: 0,
        }
    }

    /// Start the synchronization process
    pub fn start_sync(&mut self, target_ledger: LedgerIndex) {
        info!(
            "Starting ledger sync: current={}, target={}",
            self.current_ledger_seq, target_ledger
        );

        self.stats.status = SyncStatus::Backfilling;
        self.stats.target_ledger = target_ledger;
        self.stats.ledgers_to_fetch =
            (target_ledger - self.current_ledger_seq) as usize;
        self.stats.start_time = Instant::now();

        // Queue ledgers for fetching
        for seq in (self.current_ledger_seq + 1)..=target_ledger {
            // In a real implementation, we'd compute or fetch the ledger hash
            let placeholder_hash = UInt256::zero();
            self.request_ledger(placeholder_hash, seq);
        }
    }

    /// Request a specific ledger from peers
    pub fn request_ledger(&mut self, ledger_hash: UInt256, ledger_index: LedgerIndex) {
        if self.pending_fetch.contains_key(&ledger_hash)
            || self.processed_ledgers.contains(&ledger_hash)
        {
            return;
        }

        let request = LedgerFetchRequest {
            ledger_hash,
            ledger_index,
            requested_from: Vec::new(),
            request_time: Instant::now(),
        };

        self.pending_fetch.insert(ledger_hash, request);

        debug!(
            "Requesting ledger {} (hash: {})",
            ledger_index,
            ledger_hash.to_hex()
        );

        // In a real implementation, this would send GetLedger messages to peers
    }

    /// Receive a ledger from a peer
    pub fn receive_ledger(
        &mut self,
        ledger: LedgerInfo,
        transactions: Vec<Transaction>,
        from: AccountID,
    ) {
        let hash = ledger.hash;

        debug!("Received ledger {} from {}", ledger.seq, from.to_hex());

        // Remove from pending fetch
        self.pending_fetch.remove(&hash);

        // Add to pending validation
        let pending = PendingLedger {
            info: ledger,
            transactions,
            received_from: from,
            received_time: Instant::now(),
        };

        self.pending_validation
            .entry(hash)
            .or_default()
            .push(pending);

        self.stats.last_progress = Instant::now();
    }

    /// Process validations and determine which ledgers are trusted
    pub fn process_validations(&mut self, validations: &[LedgerValidation]) {
        for validation in validations {
            let hash = validation.ledger_hash;

            // Count validations for this ledger
            let valid_count = validations
                .iter()
                .filter(|v| v.ledger_hash == hash)
                .count();

            if valid_count >= self.config.validation_threshold {
                // Ledger is validated, check if we have it
                if let Some(pending_list) = self.pending_validation.remove(&hash) {
                    // Use the first one (could be smarter about choosing)
                    if let Some(pending) = pending_list.into_iter().next() {
                        let seq = pending.info.seq;
                        let ledger = Ledger::new(pending.info);
                        self.validated_queue.push_back(ledger);

                        info!(
                            "Ledger {} validated with {} validations",
                            seq,
                            valid_count
                        );
                    }
                }
            }
        }
    }

    /// Apply the next validated ledger
    pub fn apply_next_ledger(&mut self) -> Option<Ledger> {
        let ledger = self.validated_queue.pop_front()?;
        let hash = ledger.get_hash();
        let seq = ledger.get_seq();

        self.processed_ledgers.insert(hash);
        self.current_ledger_seq = seq;
        self.stats.ledgers_fetched += 1;
        self.stats.current_ledger = seq;

        debug!("Applied ledger {} (hash: {})", seq, hash.to_hex());

        // Check if we're fully synced
        if self.current_ledger_seq >= self.stats.target_ledger
            && self.pending_fetch.is_empty()
        {
            self.stats.status = SyncStatus::Synced;
            let duration = self.stats.start_time.elapsed();
            info!(
                "Ledger sync complete: {} ledgers in {:?}",
                self.stats.ledgers_fetched, duration
            );
        }

        Some(ledger)
    }

    /// Check for timed out fetch requests
    pub fn check_timeouts(&mut self) {
        let now = Instant::now();
        let timed_out: Vec<_> = self
            .pending_fetch
            .iter()
            .filter(|(_, req)| now.duration_since(req.request_time) > self.config.fetch_timeout)
            .map(|(hash, _)| *hash)
            .collect();

        for hash in timed_out {
            if let Some(req) = self.pending_fetch.remove(&hash) {
                warn!(
                    "Ledger {} fetch timed out, requeueing",
                    req.ledger_index
                );
                // Re-request
                self.request_ledger(hash, req.ledger_index);
            }
        }
    }

    /// Get current sync status
    pub fn status(&self) -> SyncStatus {
        self.stats.status
    }

    /// Get sync statistics
    pub fn stats(&self) -> &SyncStats {
        &self.stats
    }

    /// Get progress percentage
    pub fn progress_percent(&self) -> f64 {
        if self.stats.ledgers_to_fetch == 0 {
            return 100.0;
        }
        (self.stats.ledgers_fetched as f64 / self.stats.ledgers_to_fetch as f64) * 100.0
    }

    /// Whether sync is complete
    pub fn is_synced(&self) -> bool {
        self.stats.status == SyncStatus::Synced
    }

    /// Get ledgers that need to be fetched
    pub fn pending_fetches(&self) -> Vec<&LedgerFetchRequest> {
        self.pending_fetch.values().collect()
    }
}

/// Validation of a ledger hash by a validator
#[derive(Debug, Clone)]
pub struct LedgerValidation {
    pub ledger_hash: UInt256,
    pub ledger_index: LedgerIndex,
    pub validator_id: AccountID,
    pub signature: Vec<u8>,
    pub timestamp: u64,
}

/// Genesis ledger initialization
pub struct GenesisLoader;

impl GenesisLoader {
    /// Load or create the genesis ledger
    pub fn load_or_create(_genesis_config: &GenesisConfig) -> Ledger {
        info!("Loading genesis ledger");

        // In a real implementation, this would:
        // 1. Try to load from disk
        // 2. If not found, create from config
        // 3. Verify hash matches expected

        let genesis_info = LedgerInfo::genesis();
        Ledger::new(genesis_info)
    }

    /// Create a genesis ledger with initial accounts
    pub fn create_with_accounts(
        accounts: &[(AccountID, u64)],
    ) -> (LedgerInfo, Vec<Transaction>) {
        let mut genesis_info = LedgerInfo::genesis();

        // Create initial funding transactions
        let initial_txs: Vec<Transaction> = accounts
            .iter()
            .map(|(account, balance)| {
                // Create genesis funding transaction
                Transaction::new_payment(
                    AccountID::new([0u8; 20]), // Genesis account
                    *account,
                    serialization::Amount::call(*balance),
                )
            })
            .collect();

        // Calculate genesis hash
        genesis_info.hash = Self::calculate_genesis_hash(&genesis_info, &initial_txs);

        (genesis_info, initial_txs)
    }

    fn calculate_genesis_hash(
        _info: &LedgerInfo,
        _txs: &[Transaction],
    ) -> UInt256 {
        // In a real implementation, this would compute the actual hash
        UInt256::zero()
    }
}

/// Configuration for genesis ledger
#[derive(Debug, Clone)]
pub struct GenesisConfig {
    pub genesis_time: u64,
    pub initial_accounts: Vec<(AccountID, u64)>,
    pub expected_hash: UInt256,
}

impl Default for GenesisConfig {
    fn default() -> Self {
        Self {
            genesis_time: 0,
            initial_accounts: Vec::new(),
            expected_hash: UInt256::zero(),
        }
    }
}

/// Peer discovery for bootstrap
pub struct PeerDiscovery {
    /// Known bootstrap nodes
    bootstrap_nodes: Vec<String>,
    /// Discovered peers
    discovered_peers: HashSet<String>,
    /// Last discovery time
    last_discovery: Instant,
}

impl PeerDiscovery {
    pub fn new(bootstrap_nodes: Vec<String>) -> Self {
        Self {
            bootstrap_nodes,
            discovered_peers: HashSet::new(),
            last_discovery: Instant::now(),
        }
    }

    /// Get bootstrap nodes
    pub fn bootstrap_nodes(&self) -> &[String] {
        &self.bootstrap_nodes
    }

    /// Add a discovered peer
    pub fn add_peer(&mut self, address: String) {
        self.discovered_peers.insert(address);
        self.last_discovery = Instant::now();
    }

    /// Get discovered peers
    pub fn discovered_peers(&self) -> &HashSet<String> {
        &self.discovered_peers
    }

    /// Whether we should perform peer discovery
    pub fn should_discover(&self) -> bool {
        self.last_discovery.elapsed() > Duration::from_secs(300) // 5 minutes
    }
}

/// Bootstrap manager coordinates the entire startup process
pub struct BootstrapManager {
    config: BootstrapConfig,
    _genesis_config: GenesisConfig,
    synchronizer: Option<LedgerSynchronizer>,
    peer_discovery: PeerDiscovery,
}

impl BootstrapManager {
    pub fn new(config: BootstrapConfig, _genesis_config: GenesisConfig) -> Self {
        let peer_discovery = PeerDiscovery::new(config.bootstrap_peers.clone());

        Self {
            config,
            _genesis_config,
            synchronizer: None,
            peer_discovery,
        }
    }

    /// Initialize the node (load genesis, start sync if needed)
    pub fn initialize(&mut self) -> Ledger {
        info!("Initializing node bootstrap");

        // Load genesis ledger
        let genesis = GenesisLoader::load_or_create(&self._genesis_config);

        // Initialize synchronizer
        self.synchronizer = Some(LedgerSynchronizer::new(self.config.clone()));

        genesis
    }

    /// Start synchronization to target ledger
    pub fn start_sync(&mut self, target_ledger: LedgerIndex) {
        if let Some(sync) = &mut self.synchronizer {
            sync.start_sync(target_ledger);
        }
    }

    /// Process incoming ledger data
    pub fn process_ledger_data(
        &mut self,
        ledger: LedgerInfo,
        transactions: Vec<Transaction>,
        from: AccountID,
    ) {
        if let Some(sync) = &mut self.synchronizer {
            sync.receive_ledger(ledger, transactions, from);
        }
    }

    /// Process validations
    pub fn process_validations(&mut self, validations: &[LedgerValidation]) {
        if let Some(sync) = &mut self.synchronizer {
            sync.process_validations(validations);
        }
    }

    /// Get next ledger to apply
    pub fn get_next_ledger(&mut self) -> Option<Ledger> {
        self.synchronizer.as_mut()?.apply_next_ledger()
    }

    /// Check sync status
    pub fn sync_status(&self) -> SyncStatus {
        self.synchronizer
            .as_ref()
            .map(|s| s.status())
            .unwrap_or(SyncStatus::Idle)
    }

    /// Whether node is fully synced
    pub fn is_synced(&self) -> bool {
        self.synchronizer.as_ref().map(|s| s.is_synced()).unwrap_or(false)
    }

    /// Get bootstrap peers
    pub fn get_bootstrap_peers(&self) -> &[String] {
        self.peer_discovery.bootstrap_nodes()
    }

    /// Add discovered peer
    pub fn add_discovered_peer(&mut self, address: String) {
        self.peer_discovery.add_peer(address);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_stats_default() {
        let stats = SyncStats::default();
        assert_eq!(stats.status, SyncStatus::Idle);
        assert_eq!(stats.ledgers_fetched, 0);
        assert_eq!(stats.current_ledger, 0);
    }

    #[test]
    fn test_ledger_synchronizer_new() {
        let config = BootstrapConfig::default();
        let sync = LedgerSynchronizer::new(config);

        assert_eq!(sync.status(), SyncStatus::Idle);
        assert_eq!(sync.progress_percent(), 100.0); // Nothing to fetch
    }

    #[test]
    fn test_ledger_synchronizer_progress() {
        let config = BootstrapConfig::default();
        let mut sync = LedgerSynchronizer::new(config);

        sync.start_sync(100);
        assert_eq!(sync.status(), SyncStatus::Backfilling);
        assert_eq!(sync.stats().ledgers_to_fetch, 100);

        // Simulate fetching
        for _ in 0..25 {
            let info = LedgerInfo::genesis();
            let ledger = Ledger::new(info);
            sync.validated_queue.push_back(ledger);
            sync.apply_next_ledger();
        }

        assert_eq!(sync.stats().ledgers_fetched, 25);
        assert_eq!(sync.progress_percent(), 25.0);
    }

    #[test]
    fn test_genesis_loader_create() {
        let accounts = vec![
            (AccountID::new([1u8; 20]), 1_000_000),
            (AccountID::new([2u8; 20]), 2_000_000),
        ];

        let (info, txs) = GenesisLoader::create_with_accounts(&accounts);

        assert_eq!(info.seq, 1); // Genesis is seq 1
        assert_eq!(txs.len(), 2);
    }

    #[test]
    fn test_peer_discovery() {
        let bootstrap = vec!["127.0.0.1:51235".to_string()];
        let mut discovery = PeerDiscovery::new(bootstrap);

        assert_eq!(discovery.bootstrap_nodes().len(), 1);
        assert!(discovery.discovered_peers().is_empty());

        discovery.add_peer("192.168.1.1:51235".to_string());
        assert_eq!(discovery.discovered_peers().len(), 1);
    }

    #[test]
    fn test_bootstrap_manager() {
        let config = BootstrapConfig::default();
        let _genesis_config = GenesisConfig::default();
        let mut manager = BootstrapManager::new(config, _genesis_config);

        let genesis = manager.initialize();
        assert_eq!(genesis.get_seq(), 1);

        assert_eq!(manager.sync_status(), SyncStatus::Idle);
        assert!(!manager.is_synced());
    }

    #[test]
    fn test_ledger_validation_processing() {
        let config = BootstrapConfig {
            validation_threshold: 2,
            ..Default::default()
        };
        let mut sync = LedgerSynchronizer::new(config);

        // Create a pending ledger
        let info = LedgerInfo::genesis();
        let hash = info.hash;
        sync.receive_ledger(info.clone(), vec![], AccountID::new([1u8; 20]));

        // Should not be validated yet
        assert!(sync.pending_validation.contains_key(&hash));

        // Add validations
        let validations = vec![
            LedgerValidation {
                ledger_hash: hash,
                ledger_index: info.seq,
                validator_id: AccountID::new([1u8; 20]),
                signature: vec![1],
                timestamp: 0,
            },
            LedgerValidation {
                ledger_hash: hash,
                ledger_index: info.seq,
                validator_id: AccountID::new([2u8; 20]),
                signature: vec![2],
                timestamp: 0,
            },
        ];

        sync.process_validations(&validations);

        // Now should be validated
        assert!(!sync.pending_validation.contains_key(&hash));
        assert_eq!(sync.validated_queue.len(), 1);
    }
}
