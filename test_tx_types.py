#!/usr/bin/env python3
"""Test individual transaction types with local signing"""

import subprocess
import json
import time
import os

# Get the directory where the script is located
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
CALLD_PATH = os.path.join(os.path.dirname(SCRIPT_DIR), "target", "release", "calld")

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

def sign_local(key, tx_json):
    """Sign transaction locally using calld"""
    cmd = [
        CALLD_PATH, "sign-local",
        "--key", key,
        "--tx", json.dumps(tx_json)
    ]
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode == 0:
        return result.stdout.strip()
    else:
        print(f"Sign error: {result.stderr}")
        return None

def submit_tx(tx_blob):
    return rpc_call("submit", {"tx_blob": tx_blob})

# Private keys from BIP44 derivation
PRIVATE_KEYS = [
    "84bae4ac1031a98e5268ff4a9f1e983ada5822622a4040edd5e694a05ed4c7d8",  # Account 0
    "80a2abd1ed4321180d246759c1e183bc5f20e8675d7ec515e9e66962cbc4ae59",  # Account 1
    "c87e982930e9b7494f0cdd7510bb4d441596d7e74a210f3c5cc8153e105cf43c",  # Account 2
    "fe83cb0e5a07fe24abc53439b36d7cc3dae577dbe4ebdf8cabbeff6dcb004289",  # Account 3
    "214b5a897577647adcb763492f9f4a81ba5f7c618cc77efa590ea63cb54d9711",  # Account 4
]

# Addresses
ADDRESSES = [
    "cagpMvZf6Z8GrnAPZdQgXrCqBvCFmQYfJi",  # 0 - 100B CALL
    "cGMbLyeUsUpjTtMFcCWZDVA4hdMXpjvPi3",  # 1 - 50B CALL
    "cadFKa9yk1Fv4PBj2DeZfYW48APXj4L8g",   # 2 - 25B CALL
    "cLxud1QujAfeYszuY6DBwBhGE4fQ3GhKS5",  # 3 - 10B CALL
    "cGUQS4suRWMibEGCz46xxMVw45yB3h5Pio",  # 4 - 5B CALL
]

print(f"Using calld: {CALLD_PATH}")
print(f"Script dir: {SCRIPT_DIR}")

passed = 0
failed = 0

# Test 1: Payment
print("\n" + "=" * 50)
print("TEST 1: Payment")
print("=" * 50)
payment_tx = {
    "TransactionType": "Payment",
    "Account": ADDRESSES[0],
    "Destination": ADDRESSES[1],
    "Amount": "1000000",
    "Sequence": 1,
    "Fee": "10"
}
tx_blob = sign_local(PRIVATE_KEYS[0], payment_tx)
if tx_blob:
    print(f"Signed successfully (blob length: {len(tx_blob)})")
    result = submit_tx(tx_blob)
    if result and "result" in result:
        print(f"✓ Payment submitted successfully")
        passed += 1
    else:
        error = result.get("error", {}).get("message", "Unknown error") if result else "No response"
        print(f"✗ Submit failed: {error}")
        failed += 1
else:
    print("✗ Signing failed")
    failed += 1

# Test 2: AccountSet
print("\n" + "=" * 50)
print("TEST 2: AccountSet")
print("=" * 50)
accountset_tx = {
    "TransactionType": "AccountSet",
    "Account": ADDRESSES[2],
    "Domain": "6578616D706C652E636F6D",  # "example.com" in hex
    "Sequence": 1,
    "Fee": "10"
}
tx_blob = sign_local(PRIVATE_KEYS[2], accountset_tx)
if tx_blob:
    print(f"Signed successfully (blob length: {len(tx_blob)})")
    result = submit_tx(tx_blob)
    if result and "result" in result:
        print(f"✓ AccountSet submitted successfully")
        passed += 1
    else:
        error = result.get("error", {}).get("message", "Unknown error") if result else "No response"
        print(f"✗ Submit failed: {error}")
        failed += 1
else:
    print("✗ Signing failed")
    failed += 1

# Test 3: TrustSet
print("\n" + "=" * 50)
print("TEST 3: TrustSet")
print("=" * 50)
trustset_tx = {
    "TransactionType": "TrustSet",
    "Account": ADDRESSES[3],
    "LimitAmount": {
        "currency": "USD",
        "issuer": ADDRESSES[0],
        "value": "1000"
    },
    "Sequence": 1,
    "Fee": "10"
}
tx_blob = sign_local(PRIVATE_KEYS[3], trustset_tx)
if tx_blob:
    print(f"Signed successfully (blob length: {len(tx_blob)})")
    result = submit_tx(tx_blob)
    if result and "result" in result:
        print(f"✓ TrustSet submitted successfully")
        passed += 1
    else:
        error = result.get("error", {}).get("message", "Unknown error") if result else "No response"
        print(f"✗ Submit failed: {error}")
        failed += 1
else:
    print("✗ Signing failed")
    failed += 1

# Test 4: OfferCreate
print("\n" + "=" * 50)
print("TEST 4: OfferCreate")
print("=" * 50)
offercreate_tx = {
    "TransactionType": "OfferCreate",
    "Account": ADDRESSES[4],
    "TakerPays": "5000000",  # 5 CALL
    "TakerGets": {
        "currency": "USD",
        "issuer": ADDRESSES[0],
        "value": "100"
    },
    "Sequence": 1,
    "Fee": "10"
}
tx_blob = sign_local(PRIVATE_KEYS[4], offercreate_tx)
if tx_blob:
    print(f"Signed successfully (blob length: {len(tx_blob)})")
    result = submit_tx(tx_blob)
    if result and "result" in result:
        print(f"✓ OfferCreate submitted successfully")
        passed += 1
    else:
        error = result.get("error", {}).get("message", "Unknown error") if result else "No response"
        print(f"✗ Submit failed: {error}")
        failed += 1
else:
    print("✗ Signing failed")
    failed += 1

# Test 5: SetRegularKey
print("\n" + "=" * 50)
print("TEST 5: SetRegularKey")
print("=" * 50)
setregularkey_tx = {
    "TransactionType": "SetRegularKey",
    "Account": ADDRESSES[1],
    "RegularKey": ADDRESSES[2],
    "Sequence": 1,
    "Fee": "10"
}
tx_blob = sign_local(PRIVATE_KEYS[1], setregularkey_tx)
if tx_blob:
    print(f"Signed successfully (blob length: {len(tx_blob)})")
    result = submit_tx(tx_blob)
    if result and "result" in result:
        print(f"✓ SetRegularKey submitted successfully")
        passed += 1
    else:
        error = result.get("error", {}).get("message", "Unknown error") if result else "No response"
        print(f"✗ Submit failed: {error}")
        failed += 1
else:
    print("✗ Signing failed")
    failed += 1

print("\n" + "=" * 50)
print(f"RESULTS: {passed} passed, {failed} failed")
print("=" * 50)
