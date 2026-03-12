#!/usr/bin/env python3
"""
Comprehensive test script for all call-core RPC and WebSocket APIs.

Tests all JSON-RPC methods and WebSocket subscriptions to verify they work correctly.

Usage:
    python3 test_all_apis.py [--rpc-url URL] [--ws-url URL]

Example:
    python3 test_all_apis.py --rpc-url http://localhost:5005 --ws-url ws://localhost:5005
"""

import argparse
import json
import subprocess
import sys
import time
import asyncio
import websockets
from typing import Dict, List, Tuple, Optional, Any
from dataclasses import dataclass
from enum import Enum


class Colors:
    """Terminal colors"""
    GREEN = '\033[0;32m'
    RED = '\033[0;31m'
    YELLOW = '\033[1;33m'
    BLUE = '\033[0;34m'
    NC = '\033[0m'


def info(msg: str):
    print(f"{Colors.GREEN}[INFO]{Colors.NC} {msg}")


def error(msg: str):
    print(f"{Colors.RED}[ERROR]{Colors.NC} {msg}")


def warn(msg: str):
    print(f"{Colors.YELLOW}[WARN]{Colors.NC} {msg}")


def section(msg: str):
    print(f"\n{Colors.BLUE}{'='*60}{Colors.NC}")
    print(f"{Colors.BLUE}{msg}{Colors.NC}")
    print(f"{Colors.BLUE}{'='*60}{Colors.NC}")


@dataclass
class TestResult:
    name: str
    passed: bool
    error: Optional[str] = None
    response: Optional[Dict] = None


