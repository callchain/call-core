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

/// Parse an AssetAmount from JSON value
fn parse_asset_amount(value: &serde_json::Value) -> anyhow::Result<crypto::transaction_signer::AssetAmount> {
    use crypto::transaction_signer::AssetAmount;

    // Helper to parse account address (hex or base58)
    fn parse_account_addr(addr_str: &str) -> anyhow::Result<primitives::AccountID> {
        // Try hex first (40 hex chars = 20 bytes)
        if addr_str.len() == 40 {
            if let Ok(bytes) = hex::decode(addr_str) {
                if bytes.len() == 20 {
                    return Ok(primitives::AccountID::new(bytes.try_into().unwrap()));
                }
            }
        }
        // Try base58 (addresses starting with 'c')
        if addr_str.starts_with('c') {
            match crypto::base58::decode(addr_str) {
                Ok(decoded) => {
                    // Format: version (1 byte) + account_id (20 bytes) + checksum (4 bytes)
                    if decoded.len() == 25 {
                        let mut bytes = [0u8; 20];
                        bytes.copy_from_slice(&decoded[1..21]);
                        return Ok(primitives::AccountID::new(bytes));
                    }
                }
                Err(_) => {}
            }
        }
        Err(anyhow::anyhow!("Invalid account address format: {}", addr_str))
    }

    if let Some(amount_str) = value.as_str() {
        // Native amount (string of drops)
        let amount: u64 = amount_str.parse()
            .map_err(|_| anyhow::anyhow!("Invalid native amount"))?;
        Ok(AssetAmount::native(amount))
    } else if let Some(obj) = value.as_object() {
        // Issued currency amount
        let currency_str = obj.get("currency")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing currency field"))?;

        let issuer_str = obj.get("issuer")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing issuer field"))?;
        let issuer = parse_account_addr(issuer_str)?;

        let value_str = obj.get("value")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing value field"))?;
        let value: i64 = value_str.parse()?;

        Ok(AssetAmount::issued(value, currency_str, issuer))
    } else {
        Err(anyhow::anyhow!("Invalid amount format"))
    }
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

    // Helper function to parse account address (hex or base58)
    fn parse_account(addr_str: &str) -> anyhow::Result<primitives::AccountID> {
        // Try hex first (40 hex chars = 20 bytes)
        if addr_str.len() == 40 {
            if let Ok(bytes) = hex::decode(addr_str) {
                if bytes.len() == 20 {
                    return Ok(primitives::AccountID::new(bytes.try_into().unwrap()));
                }
            }
        }
        // Try base58 (addresses starting with 'c')
        if addr_str.starts_with('c') {
            match crypto::base58::decode(addr_str) {
                Ok(decoded) => {
                    // Format: version (1 byte) + account_id (20 bytes) + checksum (4 bytes)
                    if decoded.len() == 25 {
                        let mut bytes = [0u8; 20];
                        bytes.copy_from_slice(&decoded[1..21]);
                        return Ok(primitives::AccountID::new(bytes));
                    }
                }
                Err(_) => {}
            }
        }
        Err(anyhow::anyhow!("Invalid account address format: {}", addr_str))
    }

    // Extract transaction fields from JSON
    let account_str = tx_json.get("Account")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing Account field"))?;
    let account = parse_account(account_str)?;

    let sequence = tx_json.get("Sequence")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("Missing Sequence field"))? as u32;

    let tx_type = tx_json.get("TransactionType")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing TransactionType field"))?;

    // Create and sign transaction based on type
    let tx_blob = match tx_type {
        "Payment" => {
            let dest_str = tx_json.get("Destination")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing Destination field"))?;
            let destination = parse_account(dest_str)?;

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
        "TrustSet" => {
            use crypto::transaction_signer::{SignableTransaction, TransactionType, AssetAmount};

            let limit_amount = tx_json.get("LimitAmount")
                .ok_or_else(|| anyhow::anyhow!("Missing LimitAmount field"))?;

            let currency_str = limit_amount.get("currency")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing currency field"))?;

            let issuer_str = limit_amount.get("issuer")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing issuer field"))?;
            let issuer = parse_account(issuer_str)?;

            let value_str = limit_amount.get("value")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing value field"))?;
            let value: i64 = value_str.parse()?;

            // Convert currency string to 20-byte array
            let mut currency = [0u8; 20];
            if currency_str.len() == 3 {
                currency[12] = currency_str.as_bytes()[0];
                currency[13] = currency_str.as_bytes()[1];
                currency[14] = currency_str.as_bytes()[2];
            } else if currency_str.len() == 40 {
                let hex_bytes = hex::decode(currency_str)?;
                currency.copy_from_slice(&hex_bytes);
            }

            let tx = SignableTransaction::new_trust_set(account, issuer, currency, value, sequence);
            TransactionSigner::sign_transaction(&tx, &private_key)
                .map_err(|e| anyhow::anyhow!("Signing failed: {}", e))?
        }
        "OfferCreate" => {
            use crypto::transaction_signer::{SignableTransaction, TransactionType, AssetAmount};

            let taker_pays = parse_asset_amount(tx_json.get("TakerPays")
                .ok_or_else(|| anyhow::anyhow!("Missing TakerPays field"))?)?;
            let taker_gets = parse_asset_amount(tx_json.get("TakerGets")
                .ok_or_else(|| anyhow::anyhow!("Missing TakerGets field"))?)?;

            let tx = SignableTransaction::new_offer_create(account, taker_pays, taker_gets, sequence);
            TransactionSigner::sign_transaction(&tx, &private_key)
                .map_err(|e| anyhow::anyhow!("Signing failed: {}", e))?
        }
        "OfferCancel" => {
            use crypto::transaction_signer::{SignableTransaction, TransactionType};

            let offer_seq = tx_json.get("OfferSequence")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| anyhow::anyhow!("Missing OfferSequence field"))? as u32;

            let tx = SignableTransaction::new_offer_cancel(account, offer_seq, sequence);
            TransactionSigner::sign_transaction(&tx, &private_key)
                .map_err(|e| anyhow::anyhow!("Signing failed: {}", e))?
        }
        "SetRegularKey" => {
            use crypto::transaction_signer::{SignableTransaction, TransactionType};

            let key_str = tx_json.get("RegularKey")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing RegularKey field"))?;
            let regular_key = parse_account(key_str)?;

            let tx = SignableTransaction::new_set_regular_key(account, regular_key, sequence);
            TransactionSigner::sign_transaction(&tx, &private_key)
                .map_err(|e| anyhow::anyhow!("Signing failed: {}", e))?
        }
        "SignerListSet" => {
            use crypto::transaction_signer::{SignableTransaction, SignerEntry, TransactionType};

            let quorum = tx_json.get("SignerQuorum")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| anyhow::anyhow!("Missing SignerQuorum field"))? as u32;

            let signers_json = tx_json.get("Signers")
                .and_then(|v| v.as_array())
                .ok_or_else(|| anyhow::anyhow!("Missing Signers field"))?;

            let mut signers = Vec::new();
            for signer_json in signers_json {
                let signer_account_str = signer_json.get("Account")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing Signer Account field"))?;
                let signer_account = parse_account(signer_account_str)?;

                let weight = signer_json.get("SignerWeight")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| anyhow::anyhow!("Missing SignerWeight field"))? as u16;

                signers.push(SignerEntry { account: signer_account, weight });
            }

            let tx = SignableTransaction::new_signer_list_set(account, quorum, signers, sequence);
            TransactionSigner::sign_transaction(&tx, &private_key)
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
