# Configuration Guide

This guide covers all configuration options for Call-Core nodes.

## Configuration File

Call-Core uses TOML format for configuration. Default location: `~/.call-core/call-core.toml`

### Minimal Configuration

```toml
[network]
listen_address = "0.0.0.0:5333"

[node]
# Run as a stock node (non-validator)
```

### Validator Configuration

```toml
[network]
listen_address = "0.0.0.0:5333"
bootstrap_peers = [
    "seed1.callchain.io:5333",
    "seed2.callchain.io:5333"
]

[node]
validation_seed = "sn3nxiW7v8KXzPzA8wJcrZ7J2r4qz"
validation_public_key = "n9JZF7Q5K7U7ZQZ3dCykFbNnSYoG7J8"

[consensus]
validator_list_sites = ["https://vl.callchain.io"]
validation_quorum = 80

[rpc]
enabled = true
address = "127.0.0.1:5005"
admin_only = true

[websocket]
enabled = true
address = "127.0.0.1:6006"
```

## Section Reference

### [node]

General node settings.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `validation_seed` | string | (none) | Secret seed for validator (starts with 's') |
| `validation_public_key` | string | (none) | Public validation key (starts with 'n') |
| `database_path` | string | "~/.call-core/db" | Ledger database location |
| `log_level` | string | "info" | Log level: debug, info, warn, error |
| `log_file` | string | (stdout) | Log file path |
| `max_peers` | uint | 50 | Maximum peer connections |
| `min_peers` | uint | 5 | Minimum peer connections to maintain |

### [network]

P2P network configuration.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `listen_address` | string | (required) | Bind address for P2P (e.g., "0.0.0.0:5333") |
| `bootstrap_peers` | array | [] | Initial peers to connect to |
| `public_address` | string | (auto) | Publicly advertised address |
| `allow_inbound` | bool | true | Accept incoming connections |
| `max_inbound` | uint | 20 | Maximum inbound connections |
| `max_outbound` | uint | 10 | Maximum outbound connections |
| `handshake_timeout` | uint | 10 | Handshake timeout in seconds |
| `ping_interval` | uint | 30 | Ping interval in seconds |

### [consensus]

Consensus algorithm settings.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `validator_list_sites` | array | [] | URLs to fetch validator lists |
| `validation_quorum` | uint | 80 | Required validator agreement % |
| `ledger_close_time` | uint | 5 | Target seconds between ledgers |
| `max_transactions` | uint | 5000 | Max transactions per ledger |
| `proposal_timeout` | uint | 5 | Seconds to wait for proposals |
| `validation_timeout` | uint | 2 | Seconds to wait for validations |

### [rpc]

HTTP RPC API configuration.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | bool | true | Enable RPC server |
| `address` | string | "127.0.0.1:5005" | Bind address |
| `admin_only` | bool | false | Only accept admin commands |
| `cors_allowed` | array | [] | Allowed CORS origins |
| `rate_limit` | uint | 100 | Requests per minute |

### [websocket]

WebSocket API configuration.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | bool | true | Enable WebSocket server |
| `address` | string | "127.0.0.1:6006" | Bind address |
| `admin_only` | bool | false | Only accept admin commands |
| `max_subscriptions` | uint | 100 | Max subscriptions per connection |
| `ping_interval` | uint | 30 | Ping interval in seconds |

### [ledger]

Ledger storage configuration.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `database_type` | string | "rocksdb" | Database backend |
| `path` | string | "~/.call-core/ledger" | Database path |
| `cache_size_mb` | uint | 256 | Cache size in MB |
| `sync_interval` | uint | 100 | Ledgers between syncs |
| `online_delete` | uint | 0 | Ledgers to keep (0 = all) |

### [fees]

Fee configuration (validator voting).

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `base_fee` | uint | 10 | Base transaction fee in drops |
| `reserve_base` | uint | 10000000 | Base reserve in drops |
| `reserve_increment` | uint | 2000000 | Owner reserve in drops |
| `reference_fee` | uint | 10 | Reference transaction fee |

## Environment Variables

Override config file settings with environment variables:

```bash
CALLCORE_NODE_VALIDATION_SEED=sn3nxiW7v8KXzPzA...
CALLCORE_NETWORK_LISTEN_ADDRESS=0.0.0.0:5333
CALLCORE_RPC_ENABLED=true
CALLCORE_LOG_LEVEL=debug
```

Format: `CALLCORE_<SECTION>_<KEY>`

## Command Line Options

