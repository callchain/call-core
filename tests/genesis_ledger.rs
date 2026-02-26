//! Genesis ledger creation and validation tests

use primitives::{AccountID, UInt256, Currency};
use protocol::{Ledger, TxType, Transaction};

/// Test creating a genesis ledger with initial accounts
#[test]
fn test_genesis_ledger_creation() {
    // Create genesis ledger
    let genesis = Ledger::genesis();

    // Verify genesis properties
    assert_eq!(genesis.info.seq, 1);
    assert_eq!(genesis.info.parent_hash, UInt256::zero());

    println!("Genesis ledger created successfully");
}

/// Test creating a chain of ledgers
#[test]
fn test_ledger_chain() {
    // Create genesis
    let ledger1 = Ledger::genesis();
    let hash1 = ledger1.info.hash;

    // Create second ledger linking to genesis
    let ledger2 = ledger1.create_child(10);

    // Create third ledger
    let ledger3 = ledger2.create_child(20);

    // Verify chain - child ledgers have parent hash set correctly
    assert_eq!(ledger2.info.parent_hash, hash1);
    assert_eq!(ledger3.info.parent_hash, ledger2.info.hash);

    // Verify sequence numbers
    assert_eq!(ledger1.info.seq, 1);
    assert_eq!(ledger2.info.seq, 2);
    assert_eq!(ledger3.info.seq, 3);

    // Verify close times
    assert_eq!(ledger2.info.close_time, 10);
    assert_eq!(ledger3.info.close_time, 20);
}

/// Test genesis transaction (IssueSet)
#[test]
fn test_genesis_issue_set() {
    let issuer = AccountID::new([0xAA; 20]);
    let _currency = Currency::new([0x01; 20]);

    // Create an IssueSet transaction
    let tx = Transaction::new(TxType::IssueSet, issuer, 1);

    assert_eq!(tx.get_tx_type(), TxType::IssueSet);
    assert_eq!(tx.get_account(), issuer);
}
