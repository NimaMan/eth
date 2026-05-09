use alloy_primitives::{utils::format_ether, B256, U256};
use reth_chain_query::{Result, RethQueryProvider};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let mut datadir: Option<String> = None;
    let mut tx_number: Option<u64> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--datadir" => {
                if let Some(path) = args.next() {
                    datadir = Some(path);
                } else {
                    eprintln!("--datadir requires a value");
                    std::process::exit(1);
                }
            }
            "--tx-number" => {
                if let Some(value) = args.next() {
                    tx_number = value.parse().ok();
                } else {
                    eprintln!("--tx-number requires a value");
                    std::process::exit(1);
                }
            }
            "--help" | "-h" => {
                print_usage();
                return Ok(());
            }
            other => {
                eprintln!("Unknown argument: {}", other);
                print_usage();
                std::process::exit(1);
            }
        }
    }

    let tx_number = tx_number.unwrap_or_else(|| {
        eprintln!("Missing required --tx-number argument");
        print_usage();
        std::process::exit(1);
    });

    let provider = if let Some(path) = datadir {
        RethQueryProvider::new(&path)?
    } else {
        let path = tx_simulator::config::repo::reth_datadir()?;
        RethQueryProvider::new(&path)?
    };

    println!("🔍 Looking up transaction {}", tx_number);

    let tx = provider.get_transaction_by_number(tx_number).await?;
    println!("✅ Found transaction");
    println!("  Hash: 0x{:x}", tx.hash);
    println!("  Block: #{} (index {})", tx.block_number, tx.tx_index);
    println!(
        "  From: 0x{:x} -> {}",
        tx.from,
        tx.to
            .map(|addr| format!("0x{:x}", addr))
            .unwrap_or_else(|| "Contract Creation".to_string())
    );
    println!("  Value: {} ETH", format_ether(tx.value));
    println!("  Gas limit: {}", tx.gas_limit);
    println!(
        "  Gas price: {} gwei",
        tx.gas_price / U256::from(1_000_000_000u64)
    );
    println!("  Nonce: {}", tx.nonce);
    println!("  Type: {}", tx.transaction_type);

    // Simple guard to ensure the canonical hash still exists.
    let hash = B256::from(tx.hash);
    if provider.transaction_exists(hash).await? {
        println!("  Hash lookup confirmed in database.");
    } else {
        println!("  Warning: hash lookup failed unexpectedly.");
    }

    Ok(())
}

fn print_usage() {
    println!("Usage:");
    println!(
        "  cargo run --example fetch_transaction_by_number -- --tx-number <u64> [--datadir <path>]"
    );
}
