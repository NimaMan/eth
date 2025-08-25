/// Debug test for time converter

use reth_chain_query::ChainQuery;

fn main() -> eyre::Result<()> {
    let reth_datadir = "/home/nima/.local/share/reth/mainnet";
    let chain_query = ChainQuery::new(reth_datadir)?;
    
    let latest = chain_query.get_latest_block()?;
    println!("Latest block: {}", latest);
    
    // Test get_block_header_info directly
    let simulator = chain_query.get_simulator();
    let (number, timestamp, gas_limit, base_fee) = simulator.get_block_header_info(latest)?;
    
    println!("Block header info for block {}:", latest);
    println!("  Number: {}", number);
    println!("  Timestamp: {}", timestamp);
    println!("  Gas limit: {}", gas_limit);
    println!("  Base fee: {:?}", base_fee);
    
    // Test an older block
    let old_block = 20_000_000;
    let (number, timestamp, gas_limit, base_fee) = simulator.get_block_header_info(old_block)?;
    
    println!("\nBlock header info for block {}:", old_block);
    println!("  Number: {}", number);
    println!("  Timestamp: {}", timestamp);
    println!("  Gas limit: {}", gas_limit);
    println!("  Base fee: {:?}", base_fee);
    
    // Convert timestamp to human readable
    use chrono::{DateTime, Utc};
    let dt = DateTime::from_timestamp(timestamp as i64, 0);
    println!("  Human readable: {:?}", dt);
    
    Ok(())
}