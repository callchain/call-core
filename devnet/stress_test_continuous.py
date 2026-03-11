#!/usr/bin/env python3
"""
Call-Core Continuous Sequence Stress Test

This test uses continuous sequences (1, 2, 3, ...) per account for maximum
success rates. Each worker gets its own dedicated account, ensuring no
sequence gaps that cause PRE_SEQ errors.

Usage:
    python3 stress_test_continuous.py [--url URL] [--count COUNT] [--threads THREADS]

Example:
    python3 stress_test_continuous.py --url http://localhost:5005 --count 1000 --threads 4
"""

import argparse
import json
import sys
import time
import threading
from dataclasses import dataclass, field
from typing import List, Dict, Optional
import subprocess
import os
from urllib.request import urlopen, Request
from urllib.error import URLError


# Genesis wallet information - one per thread for continuous sequences
GENESIS_WALLETS = [
    {
        "name": "Genesis Account 1",
        "seed": "ssyB7KxAvfRwQ6mseEjt3iY1qeqMC",
        "address": "cGmJBrEfFssWuas4kCoHTX5r6aMEf6QHhy",
    },
    {
        "name": "Genesis Account 2",
        "seed": "snTHcoLdd2vwrCmNunkMZRhbfLuLs",
        "address": "c3K3xXhvsWBnP3TitQfeg2ihAuaYybvtc7",
    },
    {
        "name": "Genesis Account 3",
        "seed": "shCQseDN6PMeeBK31bCjhQqYY4LrG",
        "address": "cHSFoKcGXFZdbB7EKmWQMTUJbr66dwfMR1",
    },
    {
        "name": "Genesis Account 4",
        "seed": "shiWcE5Y2DJqZ2X2oLCrqYmovqz45",
        "address": "cKKeufyrSZymFeGmtF1Vhi11eCSf2i6MhR",
    },
]


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
        if os.path.exists(path) or subprocess.run(["which", path], capture_output=True).returncode == 0:
            calld_sign = path
            break

    if not calld_sign:
        result = subprocess.run(["which", "calld-sign"], capture_output=True, text=True)
        if result.returncode == 0:
            calld_sign = result.stdout.strip()

    if not calld_sign:
        print("[ERROR] calld-sign binary not found")
        return None

    tx_json_str = json.dumps(tx_json)
    cmd = [calld_sign, "--secret", secret, "--tx-json", tx_json_str, "--format", "blob"]

    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=10)
        if result.returncode != 0:
            print(f"[SIGN ERROR] {result.stderr.strip()}")
            return None
        return result.stdout.strip() or None
    except Exception as e:
        print(f"[SIGN ERROR] {e}")
        return None


@dataclass
class WorkerResult:
    """Result from a single worker thread"""
    worker_id: int
    account: str
    success_count: int = 0
    fail_count: int = 0
    total_time: float = 0.0
    errors: List[str] = field(default_factory=list)


