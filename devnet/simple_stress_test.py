#!/usr/bin/env python3
"""
Call-Core Devnet Simple Stress Test

Tests RPC endpoints (ping, server_info, ledger_current).
No transaction submission - for basic connectivity testing.

Usage:
    python3 simple_stress_test.py [--url URL] [--iterations N]

Example:
    python3 simple_stress_test.py --url http://localhost:5005 --iterations 100
"""

import argparse
import json
import sys
import time
import urllib.request
import urllib.error


def rpc_call(url: str, method: str, params=None):
    """Make an RPC call to the node"""
    payload = {
        "jsonrpc": "2.0",
        "method": method,
        "id": 1
    }
    if params:
        payload["params"] = params

    data = json.dumps(payload).encode('utf-8')
    req = urllib.request.Request(
        url,
        data=data,
        headers={'Content-Type': 'application/json'},
        method='POST'
    )

    try:
        with urllib.request.urlopen(req, timeout=10) as response:
            return json.loads(response.read().decode('utf-8'))
    except urllib.error.URLError as e:
        return {"error": f"Connection failed: {e}"}
    except Exception as e:
        return {"error": str(e)}


def test_endpoint(url: str, name: str, method: str, params=None):
    """Test a single RPC endpoint"""
    start = time.time()
    response = rpc_call(url, method, params)
    elapsed = (time.time() - start) * 1000

    if "error" in response:
        print(f"  {name:20} FAILED ({elapsed:.2f}ms): {response['error']}")
        return False
    else:
        print(f"  {name:20} OK ({elapsed:.2f}ms)")
        return True


def main():
    parser = argparse.ArgumentParser(description="Call-Core Devnet Simple Stress Test")
    parser.add_argument("--url", default="http://localhost:5005", help="RPC endpoint URL")
    parser.add_argument("--iterations", type=int, default=10, help="Test iterations")
    args = parser.parse_args()

    print(f"\n{'='*60}")
    print("Call-Core Devnet Simple Stress Test")
    print(f"{'='*60}")
    print(f"Target URL: {args.url}")
    print(f"Iterations: {args.iterations}")
    print(f"{'='*60}\n")

    # Test basic connectivity
    print("Testing basic connectivity...")
    result = rpc_call(args.url, "ping")
    if "error" in result:
        print(f"  ERROR: Node not responding - {result['error']}")
        print(f"\n  Make sure the devnet is running:")
        print(f"    ./devnet/devnet-up.sh start")
        sys.exit(1)
    print(f"  ping: {result.get('result', 'N/A')}")

    # Get server info
    print("\nGetting server info...")
    info = rpc_call(args.url, "server_info")
    if "error" not in info:
        # Handle both nested and flat response formats
        result = info.get("result", info)
        print(f"  Server State: {result.get('server_state', result.get('state', 'N/A'))}")
        print(f"  Ledger Seq: {result.get('ledger_seq', result.get('ledger_index', 'N/A'))}")
        print(f"  Complete Ledgers: {result.get('complete_ledgers', 'N/A')}")
        print(f"  Peers: {result.get('peers', result.get('peer_count', 'N/A'))}")
    else:
        print(f"  ERROR: {info.get('error', 'Unknown error')}")

    # Get current ledger
    print("\nGetting current ledger...")
    ledger = rpc_call(args.url, "ledger_current")
    if "error" not in ledger:
        # Handle both nested and flat response formats
        result = ledger.get("result", ledger)
        print(f"  Ledger Index: {result.get('ledger_index', result.get('ledger_seq', 'N/A'))}")
        ledger_hash = result.get('ledger_hash', 'N/A')
        if ledger_hash != 'N/A' and len(str(ledger_hash)) > 16:
            print(f"  Ledger Hash: {str(ledger_hash)[:16]}...")
        else:
            print(f"  Ledger Hash: {ledger_hash}")
    else:
        print(f"  ERROR: {ledger.get('error', 'Unknown error')}")

    # Run stress tests
    print(f"\nRunning stress test ({args.iterations} iterations)...")
    print(f"{'='*60}")

    results = {
        "ping": {"success": 0, "failed": 0, "times": []},
        "server_info": {"success": 0, "failed": 0, "times": []},
        "ledger_current": {"success": 0, "failed": 0, "times": []},
    }

    for i in range(args.iterations):
        # Test ping
        start = time.time()
        result = rpc_call(args.url, "ping")
        elapsed = (time.time() - start) * 1000
        results["ping"]["times"].append(elapsed)
        if "error" not in result:
            results["ping"]["success"] += 1
        else:
            results["ping"]["failed"] += 1

        # Test server_info
        start = time.time()
        result = rpc_call(args.url, "server_info")
        elapsed = (time.time() - start) * 1000
        results["server_info"]["times"].append(elapsed)
        if "error" not in result:
            results["server_info"]["success"] += 1
        else:
            results["server_info"]["failed"] += 1

        # Test ledger_current
        start = time.time()
        result = rpc_call(args.url, "ledger_current")
        elapsed = (time.time() - start) * 1000
        results["ledger_current"]["times"].append(elapsed)
        if "error" not in result:
            results["ledger_current"]["success"] += 1
        else:
            results["ledger_current"]["failed"] += 1

        if (i + 1) % 10 == 0:
            print(f"  Completed {i + 1}/{args.iterations} iterations...")

    # Print results
    print(f"\n{'='*60}")
    print("Stress Test Results")
    print(f"{'='*60}")

    total_requests = args.iterations * 3
    total_success = sum(r["success"] for r in results.values())
    total_failed = sum(r["failed"] for r in results.values())

    print(f"\nTotal Requests: {total_requests}")
    print(f"Successful: {total_success} ({100*total_success/total_requests:.1f}%)")
    print(f"Failed: {total_failed} ({100*total_failed/total_requests:.1f}%)")

    print(f"\nResponse Times by Endpoint:")
    print(f"  {'Endpoint':<20} {'Avg (ms)':>10} {'Min (ms)':>10} {'Max (ms)':>10}")
    print(f"  {'-'*54}")

    for endpoint, data in results.items():
        if data["times"]:
            avg_time = sum(data["times"]) / len(data["times"])
            min_time = min(data["times"])
            max_time = max(data["times"])
            print(f"  {endpoint:<20} {avg_time:>10.2f} {min_time:>10.2f} {max_time:>10.2f}")

    # Calculate throughput
    total_time = sum(sum(r["times"]) for r in results.values()) / 1000  # seconds
    if total_time > 0:
        rps = total_requests / total_time
        print(f"\nThroughput: {rps:.2f} requests/second")

    print(f"\n{'='*60}")

    # Test ledger_accept if available
    print("\nTesting ledger_accept (manual ledger close)...")
    accept_result = rpc_call(args.url, "ledger_accept")
    if "error" not in accept_result:
        result = accept_result.get("result", accept_result)
        ledger_idx = result.get('ledger_current_index', result.get('ledger_index', 'N/A'))
        print(f"  ledger_accept: success (ledger: {ledger_idx})")
    else:
        print(f"  ledger_accept: Not available or failed ({accept_result.get('error', 'Unknown')})")

    print(f"\n{'='*60}")

    if total_failed > 0:
        print("\nWARNING: Some requests failed!")
        return 1

    print("\nAll tests passed!")
    return 0


if __name__ == "__main__":
    sys.exit(main())
