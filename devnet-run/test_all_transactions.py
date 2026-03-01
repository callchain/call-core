#!/usr/bin/env python3
"""
Comprehensive transaction testing for Callchain devnet.

This script tests all supported transaction types using genesis accounts
with known seeds.
"""

import json
import subprocess
import sys
import time

# Genesis wallet information - BIP44 derived from test mnemonic
# Mnemonic: "test test test test test test test test test test test junk"
# Path: m/44'/644'/0'/0/{index} (Callchain cointype: 644)
GENESIS_WALLETS = [
    {
        "name": "Genesis Account 1",
        "seed": "snTwCbEYk7ui94RuHn8SfWjs2jd5Y",
        "address": "cagpMvZf6Z8GrnAPZdQgXrCqBvCFmQYfJi",
        "hex_id": "3e39fcc137a272773c1e63fb52db14c5402ed192",
        "balance": "100000000000"
    },
    {
        "name": "Genesis Account 2",
        "seed": "snMJdHz8J4hqCAUFctxft3fYLrXHR",
        "address": "cGMbLyeUsUpjTtMFcCWZDVA4hdMXpjvPi3",
        "hex_id": "a86ee011ed6d1aa7479d0c83cedf844b0b9805b6",
        "balance": "50000000000"
    },
    {
        "name": "Genesis Account 3",
        "seed": "ssurbhFSvTy6iq3SqkwTE3PFVUc6o",
        "address": "cadFKa9yk1Fv4PBj2DeZfYW48APXj4L8g",
        "hex_id": "0110cfe0f005c09fff426f38477ae2f6e01d958e",
        "balance": "25000000000"
    },
    {
        "name": "Genesis Account 4",
        "seed": "ssiRK6eJy1UKup76Cb4tXf6H37ZXy",
        "address": "cLxud1QujAfeYszuY6DBwBhGE4fQ3GhKS5",
        "hex_id": "dafd34b1e33a5d9aabb14cb49bafa974485142f9",
        "balance": "10000000000"
    },
    {
        "name": "Genesis Account 5",
        "seed": "shNsiWGT6PqQKQGHYvtEuiaQWkjtf",
        "address": "cGUQS4suRWMibEGCz46xxMVw45yB3h5Pio",
        "hex_id": "a6b206dc858a38f5213336b49173a25cf2118a8d",
        "balance": "5000000000"
    }
]


class CallchainRPC:
    """RPC client for Callchain node."""

    def __init__(self, url="http://127.0.0.1:5005"):
        self.url = url

    def call(self, method, params=None, req_id=1):
        """Make an RPC call to the node."""
        payload = {
            "jsonrpc": "2.0",
            "method": method,
            "id": req_id
        }
        if params:
            payload["params"] = params

        result = subprocess.run([
            "curl", "-s", "-X", "POST", self.url,
            "-H", "Content-Type: application/json",
            "-d", json.dumps(payload)
        ], capture_output=True, text=True)

        try:
            return json.loads(result.stdout)
        except json.JSONDecodeError:
            print(f"Failed to parse response: {result.stdout}")
            return None

    def account_info(self, account):
        """Get account info by hex ID."""
        return self.call("account_info", {"account": account})

    def submit(self, tx_blob):
        """Submit a signed transaction."""
        return self.call("submit", {"tx_blob": tx_blob})

    def sign(self, secret, tx_json):
        """
        Sign a transaction using the node's sign RPC method.
        Returns the signed tx_blob or None if signing failed.
        """
        params = {
            "secret": secret,
            "tx_json": tx_json
        }
        result = self.call("sign", params)

        if result and "result" in result:
            if "tx_blob" in result["result"]:
                return result["result"]["tx_blob"]
            elif "error" in result["result"]:
                print(f"  Sign error: {result['result'].get('error_message', 'Unknown error')}")
                return None
            else:
                print(f"  Sign error: Unexpected response format")
                return None
        elif result and "error" in result:
            print(f"  Sign error: {result['error'].get('message', 'Unknown error')}")
            return None
        else:
            print(f"  Sign error: No response from node")
            return None


# Global RPC client
rpc = CallchainRPC()


