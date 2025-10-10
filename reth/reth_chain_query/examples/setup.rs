use alloy_primitives::{utils::format_ether, Address};
/// Quickstart: Initialize RethQueryProvider and run basic queries
///
/// This example shows how to:
/// 1. Initialize the provider
/// 2. Run basic blockchain queries
/// 3. Handle common patterns
///
/// Run with: cargo run --example setup
use reth_chain_query::{Result, RethQueryProvider};
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Reth Chain Query Quickstart ===\n");

    // Path to your Reth data directory
    let reth_datadir = "/home/nima/.local/share/reth/mainnet";

    // Initialize the provider
    println!("Initializing RethQueryProvider...");
    let provider = RethQueryProvider::new(reth_datadir)?;

    // Optional: Add RPC endpoint for trace data
    // let provider = RethQueryProvider::new(reth_datadir)?
    //     .with_rpc_endpoint("http://localhost:8545")?;

    println!("✓ Provider initialized\n");

    // === Basic Queries ===

    // 1. Get the latest block
    let latest_block = provider.get_latest_block()?;
    println!("Latest synced block: #{}", latest_block);

    // 2. Get block header information
    let header = provider.fetch_block_header_only(latest_block).await?;
    println!("Block timestamp: {}", header.timestamp);
    println!(
        "Gas used: {} / {} ({:.1}%)",
        header.gas_used,
        header.gas_limit,
        (header.gas_used as f64 / header.gas_limit as f64) * 100.0
    );
    if let Some(base_fee) = header.base_fee_per_gas {
        println!("Base fee: {} gwei", base_fee / 1_000_000_000);
    }

    println!();

    // 3. Query an account balance
    // Vitalik's address
    let address = Address::from_str("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045")?;

    // Get current balance
    let account = provider.get_account(address, None).await?;
    println!("Address: 0x{}", address);
    println!("ETH Balance: {} ETH", format_ether(account.balance));
    println!("Nonce: {}", account.nonce);
    println!("Is contract: {}", account.code_hash.is_some());

    // Get balance at specific block (The Merge)
    let merge_block = 15_537_393;
    let account_at_merge = provider.get_account(address, Some(merge_block)).await?;
    println!("\nAt The Merge (block {}):", merge_block);
    println!(
        "ETH Balance: {} ETH",
        format_ether(account_at_merge.balance)
    );

    println!();

    // 4. Check if address is a contract
    let usdc_address = Address::from_str("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")?;
    let usdc_account = provider.get_account(usdc_address, None).await?;
    println!("USDC contract: 0x{}", usdc_address);
    println!("Is contract: {}", usdc_account.code_hash.is_some());

    println!("\n✅ Quickstart complete!");
    println!("\nYou're now ready to query the blockchain directly from Reth's database!");
    println!("This is 100-1000x faster than using RPC calls.");

    Ok(())
}
