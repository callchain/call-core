#!/usr/bin/env python3
"""
Test script for validation quorum functionality.

Tests that the consensus properly uses the validation_quorum configuration.

Usage:
    python3 test_quorum.py [--config CONFIG]

Example:
    python3 test_quorum.py --config ./devnet/node1/config-native.toml
"""

import argparse
import json
import subprocess
import sys
from typing import Dict, Tuple


class Colors:
    """Terminal colors"""
    GREEN = '\033[0;32m'
    RED = '\033[0;31m'
    YELLOW = '\033[1;33m'
    NC = '\033[0m'


def info(msg: str):
    print(f"{Colors.GREEN}[INFO]{Colors.NC} {msg}")


def error(msg: str):
    print(f"{Colors.RED}[ERROR]{Colors.NC} {msg}")


def warn(msg: str):
    print(f"{Colors.YELLOW}[WARN]{Colors.NC} {msg}")


def run_command(cmd: list) -> Tuple[bool, str]:
    """Run a command and return success and output (combines stdout + stderr)"""
    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=30
        )
        # calld CLI outputs JSON to stderr, so combine both
        return result.returncode == 0, result.stdout + result.stderr
    except Exception as e:
        return False, str(e)


def extract_json(output: str) -> dict:
    """Extract JSON object from output that may contain log lines"""
    # Find the last { } block in the output
    lines = output.strip().split('\n')

    # Find the first line that starts a JSON object
    start_idx = None
    for i, line in enumerate(lines):
        if line.strip().startswith('{'):
            start_idx = i
            break

    if start_idx is None:
        raise ValueError("No JSON object found in output")

    # Reconstruct the JSON from the lines
    json_str = '\n'.join(lines[start_idx:])
    return json.loads(json_str)


def check_quorum_in_config(config_path: str) -> bool:
    """Check if quorum is configured in the config file"""
    info("Checking quorum configuration in config...")

    try:
        with open(config_path, 'r') as f:
            content = f.read()

        if '[consensus]' not in content:
            warn("No [consensus] section found in config")
            return False

        if 'validation_quorum' in content:
            # Extract the value
            for line in content.split('\n'):
                if 'validation_quorum' in line and '=' in line:
                    value = line.split('=')[1].strip()
                    info(f"Found validation_quorum = {value}")
                    return True
        else:
            warn("validation_quorum not found in config")
            return False

    except Exception as e:
        error(f"Failed to read config: {e}")
        return False


def test_validator_quorum_rpc(calld: str, config: str) -> bool:
    """Test validator quorum via RPC"""
    info("Testing validator quorum via RPC...")

    cmd = [calld, "--config", config, "validator", "quorum"]
    success, output = run_command(cmd)

    if not success:
        error(f"Command failed: {' '.join(cmd)}")
        return False

    try:
        data = extract_json(output)
        quorum = data.get('quorum', {})

        validation_quorum = quorum.get('validation_quorum', 0)
        quorum_ratio = quorum.get('quorum_ratio', 'N/A')
        trusted_count = quorum.get('trusted_validator_count', 0)

        info(f"  Validation quorum threshold: {validation_quorum}%")
        info(f"  Current quorum ratio: {quorum_ratio}")
        info(f"  Trusted validators: {trusted_count}")

        # Check if quorum is properly configured (> 0)
        if validation_quorum > 0:
            info("✓ Quorum is configured")
            return True
        else:
            warn("Quorum is 0 or not configured")
            return False

    except json.JSONDecodeError as e:
        error(f"Failed to parse JSON: {e}")
        return False


def test_consensus_params(calld: str, config: str) -> bool:
    """Test consensus parameters via server_info"""
    info("Testing consensus parameters via server_info...")

    cmd = [calld, "--config", config, "server-info"]
    success, output = run_command(cmd)

    if not success:
        error(f"Command failed: {' '.join(cmd)}")
        return False

    try:
        data = extract_json(output)
        info_data = data.get('info', {})

        # Check for validation_quorum in server_info
        validation_quorum = info_data.get('validation_quorum')

        if validation_quorum is not None:
            info(f"  Server validation_quorum: {validation_quorum}")
            if validation_quorum > 0:
                info("✓ Quorum is active")
                return True
            else:
                warn("Quorum is 0")
                return False
        else:
            warn("validation_quorum not in server_info (may need to restart node)")
            return False

    except json.JSONDecodeError as e:
        error(f"Failed to parse JSON: {e}")
        return False


def main():
    parser = argparse.ArgumentParser(
        description="Test validation quorum functionality"
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

    print("=" * 60)
    print("Validation Quorum Test")
    print("=" * 60)
    print(f"Config: {args.config}")
    print(f"Binary: {args.calld}")

    # Check if binary exists
    import os
    if not os.path.isfile(args.calld):
        error(f"calld binary not found: {args.calld}")
        sys.exit(1)

    tests_passed = 0
    tests_failed = 0

    # Test 1: Check config file
    if check_quorum_in_config(args.config):
        tests_passed += 1
    else:
        tests_failed += 1

    # Test 2: Test validator quorum RPC
    if test_validator_quorum_rpc(args.calld, args.config):
        tests_passed += 1
    else:
        tests_failed += 1

    # Test 3: Test consensus params
    if test_consensus_params(args.calld, args.config):
        tests_passed += 1
    else:
        tests_failed += 1

    # Summary
    print()
    print("=" * 60)
    print("Test Summary")
    print("=" * 60)
    info(f"Passed: {tests_passed}")
    if tests_failed > 0:
        error(f"Failed: {tests_failed}")
        sys.exit(1)
    else:
        info("All tests passed!")
        sys.exit(0)


if __name__ == "__main__":
    main()
