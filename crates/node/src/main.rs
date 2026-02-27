use clap::{Parser, Subcommand};
use node::{Application, Config};
use std::net::SocketAddr;
use std::path::PathBuf;
use tracing::{info, error};

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
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

    let mut app = Application::new(config)?;

    // Handle shutdown gracefully
    let ctrl_c = tokio::signal::ctrl_c();

    tokio::select! {
        result = app.run() => {
            if let Err(e) = result {
                error!("Application error: {}", e);
            }
        }
        _ = ctrl_c => {
            info!("Received shutdown signal");
            app.shutdown().await;
        }
    }

    Ok(())
}

async fn show_ledger_info(_config: &Config, ledger: &str) -> anyhow::Result<()> {
    println!("Ledger information for: {}", ledger);
    // TODO: Implement ledger info retrieval
    println!("Ledger info not yet implemented");
    Ok(())
}

async fn submit_transaction(_config: &Config, tx_blob: &str) -> anyhow::Result<()> {
    println!("Submitting transaction: {}", tx_blob);
    // TODO: Implement transaction submission
    println!("Transaction submission not yet implemented");
    Ok(())
}

async fn show_account_info(_config: &Config, account: &str) -> anyhow::Result<()> {
    println!("Account information for: {}", account);
    // TODO: Implement account info retrieval
    println!("Account info not yet implemented");
    Ok(())
}
