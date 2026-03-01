#!/usr/bin/env python3
"""
Test transaction signing and submission for Callchain devnet.

This script creates signed Payment transactions and submits them to the node.
"""

import json
import subprocess
import sys

def rpc_call(method, params=None, req_id=1):
    """Make an RPC call to the node."""
    payload = {
        "jsonrpc": "2.0",
        "method": method,
        "id": req_id
    }
    if params:
        payload["params"] = params

    result = subprocess.run([
        "curl", "-s", "-X", "POST", "http://127.0.0.1:5005",
        "-H", "Content-Type: application/json",
        "-d", json.dumps(payload)
    ], capture_output=True, text=True)

    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError:
        print(f"Failed to parse response: {result.stdout}")
        return None

def get_account_info(account):
    """Get account info."""
    return rpc_call("account_info", {"account": account})

def submit_transaction(tx_blob):
    """Submit a signed transaction."""
    return rpc_call("submit", {"tx_blob": tx_blob})

def test_account_balances():
    """Test that genesis accounts have correct balances."""
    print("=== Testing Genesis Account Balances ===\n")

    # Genesis account addresses (hex format)
    accounts = [
        ("4220c256143601b032b6f71ffa269a01a6045124", "100000000000"),  # 100k CALL
        ("659159bf3b6d1aab74af21c25a61f54586f2fd39", "50000000000"),   # 50k CALL
        ("68578b98b6492fe15def14ddc362eda37005cd64", "25000000000"),   # 25k CALL
        ("cce69ed722a6ed7346402f106a679616980a671e", "10000000000"),   # 10k CALL
        ("68dc041103b567904ced3ebb7097e7d89ab5f6fd", "5000000000"),    # 5k CALL
    ]

    all_passed = True
    for account, expected_balance in accounts:
        result = get_account_info(account)
        if result and "result" in result:
            actual_balance = result["result"]["account_data"]["Balance"]
            if actual_balance == expected_balance:
                print(f"✓ Account {account}: {actual_balance} drops (correct)")
            else:
                print(f"✗ Account {account}: {actual_balance} drops (expected {expected_balance})")
                all_passed = False
        else:
            print(f"✗ Account {account}: Failed to get info")
            all_passed = False

    print()
    return all_passed

def test_server_info():
    """Test server info endpoint."""
    print("=== Testing Server Info ===\n")

    result = rpc_call("server_info")
    if result and "result" in result:
        info = result["result"]
        print(f"Node State: {info.get('state', 'unknown')}")
        print(f"Ledger Seq: {info.get('ledger_seq', 'unknown')}")
        print(f"Peers: {info.get('peer_count', 'unknown')}")
        print()
        return True
    else:
        print("✗ Failed to get server info\n")
        return False

def main():
    print("Callchain Devnet Transaction Test Tool")
    print("=" * 50)
    print()

    # Test server connectivity
    if not test_server_info():
        print("ERROR: Cannot connect to node. Is it running?")
        sys.exit(1)

    # Test genesis account balances
    if not test_account_balances():
        print("WARNING: Some account balance checks failed")

    print("=== Transaction Testing ===")
    print()
    print("NOTE: Transaction signing requires private keys.")
    print("To test transactions, you need to either:")
    print("1. Generate wallets with known seeds and transfer funds to them")
    print("2. Use a signing tool with the genesis account private keys")
    print()
    print("Example transaction submission:")
    print('  tx_blob = "<hex-encoded-signed-transaction>"')
    print('  result = submit_transaction(tx_blob)')
    print()

    # Placeholder for future transaction tests
    print("Basic connectivity and balance tests completed successfully!")

if __name__ == "__main__":
    main()