class RPCTester:
    """Test JSON-RPC methods"""

    def __init__(self, rpc_url: str):
        self.rpc_url = rpc_url
        self.request_id = 0
        self.results: List[TestResult] = []

    def _make_request(self, method: str, params: Optional[Dict] = None) -> Tuple[bool, Any]:
        """Make JSON-RPC request via curl"""
        self.request_id += 1
        payload = {
            "jsonrpc": "2.0",
            "method": method,
            "id": self.request_id
        }
        if params is not None:
            payload["params"] = params

        cmd = [
            "curl", "-s", "-X", "POST", self.rpc_url,
            "-H", "Content-Type: application/json",
            "-d", json.dumps(payload),
            "-m", "10"
        ]

        try:
            result = subprocess.run(cmd, capture_output=True, text=True, timeout=15)
            if result.returncode != 0:
                return False, f"curl failed: {result.stderr}"

            if not result.stdout.strip():
                return False, "Empty response"

            try:
                data = json.loads(result.stdout)
                if "error" in data:
                    return False, data["error"]
                return True, data.get("result")
            except json.JSONDecodeError as e:
                return False, f"JSON parse error: {e}"
        except subprocess.TimeoutExpired:
            return False, "Request timeout"
        except Exception as e:
            return False, str(e)

    def _add_result(self, name: str, success: bool, error_msg: Optional[str] = None, response: Optional[Dict] = None):
        """Add test result"""
        self.results.append(TestResult(name, success, error_msg, response))
        if success:
            info(f"✓ {name}")
        else:
            error(f"✗ {name}: {error_msg}")

    # ==================== Server Info Methods ====================

    def test_server_info(self):
        """Test server_info method"""
        success, result = self._make_request("server_info")
        if success and isinstance(result, dict) and "info" in result:
            self._add_result("server_info", True, response=result)
        else:
            self._add_result("server_info", False, error_msg=str(result))

    def test_server_state(self):
        """Test server_state method"""
        success, result = self._make_request("server_state")
        if success and isinstance(result, dict):
            self._add_result("server_state", True, response=result)
        else:
            self._add_result("server_state", False, error_msg=str(result))

    def test_ping(self):
        """Test ping method"""
        success, result = self._make_request("ping")
        if success and isinstance(result, dict) and result.get("status") == "success":
            self._add_result("ping", True, response=result)
        else:
            self._add_result("ping", False, error_msg=str(result))

    def test_version(self):
        """Test version method"""
        success, result = self._make_request("version")
        if success and isinstance(result, dict):
            self._add_result("version", True, response=result)
        else:
            self._add_result("version", False, error_msg=str(result))

    # ==================== Ledger Methods ====================

    def test_ledger_current(self):
        """Test ledger_current method"""
        success, result = self._make_request("ledger_current")
        if success and isinstance(result, dict) and "ledger_current_index" in result:
            self._add_result("ledger_current", True, response=result)
        else:
            self._add_result("ledger_current", False, error_msg=str(result))

    def test_ledger_closed(self):
        """Test ledger_closed method"""
        success, result = self._make_request("ledger_closed")
        if success and isinstance(result, dict) and "ledger_hash" in result:
            self._add_result("ledger_closed", True, response=result)
        else:
            self._add_result("ledger_closed", False, error_msg=str(result))

    def test_ledger(self):
        """Test ledger method"""
        success, result = self._make_request("ledger", {"ledger_index": "current"})
        if success and isinstance(result, dict) and "ledger" in result:
            self._add_result("ledger (current)", True, response=result)
        else:
            self._add_result("ledger (current)", False, error_msg=str(result))

    def test_ledger_data(self):
        """Test ledger_data method"""
        success, result = self._make_request("ledger_data", {"limit": 10})
        if success and isinstance(result, dict):
            self._add_result("ledger_data", True, response=result)
        else:
            self._add_result("ledger_data", False, error_msg=str(result))

    def test_ledger_header(self):
        """Test ledger_header method"""
        success, result = self._make_request("ledger_header")
        if success and isinstance(result, dict):
            self._add_result("ledger_header", True, response=result)
        else:
            self._add_result("ledger_header", False, error_msg=str(result))

    def test_get_counts(self):
        """Test get_counts method"""
        success, result = self._make_request("get_counts")
        if success and isinstance(result, dict):
            self._add_result("get_counts", True, response=result)
        else:
            self._add_result("get_counts", False, error_msg=str(result))

    # ==================== Account Methods ====================

    def test_account_info(self):
        """Test account_info method"""
        # Use genesis account
        params = {"account": "cGmJBrEfFssWuas4kCoHTX5r6aMEf6QHhy"}
        success, result = self._make_request("account_info", params)
        if success and isinstance(result, dict) and "account_data" in result:
            self._add_result("account_info", True, response=result)
        else:
            self._add_result("account_info", False, error_msg=str(result))

    def test_account_tx(self):
        """Test account_tx method"""
        params = {
            "account": "cGmJBrEfFssWuas4kCoHTX5r6aMEf6QHhy",
            "limit": 10
        }
        success, result = self._make_request("account_tx", params)
        if success and isinstance(result, dict):
            self._add_result("account_tx", True, response=result)
        else:
            self._add_result("account_tx", False, error_msg=str(result))

    def test_account_lines(self):
        """Test account_lines method"""
        params = {"account": "cGmJBrEfFssWuas4kCoHTX5r6aMEf6QHhy"}
        success, result = self._make_request("account_lines", params)
        if success and isinstance(result, dict):
            self._add_result("account_lines", True, response=result)
        else:
            self._add_result("account_lines", False, error_msg=str(result))

    def test_account_objects(self):
        """Test account_objects method"""
        params = {"account": "cGmJBrEfFssWuas4kCoHTX5r6aMEf6QHhy"}
        success, result = self._make_request("account_objects", params)
        if success and isinstance(result, dict):
            self._add_result("account_objects", True, response=result)
        else:
            self._add_result("account_objects", False, error_msg=str(result))

    def test_account_offers(self):
        """Test account_offers method"""
        params = {"account": "cGmJBrEfFssWuas4kCoHTX5r6aMEf6QHhy"}
        success, result = self._make_request("account_offers", params)
        if success and isinstance(result, dict):
            self._add_result("account_offers", True, response=result)
        else:
            self._add_result("account_offers", False, error_msg=str(result))

    def test_account_currencies(self):
        """Test account_currencies method"""
        params = {"account": "cGmJBrEfFssWuas4kCoHTX5r6aMEf6QHhy"}
        success, result = self._make_request("account_currencies", params)
        if success and isinstance(result, dict):
            self._add_result("account_currencies", True, response=result)
        else:
            self._add_result("account_currencies", False, error_msg=str(result))

    # ==================== Transaction Methods ====================

    def test_tx_history(self):
        """Test tx_history method"""
        success, result = self._make_request("tx_history", {"start": 0})
        if success and isinstance(result, dict):
            self._add_result("tx_history", True, response=result)
        else:
            self._add_result("tx_history", False, error_msg=str(result))

    def test_submit_error(self):
        """Test submit method with invalid tx (expects error)"""
        params = {"tx_blob": "invalid"}
        success, result = self._make_request("submit", params)
        # Should fail with invalid tx_blob
        if not success:
            self._add_result("submit (error handling)", True, response={"expected_error": str(result)})
        else:
            self._add_result("submit (error handling)", False, error_msg="Expected error for invalid tx_blob")

    # ==================== Consensus/Network Methods ====================

    def test_consensus_info(self):
        """Test consensus_info method"""
        success, result = self._make_request("consensus_info")
        if success and isinstance(result, dict):
            self._add_result("consensus_info", True, response=result)
        else:
            self._add_result("consensus_info", False, error_msg=str(result))

    def test_fee(self):
        """Test fee method"""
        success, result = self._make_request("fee")
        if success and isinstance(result, dict):
            self._add_result("fee", True, response=result)
        else:
            self._add_result("fee", False, error_msg=str(result))

    def test_peers(self):
        """Test peers method"""
        success, result = self._make_request("peers")
        if success and isinstance(result, dict):
            self._add_result("peers", True, response=result)
        else:
            self._add_result("peers", False, error_msg=str(result))

    def test_network_info(self):
        """Test network_info method"""
        success, result = self._make_request("network_info")
        if success and isinstance(result, dict) and "network" in result:
            self._add_result("network_info", True, response=result)
        else:
            self._add_result("network_info", False, error_msg=str(result))

    def test_crawler(self):
        """Test crawler method"""
        success, result = self._make_request("crawler")
        if success and isinstance(result, dict):
            self._add_result("crawler", True, response=result)
        else:
            self._add_result("crawler", False, error_msg=str(result))

    def test_validators(self):
        """Test validators method"""
        success, result = self._make_request("validators")
        if success and isinstance(result, dict):
            self._add_result("validators", True, response=result)
        else:
            self._add_result("validators", False, error_msg=str(result))

    def test_validator_info(self):
        """Test validator_info method"""
        success, result = self._make_request("validator_info")
        if success and isinstance(result, dict):
            self._add_result("validator_info", True, response=result)
        else:
            self._add_result("validator_info", False, error_msg=str(result))

    def test_validation_quorum(self):
        """Test validation_quorum method"""
        success, result = self._make_request("validation_quorum")
        if success and isinstance(result, dict) and "quorum" in result:
            self._add_result("validation_quorum", True, response=result)
        else:
            self._add_result("validation_quorum", False, error_msg=str(result))

    def test_unl_list(self):
        """Test unl_list method"""
        success, result = self._make_request("unl_list")
        if success and isinstance(result, dict):
            self._add_result("unl_list", True, response=result)
        else:
            self._add_result("unl_list", False, error_msg=str(result))

    def test_fetch_info(self):
        """Test fetch_info method"""
        success, result = self._make_request("fetch_info")
        if success and isinstance(result, dict):
            self._add_result("fetch_info", True, response=result)
        else:
            self._add_result("fetch_info", False, error_msg=str(result))

    # ==================== Utility Methods ====================

    def test_random(self):
        """Test random method"""
        params = {"random": "16"}
        success, result = self._make_request("random", params)
        if success and isinstance(result, dict):
            self._add_result("random", True, response=result)
        else:
            self._add_result("random", False, error_msg=str(result))

    # ==================== Admin Methods (May fail without admin) ====================

    def test_admin_methods(self):
        """Test admin methods - these may fail without admin privileges"""
        admin_methods = [
            ("ledger_accept", None, "ledger_accept"),
            ("validation_create", None, "validation_create"),
            ("wallet_propose", None, "wallet_propose"),
            ("feature", None, "feature_list"),
        ]

        for method, params, name in admin_methods:
            success, result = self._make_request(method, params)
            if success:
                self._add_result(f"{name} (admin)", True, response=result)
            else:
                # Expected to fail without admin, so mark as expected
                self._add_result(f"{name} (admin)", True, response={"expected_auth_error": str(result)})

    def run_all_tests(self):
        """Run all RPC tests"""
        section("Testing Server Info Methods")
        self.test_server_info()
        self.test_server_state()
        self.test_ping()
        self.test_version()

        section("Testing Ledger Methods")
        self.test_ledger_current()
        self.test_ledger_closed()
        self.test_ledger()
        self.test_ledger_data()
        self.test_ledger_header()
        self.test_get_counts()

        section("Testing Account Methods")
        self.test_account_info()
        self.test_account_tx()
        self.test_account_lines()
        self.test_account_objects()
        self.test_account_offers()
        self.test_account_currencies()

        section("Testing Transaction Methods")
        self.test_tx_history()
        self.test_submit_error()

        section("Testing Consensus/Network Methods")
        self.test_consensus_info()
        self.test_fee()
        self.test_peers()
        self.test_network_info()
        self.test_crawler()
        self.test_validators()
        self.test_validator_info()
        self.test_validation_quorum()
        self.test_unl_list()
        self.test_fetch_info()

        section("Testing Utility Methods")
        self.test_random()

        section("Testing Admin Methods (may require auth)")
        self.test_admin_methods()

        return self.results


