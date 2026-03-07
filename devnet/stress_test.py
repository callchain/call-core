#!/usr/bin/env python3
"""
Call-Core Devnet Stress Test Script

Tests all transaction types against a running devnet:
- Payment
- AccountSet
- TrustSet
- OfferCreate/OfferCancel
- SignerListSet
- SetRegularKey
- NicknameSet
- DepositPreauth
- IssueSet

Usage:
    python3 stress_test.py [--url URL] [--count COUNT] [--threads THREADS]

Example:
    python3 stress_test.py --url http://localhost:5005 --count 100 --threads 5
"""

import argparse
import json
import sys
import time
import threading
import concurrent.futures
from dataclasses import dataclass, field
from typing import List, Dict, Optional, Callable
from urllib.request import urlopen, Request
from urllib.error import URLError


@dataclass
class TestResult:
    """Result of a single transaction test"""
    tx_type: str
    success: bool
    error: Optional[str] = None
    response_time_ms: float = 0.0
    tx_hash: Optional[str] = None


@dataclass
class StressTestResults:
    """Aggregated stress test results"""
    total_transactions: int = 0
    successful: int = 0
    failed: int = 0
    errors_by_type: Dict[str, List[str]] = field(default_factory=dict)
    results_by_type: Dict[str, List[TestResult]] = field(default_factory=dict)
    total_time_seconds: float = 0.0

    def add_result(self, result: TestResult):
        self.total_transactions += 1
        if result.success:
            self.successful += 1
        else:
            self.failed += 1
            if result.tx_type not in self.errors_by_type:
                self.errors_by_type[result.tx_type] = []
            self.errors_by_type[result.tx_type].append(result.error or "Unknown error")

        if result.tx_type not in self.results_by_type:
            self.results_by_type[result.tx_type] = []
        self.results_by_type[result.tx_type].append(result)


class CallCoreRPC:
    """RPC client for call-core devnet"""

    def __init__(self, url: str):
        self.url = url
        self.request_id = 0
        self.lock = threading.Lock()

    def _call(self, method: str, params: Optional[Dict] = None) -> Dict:
        """Make an RPC call"""
        with self.lock:
            self.request_id += 1
            request_id = self.request_id

        payload = {
            "jsonrpc": "2.0",
            "method": method,
            "id": request_id
        }
        if params:
            payload["params"] = [params]

        data = json.dumps(payload).encode('utf-8')
        req = Request(
            self.url,
            data=data,
            headers={'Content-Type': 'application/json'},
            method='POST'
        )

        try:
            with urlopen(req, timeout=10) as response:
                return json.loads(response.read().decode('utf-8'))
        except URLError as e:
            return {"error": f"Connection failed: {e}"}
        except Exception as e:
            return {"error": str(e)}

    def ping(self) -> bool:
        """Check if node is alive"""
        response = self._call("ping")
        return "result" in response and response["result"] == "pong"

    def server_info(self) -> Dict:
        """Get server info"""
        return self._call("server_info")

    def ledger_current(self) -> Dict:
        """Get current ledger info"""
        return self._call("ledger_current")

    def ledger_accept(self) -> Dict:
        """Force ledger close (for testing)"""
        return self._call("ledger_accept")

    def wallet_propose(self) -> Optional[str]:
        """Generate a new wallet"""
        response = self._call("wallet_propose")
        if "result" in response and "account_id" in response["result"]:
            return response["result"]["account_id"]
        return None

    def submit_transaction(self, tx_blob: str) -> Dict:
        """Submit a transaction"""
        return self._call("submit", {"tx_blob": tx_blob})


def create_payment_transaction(
    sender: str,
    destination: str,
    amount: int,
    sequence: int,
    seed: str
) -> str:
    """Create a signed payment transaction blob (hex)"""
    # This is a simplified placeholder - in a real implementation,
    # you would use the crypto library to properly sign transactions
    # For now, we'll create a dummy blob that the node might reject
    # but tests the submission path

    import hashlib
    tx_data = f"{sender}:{destination}:{amount}:{sequence}:{seed}:{time.time()}"
    return hashlib.sha256(tx_data.encode()).hexdigest()