def test_genesis_accounts():
    """Test that genesis accounts have correct balances."""
    print("=" * 60)
    print("TEST 1: Genesis Account Balances")
    print("=" * 60)

    all_passed = True
    for wallet in GENESIS_WALLETS:
        result = rpc.account_info(wallet["hex_id"])
        if result and "result" in result:
            actual_balance = result["result"]["account_data"]["Balance"]
            expected_balance = wallet["balance"]
            if actual_balance == expected_balance:
                print(f"✓ {wallet['name']}: {actual_balance} drops")
            else:
                print(f"✗ {wallet['name']}: {actual_balance} drops (expected {expected_balance})")
                all_passed = False
        else:
            print(f"✗ {wallet['name']}: Failed to get info")
            all_passed = False

    print()
    return all_passed


def test_payment_transaction():
    """Test Payment transaction."""
    print("=" * 60)
    print("TEST 2: Payment Transaction")
    print("=" * 60)

    # Use Account 1 to pay Account 2
    sender = GENESIS_WALLETS[0]
    receiver = GENESIS_WALLETS[1]

    # Create payment transaction
    tx_json = {
        "TransactionType": "Payment",
        "Account": sender["hex_id"],
        "Destination": receiver["hex_id"],
        "Amount": "1000000",  # 1 CALL in drops
        "Sequence": 1,
        "Fee": "10"
    }

    print(f"Creating payment from {sender['name']} to {receiver['name']}")
    print(f"  Amount: 1 CALL (1000000 drops)")

    # Sign the transaction
    tx_blob = rpc.sign(sender["seed"], tx_json)
    if not tx_blob:
        print("✗ Failed to sign transaction")
        print()
        return False

    print(f"  Signed successfully (tx_blob length: {len(tx_blob)})")

    # Submit the transaction
    result = rpc.submit(tx_blob)
    if result and "result" in result:
        engine_result = result["result"].get("engine_result", "Unknown")
        if engine_result == "tesSUCCESS":
            print(f"✓ Payment submitted successfully")
            print(f"  Transaction hash: {result['result'].get('tx_hash', 'N/A')}")
        else:
            print(f"✗ Payment failed: {engine_result}")
            print(f"  Message: {result['result'].get('engine_result_message', 'N/A')}")
            print()
            return False
    else:
        print("✗ Failed to submit transaction")
        print()
        return False

    print()
    return True


def test_account_set_transaction():
    """Test AccountSet transaction."""
    print("=" * 60)
    print("TEST 3: AccountSet Transaction")
    print("=" * 60)

    account = GENESIS_WALLETS[2]  # Use Account 3

    # Create AccountSet transaction
    tx_json = {
        "TransactionType": "AccountSet",
        "Account": account["hex_id"],
        "Sequence": 1,
        "Fee": "10",
        "Domain": "6578616D706C652E636F6D"  # "example.com" in hex
    }

    print(f"Creating AccountSet for {account['name']}")
    print(f"  Setting domain to: example.com")

    # Sign the transaction
    tx_blob = rpc.sign(account["seed"], tx_json)
    if not tx_blob:
        print("✗ Failed to sign transaction")
        print()
        return False

    print(f"  Signed successfully")

    # Submit the transaction
    result = rpc.submit(tx_blob)
    if result and "result" in result:
        engine_result = result["result"].get("engine_result", "Unknown")
        if engine_result == "tesSUCCESS":
            print(f"✓ AccountSet submitted successfully")
        else:
            print(f"✗ AccountSet failed: {engine_result}")
            print()
            return False
    else:
        print(f"✗ Failed to submit transaction")
        if result:
            print(f"  Debug: Response = {json.dumps(result, indent=2)}")
        else:
            print(f"  Debug: No response from submit")
        print()
        return False

    print()
    return True


