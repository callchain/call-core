//! Shard storage and management for Callchain
//!
//! Sharding allows splitting ledger history into chunks (shards) that can be
//! stored separately, downloaded on demand, and shared between peers.
//!
//! A shard contains ledgers within a specific range (e.g., ledgers 0-16383).

use crate::NodeObject;
use primitives::{UInt256, LedgerIndex};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

/// Size of a shard in ledgers (16384 = 2^14)
pub const SHARD_SIZE: u64 = 16384;

/// Status of a shard
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ShardStatus {
    /// Not present locally
    NotPresent,
    /// Download in progress
    Downloading,
    /// Downloaded but not verified
    Pending,
    /// Verified and complete
    Complete,
    /// Error during download/processing
    Error,
}

impl std::fmt::Display for ShardStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShardStatus::NotPresent => write!(f, "not_present"),
            ShardStatus::Downloading => write!(f, "downloading"),
            ShardStatus::Pending => write!(f, "pending"),
            ShardStatus::Complete => write!(f, "complete"),
            ShardStatus::Error => write!(f, "error"),
        }
    }
}

/// Information about a shard
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ShardInfo {
    /// Shard index (ledger_index / SHARD_SIZE)
    pub index: u64,
    /// Start ledger (inclusive)
    pub start_ledger: LedgerIndex,
    /// End ledger (inclusive)
    pub end_ledger: LedgerIndex,
    /// SHA-256 hash of the shard archive
    pub hash: Option<UInt256>,
    /// Size in bytes
    pub size: u64,
    /// Number of ledgers in this shard
    pub ledger_count: u32,
    /// Download/verification status
    pub status: ShardStatus,
    /// When the shard was downloaded/verified
    pub timestamp: Option<SystemTime>,
    /// Progress (0-100) for downloading shards
    pub progress: u8,
}

impl ShardInfo {
    /// Create a new shard info for the given shard index
    pub fn new(index: u64) -> Self {
        let start_ledger = (index * SHARD_SIZE) as LedgerIndex;
        let end_ledger = ((index + 1) * SHARD_SIZE - 1) as LedgerIndex;

        Self {
            index,
            start_ledger,
            end_ledger,
            hash: None,
            size: 0,
            ledger_count: 0,
            status: ShardStatus::NotPresent,
            timestamp: None,
            progress: 0,
        }
    }

    /// Check if a ledger index is in this shard
    pub fn contains_ledger(&self, ledger_index: LedgerIndex) -> bool {
        ledger_index >= self.start_ledger && ledger_index <= self.end_ledger
    }
}

/// A shard archive containing compressed ledger data
#[derive(Debug)]
pub struct ShardArchive {
    /// Shard index
    pub index: u64,
    /// Raw compressed data
    pub data: Vec<u8>,
    /// SHA-256 hash of the data
    pub hash: UInt256,
}

/// Peer shard information for crawling
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PeerShard {
    /// Peer node ID
    pub peer_id: String,
    /// Peer IP:port
    pub peer_address: String,
    /// Shard index
    pub shard_index: u64,
    /// Shard hash
    pub shard_hash: Option<String>,
    /// Whether peer has complete shard
    pub is_complete: bool,
}

/// Shard store for managing local shards
pub struct ShardStore {
    /// Base directory for shard storage
    shard_dir: String,
    /// In-memory cache of shard info
    shards: RwLock<HashMap<u64, ShardInfo>>,
    /// Shard archive cache
    archive_cache: RwLock<HashMap<u64, Arc<ShardArchive>>>,
    /// Current shard being downloaded
    active_downloads: RwLock<HashMap<u64, ShardDownload>>,
}

/// Active shard download
#[derive(Debug)]
pub struct ShardDownload {
    pub shard_index: u64,
    pub peers: Vec<String>,
    pub progress: u8,
    pub start_time: SystemTime,
}

impl ShardStore {
    /// Create a new shard store
    pub fn new<P: AsRef<Path>>(shard_dir: P) -> Self {
        let shard_dir = shard_dir.as_ref().to_string_lossy().to_string();

        // Create shard directory if it doesn't exist
        std::fs::create_dir_all(&shard_dir).ok();

        let store = Self {
            shard_dir,
            shards: RwLock::new(HashMap::new()),
            archive_cache: RwLock::new(HashMap::new()),
            active_downloads: RwLock::new(HashMap::new()),
        };

        // Scan existing shards
        store.scan_existing_shards();

        store
    }

