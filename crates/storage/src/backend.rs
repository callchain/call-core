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
}

/// RocksDB backend implementation
pub struct RocksDBBackend {
    // For now, use in-memory HashMap
    // TODO: Replace with actual RocksDB
    data: HashMap<UInt256, (NodeObjectType, Vec<u8>)>,
    path: String,
}

impl RocksDBBackend {
    pub fn new(path: &str) -> Self {
        Self {
            data: HashMap::new(),
            path: path.to_string(),
        }
    }

    /// Get the database path
    pub fn get_path(&self) -> &str {
        &self.path
    }
}

impl Backend for RocksDBBackend {
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