def test_trust_set_transaction():
    """Test TrustSet transaction."""
    print("=" * 60)
    print("TEST 4: TrustSet Transaction")
    print("=" * 60)

    account = GENESIS_WALLETS[3]  # Use Account 4
    issuer = GENESIS_WALLETS[0]   # Account 1 as issuer

    # Create TrustSet transaction
    tx_json = {
        "TransactionType": "TrustSet",
        "Account": account["hex_id"],
        "Sequence": 1,
        "Fee": "10",
        "LimitAmount": {
            "currency": "USD",
            "issuer": issuer["hex_id"],
            "value": "1000"
        }
    }

    print(f"Creating TrustSet for {account['name']}")
    print(f"  Trusting issuer: {issuer['name']}")
    print(f"  Limit: 1000 USD")

    # Sign the transaction
    tx_blob = rpc.sign(account["seed"], tx_json)
    if not tx_blob:
        print("✗ Failed to sign transaction")
        print()
        return False

    print(f"  Signed successfully")

    # Submit the transaction
    result = rpc.submit(tx_blob)
    if result and "result" in result:
        engine_result = result["result"].get("engine_result", "Unknown")
        if engine_result == "tesSUCCESS":
            print(f"✓ TrustSet submitted successfully")
        else:
            print(f"✗ TrustSet failed: {engine_result}")
            print()
            return False
    else:
        print("✗ Failed to submit transaction")
        print()
        return False

    print()
    return True


def test_offer_transactions():
    """Test OfferCreate and OfferCancel transactions."""
    print("=" * 60)
    print("TEST 5: OfferCreate/OfferCancel Transactions")
    print("=" * 60)

    account = GENESIS_WALLETS[4]  # Use Account 5

    # First, create an offer
    tx_json = {
        "TransactionType": "OfferCreate",
        "Account": account["hex_id"],
        "Sequence": 1,
        "Fee": "10",
        "TakerPays": "5000000",  # 5 CALL
        "TakerGets": {
            "currency": "USD",
            "issuer": GENESIS_WALLETS[0]["hex_id"],
            "value": "100"
        }
    }

    print(f"Creating OfferCreate for {account['name']}")
    print(f"  TakerPays: 5 CALL")
    print(f"  TakerGets: 100 USD")

    # Sign the transaction
    tx_blob = rpc.sign(account["seed"], tx_json)
    if not tx_blob:
        print("✗ Failed to sign OfferCreate transaction")
        print()
        return False

    print(f"  Signed successfully")

    # Submit the transaction
    result = rpc.submit(tx_blob)
    if result and "result" in result:
        engine_result = result["result"].get("engine_result", "Unknown")
        if engine_result == "tesSUCCESS":
            print(f"✓ OfferCreate submitted successfully")
        else:
            print(f"✗ OfferCreate failed: {engine_result}")
            print()
            return False
    else:
        print("✗ Failed to submit transaction")
        print()
        return False

    # Now cancel the offer (using sequence 2 for the cancel tx)
    cancel_tx_json = {
        "TransactionType": "OfferCancel",
        "Account": account["hex_id"],
        "Sequence": 2,  # Next sequence after OfferCreate
        "Fee": "10",
        "OfferSequence": 1  # The sequence of the offer to cancel
    }

    print(f"Creating OfferCancel for the same account")

    # Sign the cancel transaction
    tx_blob = rpc.sign(account["seed"], cancel_tx_json)
    if not tx_blob:
        print("✗ Failed to sign OfferCancel transaction")
        print()
        return False

    # Submit the cancel transaction
    result = rpc.submit(tx_blob)
    if result and "result" in result:
        engine_result = result["result"].get("engine_result", "Unknown")
        if engine_result == "tesSUCCESS":
            print(f"✓ OfferCancel submitted successfully")
        else:
            print(f"✗ OfferCancel failed: {engine_result}")
            print()
            return False
    else:
        print("✗ Failed to submit transaction")
        print()
        return False

    print()
    return True