class ContinuousSequenceStressTest:
    """Stress test with continuous sequences per account"""

    def __init__(self, url: str, iterations: int, threads: int):
        self.url = url
        self.iterations = iterations
        self.threads = min(threads, len(GENESIS_WALLETS))
        self.results: List[WorkerResult] = []
        self.lock = threading.Lock()

    def _submit_transaction(self, tx_blob: str) -> bool:
        """Submit a transaction to the node"""
        payload = {
            "jsonrpc": "2.0",
            "method": "submit",
            "params": [{"tx_blob": tx_blob}],
            "id": 1
        }

        try:
            req = Request(
                self.url,
                data=json.dumps(payload).encode(),
                headers={"Content-Type": "application/json"},
                method="POST"
            )
            with urlopen(req, timeout=10) as response:
                result = json.loads(response.read().decode())
                return result.get("result", {}).get("status") == "success"
        except Exception as e:
            print(f"[SUBMIT ERROR] {e}")
            return False

    def worker(self, worker_id: int, wallet: Dict):
        """Worker thread with continuous sequences"""
        result = WorkerResult(worker_id=worker_id, account=wallet["address"])
        sequence = 1  # Start from 1, continuous

        print(f"  Worker {worker_id}: Starting {self.iterations} iterations for {wallet['name']} ({wallet['address'][:20]}...)")
        start_time = time.time()

        for i in range(self.iterations):
            # Simple Payment transaction (most reliable)
            tx_json = {
                "TransactionType": "Payment",
                "Account": wallet["address"],
                "Destination": "c3K3xXhvsWBnP3TitQfeg2ihAuaYybvtc7",
                "Amount": "1000000",
                "Sequence": sequence,
                "Fee": "10"
            }

            tx_blob = sign_with_calld_sign(wallet["seed"], tx_json)
            if not tx_blob:
                result.fail_count += 1
                result.errors.append(f"Failed to sign seq {sequence}")
                sequence += 1
                continue

            if self._submit_transaction(tx_blob):
                result.success_count += 1
            else:
                result.fail_count += 1
                result.errors.append(f"Failed to submit seq {sequence}")

            sequence += 1

            # Progress report every 100 transactions
            if (i + 1) % 100 == 0:
                print(f"  Worker {worker_id}: {i + 1}/{self.iterations} complete")

        result.total_time = time.time() - start_time

        with self.lock:
            self.results.append(result)

    def run(self):
        """Run the stress test"""
        print("=" * 60)
        print("CONTINUOUS SEQUENCE STRESS TEST")
        print("=" * 60)
        print(f"Target: {self.url}")
        print(f"Threads: {self.threads} (one per account)")
        print(f"Iterations per thread: {self.iterations}")
        print(f"Total transactions: {self.iterations * self.threads}")
        print(f"Sequence pattern: Continuous (1, 2, 3, ...)")
        print("")

        threads = []
        for i in range(self.threads):
            wallet = GENESIS_WALLETS[i]
            t = threading.Thread(target=self.worker, args=(i, wallet))
            threads.append(t)
            t.start()

        for t in threads:
            t.join()

        # Summary
        print("")
        print("=" * 60)
        print("RESULTS")
        print("=" * 60)

        total_success = sum(r.success_count for r in self.results)
        total_fail = sum(r.fail_count for r in self.results)
        total_time = sum(r.total_time for r in self.results)
        total_tx = total_success + total_fail

        print(f"\nTotal Transactions: {total_tx}")
        print(f"Successful: {total_success} ({100*total_success/total_tx:.1f}%)")
        print(f"Failed: {total_fail} ({100*total_fail/total_tx:.1f}%)")
        print(f"Total Time: {total_time:.2f} seconds")
        print(f"Throughput: {total_tx/total_time:.2f} tx/second")

        print("\nPer-Worker Results:")
        print("-" * 60)
        for r in self.results:
            total = r.success_count + r.fail_count
            rate = 100 * r.success_count / total if total > 0 else 0
            print(f"  Worker {r.worker_id} ({r.account[:20]}...):")
            print(f"    Success: {r.success_count}/{total} ({rate:.1f}%)")
            print(f"    Time: {r.total_time:.2f}s")
            if r.errors and len(r.errors) <= 3:
                for e in r.errors[:3]:
                    print(f"    Error: {e}")
            elif r.errors:
                print(f"    Errors: {len(r.errors)} (first 3 shown)")
                for e in r.errors[:3]:
                    print(f"      {e}")

        print("")
        if total_fail == 0:
            print("SUCCESS: All transactions submitted successfully!")
            return 0
        else:
            print(f"PARTIAL: {total_fail} transactions failed")
            return 1


def main():
    parser = argparse.ArgumentParser(description="Continuous Sequence Stress Test")
    parser.add_argument("--url", default="http://localhost:5005", help="Node RPC URL")
    parser.add_argument("--count", type=int, default=1000, help="Iterations per thread")
    parser.add_argument("--threads", type=int, default=4, help="Number of threads (max 4)")

    args = parser.parse_args()

    test = ContinuousSequenceStressTest(args.url, args.count, args.threads)
    return test.run()


if __name__ == "__main__":
    sys.exit(main())
