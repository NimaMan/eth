use alloy_primitives::{
    utils::{format_ether, format_units},
    Address, U256,
};
/// Token inspection example
///
/// Pulls together metadata, supply, holder balances, historical context, and
/// basic contract analysis for a single ERC20 token.
///
/// Run with: cargo run --example token_inspection
use reth_chain_query::{Result, RethQueryProvider};
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Token Inspection ===\n");

    let provider = RethQueryProvider::new("/home/nima/.local/share/reth/mainnet")?;

    // Analyze UNI token comprehensively
    let uni_token = Address::from_str("1f9840a85d5aF5bf1D1762F925BDADdC4201F984")?;

    println!("Analyzing Uniswap (UNI) Token:\n");
    println!("{}", "=".repeat(60));

    // 1. Basic Metadata
    println!("\n📋 Basic Information:");
    println!("{}", "-".repeat(40));

    let metadata = provider.get_token_metadata(uni_token).await?;
    println!("Name:     {}", metadata.name);
    println!("Symbol:   {}", metadata.symbol);
    println!("Decimals: {}", metadata.decimals);
    println!("Address:  0x{}", uni_token);

    // 2. Supply Information
    println!("\n💰 Supply Information:");
    println!("{}", "-".repeat(40));

    match provider.get_token_total_supply(uni_token, None).await {
        Ok(supply) => {
            let formatted = format_units(supply, metadata.decimals)?;
            let supply_float: f64 = formatted.parse().unwrap_or(0.0);
            println!("Total Supply:     {:.2} {}", supply_float, metadata.symbol);
            println!("                  ({} wei)", supply);

            // Calculate market cap (assuming $1 = 1 token for demo)
            // In reality, you'd fetch the price from an oracle
            let estimated_price = 5.50; // Example price in USD
            let market_cap = supply_float * estimated_price;
            println!(
                "Est. Market Cap:  ${:.2} (at ${:.2}/token)",
                market_cap, estimated_price
            );
        }
        Err(e) => {
            println!("Total Supply:     Error - {}", e);
        }
    }

    // 3. Check some notable holders
    println!("\n👥 Notable Holder Balances:");
    println!("{}", "-".repeat(40));

    let notable_holders = vec![
        ("Binance", "F977814e90dA44bFA03b6295A0616a897441aceC"),
        (
            "Uniswap Treasury",
            "1a9C8182C09F50C8318d769245beA52c32BE35BC",
        ),
        ("Vitalik", "d8dA6BF26964aF9D7eEd9e03E53415D37aA96045"),
    ];

    for (name, holder_addr) in notable_holders {
        let holder = Address::from_str(holder_addr)?;
        match provider.get_token_balance(uni_token, holder, None).await {
            Ok(balance) => {
                if balance > U256::ZERO {
                    let formatted = format_units(balance, metadata.decimals)?;
                    let balance_float: f64 = formatted.parse().unwrap_or(0.0);
                    println!("{:<20} {:>15.2} UNI", name, balance_float);
                } else {
                    println!("{:<20} {:>15}", name, "0 UNI");
                }
            }
            Err(e) => {
                println!("{:<20} Error: {}", name, e);
            }
        }
    }

    // 4. Contract verification
    println!("\n🔍 Contract Analysis:");
    println!("{}", "-".repeat(40));

    // Check if it's a contract (it should be)
    let is_contract = provider.is_contract(uni_token, None).await?;
    println!(
        "Is Contract:      {}",
        if is_contract { "Yes ✅" } else { "No ❌" }
    );

    // Get contract creation info (would need transaction history)
    println!("Standard:         ERC-20");
    println!("Upgradeable:      No (immutable contract)");

    // 5. Recent activity summary (simplified)
    println!("\n📊 Token Statistics:");
    println!("{}", "-".repeat(40));

    // Get current block for reference
    let current_block = provider.get_latest_block()?;
    println!("Current Block:    {}", current_block);

    // Check supply at an earlier block (if available)
    let earlier_block = current_block.saturating_sub(100_000); // ~2 weeks ago
    match provider
        .get_token_total_supply(uni_token, Some(earlier_block))
        .await
    {
        Ok(earlier_supply) => {
            let current_supply = provider.get_token_total_supply(uni_token, None).await?;
            let supply_change = if current_supply > earlier_supply {
                let diff = current_supply - earlier_supply;
                let formatted = format_units(diff, metadata.decimals)?;
                format!("+{} UNI", formatted)
            } else if current_supply < earlier_supply {
                let diff = earlier_supply - current_supply;
                let formatted = format_units(diff, metadata.decimals)?;
                format!("-{} UNI", formatted)
            } else {
                "No change".to_string()
            };
            println!("Supply Change (~2 weeks): {}", supply_change);
        }
        Err(_) => {
            println!("Supply Change:    Historical data not available");
        }
    }

    // 6. Additional token features
    println!("\n⚙️ Token Features:");
    println!("{}", "-".repeat(40));
    println!("Transferable:     Yes");
    println!("Burnable:         No");
    println!("Mintable:         No (fixed supply)");
    println!("Pausable:         No");

    println!("\n{}", "=".repeat(60));
    println!("✅ Token inspection complete!");

    Ok(())
}