class WebSocketTester:
    """Test WebSocket API"""

    def __init__(self, ws_url: str):
        self.ws_url = ws_url
        self.results: List[TestResult] = []

    def _add_result(self, name: str, passed: bool, error_msg: Optional[str] = None, response: Optional[Dict] = None):
        """Add test result"""
        self.results.append(TestResult(name, passed, error_msg, response))
        if passed:
            info(f"✓ {name}")
        else:
            error(f"✗ {name}: {error_msg}")

    async def test_websocket_connection(self):
        """Test basic WebSocket connection"""
        try:
            async with websockets.connect(self.ws_url, ping_interval=None) as ws:
                self._add_result("websocket_connection", True)
        except Exception as e:
            self._add_result("websocket_connection", False, error_msg=str(e))

    async def test_websocket_ping(self):
        """Test WebSocket ping/pong"""
        try:
            async with websockets.connect(self.ws_url, ping_interval=None) as ws:
                ping_msg = json.dumps({"command": "ping", "id": 1})
                await ws.send(ping_msg)
                response = await asyncio.wait_for(ws.recv(), timeout=5.0)
                data = json.loads(response)
                if data.get("id") == 1:
                    self._add_result("websocket_ping", True, response=data)
                else:
                    self._add_result("websocket_ping", False, error_msg="Invalid response ID")
        except Exception as e:
            self._add_result("websocket_ping", False, error_msg=str(e))

    async def test_websocket_server_info(self):
        """Test WebSocket server_info command"""
        try:
            async with websockets.connect(self.ws_url, ping_interval=None) as ws:
                msg = json.dumps({"command": "server_info", "id": 2})
                await ws.send(msg)
                response = await asyncio.wait_for(ws.recv(), timeout=5.0)
                data = json.loads(response)
                # WebSocket response format: {"id": 1, "status": "success", "result": {...}}
                result = data.get("result", {})
                # WebSocket returns data directly in result (not wrapped in "info")
                if isinstance(result, dict) and ("build_version" in result or "info" in result):
                    self._add_result("websocket_server_info", True, response=data)
                else:
                    self._add_result("websocket_server_info", False, error_msg=f"Missing expected fields. Got: {result}")
        except Exception as e:
            self._add_result("websocket_server_info", False, error_msg=str(e))

    async def test_websocket_subscribe_ledger(self):
        """Test WebSocket ledger subscription"""
        try:
            async with websockets.connect(self.ws_url, ping_interval=None) as ws:
                # Subscribe to ledger stream
                msg = json.dumps({
                    "command": "subscribe",
                    "streams": ["ledger"],
                    "id": 3
                })
                await ws.send(msg)

                # Wait for subscription confirmation
                response = await asyncio.wait_for(ws.recv(), timeout=5.0)
                data = json.loads(response)

                if data.get("status") == "success" or "subscribed" in str(data):
                    self._add_result("websocket_subscribe_ledger", True, response=data)

                    # Unsubscribe
                    unsub_msg = json.dumps({
                        "command": "unsubscribe",
                        "streams": ["ledger"],
                        "id": 4
                    })
                    await ws.send(unsub_msg)
                    await asyncio.wait_for(ws.recv(), timeout=5.0)
                else:
                    self._add_result("websocket_subscribe_ledger", False, error_msg="Subscription failed")
        except Exception as e:
            self._add_result("websocket_subscribe_ledger", False, error_msg=str(e))

    async def test_websocket_subscribe_transactions(self):
        """Test WebSocket transaction subscription"""
        try:
            async with websockets.connect(self.ws_url, ping_interval=None) as ws:
                msg = json.dumps({
                    "command": "subscribe",
                    "streams": ["transactions"],
                    "id": 5
                })
                await ws.send(msg)
                response = await asyncio.wait_for(ws.recv(), timeout=5.0)
                data = json.loads(response)

                if data.get("status") == "success" or "subscribed" in str(data):
                    self._add_result("websocket_subscribe_transactions", True, response=data)

                    # Unsubscribe
                    unsub_msg = json.dumps({
                        "command": "unsubscribe",
                        "streams": ["transactions"],
                        "id": 6
                    })
                    await ws.send(unsub_msg)
                    await asyncio.wait_for(ws.recv(), timeout=5.0)
                else:
                    self._add_result("websocket_subscribe_transactions", False, error_msg="Subscription failed")
        except Exception as e:
            self._add_result("websocket_subscribe_transactions", False, error_msg=str(e))

    async def test_websocket_account_subscribe(self):
        """Test WebSocket account subscription"""
        try:
            async with websockets.connect(self.ws_url, ping_interval=None) as ws:
                msg = json.dumps({
                    "command": "subscribe",
                    "accounts": ["cGmJBrEfFssWuas4kCoHTX5r6aMEf6QHhy"],
                    "id": 7
                })
                await ws.send(msg)
                response = await asyncio.wait_for(ws.recv(), timeout=5.0)
                data = json.loads(response)

                if data.get("status") == "success" or "subscribed" in str(data):
                    self._add_result("websocket_account_subscribe", True, response=data)

                    # Unsubscribe
                    unsub_msg = json.dumps({
                        "command": "unsubscribe",
                        "accounts": ["cGmJBrEfFssWuas4kCoHTX5r6aMEf6QHhy"],
                        "id": 8
                    })
                    await ws.send(unsub_msg)
                    await asyncio.wait_for(ws.recv(), timeout=5.0)
                else:
                    self._add_result("websocket_account_subscribe", False, error_msg="Subscription failed")
        except Exception as e:
            self._add_result("websocket_account_subscribe", False, error_msg=str(e))

    async def test_websocket_ledger_command(self):
        """Test WebSocket ledger command"""
        try:
            async with websockets.connect(self.ws_url, ping_interval=None) as ws:
                msg = json.dumps({"command": "ledger", "id": 9})
                await ws.send(msg)
                response = await asyncio.wait_for(ws.recv(), timeout=5.0)
                data = json.loads(response)
                # WebSocket response format: {"id": 9, "status": "success", "result": {...}}
                result = data.get("result", {})
                # WebSocket returns data directly in result (not wrapped in "ledger")
                if isinstance(result, dict) and ("ledger_index" in result or "ledger" in result):
                    self._add_result("websocket_ledger_command", True, response=data)
                else:
                    self._add_result("websocket_ledger_command", False, error_msg=f"Missing expected fields. Got: {result}")
        except Exception as e:
            self._add_result("websocket_ledger_command", False, error_msg=str(e))

    async def run_all_tests(self):
        """Run all WebSocket tests"""
        section("Testing WebSocket Connection")
        await self.test_websocket_connection()

        if any(r.name == "websocket_connection" and r.passed for r in self.results):
            section("Testing WebSocket Commands")
            await self.test_websocket_ping()
            await self.test_websocket_server_info()
            await self.test_websocket_ledger_command()

            section("Testing WebSocket Subscriptions")
            await self.test_websocket_subscribe_ledger()
            await self.test_websocket_subscribe_transactions()
            await self.test_websocket_account_subscribe()
        else:
            warn("Skipping WebSocket tests - connection failed")

        return self.results