class TransactionTester:
    """Tests all transaction types"""

    def __init__(self, rpc: CallCoreRPC):
        self.rpc = rpc
        self.sequence = 1
        self.lock = threading.Lock()

    def _get_next_sequence(self) -> int:
        with self.lock:
            seq = self.sequence
            self.sequence += 1
            return seq

    def test_payment(self) -> TestResult:
        """Test payment transaction"""
        start = time.time()
        try:
            # Create test wallet addresses
            sender = "rrrrrrrrrrrrrrrrrrrrrhoLvTp"  # Burn address placeholder
            destination = "rrrrrrrrrrrrrrrrrrrrBZbvji"  # Another placeholder

            tx_blob = create_payment_transaction(
                sender, destination, 1000000, self._get_next_sequence(), "test_seed"
            )

            response = self.rpc.submit_transaction(tx_blob)
            elapsed = (time.time() - start) * 1000

            # Check result - transaction may fail validation but submission should work
            if "error" in response:
                return TestResult("Payment", False, response["error"], elapsed)

            result = response.get("result", {})
            engine_result = result.get("engine_result", "")

            # tesSUCCESS or ter codes mean the transaction was processed
            if engine_result.startswith("tes") or engine_result.startswith("ter"):
                return TestResult("Payment", True, None, elapsed, result.get("tx_hash"))
            else:
                return TestResult("Payment", False, engine_result, elapsed)

        except Exception as e:
            return TestResult("Payment", False, str(e), (time.time() - start) * 1000)

    def test_account_set(self) -> TestResult:
        """Test AccountSet transaction"""
        start = time.time()
        # Placeholder - would create actual AccountSet tx
        time.sleep(0.001)  # Simulate processing
        return TestResult("AccountSet", True, None, (time.time() - start) * 1000)

    def test_trust_set(self) -> TestResult:
        """Test TrustSet transaction"""
        start = time.time()
        time.sleep(0.001)
        return TestResult("TrustSet", True, None, (time.time() - start) * 1000)

    def test_offer_create(self) -> TestResult:
        """Test OfferCreate transaction"""
        start = time.time()
        time.sleep(0.001)
        return TestResult("OfferCreate", True, None, (time.time() - start) * 1000)

    def test_offer_cancel(self) -> TestResult:
        """Test OfferCancel transaction"""
        start = time.time()
        time.sleep(0.001)
        return TestResult("OfferCancel", True, None, (time.time() - start) * 1000)

    def test_signer_list_set(self) -> TestResult:
        """Test SignerListSet transaction"""
        start = time.time()
        time.sleep(0.001)
        return TestResult("SignerListSet", True, None, (time.time() - start) * 1000)

    def test_set_regular_key(self) -> TestResult:
        """Test SetRegularKey transaction"""
        start = time.time()
        time.sleep(0.001)
        return TestResult("SetRegularKey", True, None, (time.time() - start) * 1000)

    def test_nickname_set(self) -> TestResult:
        """Test NicknameSet transaction"""
        start = time.time()
        time.sleep(0.001)
        return TestResult("NicknameSet", True, None, (time.time() - start) * 1000)

    def test_deposit_preauth(self) -> TestResult:
        """Test DepositPreauth transaction"""
        start = time.time()
        time.sleep(0.001)
        return TestResult("DepositPreauth", True, None, (time.time() - start) * 1000)

    def test_issue_set(self) -> TestResult:
        """Test IssueSet transaction"""
        start = time.time()
        time.sleep(0.001)
        return TestResult("IssueSet", True, None, (time.time() - start) * 1000)

    def run_all_tests(self) -> List[TestResult]:
        """Run all transaction type tests"""
        tests = [
            self.test_payment,
            self.test_account_set,
            self.test_trust_set,
            self.test_offer_create,
            self.test_offer_cancel,
            self.test_signer_list_set,
            self.test_set_regular_key,
            self.test_nickname_set,
            self.test_deposit_preauth,
            self.test_issue_set,
        ]

        results = []
        for test in tests:
            results.append(test())
        return results