    /// Scan the shard directory for existing shards
    fn scan_existing_shards(&self) {
        let mut shards = self.shards.write().unwrap();

        if let Ok(entries) = std::fs::read_dir(&self.shard_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "shard") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        if let Ok(index) = stem.parse::<u64>() {
                            let mut info = ShardInfo::new(index);

                            // Check if file exists and get metadata
                            if let Ok(metadata) = entry.metadata() {
                                info.size = metadata.len();
                                info.status = ShardStatus::Complete;
                                info.timestamp = metadata.modified().ok();
                                info.progress = 100;

                                // Load hash from companion file if it exists
                                let hash_path = path.with_extension("hash");
                                if let Ok(hash_data) = std::fs::read(&hash_path) {
                                    if hash_data.len() >= 32 {
                                        let mut hash_bytes = [0u8; 32];
                                        hash_bytes.copy_from_slice(&hash_data[..32]);
                                        info.hash = Some(UInt256::new(hash_bytes));
                                    }
                                }
                            }

                            shards.insert(index, info);
                        }
                    }
                }
            }
        }
    }

    /// Get shard info for a specific shard index
    pub fn get_shard(&self, index: u64) -> Option<ShardInfo> {
        let shards = self.shards.read().unwrap();
        shards.get(&index).cloned()
    }

    /// Get all shard info
    pub fn get_all_shards(&self) -> Vec<ShardInfo> {
        let shards = self.shards.read().unwrap();
        shards.values().cloned().collect()
    }

    /// Get shard info for ledgers we have locally
    pub fn get_local_shards(&self) -> Vec<ShardInfo> {
        let shards = self.shards.read().unwrap();
        shards
            .values()
            .filter(|s| s.status == ShardStatus::Complete)
            .cloned()
            .collect()
    }

    /// Get the shard index for a ledger
    pub fn shard_index_for_ledger(ledger_index: LedgerIndex) -> u64 {
        (ledger_index as u64) / SHARD_SIZE
    }

    /// Start downloading a shard
    pub fn start_download(&self, shard_index: u64, peers: Vec<String>) -> Result<(), ShardError> {
        // Check if already downloading
        {
            let downloads = self.active_downloads.read().unwrap();
            if downloads.contains_key(&shard_index) {
                return Err(ShardError::AlreadyDownloading);
            }
        }

        // Check if already complete
        {
            let shards = self.shards.read().unwrap();
            if let Some(info) = shards.get(&shard_index) {
                if info.status == ShardStatus::Complete {
                    return Err(ShardError::AlreadyComplete);
                }
            }
        }

        // Start download
        let download = ShardDownload {
            shard_index,
            peers,
            progress: 0,
            start_time: SystemTime::now(),
        };

        let mut downloads = self.active_downloads.write().unwrap();
        downloads.insert(shard_index, download);

        // Update status
        let mut shards = self.shards.write().unwrap();
        let info = shards.entry(shard_index).or_insert_with(|| ShardInfo::new(shard_index));
        info.status = ShardStatus::Downloading;

        Ok(())
    }

    /// Update download progress
    pub fn update_download_progress(&self, shard_index: u64, progress: u8) {
        let mut downloads = self.active_downloads.write().unwrap();
        if let Some(download) = downloads.get_mut(&shard_index) {
            download.progress = progress;
        }

        let mut shards = self.shards.write().unwrap();
        if let Some(info) = shards.get_mut(&shard_index) {
            info.progress = progress;
        }
    }

    /// Get download progress
    pub fn get_download_progress(&self, shard_index: u64) -> Option<u8> {
        let downloads = self.active_downloads.read().unwrap();
        downloads.get(&shard_index).map(|d| d.progress)
    }

    /// Complete a shard download
    pub fn complete_download(
        &self,
        shard_index: u64,
        data: Vec<u8>,
        hash: UInt256,
    ) -> Result<(), ShardError> {
        // Remove from active downloads
        {
            let mut downloads = self.active_downloads.write().unwrap();
            downloads.remove(&shard_index);
        }

        // Save shard to disk
        let shard_path = format!("{}/{}.shard", self.shard_dir, shard_index);
        std::fs::write(&shard_path, &data)?;

        // Save hash
        let hash_path = format!("{}/{}.hash", self.shard_dir, shard_index);
        std::fs::write(&hash_path, hash.as_bytes())?;

        // Update shard info
        let mut shards = self.shards.write().unwrap();
        let info = shards.entry(shard_index).or_insert_with(|| ShardInfo::new(shard_index));
        info.hash = Some(hash);
        info.size = data.len() as u64;
        info.status = ShardStatus::Complete;
        info.timestamp = Some(SystemTime::now());
        info.progress = 100;

        // Cache archive
        let archive = ShardArchive {
            index: shard_index,
            data,
            hash,
        };

        let mut cache = self.archive_cache.write().unwrap();
        cache.insert(shard_index, Arc::new(archive));

        Ok(())
    }

    /// Mark download as failed
    pub fn fail_download(&self, shard_index: u64) {
        let mut downloads = self.active_downloads.write().unwrap();
        downloads.remove(&shard_index);

        let mut shards = self.shards.write().unwrap();
        if let Some(info) = shards.get_mut(&shard_index) {
            info.status = ShardStatus::Error;
            info.progress = 0;
        }
    }

    /// Get the path to a shard file
    pub fn get_shard_path(&self, shard_index: u64) -> String {
        format!("{}/{}.shard", self.shard_dir, shard_index)
    }

    /// Check if a shard is complete
    pub fn is_shard_complete(&self, shard_index: u64) -> bool {
        let shards = self.shards.read().unwrap();
        shards
            .get(&shard_index)
            .map(|s| s.status == ShardStatus::Complete)
            .unwrap_or(false)
    }

    /// Delete a shard
    pub fn delete_shard(&self, shard_index: u64) -> Result<(), ShardError> {
        // Remove from memory
        {
            let mut shards = self.shards.write().unwrap();
            shards.remove(&shard_index);
        }

        {
            let mut cache = self.archive_cache.write().unwrap();
            cache.remove(&shard_index);
        }

        // Delete files
        let shard_path = format!("{}/{}.shard", self.shard_dir, shard_index);
        let hash_path = format!("{}/{}.hash", self.shard_dir, shard_index);

        std::fs::remove_file(shard_path).ok();
        std::fs::remove_file(hash_path).ok();

        Ok(())
    }

    /// Create a shard archive from ledgers in the node store
    pub fn create_shard_from_ledgers(
        &self,
        shard_index: u64,
        ledgers: Vec<(LedgerIndex, Vec<NodeObject>)>,
    ) -> Result<ShardArchive, ShardError> {
        // Serialize ledgers
        let mut data = Vec::new();

        for (ledger_index, objects) in ledgers {
            // Add ledger index header
            data.extend_from_slice(&ledger_index.to_be_bytes());

            // Add object count
            data.extend_from_slice(&(objects.len() as u32).to_be_bytes());

            // Add each object
            for obj in objects {
                let obj_bytes = obj.encode();
                data.extend_from_slice(&(obj_bytes.len() as u32).to_be_bytes());
                data.extend_from_slice(&obj_bytes);
            }
        }

        // Compress data
        let compressed = compress_data(&data)?;

        // Compute hash
        let hash = crypto::sha512_half(&compressed);

        Ok(ShardArchive {
            index: shard_index,
            data: compressed,
            hash,
        })
    }

    /// Get active downloads
    pub fn get_active_downloads(&self) -> Vec<ShardDownloadInfo> {
        let downloads = self.active_downloads.read().unwrap();
        downloads
            .values()
            .map(|d| ShardDownloadInfo {
                shard_index: d.shard_index,
                progress: d.progress,
                elapsed_secs: d
                    .start_time
                    .elapsed()
                    .unwrap_or_default()
                    .as_secs(),
            })
            .collect()
    }
}

