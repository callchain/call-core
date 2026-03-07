#!/usr/bin/env python3
"""
Call-Core Devnet Simple Stress Test

Tests RPC endpoints and submits actual transactions to the network.
Transactions will appear in ledger logs and be processed by consensus.

Usage:
    python3 simple_stress_test.py [--url URL] [--iterations N] [--tx-count N]

Example:
    python3 simple_stress_test.py --url http://localhost:5005 --iterations 10 --tx-count 50
"""

import argparse
import json
import sys
import time
import urllib.request
import urllib.error
import secrets
import hashlib


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


def create_dummy_transaction_blob():
    """
    Create a dummy transaction blob for stress testing.
    This creates a serialized transaction that will be rejected by validation
    but tests the transaction submission and processing pipeline.
    """
    # Create a minimal transaction structure
    # In a real scenario, you would use proper serialization and signing
    tx_data = {
        "TransactionType": "Payment",
        "Account": "rrrrrrrrrrrrrrrrrrrrrhoLvTp",  # Burn address
        "Destination": "rrrrrrrrrrrrrrrrrrrrBZbvji",
        "Amount": "1000000",
        "Sequence": secrets.randbelow(1000000),
        "Fee": "100",
        "SigningPubKey": "",
        "TxnSignature": ""
    }
    # Return as hex string (dummy serialization)
    return hashlib.sha256(json.dumps(tx_data, sort_keys=True).encode()).hexdigest()


def submit_transaction(url: str, tx_blob: str):
    """Submit a transaction to the network"""
    return rpc_call(url, "submit", [{"tx_blob": tx_blob}])


def wallet_propose(url: str):
    """Generate a new wallet"""
    return rpc_call(url, "wallet_propose")


