/// Minimal test example to verify basic functionality
/// 
/// Run with: cargo run --example minimal_test

use reth_chain_query::provider::RethQueryProvider;
use eyre::Result;

fn main() -> Result<()> {
    println!("=== Minimal Reth Chain Query Test ===\n");
    
    // Path to your Reth data directory
    let reth_datadir = "/home/nima/.local/share/reth/mainnet";
    
    // Initialize the provider
    println!("Initializing RethQueryProvider...");
    let provider = RethQueryProvider::new(reth_datadir)?;
    
    println!("✓ Provider initialized\n");
    
    // Get the latest block
    let latest_block = provider.get_latest_block()?;
    println!("Latest synced block: #{}", latest_block);
    
    println!("\n✓ Basic functionality works!");
    
    Ok(())
}