use clap::{Parser, Subcommand};
use node::{Application, Config};
use std::net::SocketAddr;
use std::path::PathBuf;
use tracing::{info, error};

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Make a JSON-RPC request to the node
async fn rpc_request(
    rpc_url: &str,
    method: &str,
    params: Option<serde_json::Value>,
) -> anyhow::Result<serde_json::Value> {
    let client = reqwest::Client::new();

    let request_body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": 1
    });

    let response = client
        .post(rpc_url)
        .json(&request_body)
        .send()
        .await?;

    let response_json: serde_json::Value = response.json().await?;

    // Check for RPC error
    if let Some(error) = response_json.get("error") {
        let error_msg = error.get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown error");
        return Err(anyhow::anyhow!("RPC error: {}", error_msg));
    }

    Ok(response_json.get("result").cloned().unwrap_or(serde_json::json!({})))
}

fn parse_log_level(level: &str) -> tracing::Level {
    match level.to_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    }
}

#[derive(Parser)]
#[command(name = "call-core")]
#[command(about = "Callchain reference implementation in Rust")]
#[command(version)]
struct Cli {
    /// Configuration file path
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Data directory
    #[arg(short, long, value_name = "DIR")]
    data_dir: Option<PathBuf>,

    /// RPC port
    #[arg(long, value_name = "PORT")]
    rpc_port: Option<u16>,

    /// P2P port
    #[arg(long, value_name = "PORT")]
    peer_port: Option<u16>,

    /// Bootstrap peers (comma-separated)
    #[arg(short, long, value_name = "PEERS")]
    peers: Option<String>,

    /// Validation seed (for validator nodes)
    #[arg(long, value_name = "SEED")]
    validation_seed: Option<String>,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: String,

    /// Subcommand
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the node (default)
    Start,

    /// Generate a new validation seed
    GenerateSeed,

    /// Validate a seed
    ValidateSeed {
        /// Seed to validate
        seed: String,
    },

    /// Show ledger information
    LedgerInfo {
        /// Ledger index (or "current" or "validated")
        #[arg(default_value = "current")]
        ledger: String,
    },

    /// Submit a transaction
    Submit {
        /// Transaction blob (hex)
        tx_blob: String,
    },

    /// Account information
    AccountInfo {
        /// Account address
        account: String,
    },

    /// Generate a new wallet
    GenerateWallet,

    /// Sign a transaction
    Sign {
        /// Secret (seed or hex private key)
        #[arg(short, long)]
        secret: String,

        /// Transaction JSON (as string)
        #[arg(short, long)]
        tx_json: String,
    },

    /// Derive Callchain accounts from BIP39/BIP44 mnemonic
    DeriveFromMnemonic {
        /// Mnemonic phrase (BIP39)
        #[arg(short, long)]
        mnemonic: String,

        /// Number of accounts to derive
        #[arg(short, long, default_value = "5")]
        count: u32,

        /// Output format (json or text)
        #[arg(short, long, default_value = "json")]
        format: String,
    },

