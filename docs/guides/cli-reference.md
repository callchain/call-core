# CLI Reference

Complete reference for the Call-Core command-line interface.

## Synopsis

```bash
calld [OPTIONS] [COMMAND]
```

## Global Options

| Option | Description |
|--------|-------------|
| `-c, --config <PATH>` | Path to configuration file |
| `-d, --datadir <PATH>` | Data directory path |
| `-v, --verbose` | Enable verbose logging |
| `-q, --quiet` | Suppress output |
| `--log-level <LEVEL>` | Set log level: trace, debug, info, warn, error |
| `--version` | Print version information |
| `-h, --help` | Print help information |

## Commands

### `run`

Start the Call-Core node (default command).

```bash
calld run [OPTIONS]
```

Options:
- `--validate` - Run as validator
- `--testnet` - Connect to testnet
- `--devnet` - Connect to devnet
- `--standalone` - Run in standalone mode
- `--offline` - Run without network

Examples:
```bash
# Start with default config
calld

# Start as validator
calld run --validate

# Start on testnet
calld run --testnet

# Start with custom config
calld run --config /path/to/config.toml
```

### `wallet-generate`

Generate a new wallet with random seed.

```bash
calld wallet-generate [OPTIONS]
```

Options:
- `--word-count <COUNT>` - Mnemonic word count: 12, 15, 18, 21, 24 (default: 24)
- `--key-type <TYPE>` - Key type: secp256k1, ed25519 (default: secp256k1)
- `--passphrase <PHRASE>` - Optional passphrase
- `--output <FORMAT>` - Output format: text, json

Examples:
```bash
# Generate standard wallet
calld wallet-generate

# Generate 12-word mnemonic wallet
calld wallet-generate --word-count 12

# Generate Ed25519 wallet
calld wallet-generate --key-type ed25519

# Generate with passphrase
calld wallet-generate --passphrase "my secret phrase"

# JSON output
calld wallet-generate --output json
```

Output:
```
Account ID: cLSKzJZg4w2dgLfwf
Master Seed: sn3nxiW7v8KXzPzA8wJcrZ7J2r4qz
Mnemonic: abandon abandon abandon ... art
Public Key: n9JZF7Q5K7U7ZQZ3dCykFbNnSYoG7J8
Key Type: secp256k1
```

### `wallet-from-mnemonic`

Derive wallet from BIP39 mnemonic phrase.

```bash
calld wallet-from-mnemonic [OPTIONS] <MNEMONIC>
```

Options:
- `--passphrase <PHRASE>` - BIP39 passphrase
- `--account-index <INDEX>` - Account index for BIP44 derivation (default: 0)
- `--key-type <TYPE>` - Key type: secp256k1, ed25519

Examples:
```bash
# Derive from mnemonic
calld wallet-from-mnemonic "abandon abandon abandon ... art"

# With passphrase
calld wallet-from-mnemonic "abandon abandon ..." --passphrase "secret"

# Derive account #5
calld wallet-from-mnemonic "abandon ..." --account-index 5
```

### `validation-create`

Generate validator keys.

```bash
calld validation-create [OPTIONS]
```

Options:
- `--key-type <TYPE>` - Key type: secp256k1, ed25519

Examples:
```bash
# Generate validator keys
calld validation-create

# Generate Ed25519 validator keys
calld validation-create --key-type ed25519
```

Output:
```
Validation Seed: sn3nxiW7v8KXzPzA8wJcrZ7J2r4qz
Validation Public Key: n9JZF7Q5K7U7ZQZ3dCykFbNnSYoG7J8
Validation Private Key: paKVx...
Key Type: secp256k1
```

### `validation-seed`

Derive validation keys from seed.

```bash
calld validation-seed [OPTIONS] <SEED>
```

Examples:
```bash
# Derive from seed
calld validation-seed sn3nxiW7v8KXzPzA8wJcrZ7J2r4qz
```

### `sign`

Sign a transaction.

```bash
calld sign [OPTIONS] <SECRET> <TX_JSON>
```

Options:
- `--offline` - Skip validation
- `--key-type <TYPE>` - Key type override

Examples:
```bash
# Sign a payment
calld sign sn3nxiW7v8KXzPzA '{"TransactionType":"Payment","Account":"cLSKz...","Destination":"cN5E7...","Amount":"1000000"}'

# Sign from file
calld sign sn3nxiW7v8KXzPzA "$(cat tx.json)"
```

Output:
```json
{
  "tx_blob": "120000228000000024000000016140...",
  "tx_json": {
    "Account": "cLSKzJZg4w2dgLfwf",
    "Amount": "1000000",
    "Destination": "cN5E7s8x9y2z3w4v5u6t",
    "Fee": "10",
    "Sequence": 1,
    "SigningPubKey": "0330E7FC9D56BB25...",
    "TransactionType": "Payment",
    "TxnSignature": "3045022100..."
  }
}
```

