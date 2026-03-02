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
use crypto::sha512_half;
use primitives::{AccountID, UInt256};
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Trait for network peer communication
pub trait PeerNetwork: Send + Sync {
    /// Broadcast a GetLedger request to connected peers
    fn broadcast_get_ledger(&self, ledger_index: LedgerIndex, ledger_hash: Option<UInt256>);
    /// Send GetLedger request to a specific peer
    fn send_get_ledger(&self, peer_id: &AccountID, ledger_index: LedgerIndex, ledger_hash: Option<UInt256>);
    /// Get list of connected peers
    fn get_connected_peers(&self) -> Vec<AccountID>;
    /// Check if we have any connected peers
    fn has_peers(&self) -> bool;
}

/// Trait for persistent ledger storage
pub trait LedgerStorage: Send + Sync {
    /// Load a ledger from storage by its hash
    fn load_ledger(&self, hash: &UInt256) -> Option<Ledger>;
    /// Load a ledger from storage by its sequence number
    fn load_ledger_by_index(&self, index: LedgerIndex) -> Option<Ledger>;
    /// Save a ledger to storage
    fn save_ledger(&self, ledger: &Ledger) -> Result<(), String>;
    /// Check if a ledger exists in storage
    fn has_ledger(&self, hash: &UInt256) -> bool;
    /// Get the highest ledger sequence we have stored
    fn get_latest_ledger_index(&self) -> Option<LedgerIndex>;
    /// Load the genesis ledger from storage
    fn load_genesis_ledger(&self) -> Option<Ledger>;
    /// Save the genesis ledger to storage
    fn save_genesis_ledger(&self, ledger: &Ledger) -> Result<(), String>;
}

/// Null implementation of PeerNetwork for testing
pub struct NullPeerNetwork;

impl PeerNetwork for NullPeerNetwork {
    fn broadcast_get_ledger(&self, _ledger_index: LedgerIndex, _ledger_hash: Option<UInt256>) {}
    fn send_get_ledger(&self, _peer_id: &AccountID, _ledger_index: LedgerIndex, _ledger_hash: Option<UInt256>) {}
    fn get_connected_peers(&self) -> Vec<AccountID> { Vec::new() }
    fn has_peers(&self) -> bool { false }
}

/// Null implementation of LedgerStorage for testing
pub struct NullLedgerStorage;

