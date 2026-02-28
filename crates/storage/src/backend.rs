use crate::node_object::{NodeObject, NodeObjectType};
use primitives::UInt256;
use std::collections::HashMap;

/// Write operation for batch processing
#[derive(Debug, Clone)]
pub enum WriteOp {
    /// Store a node object
    Store(NodeObject),
    /// Delete a node by hash
    Delete(UInt256),
}

/// Batch of write operations
#[derive(Debug, Default)]
pub struct WriteBatch {
    pub ops: Vec<WriteOp>,
}

impl WriteBatch {
    pub fn new() -> Self {
        Self { ops: Vec::new() }
    }

    pub fn store(&mut self, obj: NodeObject) {
        self.ops.push(WriteOp::Store(obj));
    }

    pub fn delete(&mut self, hash: UInt256) {
        self.ops.push(WriteOp::Delete(hash));
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    pub fn len(&self) -> usize {
        self.ops.len()
    }
}

/// Backend trait for storage operations
pub trait Backend: Send + Sync {
    /// Fetch a node by its hash
    fn fetch(&self, hash: &UInt256) -> Option<NodeObject>;

    /// Store a single node
    fn store(&mut self, obj: NodeObject);

    /// Delete a node by hash
    fn delete(&mut self, hash: &UInt256);

    /// Check if a node exists
    fn exists(&self, hash: &UInt256) -> bool;

    /// Execute a batch of write operations
    fn write_batch(&mut self, batch: WriteBatch);

    /// Mark nodes as potentially deleted (cleanup hint)
    fn set_deleted(&mut self, _previous_ledger: UInt256) {
        // Default: no-op
    }

    /// Iterate over all nodes of a specific type
    fn iterate(&self, obj_type: NodeObjectType) -> Box<dyn Iterator<Item = NodeObject> + '_>;
}

/// Column family names for RocksDB
pub mod cf {
    pub const LEDGERS: &str = "ledgers";
    pub const TRANSACTIONS: &str = "transactions";
    pub const ACCOUNTS: &str = "accounts";
    pub const METADATA: &str = "metadata";
}

/// RocksDB backend implementation
pub struct RocksDBBackend {
    db: rocksdb::DB,
    path: String,
}

impl RocksDBBackend {
    pub fn new(path: &str) -> Result<Self, StorageError> {
        let mut opts = rocksdb::Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        // Configure column families
        let cf_names = vec![cf::LEDGERS, cf::TRANSACTIONS, cf::ACCOUNTS, cf::METADATA];
        let cf_descriptors: Vec<_> = cf_names
            .iter()
            .map(|name| rocksdb::ColumnFamilyDescriptor::new(*name, opts.clone()))
            .collect();

        let db = rocksdb::DB::open_cf_descriptors(&opts, path, cf_descriptors)
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(Self {
            db,
            path: path.to_string(),
        })
    }

    /// Get the database path
    pub fn get_path(&self) -> &str {
        &self.path
    }

    /// Get column family handle
    fn get_cf(&self, cf_name: &str) -> Result<&rocksdb::ColumnFamily, StorageError> {
        self.db.cf_handle(cf_name)
            .ok_or_else(|| StorageError::DatabaseError(format!("Column family {} not found", cf_name)))
    }

    /// Get the appropriate column family for an object type
    fn cf_for_type(&self, obj_type: NodeObjectType) -> Result<&rocksdb::ColumnFamily, StorageError> {
        match obj_type {
            NodeObjectType::Ledger => self.get_cf(cf::LEDGERS),
            NodeObjectType::TransactionNode => self.get_cf(cf::TRANSACTIONS),
            NodeObjectType::AccountNode => self.get_cf(cf::ACCOUNTS),
            NodeObjectType::Metadata => self.get_cf(cf::METADATA),
            NodeObjectType::Unknown => self.get_cf(cf::METADATA),
        }
    }
}

impl Backend for RocksDBBackend {
    fn fetch(&self, hash: &UInt256) -> Option<NodeObject> {
        // Try each column family in order
        for cf_name in [cf::LEDGERS, cf::TRANSACTIONS, cf::ACCOUNTS, cf::METADATA] {
            if let Ok(cf) = self.get_cf(cf_name) {
                if let Ok(Some(data)) = self.db.get_cf(cf, hash.as_bytes()) {
                    // Determine type from column family
                    let obj_type = match cf_name {
                        cf::LEDGERS => NodeObjectType::Ledger,
                        cf::TRANSACTIONS => NodeObjectType::TransactionNode,
                        cf::ACCOUNTS => NodeObjectType::AccountNode,
                        cf::METADATA => NodeObjectType::Metadata,
                        _ => NodeObjectType::Unknown,
                    };
                    return Some(NodeObject::new(obj_type, *hash, data));
                }
            }
        }
        None
    }

    fn store(&mut self, obj: NodeObject) {
        if let Ok(cf) = self.cf_for_type(obj.object_type) {
            let _ = self.db.put_cf(cf, obj.hash.as_bytes(), &obj.data);
        }
    }

