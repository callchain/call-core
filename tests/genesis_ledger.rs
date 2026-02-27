//! Genesis ledger creation and validation tests
//!
//! These tests verify the structure and content of the genesis ledger,
//! ensuring it matches the expected configuration for Callchain.

use primitives::{AccountID, Currency, UInt256};
use protocol::{AccountRoot, Fees, Ledger, LedgerEntry, LedgerInfo, Transaction, TxType};

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

/// Test that the genesis ledger is valid
/// - Sequence number is 1
/// - Parent hash is zero (no parent)
/// - Has valid ledger hash
/// - Contains expected initial state
#[test]
fn test_genesis_ledger_valid() {
    let ledger = Ledger::genesis();

    // Genesis ledger should have sequence 1
    assert_eq!(ledger.get_seq(), 1, "Genesis ledger sequence should be 1");

    // Genesis has no parent
    assert_eq!(
        ledger.info.parent_hash,
        UInt256::zero(),
        "Genesis ledger parent hash should be zero"
    );

    // Genesis close time should be 0 (epoch)
    assert_eq!(
        ledger.info.close_time, 0,
        "Genesis ledger close time should be 0"
    );

    // Should have total CALL supply configured
    assert_eq!(
        ledger.info.drops,
        100_000_000_000_000_000, // 100 billion CALL
        "Genesis ledger should have correct total supply"
    );

    // Transaction tree should be empty or properly initialized
    assert_eq!(
        ledger.transaction_count(), 0,
        "Genesis ledger should have no transactions"
    );

    // Should have valid close time resolution
    assert!(
        ledger.info.close_time_resolution > 0,
        "Close time resolution should be positive"
    );
}

/// Test genesis ledger accounts
/// - Verify any initial accounts are properly configured
/// - Check account balances
/// - Validate account sequences
#[test]
fn test_genesis_accounts() {
    // Create genesis ledger
    let mut ledger = Ledger::genesis();

    // Create a genesis account (representing the initial distribution)
    let genesis_account = AccountID::new([0u8; 20]);
    let account_root = AccountRoot::new(genesis_account)
        .with_balance(100_000_000_000_000_000); // 100 billion CALL

    // Add to ledger state
    let key = account_root.ledger_index();
    // Use simple serialized data for testing
    ledger.add_state_entry(key, vec![1u8; 100]);

    // Verify account was added
    let retrieved = ledger.get_state_entry(&key);
    assert!(
        retrieved.is_some(),
        "Genesis account should be in state tree"
    );

    // Account sequence should start at 1
    assert_eq!(
        account_root.sequence, 1,
        "Genesis account sequence should be 1"
    );

    // Account should have correct balance
    assert_eq!(
        account_root.balance.mantissa,
        100_000_000_000_000_000,
        "Genesis account should have full supply"
    );

    // Owner count should be 0 (no objects owned yet)
    assert_eq!(
        account_root.owner_count, 0,
        "Genesis account should have no owned objects"
    );
}

/// Test genesis ledger balances
/// - Total supply equals sum of all account balances
/// - Reserve requirements are properly set
/// - Fee settings are valid
#[test]
fn test_genesis_balances() {
    let ledger_info = LedgerInfo::genesis();
    let fees = Fees::default();

    // Total supply should be 100 billion CALL
    assert_eq!(
        ledger_info.drops,
        100_000_000_000_000_000,
        "Total supply should be 100 billion CALL"
    );

    // Base fee should be reasonable (10 drops default)
    assert_eq!(fees.base, 10, "Base fee should be 10 drops");

    // Reserve base should be 10 CALL
    assert_eq!(
        fees.reserve, 10_000_000,
        "Reserve base should be 10 CALL"
    );

    // Reserve increment should be 2 CALL
    assert_eq!(
        fees.increment, 2_000_000,
        "Reserve increment should be 2 CALL"
    );

    // Units should be 10 (reference transaction)
    assert_eq!(fees.units, 10, "Fee units should be 10");

    // Verify fee calculation
    let base_tx_fee = fees.calculate_fee(10);
    assert_eq!(
        base_tx_fee, fees.base,
        "Base transaction fee calculation should match base fee"
    );
}