impl LedgerStorage for NullLedgerStorage {
    fn load_ledger(&self, _hash: &UInt256) -> Option<Ledger> { None }
    fn load_ledger_by_index(&self, _index: LedgerIndex) -> Option<Ledger> { None }
    fn save_ledger(&self, _ledger: &Ledger) -> Result<(), String> { Ok(()) }
    fn has_ledger(&self, _hash: &UInt256) -> bool { false }
    fn get_latest_ledger_index(&self) -> Option<LedgerIndex> { None }
    fn load_genesis_ledger(&self) -> Option<Ledger> { None }
    fn save_genesis_ledger(&self, _ledger: &Ledger) -> Result<(), String> { Ok(()) }
}

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
    /// Network peer interface for sending requests
    network: Option<Box<dyn PeerNetwork>>,
    /// Storage interface for loading/saving ledgers
    storage: Option<Box<dyn LedgerStorage>>,
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
            network: None,
            storage: None,
        }
    }

    /// Set the network interface for peer communication
    pub fn set_network(&mut self, network: Box<dyn PeerNetwork>) {
        self.network = Some(network);
    }

    /// Set the storage interface for persistent ledger storage
    pub fn set_storage(&mut self, storage: Box<dyn LedgerStorage>) {
        self.storage = Some(storage);
    }

    /// Try to load ledger from storage before requesting from network
    fn try_load_from_storage(&mut self, ledger_index: LedgerIndex) -> Option<Ledger> {
        if let Some(ref storage) = self.storage {
            // Try to load by index first
            if let Some(ledger) = storage.load_ledger_by_index(ledger_index) {
                info!("Loaded ledger {} from storage", ledger_index);
                return Some(ledger);
            }
        }
        None
    }

    /// Save a validated ledger to storage
    fn save_to_storage(&self, ledger: &Ledger) {
        if let Some(ref storage) = self.storage {
            if let Err(e) = storage.save_ledger(ledger) {
                warn!("Failed to save ledger {} to storage: {}", ledger.get_seq(), e);
            } else {
                debug!("Saved ledger {} to storage", ledger.get_seq());
            }
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

        // Try to load ledgers from storage first, then request from network for missing ones
        for seq in (self.current_ledger_seq + 1)..=target_ledger {
            // First try to load from local storage
            if let Some(ledger) = self.try_load_from_storage(seq) {
                // Verify this is the correct sequence
                if ledger.get_seq() == seq {
                    self.validated_queue.push_back(ledger);
                    self.stats.ledgers_fetched += 1;
                    continue;
                }
            }

            // Not in storage, request from network
            // Use a placeholder hash derived from the sequence number
            // This allows us to track pending requests while we wait for
            // validators to provide the actual hashes
            let placeholder_hash = self.compute_placeholder_hash(seq);
            self.request_ledger_by_seq(placeholder_hash, seq);
        }

        // Apply any ledgers we loaded from storage
        self.apply_queued_ledgers();

        // Check if we're already synced after loading from storage
        if self.current_ledger_seq >= self.stats.target_ledger {
            self.stats.status = SyncStatus::Synced;
            info!("Ledger sync complete: all ledgers loaded from storage");
        }
    }

    /// Apply all validated ledgers in the queue
    fn apply_queued_ledgers(&mut self) {
        while let Some(ledger) = self.validated_queue.pop_front() {
            let hash = ledger.get_hash();
            let seq = ledger.get_seq();

            self.processed_ledgers.insert(hash);
            self.current_ledger_seq = seq;

            debug!("Applied ledger {} (hash: {})", seq, hash.to_hex());
        }
    }

    /// Compute a placeholder hash for a ledger sequence we want to fetch
    /// This is used internally to track pending requests before we know
    /// the actual ledger hash from validators
    fn compute_placeholder_hash(&self, seq: LedgerIndex) -> UInt256 {
        // Use a hash of the sequence number as placeholder
        // This ensures each sequence has a unique placeholder
        let seq_bytes = seq.to_be_bytes();
        crypto::sha512_half(&seq_bytes)
    }

    /// Request a ledger by sequence number (before we know the hash)
    fn request_ledger_by_seq(&mut self, placeholder_hash: UInt256, ledger_index: LedgerIndex) {
        if self.pending_fetch.contains_key(&placeholder_hash) {
            return;
        }

        let request = LedgerFetchRequest {
            ledger_hash: placeholder_hash,
            ledger_index,
            requested_from: Vec::new(),
            request_time: Instant::now(),
        };

        self.pending_fetch.insert(placeholder_hash, request);

        debug!("Requesting ledger {} (placeholder hash)", ledger_index);

        // Broadcast GetLedger request to all connected peers
        if let Some(ref network) = self.network {
            if network.has_peers() {
                network.broadcast_get_ledger(ledger_index, None);
                debug!("Broadcast GetLedger request for ledger {} to peers", ledger_index);
            } else {
                debug!("No connected peers to request ledger {}", ledger_index);
            }
        }
    }

    /// Update the placeholder hash with the actual hash from validators
    pub fn update_ledger_hash(&mut self, seq: LedgerIndex, actual_hash: UInt256) {
        let placeholder = self.compute_placeholder_hash(seq);

        if let Some(mut request) = self.pending_fetch.remove(&placeholder) {
            // Update with actual hash
            request.ledger_hash = actual_hash;
            self.pending_fetch.insert(actual_hash, request);
            debug!("Updated ledger {} hash to {}", seq, actual_hash.to_hex());
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

        // Broadcast GetLedger request to all connected peers
        if let Some(ref network) = self.network {
            if network.has_peers() {
                network.broadcast_get_ledger(ledger_index, Some(ledger_hash));
                debug!("Broadcast GetLedger request for ledger {} (hash: {}) to peers",
                    ledger_index, ledger_hash.to_hex());
            } else {
                debug!("No connected peers to request ledger {}", ledger_index);
            }
        }
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

        // Verify the ledger hash matches the computed hash
        // This prevents malicious peers from sending fake ledgers
        if !Self::verify_ledger_hash(&ledger, &transactions) {
            warn!(
                "Received ledger {} with invalid hash from {}",
                ledger.seq, from.to_hex()
            );
            return;
        }

        // Remove from pending fetch (try both actual hash and placeholder)
        let placeholder = self.compute_placeholder_hash(ledger.seq);
        self.pending_fetch.remove(&placeholder);
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

    /// Verify that a ledger's hash matches its contents
    fn verify_ledger_hash(ledger: &LedgerInfo, transactions: &[Transaction]) -> bool {
        // Compute the expected ledger hash from its contents
        let computed_hash = Self::compute_ledger_hash(ledger, transactions);
        computed_hash == ledger.hash
    }

    /// Compute the ledger hash from ledger info and transactions
    fn compute_ledger_hash(ledger: &LedgerInfo, _transactions: &[Transaction]) -> UInt256 {
        use serialization::Serializer;

        // Serialize ledger header data
        let mut serializer = Serializer::with_capacity(256);
        serializer.add32(0x524C3344); // 'RL3D' - ledger master prefix
        serializer.add32(ledger.seq);
        serializer.add32(ledger.close_time);
        serializer.add32(ledger.parent_close_time);
        serializer.add32(ledger.close_time_resolution as u32);
        serializer.add8(ledger.close_flags);
        serializer.add64(ledger.drops);
        serializer.add256(ledger.parent_hash);
        serializer.add256(ledger.tx_hash);
        serializer.add256(ledger.account_hash);

        // Compute the hash
        sha512_half(serializer.as_slice())
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

        // Save to persistent storage
        self.save_to_storage(&ledger);

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
    ///
    /// Tries the following in order:
    /// 1. Load from persistent storage if available
    /// 2. Create from provided config and save to storage
    /// 3. If no config, use default devnet genesis
    pub fn load_or_create(
        genesis_config: &GenesisConfig,
        storage: Option<&dyn LedgerStorage>,
    ) -> Result<Ledger, String> {
        info!("Loading genesis ledger");

        // 1. Try to load from persistent storage first
        if let Some(store) = storage {
            if let Some(ledger) = store.load_genesis_ledger() {
                info!("Loaded genesis ledger from storage (seq={})", ledger.get_seq());

                // Verify the loaded ledger hash matches expected if configured
                if genesis_config.expected_hash != UInt256::zero()
                    && ledger.get_hash() != genesis_config.expected_hash
                {
                    return Err(format!(
                        "Genesis hash mismatch: loaded={} expected={}",
                        ledger.get_hash().to_hex(),
                        genesis_config.expected_hash.to_hex()
                    ));
                }

                return Ok(ledger);
            }
        }

        // 2. Not found in storage - create from config
        info!("Genesis ledger not in storage, creating from config");
        let (genesis_info, initial_txs) =
            Self::create_with_accounts(&genesis_config.initial_accounts);

        let mut ledger = Ledger::new(genesis_info);

        // Apply initial funding transactions to create account states
        // Note: In a real implementation, these would create AccountRoot entries
        for tx in initial_txs {
            debug!("Genesis transaction: {:?}", tx.tx_type);
        }

        // Update ledger hashes
        ledger.update_hashes();

        // Verify against expected hash
        if genesis_config.expected_hash != UInt256::zero()
            && ledger.get_hash() != genesis_config.expected_hash
        {
            return Err(format!(
                "Computed genesis hash mismatch: computed={} expected={}",
                ledger.get_hash().to_hex(),
                genesis_config.expected_hash.to_hex()
            ));
        }

        info!("Created genesis ledger with hash: {}", ledger.get_hash().to_hex());

        // 3. Save to storage for future boots
        if let Some(store) = storage {
            if let Err(e) = store.save_genesis_ledger(&ledger) {
                warn!("Failed to save genesis ledger to storage: {}", e);
            } else {
                info!("Saved genesis ledger to storage");
            }
        }

        Ok(ledger)
    }

    /// Load genesis ledger, falling back to network if not available locally
    ///
    /// This is used when bootstrapping a new node that doesn't have any ledger history.
    /// It will:
    /// 1. Try to load from local storage
    /// 2. If not found, request from network peers
    /// 3. Fall back to creating from config if network is unavailable
    pub async fn load_or_create_with_fallback(
        genesis_config: &GenesisConfig,
        storage: Option<&dyn LedgerStorage>,
        network: Option<&dyn PeerNetwork>,
    ) -> Result<Ledger, String> {
        // First try local storage
        match Self::load_or_create(genesis_config, storage) {
            Ok(ledger) => Ok(ledger),
            Err(e) => {
                warn!("Failed to load genesis from storage: {}", e);

                // Try to fetch from network if available
                if let Some(net) = network {
                    if net.has_peers() {
                        info!("Requesting genesis ledger from network peers");
                        net.broadcast_get_ledger(1, None); // Genesis is ledger 1

                        // Note: In a real implementation, we'd wait for responses
                        // For now, fall back to creating from config
                        warn!("Network fetch not yet implemented, creating from config");
                    }
                }

                // Fall back to creating from config without storage
                let (genesis_info, _initial_txs) =
                    Self::create_with_accounts(&genesis_config.initial_accounts);
                let mut ledger = Ledger::new(genesis_info);
                ledger.update_hashes();

                Ok(ledger)
            }
        }
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
        info: &LedgerInfo,
        txs: &[Transaction],
    ) -> UInt256 {
        use serialization::Serializer;
        use crypto::HashPrefix;

        // Serialize ledger header data
        let mut serializer = Serializer::with_capacity(256);
        serializer.add32(HashPrefix::LedgerMaster.as_u32());
        serializer.add32(info.seq);
        serializer.add32(info.close_time);
        serializer.add32(info.parent_close_time);
        serializer.add32(info.close_time_resolution as u32);
        serializer.add8(info.close_flags);
        serializer.add64(info.drops);

        // Add transaction hashes
        for tx in txs {
            serializer.add256(tx.get_hash());
        }

        // Compute the hash
        crypto::sha512_half(serializer.as_slice())
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
    network: Option<Box<dyn PeerNetwork>>,
    storage: Option<Box<dyn LedgerStorage>>,
}

impl BootstrapManager {
    pub fn new(config: BootstrapConfig, _genesis_config: GenesisConfig) -> Self {
        let peer_discovery = PeerDiscovery::new(config.bootstrap_peers.clone());

        Self {
            config,
            _genesis_config,
            synchronizer: None,
            peer_discovery,
            network: None,
            storage: None,
        }
    }

    /// Set the network interface for peer communication
    pub fn set_network(&mut self, network: Box<dyn PeerNetwork>) {
        self.network = Some(network);
    }

    /// Set the storage interface for persistent ledger storage
    pub fn set_storage(&mut self, storage: Box<dyn LedgerStorage>) {
        self.storage = Some(storage);
    }

    /// Initialize the node (load genesis, start sync if needed)
    pub fn initialize(&mut self) -> Result<Ledger, String> {
        info!("Initializing node bootstrap");

        // Load genesis ledger from storage or create from config
        let storage_ref = self.storage.as_ref().map(|s| s.as_ref());
        let genesis = GenesisLoader::load_or_create(&self._genesis_config, storage_ref)?;

        // Initialize synchronizer with network and storage
        let mut synchronizer = LedgerSynchronizer::new(self.config.clone());
        if let Some(network) = self.network.take() {
            synchronizer.set_network(network);
        }
        if let Some(storage) = self.storage.take() {
            synchronizer.set_storage(storage);
        }
        self.synchronizer = Some(synchronizer);

        // Set current ledger sequence from genesis
        if let Some(ref mut sync) = self.synchronizer {
            sync.current_ledger_seq = genesis.get_seq();
        }

        Ok(genesis)
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

        // Set null storage for testing
        manager.set_storage(Box::new(NullLedgerStorage));

        let genesis = manager.initialize().expect("initialize should succeed");
        assert_eq!(genesis.get_seq(), 1);

        assert_eq!(manager.sync_status(), SyncStatus::Idle);
        assert!(!manager.is_synced());
    }

    #[test]
    fn test_ledger_synchronizer_with_null_network() {
        let config = BootstrapConfig::default();
        let mut sync = LedgerSynchronizer::new(config);

        // Set null network and storage
        sync.set_network(Box::new(NullPeerNetwork));
        sync.set_storage(Box::new(NullLedgerStorage));

        // Should still work without panicking
        sync.start_sync(10);
        assert_eq!(sync.status(), SyncStatus::Backfilling);
    }

    #[test]
    fn test_genesis_loader_with_null_storage() {
        let genesis_config = GenesisConfig::default();

        // Should create genesis without storage
        let result = GenesisLoader::load_or_create(&genesis_config, None);
        assert!(result.is_ok());

        let ledger = result.unwrap();
        assert_eq!(ledger.get_seq(), 1);
    }

    #[test]
    fn test_ledger_validation_processing() {
        use serialization::Serializer;
        use crypto::sha512_half;

        let config = BootstrapConfig {
            validation_threshold: 2,
            ..Default::default()
        };
        let mut sync = LedgerSynchronizer::new(config);

        // Create a pending ledger with properly computed hash
        let mut info = LedgerInfo::genesis();
        // Compute the actual hash as verify_ledger_hash expects
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
        info.hash = sha512_half(serializer.as_slice());

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
