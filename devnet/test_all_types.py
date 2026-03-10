#!/usr/bin/env python3
"""
Call-Core Single Node Transaction Type Test

Tests one transaction of each type against a single node:
- Payment
- AccountSet
- TrustSet
- OfferCreate
- OfferCancel
- SignerListSet
- SetRegularKey
- NicknameSet
- DepositPreauth
- IssueSet

Usage:
    python3 test_all_types.py [--url URL]

Example:
    python3 test_all_types.py --url http://localhost:5005
"""

import argparse
import json
import sys
import time
import subprocess
from typing import Dict, Optional, List, Tuple
from urllib.request import urlopen, Request
from urllib.error import URLError

# Genesis wallet for testing
GENESIS_WALLET = {
    "name": "Genesis Account",
    "seed": "ssyB7KxAvfRwQ6mseEjt3iY1qeqMC",
    "account": "cGmJBrEfFssWuas4kCoHTX5r6aMEf6QHhy"
}

# Secondary account for testing
SECONDARY_WALLET = {
    "name": "Secondary Account",
    "seed": "sn3nxiqzTqQnB4dvZu6qipVno5WoR",
    "account": "cE5iB7jz7jgjXoyKqYojW2LPKbn3qrmU7N"
}


def sign_with_calld_sign(secret: str, tx_json: Dict) -> Optional[str]:
    """Sign a transaction using the local calld-sign CLI tool."""
    calld_sign_paths = [
        "./target/release/calld-sign",
        "../target/release/calld-sign",
        "./calld-sign",
        "calld-sign",
    ]

    calld_sign = None
    for path in calld_sign_paths:
        if subprocess.run(["which", path], capture_output=True).returncode == 0:
            calld_sign = path
            break

    if not calld_sign:
        result = subprocess.run(["which", "calld-sign"], capture_output=True, text=True)
        if result.returncode == 0:
            calld_sign = result.stdout.strip()

    if not calld_sign:
        print("[ERROR] calld-sign binary not found")
        return None

    cmd = [
        calld_sign,
        "--secret", secret,
        "--tx-json", json.dumps(tx_json),
        "--format", "blob"
    ]

    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=10)
        if result.returncode != 0:
            print(f"[SIGN ERROR] {result.stderr.strip()}")
            return None
        return result.stdout.strip()
    except Exception as e:
        print(f"[SIGN ERROR] {e}")
        return None


def rpc_call(url: str, method: str, params: Optional[Dict] = None) -> Optional[Dict]:
    """Make an RPC call to the node."""
    payload = {
        "jsonrpc": "2.0",
        "method": method,
        "id": 1
    }
    if params:
        payload["params"] = params  # RPC expects object, not array

    try:
        req = Request(
            url,
            data=json.dumps(payload).encode(),
            headers={"Content-Type": "application/json"},
            method="POST"
        )
        with urlopen(req, timeout=10) as response:
            return json.loads(response.read().decode())
    except Exception as e:
        print(f"[RPC ERROR] {method}: {e}")
        return None


def wait_for_ledger_close(url: str, timeout: int = 30) -> bool:
    """Wait for a ledger to close."""
    start = time.time()
    last_ledger = None

    while time.time() - start < timeout:
        result = rpc_call(url, "server_info")
        if result and "result" in result:
            info = result["result"].get("info", {})
            current_ledger = info.get("validated_ledger", {}).get("seq")
            if last_ledger is None:
                last_ledger = current_ledger
            elif current_ledger > last_ledger:
                return True
        time.sleep(1)
    return False


def compute_tx_hash(tx_blob: str) -> str:
    """Compute transaction hash from tx_blob (SHA-256)."""
    import hashlib
    return hashlib.sha256(bytes.fromhex(tx_blob)).hexdigest()


def submit_transaction(url: str, secret: str, tx_json: Dict) -> Tuple[bool, Optional[str], Optional[str]]:
    """Submit a transaction and return (success, tx_hash, error)."""
    tx_blob = sign_with_calld_sign(secret, tx_json)
    if not tx_blob:
        return False, None, "Failed to sign transaction"

    result = rpc_call(url, "submit", {"tx_blob": tx_blob})
    if not result:
        return False, None, "RPC call failed"

    if "result" in result:
        engine_result = result["result"].get("engine_result")
        # Compute hash locally since submit response doesn't include it
        tx_hash = compute_tx_hash(tx_blob)

        if engine_result == "tesSUCCESS":
            return True, tx_hash, None
        else:
            return False, tx_hash, f"Engine result: {engine_result}"

    return False, None, "Unknown error"