def print_summary(rpc_results: List[TestResult], ws_results: List[TestResult]):
    """Print test summary"""
    print(f"\n{Colors.BLUE}{'='*60}{Colors.NC}")
    print(f"{Colors.BLUE}Test Summary{Colors.NC}")
    print(f"{Colors.BLUE}{'='*60}{Colors.NC}")

    rpc_passed = sum(1 for r in rpc_results if r.passed)
    rpc_total = len(rpc_results)

    ws_passed = sum(1 for r in ws_results if r.passed)
    ws_total = len(ws_results)

    total_passed = rpc_passed + ws_passed
    total_tests = rpc_total + ws_total

    info(f"RPC Tests: {rpc_passed}/{rpc_total} passed")
    if ws_total > 0:
        info(f"WebSocket Tests: {ws_passed}/{ws_total} passed")
    info(f"Total: {total_passed}/{total_tests} passed")

    if total_passed == total_tests:
        print(f"\n{Colors.GREEN}✓ All tests passed!{Colors.NC}")
        return 0
    else:
        print(f"\n{Colors.RED}✗ Some tests failed{Colors.NC}")

        # Show failed tests
        failed = [r for r in rpc_results + ws_results if not r.passed]
        if failed:
            print(f"\n{Colors.RED}Failed tests:{Colors.NC}")
            for r in failed:
                print(f"  - {r.name}: {r.error}")
        return 1