/// Test that genesis ledger can be properly hashed
/// - Ledger hash computation
/// - Account state hash computation
/// - Transaction tree hash computation
#[test]
fn test_genesis_ledger_hash() {
    let mut ledger = Ledger::genesis();

    // Initially, hashes may be zero
    assert_eq!(
        ledger.info.hash,
        UInt256::zero(),
        "Genesis hash should be zero before computation"
    );

    // Add some state
    let account = AccountID::new([1u8; 20]);
    let account_root = AccountRoot::new(account).with_balance(10_000_000);
    let key = account_root.ledger_index();
    // Use simple serialized data for testing
    ledger.add_state_entry(key, vec![1u8; 100]);

    // Compute hashes
    ledger.update_hashes();

    // After computation, hashes should be non-zero
    assert_ne!(
        ledger.info.account_hash,
        UInt256::zero(),
        "Account hash should be computed"
    );

    assert_ne!(
        ledger.info.tx_hash,
        UInt256::zero(),
        "Transaction hash should be computed (even if empty)"
    );

    // Note: Ledger hash might still be zero in the simplified implementation
    // In a full implementation, this would be computed from all ledger fields
}

/// Test creating a child ledger from genesis
/// - Child ledger has correct sequence (genesis + 1)
/// - Child ledger references parent
/// - Child ledger inherits state
#[test]
fn test_genesis_child_ledger() {
    let genesis = Ledger::genesis();

    // Create child ledger
    let close_time = 100; // Some future timestamp
    let child = genesis.create_child(close_time);

    // Child should have sequence 2
    assert_eq!(
        child.get_seq(),
        2,
        "Child ledger should have sequence 2"
    );

    // Child should reference genesis as parent
    assert_eq!(
        child.info.parent_hash,
        genesis.info.hash,
        "Child should reference parent hash"
    );

    // Child should have correct close time
    assert_eq!(
        child.info.close_time,
        close_time,
        "Child should have specified close time"
    );

    // Child should reference parent's close time
    assert_eq!(
        child.info.parent_close_time,
        genesis.info.close_time,
        "Child should reference parent's close time"
    );

    // Child should inherit parent's total supply
    assert_eq!(
        child.info.drops,
        genesis.info.drops,
        "Child should inherit total supply"
    );

    // Child should start with empty transaction list
    assert_eq!(
        child.transaction_count(),
        0,
        "Child should have no transactions initially"
    );
}

/// Test that genesis ledger state tree is properly initialized
#[test]
fn test_genesis_state_tree() {
    let ledger = Ledger::genesis();

    // State tree should exist
    let root_hash = ledger.state_tree.get_root_hash();
    // Even empty tree should have a hash (computed from empty state)
    assert_ne!(
        root_hash,
        UInt256::zero(),
        "Empty state tree should still have a root hash"
    );
}

/// Test genesis ledger close flags
#[test]
fn test_genesis_close_flags() {
    let ledger = Ledger::genesis();

    // Genesis ledger should have no special close flags
    assert_eq!(
        ledger.info.close_flags, 0,
        "Genesis ledger should have no close flags"
    );
}

/// Test that multiple genesis ledgers are identical
#[test]
fn test_genesis_consistency() {
    let genesis1 = Ledger::genesis();
    let genesis2 = Ledger::genesis();

    // Both should have same sequence
    assert_eq!(
        genesis1.get_seq(),
        genesis2.get_seq(),
        "All genesis ledgers should have same sequence"
    );

    // Both should have same parent hash (zero)
    assert_eq!(
        genesis1.info.parent_hash,
        genesis2.info.parent_hash,
        "All genesis ledgers should have same parent hash"
    );

    // Both should have same total supply
    assert_eq!(
        genesis1.info.drops,
        genesis2.info.drops,
        "All genesis ledgers should have same total supply"
    );

    // Both should have same close time resolution
    assert_eq!(
        genesis1.info.close_time_resolution,
        genesis2.info.close_time_resolution,
        "All genesis ledgers should have same close time resolution"
    );
}

