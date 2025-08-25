/// CEX Monitoring Example
/// 
/// Demonstrates how to use the entities module for centralized exchange monitoring.
/// This example showcases:
/// - ETH balance tracking across all exchanges
/// - Top exchanges by holdings
/// - Flow analysis for transfers
/// - Large deposit/withdrawal detection

use reth_chain_query::{ChainQuery, Result, Address};
use reth_chain_query::entities::cex::{
    CexBalanceTracker, CexFlowAnalyzer,
    CEX_ADDRESSES, CEX_ADDRESS_COUNT, exchange_stats,
    is_cex_address, get_cex_by_address,
};
use reth_chain_query::entities::common::format_token_amount;
use alloy_primitives::U256;
use std::sync::Arc;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🏦 CEX Monitoring using Entities Module");
    println!("{}", "=".repeat(70));
    
    // Initialize ChainQuery
    let reth_datadir = "/home/nima/.local/share/reth/mainnet";
    let chain_query = Arc::new(ChainQuery::new(reth_datadir)?);
    
    // Create analyzers
    let balance_tracker = CexBalanceTracker::new(chain_query.clone());
    let flow_analyzer = CexFlowAnalyzer::new(chain_query.clone());
    
    // 1. Overall CEX Statistics
    println!("\n📊 CEX STATISTICS");
    println!("{}", "-".repeat(70));
    println!("Total CEX Addresses Tracked: {}", CEX_ADDRESS_COUNT);
    println!("\nExchanges by Address Count:");
    
    for (exchange, count) in exchange_stats().iter().take(10) {
        println!("  {}: {} addresses", exchange, count);
    }
    
    // 2. ETH Balance Analysis
    println!("\n💰 CEX ETH BALANCES");
    println!("{}", "-".repeat(70));
    
    let balance_summary = balance_tracker.get_all_cex_eth_balances(None).await?;
    
    println!("Block: {}", balance_summary.block_number);
    println!("Total ETH in CEXs: {:.2} ETH", balance_summary.total_cex_eth_formatted);
    println!("Total Value: ${:.2}M (assuming $2000/ETH)", 
        balance_summary.total_cex_eth_formatted * 2000.0 / 1_000_000.0);
    
    println!("\nTop 10 Exchanges by ETH Holdings:");
    for (i, (exchange, eth_amount)) in balance_summary.exchange_rankings.iter().take(10).enumerate() {
        println!("{}. {} - {:.2} ETH (${:.2}M)", 
            i + 1,
            exchange,
            eth_amount,
            eth_amount * 2000.0 / 1_000_000.0
        );
    }
    
    // 3. Specific Exchange Deep Dive
    println!("\n🔍 BINANCE DEEP DIVE");
    println!("{}", "-".repeat(70));
    
    if let Some(binance_data) = balance_summary.exchanges.iter()
        .find(|e| e.name == "Binance") 
    {
        println!("Binance Statistics:");
        println!("  Address Count: {}", binance_data.address_count);
        println!("  Total ETH: {:.2}", binance_data.total_eth_formatted);
        println!("  Average per Address: {:.2} ETH", 
            binance_data.total_eth_formatted / binance_data.address_count as f64);
        
        // Check USDC balance for Binance
        let usdc_address = Address::from_str("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")?;
        let usdc_balance = balance_tracker
            .get_exchange_token_balances("Binance", usdc_address, None)
            .await?;
        let usdc_formatted = format_token_amount(usdc_balance, 6);
        println!("  USDC Balance: ${:.2}M", usdc_formatted / 1_000_000.0);
    }
    
    // 4. Flow Analysis Examples
    println!("\n🔄 FLOW ANALYSIS EXAMPLES");
    println!("{}", "-".repeat(70));
    
    // Example transfers to analyze
    let test_transfers = vec![
        // Binance to unknown
        ("0x28C6c06298d514Db089934071355E5743bf21d60", "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb"),
        // Unknown to Coinbase
        ("0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb", "0x71660c4005BA85c37ccec55d0C4493E66Fe775d3"),
        // Binance to Binance (internal)
        ("0x28C6c06298d514Db089934071355E5743bf21d60", "0x21a31Ee1afC51d94C2eFcCAa2092aD1028285549"),
    ];
    
    for (from_str, to_str) in test_transfers {
        let from = Address::from_str(from_str)?;
        let to = Address::from_str(to_str)?;
        
        if let Some((exchange, direction)) = flow_analyzer.analyze_transfer_flow(from, to) {
            let direction_str = match direction {
                reth_chain_query::entities::common::FlowDirection::Inflow => "⬇️ DEPOSIT",
                reth_chain_query::entities::common::FlowDirection::Outflow => "⬆️ WITHDRAWAL",
                reth_chain_query::entities::common::FlowDirection::Internal => "🔄 INTERNAL",
            };
            println!("{} - {} transfer", direction_str, exchange);
        } else if flow_analyzer.involves_cex(from, to) {
            println!("🔀 Transfer involves CEX but couldn't determine direction");
        } else {
            println!("❌ No CEX involvement detected");
        }
    }
    
    // 5. Large Transfer Detection
    println!("\n🐋 LARGE TRANSFER DETECTION");
    println!("{}", "-".repeat(70));
    
    let test_amounts = vec![
        ("Small", U256::from(1) * U256::from(10).pow(U256::from(18))), // 1 ETH
        ("Medium", U256::from(50) * U256::from(10).pow(U256::from(18))), // 50 ETH
        ("Large", U256::from(500) * U256::from(10).pow(U256::from(18))), // 500 ETH
    ];
    
    let binance_address = Address::from_str("0x28C6c06298d514Db089934071355E5743bf21d60")?;
    let random_address = Address::from_str("0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb")?;
    
    for (size, amount) in test_amounts {
        println!("\n{} Transfer ({} ETH):", size, format_token_amount(amount, 18));
        
        // Test deposit
        if flow_analyzer.is_large_cex_deposit(binance_address, amount) {
            println!("  🚨 Large deposit to CEX detected!");
        }
        
        // Test withdrawal
        if flow_analyzer.is_large_cex_withdrawal(binance_address, amount) {
            println!("  🚨 Large withdrawal from CEX detected!");
        }
        
        if !flow_analyzer.is_large_cex_deposit(binance_address, amount) && 
           !flow_analyzer.is_large_cex_withdrawal(binance_address, amount) {
            println!("  ✅ Normal size transfer");
        }
    }
    
    // 6. Net Flow Example
    println!("\n📈 NET FLOW ANALYSIS (Last 100 blocks)");
    println!("{}", "-".repeat(70));
    
    let latest_block = chain_query.get_latest_block()?;
    let from_block = latest_block - 100;
    
    for exchange in ["Binance", "Coinbase", "Kraken"] {
        let net_flow = flow_analyzer
            .get_exchange_net_flow(exchange, from_block, latest_block)
            .await?;
        
        let flow_eth = net_flow as f64 / 10_f64.powi(18);
        let flow_direction = if net_flow > 0 {
            "⬆️ Net Inflow"
        } else if net_flow < 0 {
            "⬇️ Net Outflow"
        } else {
            "➡️ No Change"
        };
        
        println!("{}: {} of {:.2} ETH", exchange, flow_direction, flow_eth.abs());
    }
    
    // 7. Address Identification
    println!("\n🔍 ADDRESS IDENTIFICATION");
    println!("{}", "-".repeat(70));
    
    let test_addresses = vec![
        "0x28C6c06298d514Db089934071355E5743bf21d60", // Binance
        "0x71660c4005BA85c37ccec55d0C4493E66Fe775d3", // Coinbase
        "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb", // Random
    ];
    
    for addr_str in test_addresses {
        let addr = Address::from_str(addr_str)?;
        if let Some(cex_info) = get_cex_by_address(addr) {
            println!("✅ {} is {}", 
                &addr_str[..10], 
                cex_info.name
            );
        } else {
            println!("❌ {} is not a known CEX", &addr_str[..10]);
        }
    }
    
    println!("\n✅ CEX monitoring complete!");
    
    Ok(())
}