def test_set_regular_key_transaction():
    """Test SetRegularKey transaction."""
    print("=" * 60)
    print("TEST 6: SetRegularKey Transaction")
    print("=" * 60)

    account = GENESIS_WALLETS[1]  # Use Account 2
    regular_key = GENESIS_WALLETS[2]  # Use Account 3's address as regular key

    # Create SetRegularKey transaction
    tx_json = {
        "TransactionType": "SetRegularKey",
        "Account": account["hex_id"],
        "Sequence": 2,  # Account 2 already used sequence 1 for payment test
        "Fee": "10",
        "RegularKey": regular_key["hex_id"]
    }

    print(f"Creating SetRegularKey for {account['name']}")
    print(f"  Setting regular key to: {regular_key['name']}")

    # Sign the transaction
    tx_blob = rpc.sign(account["seed"], tx_json)
    if not tx_blob:
        print("✗ Failed to sign transaction")
        print()
        return False

    print(f"  Signed successfully")

    # Submit the transaction
    result = rpc.submit(tx_blob)
    if result and "result" in result:
        engine_result = result["result"].get("engine_result", "Unknown")
        if engine_result == "tesSUCCESS":
            print(f"✓ SetRegularKey submitted successfully")
        else:
            print(f"✗ SetRegularKey failed: {engine_result}")
            print()
            return False
    else:
        print("✗ Failed to submit transaction")
        print()
        return False

    print()
    return True


def test_signer_list_set_transaction():
    """Test SignerListSet transaction."""
    print("=" * 60)
    print("TEST 7: SignerListSet Transaction")
    print("=" * 60)

    account = GENESIS_WALLETS[0]  # Use Account 1
    signer1 = GENESIS_WALLETS[1]
    signer2 = GENESIS_WALLETS[2]

    # Create SignerListSet transaction
    tx_json = {
        "TransactionType": "SignerListSet",
        "Account": account["hex_id"],
        "Sequence": 3,  # Account 1 used sequence 1 for payment test
        "Fee": "10",
        "SignerQuorum": 2,
        "Signers": [
            {
                "Account": signer1["hex_id"],
                "SignerWeight": 1
            },
            {
                "Account": signer2["hex_id"],
                "SignerWeight": 1
            }
        ]
    }

    print(f"Creating SignerListSet for {account['name']}")
    print(f"  SignerQuorum: 2")
    print(f"  Signer 1: {signer1['name']} (weight 1)")
    print(f"  Signer 2: {signer2['name']} (weight 1)")

    # Sign the transaction
    tx_blob = rpc.sign(account["seed"], tx_json)
    if not tx_blob:
        print("✗ Failed to sign transaction")
        print()
        return False

    print(f"  Signed successfully")

    # Submit the transaction
    result = rpc.submit(tx_blob)
    if result and "result" in result:
        engine_result = result["result"].get("engine_result", "Unknown")
        if engine_result == "tesSUCCESS":
            print(f"✓ SignerListSet submitted successfully")
        else:
            print(f"✗ SignerListSet failed: {engine_result}")
            print()
            return False
    else:
        print("✗ Failed to submit transaction")
        print()
        return False

    print()
    return True


def main():
    print("\n" + "=" * 60)
    print("Callchain Devnet - Comprehensive Transaction Testing")
    print("=" * 60)
    print()

    # Check if node is running
    result = rpc.call("server_info", req_id=0)
    if not result:
        print("ERROR: Cannot connect to node at http://127.0.0.1:5005")
        print("Please start the node first:")
        print("  ./target/release/calld --data-dir ./data --rpc-port 5005")
        sys.exit(1)

    print("Connected to node successfully!")
    print()

    # Run all tests
    tests = [
        ("Genesis Accounts", test_genesis_accounts),
        ("Payment", test_payment_transaction),
        ("AccountSet", test_account_set_transaction),
        ("TrustSet", test_trust_set_transaction),
        ("Offer Transactions", test_offer_transactions),
        ("SetRegularKey", test_set_regular_key_transaction),
        ("SignerListSet", test_signer_list_set_transaction),
    ]

    results = {}
    for name, test_func in tests:
        results[name] = test_func()

    # Summary
    print("=" * 60)
    print("TEST SUMMARY")
    print("=" * 60)
    passed = sum(1 for v in results.values() if v)
    total = len(results)
    for name, result in results.items():
        status = "✓ PASS" if result else "✗ FAIL"
        print(f"  {status}: {name}")
    print()
    print(f"Total: {passed}/{total} tests passed")
    print()

    if passed == total:
        print("All tests passed! ✓")
        return 0
    else:
        print("Some tests failed.")
        return 1


if __name__ == "__main__":
    sys.exit(main())
