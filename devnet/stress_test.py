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
from typing import List, Dict, Optional
import hashlib
import subprocess
import os
from urllib.request import urlopen, Request
from urllib.error import URLError


def sign_with_calld_sign(secret: str, tx_json: Dict) -> Optional[str]:
    """
    Sign a transaction using the local calld-sign CLI tool.
    Returns the signed tx_blob or None if signing failed.
    """
    # Find calld-sign binary
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
        # Try to find in PATH
        result = subprocess.run(["which", "calld-sign"], capture_output=True, text=True)
        if result.returncode == 0:
            calld_sign = result.stdout.strip()

    if not calld_sign:
        print("[ERROR] calld-sign binary not found. Please build with: cargo build --release --bin calld-sign")
        return None

    # Prepare tx_json as string
    tx_json_str = json.dumps(tx_json)

    # Build command
    cmd = [
        calld_sign,
        "--secret", secret,
        "--tx-json", tx_json_str,
        "--format", "blob"
    ]

    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=10
        )

        if result.returncode != 0:
            print(f"[SIGN ERROR] {result.stderr.strip()}")
            return None

        # Return the tx_blob (strip whitespace)
        tx_blob = result.stdout.strip()
        if tx_blob:
            return tx_blob
        return None

    except subprocess.TimeoutExpired:
        print("[SIGN ERROR] calld-sign timed out")
        return None
    except Exception as e:
        print(f"[SIGN ERROR] {e}")
        return None


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


