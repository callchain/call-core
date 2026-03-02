#!/usr/bin/env python3
"""Test individual transaction types with local signing"""

import subprocess
import json

RPC_URL = "http://127.0.0.1:5005"

def rpc_call(method, params=None):
    cmd = [
        "curl", "-s", "-X", "POST", RPC_URL,
        "-H", "Content-Type: application/json",
        "-d", json.dumps({"jsonrpc": "2.0", "method": method, "params": params or {}, "id": 1})
    ]
    result = subprocess.run(cmd, capture_output=True, text=True)
    try:
        return json.loads(result.stdout)
    except:
        return None

def sign_local(mnemonic, account_index, tx_json):
    """Sign transaction locally using calld"""
    cmd = [
        "../target/release/calld", "sign-local",
        "--mnemonic", mnemonic,
        "--account-index", str(account_index),
        "--tx-json", json.dumps(tx_json)
    ]
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode == 0:
        return result.stdout.strip()
    else:
        print(f"Sign error: {result.stderr}")
        return None

def submit_tx(tx_blob):
    return rpc_call("submit", {"tx_blob": tx_blob})

MNEMONIC = "test test test test test test test test test test test junk"

# Test 1: Payment
print("=" * 50)
print("TEST 1: Payment")
print("=" * 50)
payment_tx = {
    "TransactionType": "Payment",
    "Account": "cagpMvZf6Z8GrnAPZdQgXrCqBvCFmQYfJi",
    "Destination": "cGMbLyeUsUpjTtMFcCWZDVA4hdMXpjvPi3",
    "Amount": "1000000",
    "Sequence": 1,
    "Fee": "10"
}
tx_blob = sign_local(MNEMONIC, 0, payment_tx)
if tx_blob:
    print(f"Signed successfully (blob length: {len(tx_blob)})")
    result = submit_tx(tx_blob)
    if result and "result" in result:
        print(f"✓ Payment submitted: {result['result'].get('tx_json', {}).get('hash', 'N/A')[:16]}...")
    else:
        print(f"✗ Submit failed: {result}")
else:
    print("✗ Signing failed")

# Check account sequence
print("\nChecking account sequences after payment...")
for i, addr in enumerate([
    "cagpMvZf6Z8GrnAPZdQgXrCqBvCFmQYfJi",
    "cGMbLyeUsUpjTtMFcCWZDVA4hdMXpjvPi3"
]):
    info = rpc_call("account_info", {"account": addr})
    if info and "result" in info:
        seq = info["result"]["account_data"]["Sequence"]
        bal = info["result"]["account_data"]["Balance"]
        print(f"  Account {i+1}: seq={seq}, balance={bal}")

# Test 2: AccountSet
print("\n" + "=" * 50)
print("TEST 2: AccountSet")
print("=" * 50)
accountset_tx = {
    "TransactionType": "AccountSet",
    "Account": "cadFKa9yk1Fv4PBj2DeZfYW48APXj4L8g",
    "Domain": "6578616D706C652E636F6D",  # "example.com" in hex
    "Sequence": 1,
    "Fee": "10"
}
tx_blob = sign_local(MNEMONIC, 2, accountset_tx)
if tx_blob:
    print(f"Signed successfully (blob length: {len(tx_blob)})")
    result = submit_tx(tx_blob)
    if result and "result" in result:
        print(f"✓ AccountSet submitted")
    else:
        error = result.get("error", {}).get("message", "Unknown error") if result else "No response"
        print(f"✗ Submit failed: {error}")
else:
    print("✗ Signing failed")

# Test 3: TrustSet
print("\n" + "=" * 50)
print("TEST 3: TrustSet")
print("=" * 50)
trustset_tx = {
    "TransactionType": "TrustSet",
    "Account": "cLxud1QujAfeYszuY6DBwBhGE4fQ3GhKS5",
    "LimitAmount": {
        "currency": "USD",
        "issuer": "cagpMvZf6Z8GrnAPZdQgXrCqBvCFmQYfJi",
        "value": "1000"
    },
    "Sequence": 1,
    "Fee": "10"
}
tx_blob = sign_local(MNEMONIC, 3, trustset_tx)
if tx_blob:
    print(f"Signed successfully (blob length: {len(tx_blob)})")
    result = submit_tx(tx_blob)
    if result and "result" in result:
        print(f"✓ TrustSet submitted")
    else:
        error = result.get("error", {}).get("message", "Unknown error") if result else "No response"
        print(f"✗ Submit failed: {error}")
else:
    print("✗ Signing failed")

print("\n" + "=" * 50)
print("DONE")
print("=" * 50)
