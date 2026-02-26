//! Integration tests for call-core blockchain

use primitives::{AccountID, Currency, NodeID, UInt256};
use protocol::{Transaction, TxType, Offer, OfferBook};
use consensus::{Consensus, ConsensusParms, Proposal};
use network::{Overlay, Peer};
use storage::Database;
use crypto::PrivateKey;
use serialization::Amount;
use shamap::{SHAMap, SHAMapType};

/// Test transaction creation
#[test]
fn test_transaction_creation() {
    let account = AccountID::new([0u8; 20]);
    let tx = Transaction::new(TxType::Payment, account, 1);

    assert_eq!(tx.get_account(), account);
    assert_eq!(tx.get_sequence(), 1);
    assert_eq!(tx.get_tx_type(), TxType::Payment);
    assert_eq!(tx.get_fee(), 10); // Default fee
}

/// Test key generation
#[test]
fn test_key_operations() {
    // Generate secp256k1 private key
    let private_key = PrivateKey::generate_secp256k1();

    // Get public key
    let public_key = private_key.to_public_key();
    assert!(!public_key.as_bytes().is_empty());
}

/// Test consensus round lifecycle
#[test]
fn test_consensus_round() {
    let node_id = NodeID::new([0u8; 32]);
    let params = ConsensusParms::default();
    let mut consensus = Consensus::new(node_id, params);

    // Start a round
    let prev_ledger = UInt256::zero();
    consensus.start_round(prev_ledger, 1);

    assert_eq!(consensus.get_ledger_index(), 1);
    assert_eq!(consensus.get_round_id(), 1);

    // Close the ledger
    let tx_set = UInt256::from_be_bytes([1u8; 32]);
    consensus.close_ledger(tx_set, 0);

    // Simulate receiving proposals from peers
    for i in 1..5 {
        let peer_id = NodeID::new([i as u8; 32]);
        let proposal = Proposal::new(peer_id, prev_ledger, tx_set, 0, 0);
        consensus.process_proposal(proposal, 1000);
    }

    assert_eq!(consensus.get_peer_count(), 4);
}

/// Test SHAMap operations
#[test]
fn test_shamap_operations() {
    use shamap::{SHAMapItem, SHAMapType};

    let mut map = SHAMap::new(SHAMapType::State);

    // Add items
    let key1 = UInt256::from_be_bytes([1u8; 32]);
    let key2 = UInt256::from_be_bytes([2u8; 32]);
    let item1 = SHAMapItem::new(key1, vec![10u8; 100]);
    let item2 = SHAMapItem::new(key2, vec![20u8; 100]);

    map.add_item(key1, item1.clone());
    map.add_item(key2, item2.clone());

    // Get hash
    let hash = map.get_root_hash();
    assert_ne!(hash, UInt256::zero());

    // Get item
    let retrieved = map.get_item(&key1);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().data(), item1.data());

    // Hash should be consistent
    let hash2 = map.get_root_hash();
    assert_eq!(hash, hash2);

    // Map should not be empty
    assert!(!map.is_empty());
}

/// Test DEX offer book
#[test]
fn test_offer_book() {
    let currency1 = Currency::CALL;
    let currency2 = Currency::new([1u8; 20]);

    let mut book = OfferBook::new(currency1, currency2);
    assert!(book.get_best_offer().is_none());

    // Add an offer
    let account = AccountID::new([0u8; 20]);
    let taker_gets = Amount::call(1000000);
    let taker_pays = Amount::issued(2000000, -6, currency2, account).unwrap();

    let offer = Offer::new(account, 1, taker_gets, taker_pays);
    book.add_offer(offer);

    assert!(book.get_best_offer().is_some());
    assert_eq!(book.offers.len(), 1);

    // Remove the offer
    let removed = book.remove_offer(account, 1);
    assert!(removed);
    assert!(book.get_best_offer().is_none());
}

/// Test overlay network
#[test]
fn test_overlay_network() {
    let mut overlay = Overlay::new();
    assert_eq!(overlay.peer_count(), 0);

    // Add peers
    let addr1: std::net::SocketAddr = "127.0.0.1:1234".parse().unwrap();
    let addr2: std::net::SocketAddr = "127.0.0.1:1235".parse().unwrap();

    overlay.add_peer(Peer::new(addr1));
    overlay.add_peer(Peer::new(addr2));

    assert_eq!(overlay.peer_count(), 2);
    assert!(overlay.can_accept_peer());

    // Remove a peer
    overlay.remove_peer(&addr1);
    assert_eq!(overlay.peer_count(), 1);
}

/// Test Amount operations
#[test]
fn test_amount_operations() {
    // Native CALL amount
    let call_amount = Amount::call(1000000);
    assert!(call_amount.is_native());
    assert_eq!(call_amount.mantissa, 1000000);

    // Issued currency amount
    let issuer = AccountID::new([0u8; 20]);
    let currency = Currency::new([1u8; 20]);
    let issued = Amount::issued(5000000, -6, currency, issuer).unwrap();

    assert!(!issued.is_native());
    assert_eq!(issued.issuer, issuer);
    assert_eq!(issued.currency, currency);
}

/// Test database operations
#[test]
fn test_database_operations() {
    use storage::{NodeObject, NodeObjectType};

    let backend = Box::new(storage::MemoryBackend::new());
    let db = Database::new(backend);

    // Create a node object
    let hash = UInt256::from_be_bytes([1u8; 32]);
    let data = vec![1u8, 2u8, 3u8, 4u8];
    let obj = NodeObject::new(NodeObjectType::Ledger, hash, data.clone());

    // Store it
    db.store_node(obj.clone());

    // Retrieve it
    let retrieved = db.fetch_node(&hash);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().data, data);
}