/// Info about an active download (for RPC responses)
#[derive(Debug, serde::Serialize)]
pub struct ShardDownloadInfo {
    pub shard_index: u64,
    pub progress: u8,
    pub elapsed_secs: u64,
}

/// Shard store errors
#[derive(Debug, thiserror::Error)]
pub enum ShardError {
    #[error("shard already downloading")]
    AlreadyDownloading,
    #[error("shard already complete")]
    AlreadyComplete,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("compression error: {0}")]
    Compression(String),
    #[error("invalid shard data")]
    InvalidData,
}

/// Compress data using a simple compression
fn compress_data(data: &[u8]) -> Result<Vec<u8>, ShardError> {
    // For now, just return the data (compression can be added later)
    // In production, this would use zstd or similar
    Ok(data.to_vec())
}

/// Shard crawler for discovering shards on the network
pub struct ShardCrawler {
    /// Known peers with shards
    peer_shards: RwLock<Vec<PeerShard>>,
    /// Last crawl time
    last_crawl: RwLock<Option<SystemTime>>,
}

impl ShardCrawler {
    /// Create a new shard crawler
    pub fn new() -> Self {
        Self {
            peer_shards: RwLock::new(Vec::new()),
            last_crawl: RwLock::new(None),
        }
    }

    /// Record shard information from a peer
    pub fn report_peer_shard(&self, peer_id: String, peer_address: String, shard_index: u64, is_complete: bool) {
        let mut peers = self.peer_shards.write().unwrap();

        // Remove existing entry for this peer+shard combo
        peers.retain(|p| !(p.peer_id == peer_id && p.shard_index == shard_index));

        peers.push(PeerShard {
            peer_id,
            peer_address,
            shard_index,
            shard_hash: None,
            is_complete,
        });
    }