```bash
calld [OPTIONS]

Options:
    -c, --config <PATH>     Config file path
    -d, --datadir <PATH>    Data directory
    --validate              Run as validator
    --testnet               Connect to testnet
    --devnet                Connect to devnet
    --offline               Run without network
    --ledger <LEDGER>       Start from specific ledger
    --import                Import existing ledger
    -v, --verbose           Verbose logging
    -h, --help              Print help
    -V, --version           Print version
```

## Network Types

### Mainnet

Production network with real value.

```toml
[network]
bootstrap_peers = [
    "seed1.callchain.io:5333",
    "seed2.callchain.io:5333",
    "seed3.callchain.io:5333"
]

[consensus]
validator_list_sites = ["https://vl.callchain.io"]
validation_quorum = 80
```

### Testnet

Public test network.

```bash
calld --testnet
```

Or in config:

```toml
[network]
bootstrap_peers = [
    "testnet-seed1.callchain.io:5333",
    "testnet-seed2.callchain.io:5333"
]

[node]
testnet = true
```

### Devnet

Development network for local testing.

```bash
calld --devnet
```

### Standalone Mode

For development and testing:

```bash
calld --standalone
```

Features:
- Single node, no consensus
- Instant ledger close on transaction
- Can manually close ledgers via RPC

## Security Configuration

### Admin Access

Restrict admin commands to localhost:

```toml
[rpc]
enabled = true
address = "127.0.0.1:5005"
admin_only = true

[websocket]
enabled = true
address = "127.0.0.1:6006"
admin_only = true
```

### Validator Security

Best practices for validator nodes:

```toml
[node]
# Use a dedicated validator key (not your funded account)
validation_seed = "sVALIDATORSEED..."
validation_public_key = "nVALIDATORKEY..."

# Restrict API access
[rpc]
address = "127.0.0.1:5005"
admin_only = true

# Firewall: Only open P2P port
# iptables -A INPUT -p tcp --dport 5333 -j ACCEPT
# iptables -A INPUT -p tcp --dport 5005 -s 127.0.0.1 -j ACCEPT
```

## Performance Tuning

### High-Throughput Node

```toml
[node]
max_peers = 100
log_level = "warn"

[ledger]
cache_size_mb = 512
sync_interval = 1000

[consensus]
max_transactions = 10000
```

### Low-Resource Node

```toml
[node]
max_peers = 10
min_peers = 2

[ledger]
cache_size_mb = 64
online_delete = 10000

[consensus]
max_transactions = 1000
```

## Monitoring

### Metrics Export

```toml
[metrics]
enabled = true
address = "127.0.0.1:9100"
format = "prometheus"
```

### Health Check

```toml
[health]
enabled = true
address = "127.0.0.1:8080"
```

## Debugging

### Verbose Logging

```toml
[node]
log_level = "debug"
log_file = "/var/log/call-core/debug.log"

[network]
log_peer_traffic = true

[consensus]
log_proposals = true
log_validations = true
```

### Trace Logging

Most verbose (warning: large log files):

```toml
[node]
log_level = "trace"
trace_transactions = true
trace_ledger = true
```

## Common Configurations

### API Node

Serving RPC/WebSocket to clients:

```toml
[network]
listen_address = "0.0.0.0:5333"
max_peers = 50

[rpc]
enabled = true
address = "0.0.0.0:5005"
admin_only = false
cors_allowed = ["https://wallet.callchain.io"]
rate_limit = 1000

[websocket]
enabled = true
address = "0.0.0.0:6006"
admin_only = false
max_subscriptions = 100

[ledger]
online_delete = 100000
```

### Archive Node

Maintaining full history:

```toml
[node]
max_peers = 20

[ledger]
online_delete = 0
sync_interval = 1000
database_type = "rocksdb"

[rpc]
enabled = true
# Allow history queries
```

### Validator Node

Participating in consensus:

```toml
[network]
listen_address = "0.0.0.0:5333"
public_address = "1.2.3.4:5333"
bootstrap_peers = ["seed1.callchain.io:5333"]

[node]
validation_seed = "s..."
validation_public_key = "n..."

[consensus]
validator_list_sites = ["https://vl.callchain.io"]
validation_quorum = 80

[rpc]
admin_only = true
address = "127.0.0.1:5005"

[websocket]
admin_only = true
address = "127.0.0.1:6006"
```

## Validation

Check configuration validity:

```bash
calld --config /path/to/config.toml --check
```

## Reloading

Some settings can be reloaded without restart:

```bash
kill -HUP $(pgrep calld)
```

Reloadable settings:
- Log level
- Peer limits
- Rate limits

Non-reloadable (requires restart):
- Network bind address
- Database path
- Validation keys

## See Also

- [Validator Setup](validator-setup.md) - Running a validator
- [CLI Reference](cli-reference.md) - Command-line options
- [Architecture Overview](../architecture/overview.md) - System design
