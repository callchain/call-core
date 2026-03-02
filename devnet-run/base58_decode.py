#!/usr/bin/env python3
"""Decode Callchain base58 addresses to hex account IDs."""

import hashlib

# Callchain base58 alphabet
CALLCHAIN_ALPHABET = b'cpshnaf39wBUDNEGHJKLM4PQRST7VWXYZ2brdeCg65jkm8oFqi1tuvAxyz'

def base58_decode(data):
    """Decode base58 string to bytes."""
    # Create alphabet mapping
    alphabet = CALLCHAIN_ALPHABET.decode('ascii')
    alphabet_map = {char: idx for idx, char in enumerate(alphabet)}

    # Convert base58 string to big integer
    num = 0
    for char in data:
        num = num * 58 + alphabet_map[char]

    # Convert to bytes
    result = []
    while num > 0:
        num, rem = divmod(num, 256)
        result.append(rem)

    # Reverse to get correct byte order
    result = bytes(reversed(result))

    # Add leading zero bytes (represented as 'c' in base58)
    for char in data:
        if char == 'c':
            result = b'\x00' + result
        else:
            break

    return result

def decode_address(address):
    """Decode Callchain address to account ID (hex)."""
    decoded = base58_decode(address)

    # Format: version (1 byte) + account_id (20 bytes) + checksum (4 bytes)
    if len(decoded) != 25:
        raise ValueError(f"Invalid address length: {len(decoded)}")

    version = decoded[0]
    account_id = decoded[1:21]
    checksum = decoded[21:25]

    return account_id.hex()

if __name__ == "__main__":
    addresses = [
        "cBS7WQWHYS9CVJgC8wWtyKJiz5bAARq58Z",
        "cEECbEqxuEsBdsSGtR2UXpus8omfg4a8q4",
        "csfx7Ps529AVMxkcY8g6DgDGokBEKY99Gg",
        "c3mPDgpp9VvWTgwNyUGsDJhgJbxtruFEFA",
        "cU38eXc8dzvACdECq5SeatvB3eqAJTKrhE",
    ]

    print("Genesis Account Hex IDs:")
    for addr in addresses:
        hex_id = decode_address(addr)
        print(f"  {addr} -> {hex_id}")