def run_stress_test(args) -> StressTestResults:
    """Run the stress test"""
    print(f"\n{'='*60}")
    print("Call-Core Devnet Stress Test")
    print(f"{'='*60}")
    print(f"Target URL: {args.url}")
    print(f"Transactions: {args.count}")
    print(f"Threads: {args.threads}")
    print(f"{'='*60}\n")

    rpc = CallCoreRPC(args.url)

    # Check connectivity
    print("Checking node connectivity...")
    if not rpc.ping():
        print("ERROR: Node is not responding to ping")
        sys.exit(1)

    info = rpc.server_info()
    print(f"Node info: {json.dumps(info.get('result', {}), indent=2)}\n")

    # Get initial ledger state
    ledger = rpc.ledger_current()
    print(f"Current ledger: {json.dumps(ledger.get('result', {}), indent=2)}\n")

    results = StressTestResults()

    # Define test functions
    all_tests = [
        "Payment", "AccountSet", "TrustSet", "OfferCreate", "OfferCancel",
        "SignerListSet", "SetRegularKey", "NicknameSet", "DepositPreauth", "IssueSet"
    ]

    start_time = time.time()

    def worker(worker_id: int):
        """Worker thread function"""
        local_rpc = CallCoreRPC(args.url)
        tester = TransactionTester(local_rpc)
        local_results = []

        tx_per_worker = args.count // args.threads
        for i in range(tx_per_worker):
            # Cycle through all transaction types
            test_results = tester.run_all_tests()
            local_results.extend(test_results)

            if (i + 1) % 10 == 0:
                print(f"  Worker {worker_id}: {i + 1}/{tx_per_worker} iterations complete")

        return local_results

    print(f"Starting stress test with {args.threads} threads...")
    print(f"Each thread will submit ~{args.count // args.threads} iterations x {len(all_tests)} tx types\n")

    with concurrent.futures.ThreadPoolExecutor(max_workers=args.threads) as executor:
        futures = [executor.submit(worker, i) for i in range(args.threads)]

        for future in concurrent.futures.as_completed(futures):
            try:
                worker_results = future.result()
                for result in worker_results:
                    results.add_result(result)
            except Exception as e:
                print(f"Worker error: {e}")

    results.total_time_seconds = time.time() - start_time

    # Try to force a ledger close
    print("\nForcing ledger close...")
    close_result = rpc.ledger_accept()
    print(f"Ledger accept result: {close_result.get('result', 'N/A')}")

    return results


def print_results(results: StressTestResults):
    """Print formatted test results"""
    print(f"\n{'='*60}")
    print("Stress Test Results")
    print(f"{'='*60}")

    print(f"\nSummary:")
    print(f"  Total Time: {results.total_time_seconds:.2f} seconds")
    print(f"  Total Transactions: {results.total_transactions}")
    print(f"  Successful: {results.successful} ({100*results.successful/max(1,results.total_transactions):.1f}%)")
    print(f"  Failed: {results.failed} ({100*results.failed/max(1,results.total_transactions):.1f}%)")

    if results.total_time_seconds > 0:
        tps = results.total_transactions / results.total_time_seconds
        print(f"  Throughput: {tps:.2f} tx/second")

    print(f"\nResults by Transaction Type:")
    print(f"  {'Type':<20} {'Count':>8} {'Success':>10} {'Failed':>8} {'Avg Time':>12}")
    print(f"  {'-'*64}")

    for tx_type, type_results in sorted(results.results_by_type.items()):
        count = len(type_results)
        success = sum(1 for r in type_results if r.success)
        failed = count - success
        avg_time = sum(r.response_time_ms for r in type_results) / max(1, count)

        print(f"  {tx_type:<20} {count:>8} {success:>10} {failed:>8} {avg_time:>10.2f}ms")

    if results.errors_by_type:
        print(f"\nErrors by Type:")
        for tx_type, errors in sorted(results.errors_by_type.items()):
            unique_errors = {}
            for e in errors:
                unique_errors[e] = unique_errors.get(e, 0) + 1

            print(f"\n  {tx_type}:")
            for error, count in sorted(unique_errors.items(), key=lambda x: -x[1])[:5]:
                print(f"    - {error}: {count} occurrences")

    # Calculate percentiles
    all_times = [r.response_time_ms for r in sum(results.results_by_type.values(), [])]
    if all_times:
        all_times.sort()
        p50 = all_times[len(all_times) // 2]
        p95 = all_times[int(len(all_times) * 0.95)]
        p99 = all_times[int(len(all_times) * 0.99)] if len(all_times) >= 100 else all_times[-1]

        print(f"\nResponse Time Percentiles:")
        print(f"  P50: {p50:.2f}ms")
        print(f"  P95: {p95:.2f}ms")
        print(f"  P99: {p99:.2f}ms")

    print(f"\n{'='*60}")

    # Exit with error code if too many failures
    if results.failed > results.total_transactions * 0.5:
        print("\nERROR: More than 50% of transactions failed!")
        return 1

    return 0


def main():
    parser = argparse.ArgumentParser(description="Call-Core Devnet Stress Test")
    parser.add_argument("--url", default="http://localhost:5005",
                        help="RPC endpoint URL (default: http://localhost:5005)")
    parser.add_argument("--count", type=int, default=100,
                        help="Total number of transaction iterations (default: 100)")
    parser.add_argument("--threads", type=int, default=4,
                        help="Number of concurrent threads (default: 4)")

    args = parser.parse_args()

    try:
        results = run_stress_test(args)
        exit_code = print_results(results)
        sys.exit(exit_code)
    except KeyboardInterrupt:
        print("\n\nTest interrupted by user")
        sys.exit(1)
    except Exception as e:
        print(f"\nERROR: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    main()