def test_payment(url: str, sequence: int) -> Tuple[bool, str]:
    """Test Payment transaction."""
    print("  Testing Payment...")
    tx_json = {
        "TransactionType": "Payment",
        "Account": GENESIS_WALLET["account"],
        "Destination": SECONDARY_WALLET["account"],
        "Amount": "1000000",
        "Fee": "10",
        "Sequence": sequence
    }

    success, tx_hash, error = submit_transaction(url, GENESIS_WALLET["seed"], tx_json)
    if success:
        print(f"    ✓ Payment submitted (hash: {tx_hash[:16]}...)")
        return True, tx_hash
    else:
        print(f"    ✗ Payment failed: {error}")
        return False, None


def test_account_set(url: str, sequence: int) -> Tuple[bool, str]:
    """Test AccountSet transaction."""
    print("  Testing AccountSet...")
    tx_json = {
        "TransactionType": "AccountSet",
        "Account": GENESIS_WALLET["account"],
        "Domain": "68656c6c6f776f726c64",  # "helloworld" in hex
        "Fee": "10",
        "Sequence": sequence
    }

    success, tx_hash, error = submit_transaction(url, GENESIS_WALLET["seed"], tx_json)
    if success:
        print(f"    ✓ AccountSet submitted (hash: {tx_hash[:16]}...)")
        return True, tx_hash
    else:
        print(f"    ✗ AccountSet failed: {error}")
        return False, None


def test_trust_set(url: str, sequence: int) -> Tuple[bool, str]:
    """Test TrustSet transaction."""
    print("  Testing TrustSet...")
    tx_json = {
        "TransactionType": "TrustSet",
        "Account": GENESIS_WALLET["account"],
        "LimitAmount": {
            "currency": "USD",
            "issuer": SECONDARY_WALLET["account"],
            "value": "1000000"
        },
        "Fee": "10",
        "Sequence": sequence
    }

    success, tx_hash, error = submit_transaction(url, GENESIS_WALLET["seed"], tx_json)
    if success:
        print(f"    ✓ TrustSet submitted (hash: {tx_hash[:16]}...)")
        return True, tx_hash
    else:
        print(f"    ✗ TrustSet failed: {error}")
        return False, None


def test_offer_create(url: str, sequence: int) -> Tuple[bool, str]:
    """Test OfferCreate transaction."""
    print("  Testing OfferCreate...")
    tx_json = {
        "TransactionType": "OfferCreate",
        "Account": GENESIS_WALLET["account"],
        "TakerPays": "1000000",
        "TakerGets": {
            "currency": "USD",
            "issuer": SECONDARY_WALLET["account"],
            "value": "100"
        },
        "Fee": "10",
        "Sequence": sequence
    }

    success, tx_hash, error = submit_transaction(url, GENESIS_WALLET["seed"], tx_json)
    if success:
        print(f"    ✓ OfferCreate submitted (hash: {tx_hash[:16]}...)")
        return True, tx_hash
    else:
        print(f"    ✗ OfferCreate failed: {error}")
        return False, None


def test_offer_cancel(url: str, sequence: int, offer_sequence: int) -> Tuple[bool, str]:
    """Test OfferCancel transaction."""
    print("  Testing OfferCancel...")
    tx_json = {
        "TransactionType": "OfferCancel",
        "Account": GENESIS_WALLET["account"],
        "OfferSequence": offer_sequence,
        "Fee": "10",
        "Sequence": sequence
    }

    success, tx_hash, error = submit_transaction(url, GENESIS_WALLET["seed"], tx_json)
    if success:
        print(f"    ✓ OfferCancel submitted (hash: {tx_hash[:16]}...)")
        return True, tx_hash
    else:
        print(f"    ✗ OfferCancel failed: {error}")
        return False, None


def test_set_regular_key(url: str, sequence: int) -> Tuple[bool, str]:
    """Test SetRegularKey transaction."""
    print("  Testing SetRegularKey...")
    tx_json = {
        "TransactionType": "SetRegularKey",
        "Account": GENESIS_WALLET["account"],
        "RegularKey": SECONDARY_WALLET["account"],
        "Fee": "10",
        "Sequence": sequence
    }

    success, tx_hash, error = submit_transaction(url, GENESIS_WALLET["seed"], tx_json)
    if success:
        print(f"    ✓ SetRegularKey submitted (hash: {tx_hash[:16]}...)")
        return True, tx_hash
    else:
        print(f"    ✗ SetRegularKey failed: {error}")
        return False, None


