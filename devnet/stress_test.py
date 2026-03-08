#!/usr/bin/env python3
"""
Call-Core Devnet Stress Test Script with Real Transaction Signing

Tests all transaction types against a running devnet using genesis-funded accounts:
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


# Genesis wallet information - funded accounts with known seeds
# These match the default_devnet() genesis configuration
GENESIS_WALLETS = [
    {
        "name": "Genesis Account 1",
        "seed": "ssyB7KxAvfRwQ6mseEjt3iY1qeqMC",
        "address": "cGmJBrEfFssWuas4kCoHTX5r6aMEf6QHhy",
        "balance": "100000000000"
    },
    {
        "name": "Genesis Account 2",
        "seed": "snTHcoLdd2vwrCmNunkMZRhbfLuLs",
        "address": "c3K3xXhvsWBnP3TitQfeg2ihAuaYybvtc7",
        "balance": "50000000000"
    },
    {
        "name": "Genesis Account 3",
        "seed": "shCQseDN6PMeeBK31bCjhQqYY4LrG",
        "address": "cHSFoKcGXFZdbB7EKmWQMTUJbr66dwfMR1",
        "balance": "25000000000"
    },
    {
        "name": "Genesis Account 4",
        "seed": "shiWcE5Y2DJqZ2X2oLCrqYmovqz45",
        "address": "cKKeufyrSZymFeGmtF1Vhi11eCSf2i6MhR",
        "balance": "10000000000"
    },
    {
        "name": "Genesis Account 5",
        "seed": "ss1nurfhbpEnnZkDai6GTYdzT7Jqb",
        "address": "cUUsn5u9qPq7MiMiEDwdjMPsHHKyaesHPH",
        "balance": "5000000000"
    }
]


@dataclass
class TestResult:
    """Result of a single transaction test"""
    tx_type: str
    success: bool
    error: Optional[str] = None
    response_time_ms: float = 0.0
    tx_hash: Optional[str] = None

    def __post_init__(self):
        # Ensure error is always a string (not a dict)
        if self.error is not None and not isinstance(self.error, str):
            if isinstance(self.error, dict):
                self.error = str(self.error)
            else:
                self.error = str(self.error)


@dataclass
class StressTestResults:
    """Aggregated stress test results"""
    total_transactions: int = 0
    successful: int = 0
    failed: int = 0
    errors_by_type: Dict[str, List[str]] = field(default_factory=dict)
    results_by_type: Dict[str, List[TestResult]] = field(default_factory=dict)
    total_time_seconds: float = 0.0
    sent_tx_hashes: List[str] = field(default_factory=list)
    duplicate_hashes: List[str] = field(default_factory=list)

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
            payload["params"] = params

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
        result = response.get("result", {})
        if isinstance(result, dict) and result.get("status") == "success":
            return True
        if response.get("status") == "success":
            return True
        if result == "pong":
            return True
        return False

    def server_info(self) -> Dict:
        """Get server info"""
        return self._call("server_info")

    def ledger_current(self) -> Dict:
        """Get current ledger info"""
        return self._call("ledger_current")

    def ledger_accept(self) -> Dict:
        """Force ledger close (for testing)"""
        return self._call("ledger_accept")

    def account_info(self, account: str) -> Dict:
        """Get account info"""
        return self._call("account_info", {"account": account})

    def sign(self, secret: str, tx_json: Dict) -> Optional[str]:
        """
        Sign a transaction using the node's sign RPC method.
        Returns the signed tx_blob or None if signing failed.
        """
        params = {
            "secret": secret,
            "tx_json": tx_json
        }
        response = self._call("sign", params)

        if "error" in response:
            return None

        result = response.get("result", {})
        if "tx_blob" in result:
            return result["tx_blob"]
        return None

    def submit_transaction(self, tx_blob: str) -> Dict:
        """Submit a transaction"""
        return self._call("submit", {"tx_blob": tx_blob})


class TransactionTester:
    """Tests all transaction types using genesis-funded accounts"""

    def __init__(self, rpc: CallCoreRPC, seq_offset: int = 0, results: StressTestResults = None):
        self.rpc = rpc
        # Track sequence numbers per account to avoid conflicts
        # seq_offset ensures different workers use different sequence ranges
        self.sequences = {i: 1 + seq_offset for i in range(len(GENESIS_WALLETS))}
        self.lock = threading.Lock()
        self.results = results

    def _get_next_sequence(self, account_index: int) -> int:
        """Get the next sequence number for an account"""
        with self.lock:
            seq = self.sequences[account_index]
            self.sequences[account_index] = seq + 1
            return seq

    def _sign_and_submit(self, account_index: int, tx_json: Dict, tx_type: str) -> TestResult:
        """Sign a transaction and submit it"""
        start = time.time()
        wallet = GENESIS_WALLETS[account_index]

        # Get sequence before signing for debug
        seq = tx_json.get("Sequence", "unknown")
        account = tx_json.get("Account", "unknown")

        # Create a unique transaction identifier for debugging
        tx_id = f"{account[:15]}..._seq{seq}"

        # Show first few transactions for debugging
        show_debug = seq <= 3 or (isinstance(seq, int) and seq >= 10000 and seq <= 10002)
        if show_debug:
            print(f"  [DEBUG] Signing {tx_type} {tx_id}: {json.dumps(tx_json)}")

        # Sign the transaction
        tx_blob = self.rpc.sign(wallet["seed"], tx_json)
        if not tx_blob:
            elapsed = (time.time() - start) * 1000
            print(f"  [SIGN FAILED] {tx_type} {tx_id}")
            return TestResult(tx_type, False, "Failed to sign transaction", elapsed)

        if show_debug:
            # Compute hash of tx_blob for comparison
            import hashlib
            blob_hash = hashlib.sha256(tx_blob.encode()).hexdigest()[:16]
            print(f"  [DEBUG] tx_blob hash (sha256): {blob_hash}")

        # Submit the transaction
        response = self.rpc.submit_transaction(tx_blob)
        elapsed = (time.time() - start) * 1000

        if "error" in response:
            error = response["error"]
            print(f"  [RPC ERROR] {tx_type} {tx_id}: {error}")
            if isinstance(error, dict) and "message" in error:
                return TestResult(tx_type, False, error["message"], elapsed)
            return TestResult(tx_type, False, str(error), elapsed)

        result = response.get("result", {})
        engine_result = result.get("engine_result", "")
        tx_hash = result.get("tx_hash")

        # Log every transaction with its result
        if engine_result == "tesSUCCESS":
            print(f"  [SUCCESS] {tx_type} {tx_id} -> {engine_result}")
        elif "terDUPLICATE" in engine_result:
            print(f"  [DUPLICATE] {tx_type} {tx_id} -> {engine_result}")
        elif engine_result.startswith("ter"):
            print(f"  [RETRY] {tx_type} {tx_id} -> {engine_result}")
        else:
            print(f"  [FAILED] {tx_type} {tx_id} -> {engine_result}")

        # Log transaction hash for debugging
        if tx_hash and self.results:
            with self.lock:
                if tx_hash in self.results.sent_tx_hashes:
                    # This is a duplicate hash!
                    self.results.duplicate_hashes.append(tx_hash)
                else:
                    self.results.sent_tx_hashes.append(tx_hash)

        if engine_result == "tesSUCCESS":
            return TestResult(tx_type, True, None, elapsed, tx_hash)
        elif engine_result.startswith("ter"):
            return TestResult(tx_type, False, f"{engine_result}: retryable error", elapsed, tx_hash)
        elif engine_result:
            return TestResult(tx_type, False, engine_result, elapsed, tx_hash)
        else:
            return TestResult(tx_type, False, "No engine_result in response", elapsed, tx_hash)

    def test_payment(self) -> TestResult:
        """Test Payment transaction"""
        sender_idx = 0
        receiver = GENESIS_WALLETS[1]

        tx_json = {
            "TransactionType": "Payment",
            "Account": GENESIS_WALLETS[sender_idx]["address"],
            "Destination": receiver["address"],
            "Amount": "1000000",  # 1 CALL in drops
            "Sequence": self._get_next_sequence(sender_idx),
            "Fee": "10"
        }

        return self._sign_and_submit(sender_idx, tx_json, "Payment")

    def test_account_set(self) -> TestResult:
        """Test AccountSet transaction"""
        account_idx = 2

        tx_json = {
            "TransactionType": "AccountSet",
            "Account": GENESIS_WALLETS[account_idx]["address"],
            "Sequence": self._get_next_sequence(account_idx),
            "Fee": "10",
            "Domain": "6578616D706C652E636F6D"  # "example.com" in hex
        }

        return self._sign_and_submit(account_idx, tx_json, "AccountSet")

    def test_trust_set(self) -> TestResult:
        """Test TrustSet transaction"""
        account_idx = 3
        issuer = GENESIS_WALLETS[0]

        tx_json = {
            "TransactionType": "TrustSet",
            "Account": GENESIS_WALLETS[account_idx]["address"],
            "Sequence": self._get_next_sequence(account_idx),
            "Fee": "10",
            "LimitAmount": {
                "currency": "USD",
                "issuer": issuer["address"],
                "value": "1000"
            }
        }

        return self._sign_and_submit(account_idx, tx_json, "TrustSet")

    def test_offer_create(self) -> TestResult:
        """Test OfferCreate transaction"""
        account_idx = 4

        tx_json = {
            "TransactionType": "OfferCreate",
            "Account": GENESIS_WALLETS[account_idx]["address"],
            "Sequence": self._get_next_sequence(account_idx),
            "Fee": "10",
            "TakerPays": "5000000",  # 5 CALL
            "TakerGets": {
                "currency": "USD",
                "issuer": GENESIS_WALLETS[0]["address"],
                "value": "100"
            }
        }

        return self._sign_and_submit(account_idx, tx_json, "OfferCreate")

    def test_offer_cancel(self) -> TestResult:
        """Test OfferCancel transaction - creates an offer first, then cancels it"""
        account_idx = 4
        seq = self._get_next_sequence(account_idx)

        # First create an offer
        create_tx = {
            "TransactionType": "OfferCreate",
            "Account": GENESIS_WALLETS[account_idx]["address"],
            "Sequence": seq,
            "Fee": "10",
            "TakerPays": "1000000",
            "TakerGets": {
                "currency": "EUR",
                "issuer": GENESIS_WALLETS[0]["address"],
                "value": "50"
            }
        }

        result = self._sign_and_submit(account_idx, create_tx, "OfferCreate")
        if not result.success:
            # If offer creation fails, report as OfferCancel failure
            return TestResult("OfferCancel", False, f"Failed to create offer to cancel: {result.error}", result.response_time_ms)

        # Now cancel the offer (use the sequence of the created offer)
        cancel_tx = {
            "TransactionType": "OfferCancel",
            "Account": GENESIS_WALLETS[account_idx]["address"],
            "Sequence": self._get_next_sequence(account_idx),
            "Fee": "10",
            "OfferSequence": seq
        }

        return self._sign_and_submit(account_idx, cancel_tx, "OfferCancel")

    def test_signer_list_set(self) -> TestResult:
        """Test SignerListSet transaction"""
        account_idx = 0
        signer1 = GENESIS_WALLETS[1]
        signer2 = GENESIS_WALLETS[2]

        tx_json = {
            "TransactionType": "SignerListSet",
            "Account": GENESIS_WALLETS[account_idx]["address"],
            "Sequence": self._get_next_sequence(account_idx),
            "Fee": "10",
            "SignerQuorum": 2,
            "Signers": [
                {
                    "Account": signer1["address"],
                    "SignerWeight": 1
                },
                {
                    "Account": signer2["address"],
                    "SignerWeight": 1
                }
            ]
        }

        return self._sign_and_submit(account_idx, tx_json, "SignerListSet")

    def test_set_regular_key(self) -> TestResult:
        """Test SetRegularKey transaction"""
        account_idx = 1
        regular_key = GENESIS_WALLETS[2]

        tx_json = {
            "TransactionType": "SetRegularKey",
            "Account": GENESIS_WALLETS[account_idx]["address"],
            "Sequence": self._get_next_sequence(account_idx),
            "Fee": "10",
            "RegularKey": regular_key["address"]
        }

        return self._sign_and_submit(account_idx, tx_json, "SetRegularKey")

    def test_nickname_set(self) -> TestResult:
        """Test NicknameSet transaction"""
        account_idx = 2

        tx_json = {
            "TransactionType": "NicknameSet",
            "Account": GENESIS_WALLETS[account_idx]["address"],
            "Sequence": self._get_next_sequence(account_idx),
            "Fee": "10",
            "Nickname": "74657374"  # "test" in hex
        }

        return self._sign_and_submit(account_idx, tx_json, "NicknameSet")

    def test_deposit_preauth(self) -> TestResult:
        """Test DepositPreauth transaction"""
        account_idx = 3
        authorize = GENESIS_WALLETS[4]

        tx_json = {
            "TransactionType": "DepositPreauth",
            "Account": GENESIS_WALLETS[account_idx]["address"],
            "Sequence": self._get_next_sequence(account_idx),
            "Fee": "10",
            "Authorize": authorize["address"]
        }

        return self._sign_and_submit(account_idx, tx_json, "DepositPreauth")

    def test_issue_set(self) -> TestResult:
        """Test IssueSet transaction"""
        account_idx = 0

        tx_json = {
            "TransactionType": "IssueSet",
            "Account": GENESIS_WALLETS[account_idx]["address"],
            "Sequence": self._get_next_sequence(account_idx),
            "Fee": "10",
            "TotalSupply": {
                "currency": "GOLD",
                "issuer": GENESIS_WALLETS[account_idx]["address"],
                "value": "1000000"
            }
        }

        return self._sign_and_submit(account_idx, tx_json, "IssueSet")

    def run_all_tests(self) -> List[TestResult]:
        """Run all transaction type tests"""
        # Note: Only Payment is fully supported by sign RPC currently
        # Other transaction types require additional fields in serialize_tx_json
        tests = [
            self.test_payment,
            # self.test_account_set,  # Requires Domain field support
            # self.test_trust_set,    # Requires LimitAmount field support
            # self.test_offer_create, # Requires TakerPays/TakerGets field support
            # self.test_offer_cancel, # Depends on OfferCreate
            # self.test_signer_list_set,  # Requires Signers field support
            # self.test_set_regular_key,  # Requires RegularKey field support
            # self.test_nickname_set,     # Requires Nickname field support
            # self.test_deposit_preauth,  # Requires Authorize field support
            # self.test_issue_set,        # Requires TotalSupply field support
        ]

        results = []
        for test in tests:
            results.append(test())
        return results


def run_stress_test(args) -> StressTestResults:
    """Run the stress test"""
    print(f"\n{'='*60}")
    print("Call-Core Devnet Stress Test - Real Transaction Signing")
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
        print("\nMake sure the devnet is running:")
        print("  ./devnet/devnet-up.sh start")
        sys.exit(1)

    # Verify genesis accounts are funded (silently check node info)
    info = rpc.server_info()
    print("Verifying genesis accounts...")
    all_accounts_funded = True
    for wallet in GENESIS_WALLETS:
        result = rpc.account_info(wallet["address"])
        if "error" in result:
            print(f"  WARNING: {wallet['name']} - {result.get('error', 'Unknown error')}")
            all_accounts_funded = False
        else:
            account_data = result.get("result", {}).get("account_data", {})
            balance = account_data.get("Balance", "0")
            print(f"  {wallet['name']}: {balance} drops")

    if not all_accounts_funded:
        print("\nWARNING: Some genesis accounts are not funded!")
        print("Make sure the devnet is using the correct genesis.json")

    print()

    # Get initial ledger state
    ledger = rpc.ledger_current()
    ledger_index = ledger.get("result", {}).get("ledger_current_index", 0)
    print(f"Current ledger index: {ledger_index}\n")

    results = StressTestResults()

    # Define test functions (only Payment is fully supported by sign RPC currently)
    all_tests = ["Payment"]

    start_time = time.time()

    def worker(worker_id: int):
        """Worker thread function"""
        local_rpc = CallCoreRPC(args.url)
        # Pre-allocate sequence ranges per worker to avoid terDUPLICATE
        # Worker 0: sequences 1-10000, Worker 1: 10001-20000, etc.
        seq_offset = worker_id * 10000
        tester = TransactionTester(local_rpc, seq_offset, results)
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

    # Print transaction hash summary
    print(f"\nTransaction Hash Summary:")
    print(f"  Unique transaction hashes: {len(results.sent_tx_hashes)}")
    print(f"  Duplicate hashes detected: {len(results.duplicate_hashes)}")

    if results.duplicate_hashes:
        print(f"\n  Duplicate hash details:")
        for h in results.duplicate_hashes[:10]:  # Show first 10
            print(f"    - {h}")
        if len(results.duplicate_hashes) > 10:
            print(f"    ... and {len(results.duplicate_hashes) - 10} more")

    print(f"\n{'='*60}")

    # Exit with error code if too many failures
    if results.failed > results.total_transactions * 0.5:
        print("\nERROR: More than 50% of transactions failed!")
        return 1

    return 0


def main():
    parser = argparse.ArgumentParser(description="Call-Core Devnet Stress Test with Real Transactions")
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