    /// Get all known peer shards
    pub fn get_peer_shards(&self) -> Vec<PeerShard> {
        let peers = self.peer_shards.read().unwrap();
        peers.clone()
    }

    /// Get peers that have a specific shard
    pub fn get_peers_for_shard(&self, shard_index: u64) -> Vec<PeerShard> {
        let peers = self.peer_shards.read().unwrap();
        peers
            .iter()
            .filter(|p| p.shard_index == shard_index && p.is_complete)
            .cloned()
            .collect()
    }

    /// Get unique shard indices available from peers
    pub fn get_available_shard_indices(&self) -> Vec<u64> {
        let peers = self.peer_shards.read().unwrap();
        let mut indices: Vec<u64> = peers
            .iter()
            .filter(|p| p.is_complete)
            .map(|p| p.shard_index)
            .collect();
        indices.sort_unstable();
        indices.dedup();
        indices
    }

    /// Update last crawl time
    pub fn record_crawl(&self) {
        let mut last = self.last_crawl.write().unwrap();
        *last = Some(SystemTime::now());
    }

    /// Get last crawl time
    pub fn get_last_crawl(&self) -> Option<SystemTime> {
        *self.last_crawl.read().unwrap()
    }

    /// Clear peer shard data (e.g., when peer disconnects)
    pub fn clear_peer(&self, peer_id: &str) {
        let mut peers = self.peer_shards.write().unwrap();
        peers.retain(|p| p.peer_id != peer_id);
    }
}

impl Default for ShardCrawler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shard_info() {
        let info = ShardInfo::new(0);
        assert_eq!(info.start_ledger, 0);
        assert_eq!(info.end_ledger, 16383);

        let info = ShardInfo::new(1);
        assert_eq!(info.start_ledger, 16384);
        assert_eq!(info.end_ledger, 32767);
    }

    #[test]
    fn test_shard_contains_ledger() {
        let info = ShardInfo::new(0);
        assert!(info.contains_ledger(0));
        assert!(info.contains_ledger(16383));
        assert!(!info.contains_ledger(16384));
    }

    #[test]
    fn test_shard_index_for_ledger() {
        assert_eq!(ShardStore::shard_index_for_ledger(0), 0);
        assert_eq!(ShardStore::shard_index_for_ledger(16383), 0);
        assert_eq!(ShardStore::shard_index_for_ledger(16384), 1);
        assert_eq!(ShardStore::shard_index_for_ledger(32767), 1);
    }
}