def test_signer_list_set(url: str, sequence: int) -> Tuple[bool, str]:
    """Test SignerListSet transaction."""
    print("  Testing SignerListSet...")
    tx_json = {
        "TransactionType": "SignerListSet",
        "Account": GENESIS_WALLET["account"],
        "SignerQuorum": 1,
        "Signers": [
            {
                "Account": SECONDARY_WALLET["account"],
                "SignerWeight": 1
            }
        ],
        "Fee": "10",
        "Sequence": sequence
    }

    success, tx_hash, error = submit_transaction(url, GENESIS_WALLET["seed"], tx_json)
    if success:
        print(f"    ✓ SignerListSet submitted (hash: {tx_hash[:16]}...)")
        return True, tx_hash
    else:
        print(f"    ✗ SignerListSet failed: {error}")
        return False, None


def test_nickname_set(url: str, sequence: int) -> Tuple[bool, str]:
    """Test NicknameSet transaction."""
    print("  Testing NicknameSet...")
    # Generate a unique 32-byte (64 hex char) nickname hash
    import hashlib
    seed = f"nickname{sequence}"
    nickname_hash = hashlib.sha256(seed.encode()).hexdigest()
    tx_json = {
        "TransactionType": "NicknameSet",
        "Account": GENESIS_WALLET["account"],
        "Nickname": nickname_hash,
        "Fee": "10",
        "Sequence": sequence
    }

    success, tx_hash, error = submit_transaction(url, GENESIS_WALLET["seed"], tx_json)
    if success:
        print(f"    ✓ NicknameSet submitted (hash: {tx_hash[:16]}...)")
        return True, tx_hash
    else:
        print(f"    ✗ NicknameSet failed: {error}")
        return False, None


def test_deposit_preauth(url: str, sequence: int) -> Tuple[bool, str]:
    """Test DepositPreauth transaction."""
    print("  Testing DepositPreauth...")
    tx_json = {
        "TransactionType": "DepositPreauth",
        "Account": GENESIS_WALLET["account"],
        "Authorize": SECONDARY_WALLET["account"],
        "Fee": "10",
        "Sequence": sequence
    }

    success, tx_hash, error = submit_transaction(url, GENESIS_WALLET["seed"], tx_json)
    if success:
        print(f"    ✓ DepositPreauth submitted (hash: {tx_hash[:16]}...)")
        return True, tx_hash
    else:
        print(f"    ✗ DepositPreauth failed: {error}")
        return False, None


def test_issue_set(url: str, sequence: int) -> Tuple[bool, str]:
    """Test IssueSet transaction."""
    print("  Testing IssueSet...")
    tx_json = {
        "TransactionType": "IssueSet",
        "Account": GENESIS_WALLET["account"],
        "TotalSupply": "1000000000",
        "Fee": "10",
        "Sequence": sequence
    }

    success, tx_hash, error = submit_transaction(url, GENESIS_WALLET["seed"], tx_json)
    if success:
        print(f"    ✓ IssueSet submitted (hash: {tx_hash[:16]}...)")
        return True, tx_hash
    else:
        print(f"    ✗ IssueSet failed: {error}")
        return False, None


def verify_transaction_accepted(url: str) -> int:
    """Check pending transactions in open ledger."""
    result = rpc_call(url, "server_info")
    if result and "result" in result:
        return result["result"].get("info", {}).get("pending_transactions", 0)
    return -1


def get_account_sequence(url: str, account: str) -> int:
    """Get the current sequence number for an account."""
    result = rpc_call(url, "account_info", {"account": account})
    if result and "result" in result:
        return result["result"].get("account_data", {}).get("Sequence", 1)
    return 1


