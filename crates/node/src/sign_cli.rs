//! Call-Core Transaction Signing CLI
//!
//! A standalone CLI tool for signing Call-Core transactions locally.
//! Uses the exact same signing logic as the `calld sign` RPC command.
//!
//! Usage:
//!     calld-sign --secret <SECRET> --tx-json <TX_JSON>
//!
//! Example:
//!     calld-sign --secret ssyB7KxAvfRwQ6mseEjt3iY1qeqMC \
//!         --tx-json '{"TransactionType":"Payment","Account":"cGmJBrEfFssWuas4kCoHTX5r6aMEf6QHhy","Destination":"c3K3xXhvsWBnP3TitQfeg2ihAuaYybvtc7","Amount":"1000000","Sequence":1,"Fee":"10"}'

use clap::Parser;
use crypto::wallet::Seed;
use crypto::PrivateKey;

mod signing;
use signing::{sign_transaction_local, parse_account};

/// Call-Core Transaction Signer CLI
#[derive(Parser)]
#[command(name = "calld-sign")]
#[command(about = "Sign Call-Core transactions locally")]
#[command(version)]
struct Cli {
    /// Secret (seed) for signing
    #[arg(short, long)]
    secret: String,

    /// Transaction JSON to sign
    #[arg(short, long)]
    tx_json: String,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    format: OutputFormat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
enum OutputFormat {
    /// JSON output
    Json,
    /// Hex blob only
    Blob,
}

fn main() {
    let cli = Cli::parse();

    match sign_transaction(&cli.secret, &cli.tx_json) {
        Ok(result) => {
            match cli.format {
                OutputFormat::Json => {
                    println!("{}", serde_json::json!({
                        "status": "success",
                        "tx_blob": result.tx_blob,
                        "tx_json": result.tx_json,
                        "hash": result.hash,
                    }));
                }
                OutputFormat::Blob => {
                    println!("{}", result.tx_blob);
                }
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn sign_transaction(secret: &str, tx_json_str: &str) -> Result<signing::SignResult, String> {
    // Parse the transaction JSON
    let tx_json: serde_json::Value = serde_json::from_str(tx_json_str)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    // Derive private key from secret
    let private_key = derive_private_key(secret)?;

    // Use the shared signing logic (same as RPC)
    sign_transaction_local(&private_key, &tx_json)
}

fn derive_private_key(secret: &str) -> Result<PrivateKey, String> {
    // Try hex format first (32 bytes raw private key)
    if let Ok(key_bytes) = hex::decode(secret) {
        if key_bytes.len() == 32 {
            return PrivateKey::from_bytes(crypto::KeyType::Secp256k1, &key_bytes)
                .ok_or_else(|| "Invalid private key".to_string());
        }
    }

    // Try seed format (starts with 's')
    // Use the same decoding as the RPC handler
    match crypto::wallet::decode_seed(secret) {
        Some(entropy) => {
            // Use SHA256 to derive 32 bytes from 16-byte entropy (same as wallet.rs)
            let key_hash = crypto::sha256(&entropy);
            PrivateKey::from_bytes(crypto::KeyType::Secp256k1, &key_hash)
                .ok_or_else(|| "Failed to generate key from seed".to_string())
        }
        None => Err("Invalid secret format. Expected seed (s...) or hex private key".to_string()),
    }
}