    /// Sign a transaction locally (no RPC required)
    SignLocal {
        /// Private key (hex) or mnemonic-derived path (e.g., "mnemonic:0/0/0")
        #[arg(short, long)]
        key: String,

        /// Transaction JSON file path or inline JSON
        #[arg(short, long)]
        tx: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(parse_log_level(&cli.log_level))
        .init();

    match cli.command {
        Some(Commands::GenerateSeed) => {
            generate_seed();
            return Ok(());
        }
        Some(Commands::ValidateSeed { seed }) => {
            validate_seed(&seed);
            return Ok(());
        }
        Some(Commands::GenerateWallet) => {
            generate_wallet();
            return Ok(());
        }
        Some(Commands::Sign { secret, tx_json }) => {
            sign_transaction_cli(&secret, &tx_json).await?;
            return Ok(());
        }
        Some(Commands::DeriveFromMnemonic { mnemonic, count, format }) => {
            derive_from_mnemonic(&mnemonic, count, &format)?;
            return Ok(());
        }
        Some(Commands::SignLocal { key, tx }) => {
            sign_transaction_local(&key, &tx)?;
            return Ok(());
        }
        _ => {}
    }

    // Build configuration
    let mut config = if let Some(config_path) = &cli.config {
        Config::from_file(config_path.to_str().unwrap())?
    } else {
        Config::new()
    };

    // Override config with CLI arguments
    if let Some(data_dir) = cli.data_dir {
        config.data_dir = data_dir.to_string_lossy().to_string();
    }
    if let Some(rpc_port) = cli.rpc_port {
        config.rpc_port = rpc_port;
    }
    if let Some(peer_port) = cli.peer_port {
        config.listen_address = format!("0.0.0.0:{}", peer_port).parse()?;
    }
    if let Some(peers) = cli.peers {
        for peer in peers.split(',') {
            if let Ok(addr) = peer.parse::<SocketAddr>() {
                config.add_peer(addr);
            }
        }
    }
    if let Some(seed) = cli.validation_seed {
        config.set_validator(seed);
    }
    config.log_level = cli.log_level;

    match cli.command {
        Some(Commands::LedgerInfo { ledger }) => {
            show_ledger_info(&config, &ledger).await?;
        }
        Some(Commands::Submit { tx_blob }) => {
            submit_transaction(&config, &tx_blob).await?;
        }
        Some(Commands::AccountInfo { account }) => {
            show_account_info(&config, &account).await?;
        }
        _ => {
            // Default: start the node
            start_node(config).await?;
        }
    }

    Ok(())
}

fn generate_seed() {
    generate_wallet();
}

fn generate_wallet() {
    // Use the proper wallet generation
    let wallet = crypto::Wallet::generate();

    println!("Generated new Callchain wallet:");
    println!("  Address:    {}", wallet.address);
    println!("  Public Key: {}", wallet.public_key);
    println!("  Seed:       {}", wallet.seed);
    println!("\nIMPORTANT: Store the seed securely!");
    println!("This is the only way to recover your wallet.");
}

fn validate_seed(seed: &str) {
    use crypto::validate_seed_format;
    use crypto::wallet::decode_seed;

    println!("Validating seed: {}", seed);

    // Check seed starts with 's'
    if !seed.starts_with('s') {
        println!("⚠ Warning: Seed should start with 's' for Callchain");
    }

    // Validate the seed format
    if !validate_seed_format(seed) {
        println!("❌ Invalid seed: bad format or checksum");
        return;
    }

    match decode_seed(seed) {
        Some(entropy) => {
            println!("✓ Valid seed");
            println!("  Entropy: {} bytes", entropy.len());
            println!("  Hex: {}", to_hex(&entropy));

            // Try to derive wallet
            if let Some(wallet) = crypto::Wallet::from_seed(seed) {
                println!("\n  Derived wallet:");
                println!("    Address: {}", wallet.address);
                println!("    Public Key: {}", wallet.public_key[..40].to_string() + "...");
            }
        }
        None => {
            println!("❌ Invalid seed: could not decode");
        }
    }
}

async fn start_node(config: Config) -> anyhow::Result<()> {
    info!("Starting Call Core node...");
    info!("Node name: {}", config.node_name);
    info!("Data directory: {}", config.data_dir);
    info!("P2P port: {}", config.listen_address.port());
    info!("RPC port: {}", config.rpc_port);
    info!("Validator mode: {}", config.is_validator());

    let app = Application::new(config)?;

    // Handle shutdown gracefully
    // Note: Application::run now takes self by value, so we run it directly
    // Shutdown handling is done internally via signal handlers
    if let Err(e) = app.run().await {
        error!("Application error: {}", e);
    }

    Ok(())
}

async fn show_ledger_info(config: &Config, ledger: &str) -> anyhow::Result<()> {
    let rpc_url = format!("http://{}:{}", config.rpc_bind_address, config.rpc_port);

    // Parse ledger parameter
    let params = if ledger == "current" {
        None // Use ledger_current for current ledger
    } else if ledger == "validated" || ledger == "closed" {
        Some(serde_json::json!({"ledger_index": "validated"}))
    } else if let Ok(index) = ledger.parse::<u64>() {
        Some(serde_json::json!({"ledger_index": index}))
    } else {
        Some(serde_json::json!({"ledger_hash": ledger}))
    };

    // Call the appropriate RPC method
    let result = if ledger == "current" {
        rpc_request(&rpc_url, "ledger_current", None).await?
    } else {
        rpc_request(&rpc_url, "ledger", params).await?
    };

    // Format and display the result
    println!("Ledger Information:");
    println!("{}", serde_json::to_string_pretty(&result)?);

    Ok(())
}

async fn submit_transaction(config: &Config, tx_blob: &str) -> anyhow::Result<()> {
    let rpc_url = format!("http://{}:{}", config.rpc_bind_address, config.rpc_port);

    println!("Submitting transaction...");

    let params = serde_json::json!({"tx_blob": tx_blob});
    let result = rpc_request(&rpc_url, "submit", Some(params)).await?;

    // Display the result
    println!("Submission Result:");
    println!("{}", serde_json::to_string_pretty(&result)?);

    // Extract and display key information
    if let Some(engine_result) = result.get("engine_result").and_then(|v| v.as_str()) {
        println!("\nEngine Result: {}", engine_result);
    }
    if let Some(engine_result_message) = result.get("engine_result_message").and_then(|v| v.as_str()) {
        println!("Result Message: {}", engine_result_message);
    }

    Ok(())
}

async fn show_account_info(config: &Config, account: &str) -> anyhow::Result<()> {
    let rpc_url = format!("http://{}:{}", config.rpc_bind_address, config.rpc_port);

    println!("Account information for: {}", account);

    // Parse account - could be address or public key
    // For now, assume it's a hex-encoded account ID
    let account_param = if account.len() == 40 && account.chars().all(|c| c.is_ascii_hexdigit()) {
        account.to_string()
    } else {
        // Try to decode as base58 or other format if needed
        // For now, pass as-is
        account.to_string()
    };

    let params = serde_json::json!({"account": account_param});
    let result = rpc_request(&rpc_url, "account_info", Some(params)).await?;

    // Format and display the result
    println!("Account Information:");
    println!("{}", serde_json::to_string_pretty(&result)?);

    Ok(())
}

async fn sign_transaction_cli(secret: &str, tx_json: &str) -> anyhow::Result<()> {
    // Parse tx_json to validate it's proper JSON
    let tx_value: serde_json::Value = serde_json::from_str(tx_json)
        .map_err(|e| anyhow::anyhow!("Invalid JSON in tx_json: {}", e))?;

    // Use default RPC URL (node must be running)
    let rpc_url = "http://127.0.0.1:5005";

    println!("Signing transaction...");

    let params = serde_json::json!({
        "secret": secret,
        "tx_json": tx_value
    });

    let result = rpc_request(rpc_url, "sign", Some(params)).await?;

    // Display the result
    println!("Sign Result:");
    println!("{}", serde_json::to_string_pretty(&result)?);

    // Extract and display key information
    if let Some(tx_blob) = result.get("tx_blob").and_then(|v| v.as_str()) {
        println!("\nSigned Transaction Blob (tx_blob):");
        println!("{}", tx_blob);
        println!("\nTo submit, use:");
        println!("  ./target/release/calld submit {}", tx_blob);
    }

    Ok(())
}

/// Derive Callchain accounts from a BIP39/BIP44 mnemonic
fn derive_from_mnemonic(mnemonic: &str, count: u32, format: &str) -> anyhow::Result<()> {
    use crypto::MnemonicWallet;

    // Create wallet from mnemonic
    let wallet = MnemonicWallet::from_mnemonic(mnemonic)
        .map_err(|e| anyhow::anyhow!("Failed to parse mnemonic: {}", e))?;

    // Derive accounts
    let accounts = wallet.derive_accounts(count);

    match format {
        "json" => {
            let json_accounts: Vec<_> = accounts.iter().map(|a| a.to_json()).collect();
            println!("{}", serde_json::to_string_pretty(&json_accounts)?);
        }
        _ => {
            println!("BIP44 Mnemonic-derived Accounts (Coin Type: 644)");
            println!("================================================");
            println!("Mnemonic: {}", mnemonic);
            println!();

            for (i, account) in accounts.iter().enumerate() {
                println!("Account {}", i);
                println!("  Address: {}", account.address);
                println!("  Hex ID: {}", account.hex_id);
                println!("  Seed: {}", account.seed);
                println!("  Public Key: {}", account.public_key);
                println!("  Private Key: {}", account.private_key);
                println!();
            }
        }
    }

    Ok(())
}

/// Sign a transaction locally without using RPC
fn sign_transaction_local(key: &str, tx: &str) -> anyhow::Result<()> {
    use crypto::{PrivateKey, TransactionSigner, KeyType};
    use std::fs;

    // Read transaction JSON
    let tx_json_str = if tx.starts_with('{') {
        tx.to_string()
    } else {
        fs::read_to_string(tx)?
    };

    let tx_json: serde_json::Value = serde_json::from_str(&tx_json_str)
        .map_err(|e| anyhow::anyhow!("Invalid transaction JSON: {}", e))?;

    // Parse private key
    let private_key = if key.starts_with("mnemonic:") {
        return Err(anyhow::anyhow!("Mnemonic-derived signing not yet implemented. Use hex private key instead."));
    } else {
        // Parse hex private key
        let key_bytes = hex::decode(key)
            .map_err(|_| anyhow::anyhow!("Invalid hex private key"))?;
        PrivateKey::from_bytes(KeyType::Secp256k1, &key_bytes)
            .ok_or_else(|| anyhow::anyhow!("Invalid private key"))?
    };

    // Extract transaction fields from JSON
    let account_hex = tx_json.get("Account")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing Account field"))?;
    let account_bytes = hex::decode(account_hex)?;
    let account = primitives::AccountID::new(account_bytes.try_into().unwrap());

    let sequence = tx_json.get("Sequence")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("Missing Sequence field"))? as u32;

    let tx_type = tx_json.get("TransactionType")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing TransactionType field"))?;

    // Create and sign transaction based on type
    let tx_blob = match tx_type {
        "Payment" => {
            let dest_hex = tx_json.get("Destination")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing Destination field"))?;
            let dest_bytes = hex::decode(dest_hex)?;
            let destination = primitives::AccountID::new(dest_bytes.try_into().unwrap());

            let amount_str = tx_json.get("Amount")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing Amount field"))?;
            let amount: u64 = amount_str.parse()?;

            TransactionSigner::sign_payment(account, destination, amount, sequence, &private_key)
                .map_err(|e| anyhow::anyhow!("Signing failed: {}", e))?
        }
        "AccountSet" => {
            TransactionSigner::sign_account_set(account, sequence, &private_key)
                .map_err(|e| anyhow::anyhow!("Signing failed: {}", e))?
        }
        _ => {
            return Err(anyhow::anyhow!("Unsupported transaction type: {}", tx_type));
        }
    };

    println!("Transaction signed successfully!");
    println!("TxBlob: {}", tx_blob);
    println!("\nTo submit:");
    println!("  ./target/release/calld submit {}", tx_blob);

    Ok(())
}