def main():
    parser = argparse.ArgumentParser(description="Test all Call-Core transaction types")
    parser.add_argument("--url", default="http://localhost:5005", help="Node RPC URL")
    parser.add_argument("--wait-ledger", action="store_true", help="Wait for ledger close after each tx")
    args = parser.parse_args()

    print("=" * 60)
    print("Call-Core Transaction Type Test")
    print("=" * 60)
    print(f"Node: {args.url}")
    print()

    # Check node is running
    print("Checking node connectivity...")
    result = rpc_call(args.url, "ping")
    if not result:
        print("ERROR: Cannot connect to node. Is it running?")
        sys.exit(1)
    print("  ✓ Node is online")

    # Get current ledger info
    result = rpc_call(args.url, "server_info")
    if result:
        info = result.get("result", {}).get("info", {})
        validated_ledger = info.get("validated_ledger", {})
        print(f"  Current ledger: {validated_ledger.get('seq', 'unknown')}")
        print(f"  Ledger hash: {validated_ledger.get('hash', 'unknown')[:16]}...")
    print()

    # Get starting sequence
    sequence = get_account_sequence(args.url, GENESIS_WALLET["account"])
    print(f"Starting sequence: {sequence}")
    print()

    # Test all transaction types
    print("Testing Transaction Types:")
    print("-" * 40)

    results = []
    tx_hashes = {}

    # 1. Payment
    success, tx_hash = test_payment(args.url, sequence)
    results.append(("Payment", success, tx_hash))
    if success:
        sequence += 1
        tx_hashes["Payment"] = tx_hash

    # 2. AccountSet
    success, tx_hash = test_account_set(args.url, sequence)
    results.append(("AccountSet", success, tx_hash))
    if success:
        sequence += 1
        tx_hashes["AccountSet"] = tx_hash

    # 3. TrustSet
    success, tx_hash = test_trust_set(args.url, sequence)
    results.append(("TrustSet", success, tx_hash))
    if success:
        sequence += 1
        tx_hashes["TrustSet"] = tx_hash

    # 4. OfferCreate
    success, tx_hash = test_offer_create(args.url, sequence)
    results.append(("OfferCreate", success, tx_hash))
    if success:
        sequence += 1
        tx_hashes["OfferCreate"] = tx_hash

    # 5. OfferCancel (cancels the offer we just created)
    if "OfferCreate" in tx_hashes:
        offer_seq = sequence - 1  # The sequence of the offer we created
        success, tx_hash = test_offer_cancel(args.url, sequence, offer_seq)
        results.append(("OfferCancel", success, tx_hash))
        if success:
            sequence += 1
            tx_hashes["OfferCancel"] = tx_hash
    else:
        print("  Skipping OfferCancel (no offer to cancel)")
        results.append(("OfferCancel", False, None))

    # 6. SetRegularKey
    success, tx_hash = test_set_regular_key(args.url, sequence)
    results.append(("SetRegularKey", success, tx_hash))
    if success:
        sequence += 1
        tx_hashes["SetRegularKey"] = tx_hash

    # 7. SignerListSet
    success, tx_hash = test_signer_list_set(args.url, sequence)
    results.append(("SignerListSet", success, tx_hash))
    if success:
        sequence += 1
        tx_hashes["SignerListSet"] = tx_hash

    # 8. NicknameSet
    success, tx_hash = test_nickname_set(args.url, sequence)
    results.append(("NicknameSet", success, tx_hash))
    if success:
        sequence += 1
        tx_hashes["NicknameSet"] = tx_hash

    # 9. DepositPreauth
    success, tx_hash = test_deposit_preauth(args.url, sequence)
    results.append(("DepositPreauth", success, tx_hash))
    if success:
        sequence += 1
        tx_hashes["DepositPreauth"] = tx_hash

    # 10. IssueSet
    success, tx_hash = test_issue_set(args.url, sequence)
    results.append(("IssueSet", success, tx_hash))
    if success:
        sequence += 1
        tx_hashes["IssueSet"] = tx_hash

    print()
    print("=" * 60)
    print("Test Summary")
    print("=" * 60)

    passed = sum(1 for _, success, _ in results if success)
    total = len(results)

    for tx_type, success, tx_hash in results:
        status = "✓ PASS" if success else "✗ FAIL"
        hash_str = f" ({tx_hash[:16]}...)" if tx_hash else ""
        print(f"  {tx_type:20s} {status}{hash_str}")

    print()
    print(f"Results: {passed}/{total} tests passed")

    # Check transactions are accepted (in pending pool)
    if passed > 0:
        print()
        print("Checking transactions accepted...")
        pending = verify_transaction_accepted(args.url)
        if pending >= 0:
            print(f"  Pending transactions: {pending}")

            if pending >= passed:
                print()
                print("=" * 60)
                print(f"ALL {passed} TRANSACTIONS ACCEPTED!")
                print("=" * 60)
                print()
                print("Note: Single node has slower ledger close times.")
                print("      Transactions are in the pending pool awaiting validation.")
                return 0
            else:
                print()
                print(f"WARNING: Expected {passed} pending, found {pending}")
                return 1
        else:
            print("  WARNING: Could not verify pending transactions")
            return 1
    else:
        print()
        print("ERROR: No transactions were submitted successfully")
        return 1


if __name__ == "__main__":
    sys.exit(main())
