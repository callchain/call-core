#!/usr/bin/env python3
"""
Test that RPC and WebSocket API responses contain real data, not dummy values.
"""

import json
import subprocess
import sys
import asyncio
import websockets


def rpc_call(method, params=None):
    """Make RPC call and return result"""
    payload = {"jsonrpc": "2.0", "method": method, "id": 1}
    if params:
        payload["params"] = params

    cmd = [
        "curl", "-s", "-X", "POST", "http://localhost:5005",
        "-H", "Content-Type: application/json",
        "-d", json.dumps(payload),
        "-m", "10"
    ]
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode == 0:
        try:
            return json.loads(result.stdout).get("result", {})
        except:
            return {}
    return {}


def check_not_empty(value, path, errors):
    """Check that value is not empty/dummy"""
    if value is None:
        errors.append(f"{path}: is None")
        return False
    if isinstance(value, str) and value.strip() == "":
        errors.append(f"{path}: is empty string")
        return False
    if isinstance(value, list) and len(value) == 0:
        errors.append(f"{path}: is empty list")
        return False
    if isinstance(value, dict) and len(value) == 0:
        errors.append(f"{path}: is empty dict")
        return False
    # Check for dummy zero values
    if isinstance(value, (int, float)) and value == 0 and "index" not in path.lower() and "time" not in path.lower() and "fee" not in path.lower():
        errors.append(f"{path}: is zero (possibly dummy)")
        return False
    return True


def test_server_info():
    """Test server_info has real data"""
    print("\n=== Testing server_info ===")
    result = rpc_call("server_info")
    info = result.get("info", {})
    errors = []

    # Check critical fields
    check_not_empty(info.get("build_version"), "build_version", errors)
    check_not_empty(info.get("complete_ledgers"), "complete_ledgers", errors)
    check_not_empty(info.get("server_state"), "server_state", errors)
    # Note: peers can be 0 on devnet with single node, so we just check field exists
    if "peers" not in info:
        errors.append("peers field missing")

    # Check for dummy node_id (all zeros)
    node_id = info.get("node_id", "")
    if node_id and all(c in "0" for c in node_id.replace("0x", "")):
        errors.append(f"node_id is all zeros (dummy): {node_id}")

    # Check validation_quorum is configured
    if info.get("validation_quorum", 0) == 0:
        errors.append("validation_quorum is 0 or missing")

    if errors:
        print("ERRORS:")
        for e in errors:
            print(f"  - {e}")
        return False

    print(f"✓ build_version: {info.get('build_version')}")
    print(f"✓ server_state: {info.get('server_state')}")
    print(f"✓ peers: {info.get('peers')}")
    print(f"✓ validation_quorum: {info.get('validation_quorum')}")
    return True


def test_ledger_methods():
    """Test ledger methods have real data"""
    print("\n=== Testing Ledger Methods ===")
    errors = []

    # ledger_current
    result = rpc_call("ledger_current")
    if result.get("ledger_current_index", 0) == 0:
        errors.append("ledger_current_index is 0 (dummy)")

    # ledger_closed
    result = rpc_call("ledger_closed")
    hash_val = result.get("ledger_hash", "")
    if not hash_val or all(c in "0" for c in hash_val):
        errors.append("ledger_hash is empty or all zeros")

    # ledger_data
    result = rpc_call("ledger_data", {"limit": 10})
    if "ledger" not in result:
        errors.append("ledger_data missing 'ledger' field")

    if errors:
        print("ERRORS:")
        for e in errors:
            print(f"  - {e}")
        return False

    print(f"✓ ledger_current_index: {rpc_call('ledger_current').get('ledger_current_index')}")
    print(f"✓ ledger_closed hash: {rpc_call('ledger_closed').get('ledger_hash', '')[:20]}...")
    return True


def test_account_info():
    """Test account_info has real data"""
    print("\n=== Testing account_info ===")
    result = rpc_call("account_info", {"account": "cGmJBrEfFssWuas4kCoHTX5r6aMEf6QHhy"})
    errors = []

    account_data = result.get("account_data", {})

    # Check balance
    balance = account_data.get("Balance", "")
    if not balance or balance == "0":
        errors.append(f"Balance is empty or zero: {balance}")

    # Check sequence
    sequence = account_data.get("Sequence", 0)
    if sequence == 0:
        errors.append(f"Sequence is 0 (dummy): {sequence}")

    # Check account ID
    account = account_data.get("Account", "")
    if not account or all(c in "0" for c in account):
        errors.append(f"Account is empty or all zeros: {account}")

    if errors:
        print("ERRORS:")
        for e in errors:
            print(f"  - {e}")
        return False

    print(f"✓ Balance: {balance}")
    print(f"✓ Sequence: {sequence}")
    print(f"✓ Account: {account[:20]}...")
    return True