class SequenceManager:
    """
    Thread-safe sequence manager that ensures all accounts use sequential sequences.
    Shared across all worker threads to prevent sequence gaps.
    """
    def __init__(self, initial_sequences: Optional[Dict[int, int]] = None):
        self.lock = threading.Lock()
        # Initialize sequences for each account from network or use defaults
        if initial_sequences:
            self.sequences = initial_sequences.copy()
        else:
            # Default: all accounts start at sequence 1
            self.sequences = {i: 1 for i in range(len(GENESIS_WALLETS))}

    def get_next_sequence(self, account_index: int) -> int:
        """Get the next sequence number for an account in a thread-safe manner."""
        with self.lock:
            seq = self.sequences[account_index]
            self.sequences[account_index] = seq + 1
            return seq

    def get_current_sequences(self) -> Dict[int, int]:
        """Get a snapshot of current sequences (for debugging)."""
        with self.lock:
            return self.sequences.copy()


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
    sent_tx_hashes: Dict[str, str] = field(default_factory=dict)
    duplicate_hashes: List[str] = field(default_factory=list)
    sequences_used: Dict[str, List[int]] = field(default_factory=dict)  # Track sequences per account

    def add_result(self, result: TestResult, account: str = None, seq: int = None):
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

        # Track sequences used per account for debugging
        if account and seq is not None:
            if account not in self.sequences_used:
                self.sequences_used[account] = []
            self.sequences_used[account].append(seq)


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
        Sign a transaction using the local calld-sign CLI tool.
        Returns the signed tx_blob or None if signing failed.
        """
        return sign_with_calld_sign(secret, tx_json)

    def submit_transaction(self, tx_blob: str) -> Dict:
        """Submit a transaction"""
        return self._call("submit", {"tx_blob": tx_blob})


class TransactionTester:
    """Tests all transaction types using genesis-funded accounts"""

    def __init__(self, rpc: CallCoreRPC, seq_manager: SequenceManager,
                 results: StressTestResults = None, worker_id: int = 0):
        self.rpc = rpc
        self.seq_manager = seq_manager  # Shared sequence manager
        self.lock = threading.Lock()
        self.results = results
        self.worker_id = worker_id
        # Track submitted transactions for later verification
        self.submitted_txs: List[Dict] = []

    def _get_next_sequence(self, account_index: int) -> int:
        """Get the next sequence number from the shared manager"""
        return self.seq_manager.get_next_sequence(account_index)

    def _sign_and_submit(self, account_index: int, tx_json: Dict, tx_type: str) -> TestResult:
        """Sign a transaction and submit it"""
        start = time.time()
        wallet = GENESIS_WALLETS[account_index]

        # Get sequence from shared manager (ensures sequential ordering across all threads)
        seq = tx_json.get("Sequence", "unknown")
        account = tx_json.get("Account", "unknown")

        # Create a unique transaction identifier for debugging
        tx_id = f"{account[:15]}..._seq{seq}"

        # Show first few transactions for debugging
        show_debug = seq <= 5
        if show_debug:
            print(f"  [DEBUG] Worker {self.worker_id} signing {tx_type} {tx_id}")

        # Sign the transaction
        tx_blob = self.rpc.sign(wallet["seed"], tx_json)
        if not tx_blob:
            elapsed = (time.time() - start) * 1000
            print(f"  [SIGN FAILED] {tx_type} {tx_id}")
            return TestResult(tx_type, False, "Failed to sign transaction", elapsed)

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

        # Log transaction results
        if engine_result == "tesSUCCESS":
            pass  # Suppress success messages to reduce output noise
        elif "terDUPLICATE" in engine_result:
            print(f"  [DUPLICATE] {tx_type} {tx_id} -> {engine_result}")
        elif engine_result.startswith("ter"):
            print(f"  [RETRY] {tx_type} {tx_id} -> {engine_result}")
        elif engine_result.startswith("tec"):
            print(f"  [CLAIMED] {tx_type} {tx_id} -> {engine_result}")
        elif engine_result.startswith("tem"):
            print(f"  [MALFORMED] {tx_type} {tx_id} -> {engine_result}")
        elif engine_result:
            print(f"  [FAILED] {tx_type} {tx_id} -> {engine_result}")

        # Log transaction hash for debugging
        if tx_hash and self.results:
            with self.lock:
                # Store hash with account and sequence for debugging
                hash_entry = f"{tx_hash} ({account[:12]}... seq={seq})"
                if tx_hash in self.results.sent_tx_hashes:
                    # This is a duplicate hash!
                    self.results.duplicate_hashes.append(hash_entry)
                    print(f"    [DUPLICATE HASH DETECTED] {tx_hash}")
                    print(f"      Previous: {self.results.sent_tx_hashes[tx_hash]}")
                    print(f"      Current:  {hash_entry}")
                else:
                    self.results.sent_tx_hashes[tx_hash] = hash_entry

        # Store transaction info for later verification
        tx_info = {
            "tx_type": tx_type,
            "account": account,
            "seq": seq,
            "tx_hash": tx_hash,
            "tx_blob": tx_blob,
            "elapsed": elapsed,
            "submitted": True
        }
        self.submitted_txs.append(tx_info)

        # Track sequence usage
        if self.results:
            with self.lock:
                if account not in self.results.sequences_used:
                    self.results.sequences_used[account] = []
                self.results.sequences_used[account].append(seq)

        if engine_result == "tesSUCCESS":
            # Accepted into queue - actual result unknown until ledger close
            return TestResult(tx_type, True, "ACCEPTED_INTO_QUEUE", elapsed, tx_hash)
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
        """Test OfferCancel transaction - cancels the previous offer created by this account"""
        account_idx = 4

        # Just create a new offer and immediately cancel it
        # First get sequence for the offer creation
        seq_create = self._get_next_sequence(account_idx)

        # First create an offer
        create_tx = {
            "TransactionType": "OfferCreate",
            "Account": GENESIS_WALLETS[account_idx]["address"],
            "Sequence": seq_create,
            "Fee": "10",
            "TakerPays": "1000000",
            "TakerGets": {
                "currency": "EUR",
                "issuer": GENESIS_WALLETS[0]["address"],
                "value": "50"
            }
        }

        result = self._sign_and_submit(account_idx, create_tx, "OfferCreate")
        # Note: Even if offer creation fails (e.g., tecINSUFF_FEE), we still try to cancel
        # because the sequence was consumed

        # Now cancel the offer (use the sequence of the created offer)
        seq_cancel = self._get_next_sequence(account_idx)
        cancel_tx = {
            "TransactionType": "OfferCancel",
            "Account": GENESIS_WALLETS[account_idx]["address"],
            "Sequence": seq_cancel,
            "Fee": "10",
            "OfferSequence": seq_create
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

        # Nickname is a Hash256 field - must be 32 bytes (64 hex chars)
        # Using SHA256 hash of "test" to get a proper 32-byte value
        nickname_hash = hashlib.sha256(b"test").hexdigest()

        tx_json = {
            "TransactionType": "NicknameSet",
            "Account": GENESIS_WALLETS[account_idx]["address"],
            "Sequence": self._get_next_sequence(account_idx),
            "Fee": "10",
            "Nickname": nickname_hash  # 64 hex chars = 32 bytes
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
        # Test all transaction types - some may fail if sign RPC lacks field support
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


def get_account_sequences_from_network(rpc: CallCoreRPC) -> Dict[int, int]:
    """
    Query the network for current account sequences.
    Returns a dict mapping account index to next sequence number.
    """
    sequences = {}
    print("Querying current account sequences from network...")

    for idx, wallet in enumerate(GENESIS_WALLETS):
        result = rpc.account_info(wallet["address"])
        if "error" not in result:
            account_data = result.get("result", {}).get("account_data", {})
            current_seq = account_data.get("Sequence", 1)
            # Next sequence is current + 1
            sequences[idx] = current_seq + 1
            print(f"  {wallet['name']}: current seq={current_seq}, next seq={sequences[idx]}")
        else:
            # Account might not exist yet, start at 1
            sequences[idx] = 1
            print(f"  {wallet['name']}: not found, starting at seq=1")

    return sequences


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

    # Verify genesis accounts are funded
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
            sequence = account_data.get("Sequence", 1)
            print(f"  {wallet['name']}: {balance} drops, seq={sequence}")

    if not all_accounts_funded:
        print("\nWARNING: Some genesis accounts are not funded!")

    print()

    # Get initial ledger state
    ledger = rpc.ledger_current()
    ledger_index = ledger.get("result", {}).get("ledger_current_index", 0)
    print(f"Current ledger index: {ledger_index}\n")

    # Get current sequences from network
    initial_sequences = get_account_sequences_from_network(rpc)

    # Create shared sequence manager with current network state
    seq_manager = SequenceManager(initial_sequences)

    results = StressTestResults()

    # Calculate transactions per worker
    total_tests = 10  # Number of different transaction types
    iterations_per_worker = max(1, args.count // (args.threads * total_tests))
    actual_total = iterations_per_worker * args.threads * total_tests

    print(f"\nAdjusted transaction count: {actual_total}")
    print(f"Iterations per thread: {iterations_per_worker}")
    print(f"Transaction types per iteration: {total_tests}\n")

    start_time = time.time()

    def worker(worker_id: int):
        """Worker thread function - all threads share the same sequence manager"""
        local_rpc = CallCoreRPC(args.url)
        # All workers share the same sequence manager for sequential ordering
        tester = TransactionTester(local_rpc, seq_manager, results, worker_id)
        local_results = []

        for i in range(iterations_per_worker):
            # Cycle through all transaction types
            test_results = tester.run_all_tests()
            local_results.extend(test_results)

            if (i + 1) % 10 == 0 or iterations_per_worker <= 10:
                print(f"  Worker {worker_id}: {i + 1}/{iterations_per_worker} iterations complete")

        return local_results

    print(f"Starting stress test with {args.threads} threads...")
    if args.sequential:
        print("Running in SEQUENTIAL mode (workers run one at a time for guaranteed ordering)\n")
    else:
        print("All threads share a single sequence manager to ensure sequential sequences\n")

    if args.sequential:
        # Sequential mode: run workers one at a time
        for worker_id in range(args.threads):
            try:
                worker_results = worker(worker_id)
                for result in worker_results:
                    results.add_result(result)
            except Exception as e:
                print(f"Worker {worker_id} error: {e}")
    else:
        # Parallel mode: run workers concurrently
        with concurrent.futures.ThreadPoolExecutor(max_workers=args.threads) as executor:
            futures = [executor.submit(worker, i) for i in range(args.threads)]

            for future in concurrent.futures.as_completed(futures):
                try:
                    worker_results = future.result()
                    for result in worker_results:
                        # We don't have account/seq here anymore, they're tracked in results
                        results.add_result(result)
                except Exception as e:
                    print(f"Worker error: {e}")

    results.total_time_seconds = time.time() - start_time

    # Show final sequences
    print("\nFinal account sequences:")
    final_sequences = seq_manager.get_current_sequences()
    for idx, seq in sorted(final_sequences.items()):
        wallet = GENESIS_WALLETS[idx]
        start_seq = initial_sequences.get(idx, 1)
        tx_count = seq - start_seq
        print(f"  {wallet['name']}: started at {start_seq}, ended at {seq} ({tx_count} transactions)")

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

    print(f"\nIMPORTANT: Submit returns 'success' when transaction is accepted into queue.")
    print(f"The actual application result is determined during ledger close (see Docker logs).")
    print(f"Errors shown in Docker logs (terPRE_SEQ, temBAD_*, etc.) occur during ledger application.\n")

    print(f"Submission Results (Queue Acceptance):")
    print(f"  Total Time: {results.total_time_seconds:.2f} seconds")
    print(f"  Total Transactions: {results.total_transactions}")
    print(f"  Accepted into Queue: {results.successful} ({100*results.successful/max(1,results.total_transactions):.1f}%)")
    print(f"  Rejected at Submit: {results.failed} ({100*results.failed/max(1,results.total_transactions):.1f}%)")

    if results.total_time_seconds > 0:
        tps = results.total_transactions / results.total_time_seconds
        print(f"  Throughput: {tps:.2f} tx/second")

    print(f"\nNOTE: Check Docker logs for actual transaction application results:")
    print(f"  ./devnet/devnet-up.sh logs | grep 'Transaction failed'")

    print(f"\nResults by Transaction Type (Queue Acceptance):")
    print(f"  {'Type':<20} {'Count':>8} {'Accepted':>10} {'Rejected':>8} {'Avg Time':>12}")
    print(f"  {'-'*64}")

    for tx_type, type_results in sorted(results.results_by_type.items()):
        count = len(type_results)
        accepted = sum(1 for r in type_results if r.success)
        rejected = count - accepted
        avg_time = sum(r.response_time_ms for r in type_results) / max(1, count)

        print(f"  {tx_type:<20} {count:>8} {accepted:>10} {rejected:>8} {avg_time:>10.2f}ms")

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

    # Print sequence usage summary
    print(f"\nSequence Usage Summary (per account):")
    print(f"  {'Account':<35} {'Tx Count':>10} {'Sequence Range':>20}")
    print(f"  {'-'*70}")

    for account, sequences in sorted(results.sequences_used.items()):
        if sequences:
            seq_list = sorted(set(sequences))
            min_seq = min(seq_list)
            max_seq = max(seq_list)
            print(f"  {account:<35} {len(sequences):>10} {min_seq}-{max_seq}")

            # Check for gaps
            expected_count = max_seq - min_seq + 1
            if len(seq_list) != expected_count:
                missing = set(range(min_seq, max_seq + 1)) - set(seq_list)
                print(f"    WARNING: Missing sequences: {sorted(missing)[:10]}{'...' if len(missing) > 10 else ''}")

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
                        help="Total number of transaction iterations (default: 100, use 10000+ for large scale testing)")
    parser.add_argument("--threads", type=int, default=4,
                        help="Number of concurrent threads (default: 4)")
    parser.add_argument("--sequential", action="store_true",
                        help="Submit transactions sequentially (one thread at a time) to ensure proper ordering")

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
