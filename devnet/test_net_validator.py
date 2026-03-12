#!/usr/bin/env python3
"""
Test script for calld network and validator commands.

Tests the following commands against a running devnet:
- calld net peers
- calld net crawler
- calld net status
- calld validator info
- calld validator list
- calld validator quorum

Usage:
    python3 test_net_validator.py [--config CONFIG] [--data-dir DATA_DIR]

Example:
    python3 test_net_validator.py --config ./devnet/node1/config-native.toml
"""

import argparse
import json
import subprocess
import sys
from typing import Dict, List, Tuple


class Colors:
    """Terminal colors"""
    GREEN = '\033[0;32m'
    RED = '\033[0;31m'
    YELLOW = '\033[1;33m'
    NC = '\033[0m'  # No Color


def info(msg: str):
    print(f"{Colors.GREEN}[INFO]{Colors.NC} {msg}")


def warn(msg: str):
    print(f"{Colors.YELLOW}[WARN]{Colors.NC} {msg}")


def error(msg: str):
    print(f"{Colors.RED}[ERROR]{Colors.NC} {msg}")


def run_command(cmd: List[str], description: str) -> Tuple[bool, Dict]:
    """
    Run a calld command and return success status and parsed JSON output.

    Args:
        cmd: Command arguments list
        description: Description of the test

    Returns:
        Tuple of (success: bool, output: dict)
    """
    print()
    info(f"Testing: {description}")
    info(f"Command: {' '.join(cmd)}")

    try:
        # Run command and capture stdout
        result = subprocess.run(
            cmd,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=30
        )

        # Parse JSON from stdout (filter out log lines)
        stdout = result.stdout.strip()

        # Find the JSON part (starts with '{')
        lines = stdout.split('\n')
        json_start = -1
        for i, line in enumerate(lines):
            if line.strip().startswith('{'):
                json_start = i
                break

        if json_start == -1:
            error(f"No JSON output found")
            error(f"Raw stdout: {stdout[:200]}")
            return False, {}

        # Join all lines from JSON start to end
        json_part = '\n'.join(lines[json_start:])

        try:
            output = json.loads(json_part)
        except json.JSONDecodeError as e:
            error(f"Failed to parse JSON: {e}")
            error(f"JSON part: {json_part[:200]}")
            return False, {}

        if result.returncode == 0:
            info(f"✓ PASSED: {description}")
            return True, output
        else:
            error(f"✗ FAILED: {description}")
            return False, output

    except subprocess.TimeoutExpired:
        error(f"✗ TIMEOUT: {description}")
        return False, {}
    except Exception as e:
        error(f"✗ ERROR: {description} - {e}")
        return False, {}


def test_network_commands(calld: str, config: str) -> Tuple[int, int]:
    """Test all network (net) commands"""
    print()
    print("=" * 50)
    print("Network Commands (net)")
    print("=" * 50)

    passed = 0
    failed = 0

    # Build base command
    base_cmd = [calld]
    if config:
        base_cmd.extend(["--config", config])

    # Test net peers
    success, output = run_command(
        base_cmd + ["net", "peers"],
        "List connected peers"
    )
    if success:
        peer_count = output.get("peers", 0)
        info(f"  Connected peers: {peer_count}")
        passed += 1
    else:
        failed += 1

    # Test net crawler
    success, output = run_command(
        base_cmd + ["net", "crawler"],
        "Show crawler information"
    )
    if success:
        discovered = output.get("crawler", {}).get("discovered_peers", 0)
        info(f"  Discovered peers: {discovered}")
        passed += 1
    else:
        failed += 1

    # Test net status
    success, output = run_command(
        base_cmd + ["net", "status"],
        "Show network status"
    )
    if success:
        network = output.get("network", {})
        info(f"  Listen address: {network.get('listen_address', 'N/A')}")
        info(f"  Peer count: {network.get('peer_count', 0)}")
        info(f"  Ledger index: {network.get('ledger_current_index', 0)}")
        passed += 1
    else:
        failed += 1

    return passed, failed


def test_validator_commands(calld: str, config: str) -> Tuple[int, int]:
    """Test all validator commands"""
    print()
    print("=" * 50)
    print("Validator Commands")
    print("=" * 50)

    passed = 0
    failed = 0

    # Build base command
    base_cmd = [calld]
    if config:
        base_cmd.extend(["--config", config])

    # Test validator info
    success, output = run_command(
        base_cmd + ["validator", "info"],
        "Show validator info"
    )
    if success:
        validator = output.get("validator", {})
        is_validator = validator.get("is_validator", False)
        info(f"  Is validator: {is_validator}")
        info(f"  Validation key: {validator.get('validation_public_key', 'N/A')[:20]}...")
        passed += 1
    else:
        failed += 1

    # Test validator list
    success, output = run_command(
        base_cmd + ["validator", "list"],
        "List known validators"
    )
    if success:
        validators = output.get("validators", [])
        info(f"  Known validators: {len(validators)}")
        for v in validators:
            info(f"    - {v.get('validation_public_key', 'N/A')[:20]}...")
        passed += 1
    else:
        failed += 1

    # Test validator quorum
    success, output = run_command(
        base_cmd + ["validator", "quorum"],
        "Show quorum information"
    )
    if success:
        quorum = output.get("quorum", {})
        info(f"  Quorum: {quorum.get('validation_quorum', 0)}")
        info(f"  Trusted validators: {quorum.get('trusted_validator_count', 0)}")
        info(f"  Quorum ratio: {quorum.get('quorum_ratio', 'N/A')}")
        passed += 1
    else:
        failed += 1

    return passed, failed


def main():
    parser = argparse.ArgumentParser(
        description="Test calld network and validator commands"
    )
    parser.add_argument(
        "--config",
        default="./devnet/node1/config-native.toml",
        help="Path to node configuration file"
    )
    parser.add_argument(
        "--calld",
        default="./target/release/calld",
        help="Path to calld binary"
    )

    args = parser.parse_args()

    print("=" * 50)
    print("Call-Core Network & Validator Tests")
    print("=" * 50)
    print(f"Binary: {args.calld}")
    print(f"Config: {args.config}")

    # Check binary exists
    import os
    if not os.path.isfile(args.calld):
        error(f"calld binary not found: {args.calld}")
        error("Please build with: cargo build --release --bin calld")
        sys.exit(1)

    # Run tests
    net_passed, net_failed = test_network_commands(args.calld, args.config)
    val_passed, val_failed = test_validator_commands(args.calld, args.config)

    total_passed = net_passed + val_passed
    total_failed = net_failed + val_failed

    # Summary
    print()
    print("=" * 50)
    print("Test Summary")
    print("=" * 50)
    info(f"Passed: {total_passed}")

    if total_failed > 0:
        error(f"Failed: {total_failed}")
        sys.exit(1)
    else:
        info("All tests passed!")
        sys.exit(0)


if __name__ == "__main__":
    main()