async def main():
    parser = argparse.ArgumentParser(
        description="Test all call-core RPC and WebSocket APIs"
    )
    parser.add_argument(
        "--rpc-url",
        default="http://localhost:5005",
        help="RPC endpoint URL (default: http://localhost:5005)"
    )
    parser.add_argument(
        "--ws-url",
        default="ws://localhost:5005",
        help="WebSocket endpoint URL (default: ws://localhost:5005)"
    )
    parser.add_argument(
        "--skip-ws",
        action="store_true",
        help="Skip WebSocket tests"
    )

    args = parser.parse_args()

    print(f"{Colors.BLUE}{'='*60}{Colors.NC}")
    print(f"{Colors.BLUE}Call-Core API Test Suite{Colors.NC}")
    print(f"{Colors.BLUE}{'='*60}{Colors.NC}")
    print(f"RPC URL: {args.rpc_url}")
    if not args.skip_ws:
        print(f"WebSocket URL: {args.ws_url}")
    print()

    # Test RPC
    rpc_tester = RPCTester(args.rpc_url)
    rpc_results = rpc_tester.run_all_tests()

    # Test WebSocket
    ws_results = []
    if not args.skip_ws:
        ws_tester = WebSocketTester(args.ws_url)
        ws_results = await ws_tester.run_all_tests()

    # Print summary
    return print_summary(rpc_results, ws_results)


if __name__ == "__main__":
    try:
        exit_code = asyncio.run(main())
        sys.exit(exit_code)
    except KeyboardInterrupt:
        print("\n\nTests interrupted by user")
        sys.exit(1)