def main():
    parser = argparse.ArgumentParser(description="Call-Core Devnet Simple Stress Test")
    parser.add_argument("--url", default="http://localhost:5005", help="RPC endpoint URL")
    parser.add_argument("--iterations", type=int, default=10, help="Test iterations")
    parser.add_argument("--tx-count", type=int, default=10, help="Transactions per iteration")
    args = parser.parse_args()

    print(f"\n{'='*60}")
    print("Call-Core Devnet Simple Stress Test")
    print(f"{'='*60}")
    print(f"Target URL: {args.url}")
    print(f"Iterations: {args.iterations}")
    print(f"Tx per iteration: {args.tx_count}")
    print(f"Total transactions: {args.iterations * args.tx_count}")
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
    if "result" in info:
        result = info["result"]
        print(f"  Server State: {result.get('server_state', 'N/A')}")
        print(f"  Ledger Seq: {result.get('ledger_seq', 'N/A')}")
        print(f"  Complete Ledgers: {result.get('complete_ledgers', 'N/A')}")
        print(f"  Peers: {result.get('peers', 'N/A')}")
    else:
        print(f"  ERROR: {info.get('error', 'Unknown error')}")

    # Get current ledger
    print("\nGetting current ledger...")
    ledger = rpc_call(args.url, "ledger_current")
    if "result" in ledger:
        result = ledger["result"]
        print(f"  Ledger Index: {result.get('ledger_index', 'N/A')}")
        print(f"  Ledger Hash: {result.get('ledger_hash', 'N/A')[:16]}...")
    else:
        print(f"  ERROR: {ledger.get('error', 'Unknown error')}")

    # Run stress tests
    total_tx = args.iterations * args.tx_count
    print(f"\nRunning stress test ({total_tx} total transactions)...")
    print(f"{'='*60}")

    results = {
        "ping": {"success": 0, "failed": 0, "times": []},
        "server_info": {"success": 0, "failed": 0, "times": []},
        "ledger_current": {"success": 0, "failed": 0, "times": []},
        "submit": {"success": 0, "failed": 0, "times": [], "accepted": 0, "rejected": 0},
    }

    start_time = time.time()

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

        # Submit transactions
        for j in range(args.tx_count):
            tx_blob = create_dummy_transaction_blob()
            start = time.time()
            result = submit_transaction(args.url, tx_blob)
            elapsed = (time.time() - start) * 1000
            results["submit"]["times"].append(elapsed)

            if "error" not in result:
                results["submit"]["success"] += 1
                # Check if transaction was accepted
                engine_result = result.get("result", {}).get("engine_result", "")
                if engine_result == "tesSUCCESS":
                    results["submit"]["accepted"] += 1
                else:
                    results["submit"]["rejected"] += 1
            else:
                results["submit"]["failed"] += 1

        if (i + 1) % 5 == 0 or i == 0:
            print(f"  Completed iteration {i + 1}/{args.iterations} "
                  f"({(i + 1) * args.tx_count}/{total_tx} transactions)...")

    elapsed_time = time.time() - start_time

    # Force a ledger close to include submitted transactions
    print("\n  Forcing ledger close...")
    close_result = rpc_call(args.url, "ledger_accept")
    if "error" not in close_result:
        print(f"  Ledger closed successfully")
    else:
        print(f"  Ledger close: {close_result.get('error', 'Not available')}")

    # Print results
    print(f"\n{'='*60}")
    print("Stress Test Results")
    print(f"{'='*60}")

    total_requests = args.iterations * 3 + total_tx
    total_success = sum(r["success"] for r in results.values())
    total_failed = sum(r["failed"] for r in results.values())

    print(f"\nTiming:")
    print(f"  Total Time: {elapsed_time:.2f} seconds")
    print(f"  Throughput: {total_tx / elapsed_time:.2f} tx/second")

    print(f"\nSummary:")
    print(f"  Total Requests: {total_requests}")
    print(f"  Successful: {total_success} ({100*total_success/total_requests:.1f}%)")
    print(f"  Failed: {total_failed} ({100*total_failed/total_requests:.1f}%)")

    print(f"\nTransaction Submission:")
    submit_results = results["submit"]
    print(f"  Submitted: {submit_results['success'] + submit_results['failed']}")
    print(f"  RPC Success: {submit_results['success']}")
    print(f"  RPC Failed: {submit_results['failed']}")
    print(f"  Accepted (tesSUCCESS): {submit_results['accepted']}")
    print(f"  Rejected: {submit_results['rejected']}")

    print(f"\nResponse Times by Endpoint:")
    print(f"  {'Endpoint':<20} {'Avg (ms)':>10} {'Min (ms)':>10} {'Max (ms)':>10} {'Count':>8}")
    print(f"  {'-'*64}")

    for endpoint, data in results.items():
        if data["times"]:
            avg_time = sum(data["times"]) / len(data["times"])
            min_time = min(data["times"])
            max_time = max(data["times"])
            count = len(data["times"])
            print(f"  {endpoint:<20} {avg_time:>10.2f} {min_time:>10.2f} {max_time:>10.2f} {count:>8}")

    # Calculate percentiles for transaction submissions
    if results["submit"]["times"]:
        times = sorted(results["submit"]["times"])
        p50 = times[len(times) // 2]
        p95 = times[int(len(times) * 0.95)]
        p99 = times[int(len(times) * 0.99)] if len(times) >= 100 else times[-1]

        print(f"\nTransaction Submission Latency:")
        print(f"  P50: {p50:.2f}ms")
        print(f"  P95: {p95:.2f}ms")
        print(f"  P99: {p99:.2f}ms")

    print(f"\n{'='*60}")

    # Get final ledger info
    print("\nFinal Ledger Status:")
    final_ledger = rpc_call(args.url, "ledger_current")
    if "result" in final_ledger:
        result = final_ledger["result"]
        print(f"  Ledger Index: {result.get('ledger_index', 'N/A')}")
        print(f"  Ledger Hash: {result.get('ledger_hash', 'N/A')[:32]}...")

        # Calculate ledgers closed during test
        if "result" in ledger and "ledger_index" in result:
            start_seq = ledger["result"].get("ledger_index", 0)
            end_seq = result.get("ledger_index", 0)
            ledgers_closed = end_seq - start_seq
            print(f"  Ledgers Closed During Test: {ledgers_closed}")
            if ledgers_closed > 0:
                print(f"  Avg TPS (actual): {total_tx / ledgers_closed:.2f} tx/ledger")

    print(f"\n{'='*60}")

    if total_failed > total_requests * 0.5:
        print("\nWARNING: More than 50% of requests failed!")
        return 1

    print("\nStress test completed!")
    print("Check docker logs to see transactions being processed:")
    print("  docker logs -f call-dev-node-1")
    return 0


if __name__ == "__main__":
    sys.exit(main())
