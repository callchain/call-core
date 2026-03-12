#!/usr/bin/env python3
"""
Generate genesis configuration with known seeds for testing.

This creates deterministic wallets with known seeds so we can
sign transactions for testing all transaction types.
"""

import json
import os
import subprocess
import sys

# Get the project root directory (parent of devnet-run)
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.dirname(SCRIPT_DIR)

def generate_wallet_from_seed(seed):
    """Generate wallet info from a seed using the calld binary."""
    result = subprocess.run(
        ["./target/release/calld", "validate-seed", seed],
        capture_output=True,
        text=True,
        cwd=PROJECT_ROOT
    )
    # Parse the output to get address and public key
    lines = result.stdout.strip().split('\n')
    address = None
    pubkey = None
    for line in lines:
        if line.startswith("Address:"):
            address = line.split(":")[1].strip()
        elif line.startswith("Public Key:"):
            pubkey = line.split(":")[1].strip()
    return address, pubkey

def main():
    # Known seeds for genesis accounts - these are deterministic test seeds
    genesis_seeds = [
        ("ssC6ZiXrYVDH7B2A5BCRRgiBEM8zE", "100000000000"),  # Account 1: 100k CALL
        ("ss7YQuGGj7if7rUqCxgqjFwDbM2Nb", "50000000000"),   # Account 2: 50k CALL
        ("ssYc8cMDDzpaYJCtgpGuVd7DPCvsh", "25000000000"),   # Account 3: 25k CALL
        ("ssRbEP5N6ri4Jnf1BdqkYZwnfgp5e", "10000000000"),   # Account 4: 10k CALL
        ("ss1KTKbvKWcbgXW1SF3N3MF6ZWQ7J", "5000000000"),    # Account 5: 5k CALL
    ]

    allocations = {}
    wallet_info = []

    print("Generating genesis accounts from known seeds...\n")

    for i, (seed, balance) in enumerate(genesis_seeds, 1):
        result = subprocess.run(
            ["./target/release/calld", "validate-seed", seed],
            capture_output=True,
            text=True,
            cwd=PROJECT_ROOT
        )

        # Parse output
        address = None
        pubkey = None
        for line in result.stdout.strip().split('\n'):
            if line.startswith("Address:"):
                address = line.split(":", 1)[1].strip()
            elif line.startswith("Public Key:"):
                pubkey = line.split(":", 1)[1].strip()

        if address:
            allocations[address] = {
                "balance": balance,
                "sequence": 1,
                "flags": 0
            }
            wallet_info.append({
                "name": f"Genesis Account {i}",
                "seed": seed,
                "address": address,
                "public_key": pubkey,
                "balance": balance
            })
            print(f"Account {i}: {address} - {balance} drops")
        else:
            print(f"ERROR: Failed to generate account {i}")
            print(f"stdout: {result.stdout}")
            print(f"stderr: {result.stderr}")
            sys.exit(1)

    # Create genesis config
    genesis_config = {
        "config": {
            "chainId": 1,
            "networkName": "callchain-devnet",
            "genesisTime": "2025-01-01T00:00:00Z",
            "consensusParams": {
                "ledgerMinCloseTime": 2,
                "ledgerMaxCloseTime": 20,
                "ledgerMinConsensus": 1,
                "ledgerMaxConsensus": 50,
                "validationQuorum": 1,
                "minProposeTime": 3,
                "maxProposeTime": 30
            },
            "feeSettings": {
                "baseFee": 10,
                "reserveBase": 10000000,
                "reserveIncrement": 2000000
            }
        },
        "alloc": allocations,
        "validators": [
            {
                "nodeId": "NodeID(0000000000000000000000000000000000000000000000000000000000000000)",
                "publicKey": "0330E7FC9D56BB25D6893BA3F317AE5BCF33B3291BD63DB32654A313222F7FD020",
                "domain": "validator1.callchain.local"
            }
        ],
        "coinbase": list(allocations.keys())[0]
    }

    # Save genesis.json
    with open("genesis.json", "w") as f:
        json.dump(genesis_config, f, indent=2)

    # Save wallet secrets
    wallet_doc = {
        "note": "GENESIS WALLET SECRETS - FOR TESTING ONLY",
        "wallets": wallet_info
    }
    with open("genesis_wallets.json", "w") as f:
        json.dump(wallet_doc, f, indent=2)

    print("\n✓ Generated genesis.json with 5 funded accounts")
    print("✓ Generated genesis_wallets.json with wallet secrets")
    print("\nAccount Summary:")
    for w in wallet_info:
        print(f"  {w['name']}: {w['address']} ({w['balance']} drops)")
        print(f"    Seed: {w['seed']}")

if __name__ == "__main__":
    main()
