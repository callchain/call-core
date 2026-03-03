# Validator Setup Guide

This guide walks through setting up and running a Call-Core validator node.

## Prerequisites

### Hardware Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| CPU | 4 cores | 8+ cores |
| RAM | 16 GB | 32 GB |
| Storage | 500 GB SSD | 1 TB NVMe |
| Network | 100 Mbps | 1 Gbps |

### Software Requirements

- Linux (Ubuntu 22.04 LTS recommended)
- Rust 1.70+ (for building from source)
- OpenSSL 3.0+

## Overview

Validators participate in consensus to validate transactions and create new ledgers. Running a validator requires:

1. **Validation Keys**: Unique key pair for signing proposals
2. **Server Infrastructure**: Reliable hardware and network
3. **Trust**: Other validators must include you in their UNL

## Step 1: Install Call-Core

### Build from Source

```bash
# Install dependencies
sudo apt-get update
sudo apt-get install -y build-essential cmake libssl-dev pkg-config

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Clone and build
git clone https://github.com/callchain/call-core.git
cd call-core
cargo build --release

# Install binary
sudo cp target/release/calld /usr/local/bin/
sudo chmod +x /usr/local/bin/calld
```

### Verify Installation

```bash
calld --version
```

## Step 2: Generate Validation Keys

### Create Validator Keys

```bash
# Generate new validator key pair
calld validation-create

# Output example:
# Validation Seed: sn3nxiW7v8KXzPzA8wJcrZ7J2r4qz
# Validation Public Key: n9JZF7Q5K7U7ZQZ3dCykFbNnSYoG7J8
# Validation Private Key: paKVx...
```

### Secure the Keys

**Critical**: Store the validation seed securely. It cannot be recovered if lost.

```bash
# Create secure directory
sudo mkdir -p /etc/call-core
sudo chmod 700 /etc/call-core

# Store seed (use a proper secret management system in production)
echo "sn3nxiW7v8KXzPzA8wJcrZ7J2r4qz" | sudo tee /etc/call-core/validator.seed
sudo chmod 600 /etc/call-core/validator.seed
```

**Security Recommendations**:
- Never share the validation seed
- Use hardware security modules (HSM) if possible
- Separate validator key from funded accounts
- Regular key rotation schedule

## Step 3: Create Data Directory

```bash
# Create data directory
sudo mkdir -p /var/lib/call-core
sudo chown -R callcore:callcore /var/lib/call-core

# Create config directory
sudo mkdir -p /etc/call-core
```

## Step 4: Configure the Validator

### Basic Configuration

Create `/etc/call-core/call-core.toml`:

```toml
[network]
listen_address = "0.0.0.0:5333"
public_address = "YOUR_PUBLIC_IP:5333"
bootstrap_peers = [
    "seed1.callchain.io:5333",
    "seed2.callchain.io:5333",
    "seed3.callchain.io:5333"
]
max_peers = 50

[node]
validation_seed = "sn3nxiW7v8KXzPzA8wJcrZ7J2r4qz"
validation_public_key = "n9JZF7Q5K7U7ZQZ3dCykFbNnSYoG7J8"
database_path = "/var/lib/call-core"
log_level = "info"
log_file = "/var/log/call-core/call-core.log"

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
admin_only = true

[ledger]
cache_size_mb = 512
sync_interval = 100
```

Replace `YOUR_PUBLIC_IP` with your server's public IP address.

### Testnet Configuration

For testing on testnet:

```toml
[network]
bootstrap_peers = [
    "testnet-seed1.callchain.io:5333",
    "testnet-seed2.callchain.io:5333"
]

[node]
validation_seed = "sTESTNETSEED..."
validation_public_key = "nTESTNETPUBKEY..."

[consensus]
validator_list_sites = ["https://testnet-vl.callchain.io"]
validation_quorum = 50
```

## Step 5: Create Systemd Service

Create `/etc/systemd/system/call-core.service`:

```ini
[Unit]
Description=Call-Core Validator Node
After=network.target

[Service]
Type=simple
User=callcore
Group=callcore
ExecStart=/usr/local/bin/calld --config /etc/call-core/call-core.toml
Restart=always
RestartSec=10
LimitNOFILE=65536

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/call-core /var/log/call-core

[Install]
WantedBy=multi-user.target
```

### Create User

```bash
# Create callcore user
sudo useradd -r -s /bin/false -d /var/lib/call-core callcore

# Set permissions
sudo chown -R callcore:callcore /var/lib/call-core /etc/call-core
sudo mkdir -p /var/log/call-core
sudo chown callcore:callcore /var/log/call-core
```

### Start Service

```bash
# Reload systemd
sudo systemctl daemon-reload

# Enable and start service
sudo systemctl enable call-core
sudo systemctl start call-core

# Check status
sudo systemctl status call-core
```

### View Logs

```bash
# Follow logs
sudo journalctl -u call-core -f

# View recent logs
sudo journalctl -u call-core -n 100
```

## Step 6: Verify Validator Operation

### Check Server State

```bash
# Get server info
curl -X POST http://localhost:5005/ \
  -H "Content-Type: application/json" \
  -d '{"method": "server_info"}'
```

Look for:
- `"server_state": "full"` or `"proposing"` - Validator is participating
- `"validation_quorum": 80` - Consensus threshold
- `"complete_ledgers": "1-12345"` - Sync status