### `verify`

Verify a transaction signature.

```bash
calld verify <TX_BLOB>
```

Examples:
```bash
# Verify signed transaction
calld verify 120000228000000024000000016140...
```

Output:
```
Signature valid: true
Account: cLSKzJZg4w2dgLfwf
Transaction hash: A1B2C3D4...
```

### `submit`

Submit a transaction to the network.

```bash
calld submit [OPTIONS] <SERVER> <TX_BLOB>
```

Options:
- `--wait` - Wait for validation
- `--timeout <SECONDS>` - Wait timeout (default: 60)

Examples:
```bash
# Submit to local node
calld submit http://localhost:5005 120000228000000024...

# Submit and wait for validation
calld submit http://localhost:5005 120000228000000024... --wait

# Submit with custom timeout
calld submit http://localhost:5005 120000... --wait --timeout 120
```

### `account-info`

Query account information.

```bash
calld account-info [OPTIONS] <SERVER> <ACCOUNT>
```

Options:
- `--ledger <LEDGER>` - Ledger index or hash (default: "current")

Examples:
```bash
# Get current account info
calld account-info http://localhost:5005 cLSKzJZg4w2dgLfwf

# Get validated account info
calld account-info http://localhost:5005 cLSKzJZg4w2dgLfwf --ledger validated
```

### `account-lines`

Query account trust lines.

```bash
calld account-lines [OPTIONS] <SERVER> <ACCOUNT>
```

Options:
- `--peer <PEER>` - Filter by counterparty
- `--ledger <LEDGER>` - Ledger to query

Examples:
```bash
# Get all trust lines
calld account-lines http://localhost:5005 cLSKzJZg4w2dgLfwf

# Filter by issuer
calld account-lines http://localhost:5005 cLSKzJZg4w2dgLfwf --peer cG6vVq8oTo1R3mYRg
```

### `account-tx`

Query account transaction history.

```bash
calld account-tx [OPTIONS] <SERVER> <ACCOUNT>
```

Options:
- `--limit <N>` - Maximum results (default: 20)
- `--forward` - Oldest first
- `--marker <MARKER>` - Pagination marker

Examples:
```bash
# Get recent transactions
calld account-tx http://localhost:5005 cLSKzJZg4w2dgLfwf

# Get more transactions
calld account-tx http://localhost:5005 cLSKzJZg4w2dgLfwf --limit 100

# Paginate results
calld account-tx http://localhost:5005 cLSKzJZg4w2dgLfwf --limit 100 --marker "..."
```

### `ledger`

Query ledger information.

```bash
calld ledger [OPTIONS] <SERVER>
```

Options:
- `--ledger <LEDGER>` - Ledger index or hash (default: "validated")
- `--full` - Include full ledger
- `--tx` - Include transactions
- `--expand` - Expand transaction JSON

Examples:
```bash
# Get latest validated ledger
calld ledger http://localhost:5005

# Get specific ledger
calld ledger http://localhost:5005 --ledger 12345

# Get ledger with transactions
calld ledger http://localhost:5005 --ledger validated --tx --expand
```

### `tx`

Query transaction information.

```bash
calld tx [OPTIONS] <SERVER> <HASH>
```

Options:
- `--binary` - Return binary format

Examples:
```bash
# Get transaction details
calld tx http://localhost:5005 E08D6E9754025BA2534A787A0DA63A3A8F08E3A98D2E
```

### `book-offers`

Query order book offers.

```bash
calld book-offers [OPTIONS] <SERVER> <TAKER_GETS> <TAKER_PAYS>
```

Options:
- `--limit <N>` - Maximum results (default: 100)
- `--taker <ACCOUNT>` - Taker account

Examples:
```bash
# Get CALL/USD book
calld book-offers http://localhost:5005 "CALL" "USD:cG6vVq8oTo1R3mYRg"

# Get USD/EUR book
calld book-offers http://localhost:5005 "USD:cG6vVq8oTo1R3mYRg" "EUR:cBank1234567890"
```

### `ping`

Ping server for connectivity check.

```bash
calld ping [SERVER]
```

Examples:
```bash
# Ping local server
calld ping

# Ping remote server
calld ping https://api.callchain.io
```

### `server-info`

Get server information.

```bash
calld server-info [SERVER]
```

Examples:
```bash
# Get local server info
calld server-info

# Get remote server info
calld server-info https://api.callchain.io
```

### `consensus-info`

Get consensus information (admin only).

```bash
calld consensus-info [SERVER]
```

Examples:
```bash
# Get consensus info
calld consensus-info http://localhost:5005
```

### `peers`

Get peer information (admin only).

```bash
calld peers [SERVER]
```

### `stop`

Shutdown the server (admin only).

```bash
calld stop [SERVER]
```

### `ledger-accept`