    fn delete(&mut self, hash: &UInt256) {
        // Try to delete from all column families
        for cf_name in [cf::LEDGERS, cf::TRANSACTIONS, cf::ACCOUNTS, cf::METADATA] {
            if let Ok(cf) = self.get_cf(cf_name) {
                let _ = self.db.delete_cf(cf, hash.as_bytes());
            }
        }
    }

    fn exists(&self, hash: &UInt256) -> bool {
        self.fetch(hash).is_some()
    }

    fn write_batch(&mut self, batch: WriteBatch) {
        let mut rocks_batch = rocksdb::WriteBatch::default();

        for op in batch.ops {
            match op {
                WriteOp::Store(obj) => {
                    if let Ok(cf) = self.cf_for_type(obj.object_type) {
                        rocks_batch.put_cf(cf, obj.hash.as_bytes(), &obj.data);
                    }
                }
                WriteOp::Delete(hash) => {
                    // Delete from all column families
                    for cf_name in [cf::LEDGERS, cf::TRANSACTIONS, cf::ACCOUNTS, cf::METADATA] {
                        if let Ok(cf) = self.get_cf(cf_name) {
                            rocks_batch.delete_cf(cf, hash.as_bytes());
                        }
                    }
                }
            }
        }

        let _ = self.db.write(rocks_batch);
    }

    fn iterate(&self, obj_type: NodeObjectType) -> Box<dyn Iterator<Item = NodeObject> + '_> {
        use rocksdb::IteratorMode;

        // Collect all items into a Vec for now (simpler approach)
        // A future optimization could use a custom iterator to avoid loading all at once
        let mut results = Vec::new();
        if let Ok(cf) = self.cf_for_type(obj_type) {
            let iter = self.db.iterator_cf(cf, IteratorMode::Start);
            for item in iter {
                if let Ok((key, value)) = item {
                    if key.len() == 32 {
                        let mut hash_bytes = [0u8; 32];
                        hash_bytes.copy_from_slice(&key);
                        let hash = UInt256::new(hash_bytes);
                        results.push(NodeObject::new(obj_type, hash, value.to_vec()));
                    }
                }
            }
        }

        Box::new(results.into_iter())
    }
}

/// Storage errors
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    DatabaseError(String),
}

/// Memory-backed backend for testing
#[derive(Debug, Default)]
pub struct MemoryBackend {
    data: HashMap<UInt256, (NodeObjectType, Vec<u8>)>,
}

impl MemoryBackend {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }
}

impl Backend for MemoryBackend {
    fn fetch(&self, hash: &UInt256) -> Option<NodeObject> {
        self.data.get(hash).map(|(obj_type, data)| {
            NodeObject::new(*obj_type, *hash, data.clone())
        })
    }

    fn store(&mut self, obj: NodeObject) {
        self.data.insert(obj.hash, (obj.object_type, obj.data));
    }

    fn delete(&mut self, hash: &UInt256) {
        self.data.remove(hash);
    }

    fn exists(&self, hash: &UInt256) -> bool {
        self.data.contains_key(hash)
    }

    fn write_batch(&mut self, batch: WriteBatch) {
        for op in batch.ops {
            match op {
                WriteOp::Store(obj) => {
                    self.data.insert(obj.hash, (obj.object_type, obj.data));
                }
                WriteOp::Delete(hash) => {
                    self.data.remove(&hash);
                }
            }
        }
    }

    fn iterate(&self, obj_type: NodeObjectType) -> Box<dyn Iterator<Item = NodeObject> + '_> {
        Box::new(
            self.data
                .iter()
                .filter_map(move |(hash, (stored_type, data))| {
                    if *stored_type == obj_type {
                        Some(NodeObject::new(*stored_type, *hash, data.clone()))
                    } else {
                        None
                    }
                }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_backend_basic() {
        let mut backend = MemoryBackend::new();
        let hash = UInt256::new([1u8; 32]);
        let data = vec![1, 2, 3, 4];

        assert!(!backend.exists(&hash));
        assert!(backend.fetch(&hash).is_none());

        let obj = NodeObject::new(NodeObjectType::AccountNode, hash, data.clone());
        backend.store(obj);

        assert!(backend.exists(&hash));
        let fetched = backend.fetch(&hash).unwrap();
        assert_eq!(fetched.get_data(), &data[..]);
        assert_eq!(fetched.get_type(), NodeObjectType::AccountNode);

        backend.delete(&hash);
        assert!(!backend.exists(&hash));
    }

    #[test]
    fn test_memory_backend_batch() {
        let mut backend = MemoryBackend::new();
        let hash1 = UInt256::new([1u8; 32]);
        let hash2 = UInt256::new([2u8; 32]);

        let mut batch = WriteBatch::new();
        batch.store(NodeObject::new(NodeObjectType::AccountNode, hash1, vec![1, 2]));
        batch.store(NodeObject::new(NodeObjectType::TransactionNode, hash2, vec![3, 4]));

        backend.write_batch(batch);

        assert!(backend.exists(&hash1));
        assert!(backend.exists(&hash2));
        assert_eq!(backend.len(), 2);

        let mut batch = WriteBatch::new();
        batch.delete(hash1);
        backend.write_batch(batch);

        assert!(!backend.exists(&hash1));
        assert!(backend.exists(&hash2));
    }
}