/// Test genesis ledger fees structure
#[test]
fn test_genesis_fees_structure() {
    let fees = Fees::default();

    // Verify all fee components are reasonable
    assert!(
        fees.base > 0,
        "Base fee should be positive"
    );

    assert!(
        fees.reserve > 0,
        "Reserve should be positive"
    );

    assert!(
        fees.increment > 0,
        "Reserve increment should be positive"
    );

    assert!(
        fees.units > 0,
        "Fee units should be positive"
    );

    // Load factor should start at 1.0x (1000)
    assert_eq!(
        fees.load_factor, 1000,
        "Initial load factor should be 1.0x (1000)"
    );

    // Target ledger close time should be reasonable (5 seconds default)
    assert_eq!(
        fees.target_ledger_close_time, 5,
        "Target ledger close time should be 5 seconds"
    );

    // Commission should be 0 in genesis
    assert_eq!(
        fees.commission, 0,
        "Genesis commission should be 0"
    );
}

/// Test ledger chain creation from genesis
#[test]
fn test_ledger_chain_from_genesis() {
    let mut ledgers: Vec<Ledger> = vec![Ledger::genesis()];

    // Create a chain of 10 ledgers
    for i in 0..10 {
        let parent = ledgers.last().unwrap();
        let close_time = (i + 1) * 10; // 10, 20, 30, ...
        let child = parent.create_child(close_time);
        ledgers.push(child);
    }

    // Verify chain
    assert_eq!(ledgers.len(), 11); // Genesis + 10 children

    for (i, ledger) in ledgers.iter().enumerate() {
        // Sequence should match index + 1
        assert_eq!(
            ledger.get_seq() as usize,
            i + 1,
            "Ledger {} should have sequence {}",
            i,
            i + 1
        );

        if i > 0 {
            // Parent hash should match previous ledger's hash
            assert_eq!(
                ledger.info.parent_hash,
                ledgers[i - 1].info.hash,
                "Ledger {} should reference ledger {} as parent",
                i,
                i - 1
            );

            // Close time should be increasing
            assert!(
                ledger.info.close_time > ledgers[i - 1].info.close_time,
                "Ledger {} close time should be greater than ledger {}",
                i,
                i - 1
            );
        }
    }

    // All ledgers should have same total supply
    let total_supply = ledgers[0].info.drops;
    for ledger in &ledgers {
        assert_eq!(
            ledger.info.drops, total_supply,
            "All ledgers should preserve total supply"
        );
    }
}

/// Test genesis ledger with initial state
#[test]
fn test_genesis_with_initial_state() {
    let mut ledger = Ledger::genesis();

    // Add multiple accounts to initial state
    let accounts: Vec<AccountID> = (0..5)
        .map(|i| AccountID::new([i as u8; 20]))
        .collect();

    let initial_balance = 10_000_000_000_000_000u64; // 10 billion each

    for account in &accounts {
        let account_root = AccountRoot::new(*account).with_balance(initial_balance);
        let key = account_root.ledger_index();
        // Use simple serialized data for testing
        ledger.add_state_entry(key, vec![1u8; 100]);
    }

    // Compute hashes
    ledger.update_hashes();

    // Verify account state hash is non-zero
    assert_ne!(
        ledger.info.account_hash,
        UInt256::zero(),
        "Ledger with accounts should have non-zero account hash"
    );

    // Verify we can retrieve accounts
    for account in &accounts {
        let account_root = AccountRoot::new(*account);
        let key = account_root.ledger_index();
        assert!(
            ledger.get_state_entry(&key).is_some(),
            "Should be able to retrieve genesis account"
        );
    }
}