### Check Validations

```bash
# Subscribe to validations (in a separate terminal)
wscat -c ws://localhost:6006/
> {"id": 1, "command": "subscribe", "streams": ["validations"]}
```

You should see your validator's public key in validation messages.

### Check Peer Connections

```bash
curl -X POST http://localhost:5005/ \
  -H "Content-Type: application/json" \
  -d '{"method": "peers"}'
```

## Step 7: Firewall Configuration

### UFW (Ubuntu)

```bash
# Allow P2P port
sudo ufw allow 5333/tcp

# Allow RPC/WebSocket from localhost only
sudo ufw allow from 127.0.0.1 to any port 5005
sudo ufw allow from 127.0.0.1 to any port 6006

# Enable firewall
sudo ufw enable
```

### iptables

```bash
# P2P
sudo iptables -A INPUT -p tcp --dport 5333 -j ACCEPT

# Local API access only
sudo iptables -A INPUT -p tcp -s 127.0.0.1 --dport 5005 -j ACCEPT
sudo iptables -A INPUT -p tcp -s 127.0.0.1 --dport 6006 -j ACCEPT

# Drop other API access
sudo iptables -A INPUT -p tcp --dport 5005 -j DROP
sudo iptables -A INPUT -p tcp --dport 6006 -j DROP

# Save rules
sudo iptables-save > /etc/iptables/rules.v4
```

## Step 8: Monitoring

### Prometheus Metrics

Add to config:

```toml
[metrics]
enabled = true
address = "127.0.0.1:9100"
format = "prometheus"
```

Key metrics to monitor:
- `callcore_consensus_validations_total` - Validations published
- `callcore_network_peers_connected` - Peer connections
- `callcore_ledger_sequence` - Current ledger
- `callcore_server_state` - Server state code

### Alerting Rules

Example Prometheus alerts:

```yaml
groups:
  - name: call-core
    rules:
      - alert: ValidatorNotProposing
        expr: callcore_server_state != 5
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Validator not proposing"

      - alert: LowPeerConnections
        expr: callcore_network_peers_connected < 5
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Low peer connections"
```

### Health Check Endpoint

```bash
curl http://localhost:8080/health
```

## Step 9: Backup

### Important Data to Backup

1. **Validation Seed** - Store offline, multiple secure locations
2. **Ledger Database** - Can be rebuilt, but backup saves time
3. **Configuration** - `/etc/call-core/call-core.toml`

### Automated Backup Script

```bash
#!/bin/bash
# /usr/local/bin/backup-call-core.sh

BACKUP_DIR="/backup/call-core/$(date +%Y%m%d)"
mkdir -p "$BACKUP_DIR"

# Stop node temporarily
sudo systemctl stop call-core

# Backup ledger
sudo tar czf "$BACKUP_DIR/ledger.tar.gz" -C /var/lib/call-core .

# Backup config
sudo cp /etc/call-core/call-core.toml "$BACKUP_DIR/"

# Restart node
sudo systemctl start call-core

# Sync to remote storage
rsync -avz "$BACKUP_DIR/" backup-server:/backups/call-core/
```

Add to crontab:

```bash
0 2 * * * /usr/local/bin/backup-call-core.sh
```

## Step 10: Join the UNL

To be a trusted validator, other validators must add you to their Unique Node List (UNL).

### Register Your Validator

1. **Submit to Validator Registry**:
   - Website: https://validators.callchain.io
   - Provide: Public key, domain, location

2. **Publish Domain Verification**:
   - Add TXT record to your domain:
     ```
     call-core-validation=n9JZF7Q5K7U7ZQZ3dCykFbNnSYoG7J8
     ```

3. **Wait for Inclusion**:
   - Other validators will review your application
   - Inclusion depends on reputation and diversity

### Validator List Sites

Your validator will be included in:
- `https://vl.callchain.io` - Main validator list
- Third-party lists maintained by other operators

## Maintenance

### Regular Updates

```bash
# Update Call-Core
cd /opt/call-core
git pull origin main
cargo build --release
sudo cp target/release/calld /usr/local/bin/

# Restart service
sudo systemctl restart call-core
```

### Key Rotation

1. Generate new validation keys
2. Update configuration
3. Restart validator
4. Update domain verification
5. Notify other validators

### Troubleshooting

**Issue**: Validator not proposing
- Check: `server_info` shows "proposing" or "full"
- Verify: Validation keys are correct
- Check: Peer connections (minimum 2)

**Issue**: Low consensus participation
- Verify: In other validators' UNLs
- Check: Network connectivity
- Monitor: Validation messages being received

**Issue**: High missed validations
- Check: System resources (CPU, memory, disk I/O)
- Verify: Network latency to other validators
- Monitor: Clock synchronization (use NTP)

## Security Best Practices

1. **Physical Security**: Lock server in secure datacenter
2. **Network Security**: Firewall, DDoS protection
3. **Key Security**: HSM or secure key storage
4. **Access Control**: Minimal SSH access, key-based auth
5. **Monitoring**: Alert on anomalies
6. **Updates**: Keep system and Call-Core updated

## See Also

- [Configuration Guide](configuration.md) - All configuration options
- [Consensus Algorithm](../consensus/algorithm.md) - How consensus works
- [Architecture Overview](../architecture/overview.md) - System design