def test_network_info():
    """Test network_info has real data"""
    print("\n=== Testing network_info ===")
    result = rpc_call("network_info")
    errors = []

    network = result.get("network", {})

    # Check peer_count exists (could be 0 on devnet)
    if "peer_count" not in network:
        errors.append("peer_count field missing")

    # Check listen_address
    listen = network.get("listen_address", "")
    if not listen or listen == "127.0.0.1:0":
        errors.append(f"listen_address is empty or invalid: {listen}")

    if errors:
        print("ERRORS:")
        for e in errors:
            print(f"  - {e}")
        return False

    print(f"✓ peer_count: {network.get('peer_count')}")
    print(f"✓ listen_address: {listen}")
    return True


def test_fee():
    """Test fee has real data"""
    print("\n=== Testing fee ===")
    result = rpc_call("fee")
    errors = []

    # Check fee fields exist and are reasonable
    # Note: call-core uses "droplets" not "drops"
    base_fee = result.get("fee_base", 0)

    if base_fee == 0:
        errors.append(f"fee_base is 0: {result}")

    if errors:
        print("ERRORS:")
        for e in errors:
            print(f"  - {e}")
        return False

    print(f"✓ base_fee: {base_fee}")
    return True


def test_consensus_info():
    """Test consensus_info has real data"""
    print("\n=== Testing consensus_info ===")
    result = rpc_call("consensus_info")
    errors = []

    # Check consensus phase (result is in "consensus" object)
    consensus = result.get("consensus", {})
    phase = consensus.get("phase", "")
    if not phase or phase == "unknown":
        errors.append(f"consensus.phase is empty or unknown: {phase}")

    # Check ledger index
    ledger_index = consensus.get("ledger_index", 0)
    if ledger_index == 0:
        errors.append(f"consensus.ledger_index is 0: {ledger_index}")

    if errors:
        print("ERRORS:")
        for e in errors:
            print(f"  - {e}")
        return False

    print(f"✓ consensus_phase: {phase}")
    print(f"✓ ledger_index: {ledger_index}")
    return True


async def test_websocket_content():
    """Test WebSocket responses have real data"""
    print("\n=== Testing WebSocket Content ===")
    errors = []

    try:
        async with websockets.connect("ws://localhost:6005", ping_interval=None) as ws:
            # Test server_info
            await ws.send(json.dumps({"command": "server_info", "id": 1}))
            response = await asyncio.wait_for(ws.recv(), timeout=5.0)
            data = json.loads(response)

            if data.get("status") != "success":
                errors.append(f"server_info failed: {data}")
            else:
                result = data.get("result", {})
                if "build_version" not in result:
                    errors.append("server_info missing build_version")
                # ledger_index might be 0 initially, that's OK

            # Test ledger
            await ws.send(json.dumps({"command": "ledger", "id": 2}))
            response = await asyncio.wait_for(ws.recv(), timeout=5.0)
            data = json.loads(response)

            if data.get("status") != "success":
                errors.append(f"ledger failed: {data}")
            else:
                result = data.get("result", {})
                if "ledger_index" not in result:
                    errors.append("ledger missing ledger_index")

    except Exception as e:
        errors.append(f"WebSocket error: {e}")

    if errors:
        print("ERRORS:")
        for e in errors:
            print(f"  - {e}")
        return False

    print("✓ WebSocket server_info has real data")
    print("✓ WebSocket ledger has real data")
    return True


def main():
    print("=" * 60)
    print("API Content Validation Test")
    print("=" * 60)

    all_passed = True

    all_passed &= test_server_info()
    all_passed &= test_ledger_methods()
    all_passed &= test_account_info()
    all_passed &= test_network_info()
    all_passed &= test_fee()
    all_passed &= test_consensus_info()

    # WebSocket tests
    ws_passed = asyncio.run(test_websocket_content())
    all_passed &= ws_passed

    print("\n" + "=" * 60)
    if all_passed:
        print("✓ All API responses contain real data!")
        return 0
    else:
        print("✗ Some APIs return dummy/empty data")
        return 1


if __name__ == "__main__":
    sys.exit(main())
