/// Check if specific pools are in our monitored set using checksummed addresses

use mempool_processor::common::address::{checksum_address, alloy_address_to_checksum};
use alloy_primitives::Address;
use std::str::FromStr;
use tracing::info;

fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .init();
    
    // Pools from the liquidity removal transactions
    let pools = vec![
        "0xd46f41239bd02760ea40698f0787a668ea631b48",
        "0x6ce3a16d1dd783addce2a140c5a9e6b92b84a1a3",
    ];
    
    info!("Checking checksummed versions of pool addresses:");
    
    for pool in pools {
        let checksummed = checksum_address(pool);
        info!("\nOriginal:    {}", pool);
        info!("Checksummed: {}", checksummed);
        
        // Also show what our alloy conversion would produce
        if let Ok(addr) = Address::from_str(pool) {
            let alloy_checksum = alloy_address_to_checksum(addr);
            info!("Alloy check: {}", alloy_checksum);
            
            if checksummed != alloy_checksum {
                info!("WARNING: Checksum mismatch!");
            }
        }
    }
}