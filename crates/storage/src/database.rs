use crate::backend::{Backend, WriteBatch};
use crate::node_object::{NodeObject, NodeObjectType};
use primitives::UInt256;
use std::sync::{Arc, Mutex};

/// Database provides a high-level interface for storing and retrieving
/// SHAMap nodes in a key-value database.
///
/// The database uses the hash (UInt256) as the key and stores NodeObjects
/// with a 9-byte header format compatible with calld.
pub struct Database {
    backend: Arc<Mutex<Box<dyn Backend>>>,
}

impl Database {
    /// Create a new database with the given backend
    pub fn new(backend: Box<dyn Backend>) -> Self {
        Self {
            backend: Arc::new(Mutex::new(backend)),
        }
    }

    /// Fetch a node by its hash
    pub fn fetch_node(&self, hash: &UInt256) -> Option<NodeObject> {
        let backend = self.backend.lock().unwrap();
        backend.fetch(hash)
    }

    /// Store a single node object
    pub fn store_node(&self, obj: NodeObject) {
        let mut backend = self.backend.lock().unwrap();
        backend.store(obj);
    }

    /// Store a node with raw data
    pub fn store_node_data(&self, object_type: NodeObjectType, hash: UInt256, data: Vec<u8>) {
        let obj = NodeObject::new(object_type, hash, data);
        self.store_node(obj);
    }

    /// Delete a node by hash
    pub fn delete_node(&self, hash: &UInt256) {
        let mut backend = self.backend.lock().unwrap();
        backend.delete(hash);
    }

    /// Check if a node exists
    pub fn node_exists(&self, hash: &UInt256) -> bool {
        let backend = self.backend.lock().unwrap();
        backend.exists(hash)
    }

    /// Execute a batch of write operations
    pub fn write_batch(&self, batch: WriteBatch) {
        let mut backend = self.backend.lock().unwrap();
        backend.write_batch(batch);
    }

    /// Mark nodes as potentially deleted after a ledger close
    pub fn set_deleted(&self, previous_ledger: UInt256) {
        let mut backend = self.backend.lock().unwrap();
        backend.set_deleted(previous_ledger);
    }

    /// Fetch a ledger header by hash
    pub fn fetch_ledger(&self, hash: &UInt256) -> Option<NodeObject> {
        self.fetch_node(hash).filter(|obj| obj.object_type == NodeObjectType::Ledger)
    }

    /// Store a ledger header
    pub fn store_ledger(&self, hash: UInt256, data: Vec<u8>) {
        self.store_node_data(NodeObjectType::Ledger, hash, data);
    }

    /// Fetch an account state node
    pub fn fetch_account_node(&self, hash: &UInt256) -> Option<NodeObject> {
        self.fetch_node(hash).filter(|obj| obj.object_type == NodeObjectType::AccountNode)
    }

    /// Store an account state node
    pub fn store_account_node(&self, hash: UInt256, data: Vec<u8>) {
        self.store_node_data(NodeObjectType::AccountNode, hash, data);
    }

    /// Fetch a transaction node
    pub fn fetch_transaction_node(&self, hash: &UInt256) -> Option<NodeObject> {
        self.fetch_node(hash).filter(|obj| obj.object_type == NodeObjectType::TransactionNode)
    }

    /// Store a transaction node
    pub fn store_transaction_node(&self, hash: UInt256, data: Vec<u8>) {
        self.store_node_data(NodeObjectType::TransactionNode, hash, data);
    }

    /// Iterate over all nodes of a specific type
    pub fn iterate_nodes(&self, obj_type: NodeObjectType) -> Vec<NodeObject> {
        let backend = self.backend.lock().unwrap();
        backend.iterate(obj_type).collect()
    }
}

impl Clone for Database {
    fn clone(&self) -> Self {
        Self {
            backend: Arc::clone(&self.backend),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::MemoryBackend;

    #[test]
    fn test_database_basic_operations() {
        let backend = Box::new(MemoryBackend::new());
        let db = Database::new(backend);

        let hash = UInt256::new([1u8; 32]);
        let data = vec![1, 2, 3, 4, 5];

        assert!(!db.node_exists(&hash));
        assert!(db.fetch_node(&hash).is_none());

        db.store_node_data(NodeObjectType::AccountNode, hash, data.clone());

        assert!(db.node_exists(&hash));
        let obj = db.fetch_node(&hash).unwrap();
        assert_eq!(obj.get_data(), &data[..]);
        assert_eq!(obj.get_type(), NodeObjectType::AccountNode);

        db.delete_node(&hash);
        assert!(!db.node_exists(&hash));
    }

    #[test]
    fn test_database_ledger_operations() {
        let backend = Box::new(MemoryBackend::new());
        let db = Database::new(backend);

        let hash = UInt256::new([2u8; 32]);
        let data = vec![10, 20, 30];

        db.store_ledger(hash, data.clone());

        let ledger = db.fetch_ledger(&hash).unwrap();
        assert_eq!(ledger.get_data(), &data[..]);
        assert_eq!(ledger.get_type(), NodeObjectType::Ledger);

        // fetch_account_node should not return ledger data
        assert!(db.fetch_account_node(&hash).is_none());
    }

    #[test]
    fn test_database_batch() {
        let backend = Box::new(MemoryBackend::new());
        let db = Database::new(backend);

        let hash1 = UInt256::new([1u8; 32]);
        let hash2 = UInt256::new([2u8; 32]);

        let mut batch = WriteBatch::new();
        batch.store(NodeObject::new(NodeObjectType::AccountNode, hash1, vec![1, 2]));
        batch.store(NodeObject::new(NodeObjectType::TransactionNode, hash2, vec![3, 4]));

        db.write_batch(batch);

        assert!(db.node_exists(&hash1));
        assert!(db.node_exists(&hash2));
    }

    #[test]
    fn test_database_clone() {
        let backend = Box::new(MemoryBackend::new());
        let db1 = Database::new(backend);

        let hash = UInt256::new([1u8; 32]);
        db1.store_ledger(hash, vec![1, 2, 3]);

        let db2 = db1.clone();
        assert!(db2.node_exists(&hash));

        let obj = db2.fetch_ledger(&hash).unwrap();
        assert_eq!(obj.get_data(), &[1, 2, 3]);
    }
}
