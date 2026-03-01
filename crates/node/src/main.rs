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