Manually close ledger (admin/standalone only).

```bash
calld ledger-accept [SERVER]
```

Examples:
```bash
# Close current ledger in standalone mode
calld ledger-accept http://localhost:5005
```

### `check`

Validate configuration file.

```bash
calld check [OPTIONS]
```

Options:
- `--config <PATH>` - Config file to check

Examples:
```bash
# Check default config
calld check

# Check specific config
calld check --config /path/to/config.toml
```

Output:
```
Configuration valid: true
Warnings: 0
Errors: 0
```

### `import`

Import ledger data.

```bash
calld import [OPTIONS] <PATH>
```

Options:
- `--validate` - Validate imported data
- `--resume` - Resume interrupted import

Examples:
```bash
# Import ledger archive
calld import /path/to/ledger.tar.gz

# Validate while importing
calld import /path/to/ledger.tar.gz --validate
```

### `export`

Export ledger data.

```bash
calld export [OPTIONS] <START> <END> <OUTPUT>
```

Options:
- `--format <FORMAT>` - Export format: tar, json (default: tar)

Examples:
```bash
# Export ledgers 1-1000
calld export 1 1000 /path/to/export.tar.gz

# Export as JSON
calld export 1 1000 /path/to/export.json --format json
```

### `genesis`

Create or validate genesis ledger.

```bash
calld genesis [OPTIONS]
```

Options:
- `--create` - Create new genesis
- `--validate <FILE>` - Validate genesis file
- `--output <FILE>` - Output file

Examples:
```bash
# Create genesis ledger
calld genesis --create --output genesis.json

# Validate genesis file
calld genesis --validate genesis.json
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `CALLCORE_CONFIG` | Default config file path |
| `CALLCORE_DATADIR` | Default data directory |
| `CALLCORE_LOG_LEVEL` | Default log level |
| `CALLCORE_NO_COLOR` | Disable colored output |

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments |
| 3 | Configuration error |
| 4 | Network error |
| 5 | RPC error |
| 6 | Transaction failed |
| 7 | Validation error |
| 130 | Interrupted (Ctrl+C) |

## Examples

### Create and Submit Payment

```bash
#!/bin/bash

# Configuration
ACCOUNT="cLSKzJZg4w2dgLfwf"
SECRET="sn3nxiW7v8KXzPzA8wJcrZ7J2r4qz"
DESTINATION="cN5E7s8x9y2z3w4v5u6t"
AMOUNT="1000000"  # 1 CALL
SERVER="http://localhost:5005"

# Create and sign transaction
SIGNED=$(calld sign "$SECRET" "{
  \"TransactionType\": \"Payment\",
  \"Account\": \"$ACCOUNT\",
  \"Destination\": \"$DESTINATION\",
  \"Amount\": \"$AMOUNT\"
}")

# Extract tx_blob
TX_BLOB=$(echo "$SIGNED" | jq -r '.tx_blob')

# Submit
calld submit "$SERVER" "$TX_BLOB" --wait
```

### Monitor Account

```bash
#!/bin/bash

ACCOUNT="cLSKzJZg4w2dgLfwf"
SERVER="http://localhost:5005"

while true; do
    INFO=$(calld account-info "$SERVER" "$ACCOUNT")
    BALANCE=$(echo "$INFO" | jq -r '.result.account_data.Balance')
    SEQUENCE=$(echo "$INFO" | jq -r '.result.account_data.Sequence')
    LEDGER=$(echo "$INFO" | jq -r '.result.ledger_current_index')

    echo "$(date): Balance=$BALANCE Sequence=$SEQUENCE Ledger=$LEDGER"
    sleep 5
done
```

### Check Validator Status

```bash
#!/bin/bash

SERVER="http://localhost:5005"

# Get server info
INFO=$(calld server-info "$SERVER")
STATE=$(echo "$INFO" | jq -r '.result.info.server_state')
PUBKEY=$(echo "$INFO" | jq -r '.result.info.pubkey_node')
LEDGER=$(echo "$INFO" | jq -r '.result.info.validated_ledger.seq')

# Get peer count
PEERS=$(calld peers "$SERVER" | jq '.result.peers | length')

echo "Validator Status:"
echo "  State: $STATE"
echo "  Public Key: $PUBKEY"
echo "  Validated Ledger: $LEDGER"
echo "  Connected Peers: $PEERS"
```

## Shell Completion

Generate shell completion scripts:

```bash
# Bash
calld completion bash > /etc/bash_completion.d/calld

# Zsh
calld completion zsh > /usr/local/share/zsh/site-functions/_calld

# Fish
calld completion fish > ~/.config/fish/completions/calld.fish
```

## See Also

- [Configuration Guide](configuration.md) - Configuration options
- [Validator Setup](validator-setup.md) - Running a validator
- [RPC API Reference](../api/rpc.md) - HTTP API methods
