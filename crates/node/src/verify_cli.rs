//! Call-Core Transaction Signature Verification CLI
//!
//! A standalone CLI tool for verifying Call-Core transaction signatures.
//!
//! Usage:
//!     calld-verify --tx-blob <TX_BLOB>
//!
//! Example:
//!     calld-verify --tx-blob 12000024000000016100000000000f4240...

use clap::Parser;

mod signing;
use signing::verify_transaction_blob;

/// Call-Core Transaction Signature Verifier CLI
#[derive(Parser)]
#[command(name = "calld-verify")]
#[command(about = "Verify Call-Core transaction signatures")]
#[command(version)]
struct Cli {
    /// Transaction blob (hex) to verify
    #[arg(short, long)]
    tx_blob: String,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    format: OutputFormat,

    /// Quiet mode - only output success/failure
    #[arg(short, long)]
    quiet: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
enum OutputFormat {
    /// JSON output
    Json,
    /// Simple text output
    Text,
}

fn main() {
    let cli = Cli::parse();

    match verify_transaction_blob(&cli.tx_blob) {
        Ok(result) => {
            if cli.quiet {
                if result.valid {
                    println!("VALID");
                    std::process::exit(0);
                } else {
                    println!("INVALID");
                    std::process::exit(1);
                }
            }

            match cli.format {
                OutputFormat::Json => {
                    println!(
                        "{}",
                        serde_json::json!({
                            "valid": result.valid,
                            "tx_hash": result.tx_hash,
                            "account": result.account,
                            "sequence": result.sequence,
                            "tx_type": result.tx_type,
                            "error": result.error,
                        })
                    );
                }
                OutputFormat::Text => {
                    println!("Signature Verification Result");
                    println!("=============================");
                    println!("Valid:       {}", if result.valid { "✓ YES" } else { "✗ NO" });
                    println!("Transaction: {} (seq: {})", result.tx_type, result.sequence);
                    println!("Account:     {}", result.account);
                    println!("Hash:        {}", result.tx_hash);
                    if let Some(err) = &result.error {
                        println!("Error:       {}", err);
                    }
                }
            }

            if !result.valid {
                std::process::exit(1);
            }
        }
        Err(e) => {
            if cli.quiet {
                println!("ERROR");
            } else {
                eprintln!("Error: {}", e);
            }
            std::process::exit(2);
        }
    }
}
