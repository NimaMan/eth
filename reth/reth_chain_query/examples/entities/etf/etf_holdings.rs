/// ETF Holdings Example
/// 
/// Demonstrates how to use the entities module for ETF provider analysis.
/// This example showcases:
/// - ETH holdings tracking by provider
/// - Provider comparison (BlackRock vs Grayscale)
/// - Creation/redemption flow analysis
/// - Market share calculations

use reth_chain_query::{ChainQuery, Result, Address, U256};
use reth_chain_query::entities::etfs::{
    EtfHoldingsTracker, EtfFlowAnalyzer,
    ETF_ADDRESSES, ETF_ADDRESS_COUNT, provider_stats,
    is_etf_address, get_etf_by_address,
};
use reth_chain_query::entities::common::format_token_amount;
use std::sync::Arc;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<()> {
    println!("📈 ETF Holdings Analysis using Entities Module");
    println!("{}", "=".repeat(70));
    
    // Initialize ChainQuery
    let reth_datadir = "/home/nima/.local/share/reth/mainnet";
    let chain_query = Arc::new(ChainQuery::new(reth_datadir)?);
    
    // Create analyzers
    let holdings_tracker = EtfHoldingsTracker::new(chain_query.clone());
    let flow_analyzer = EtfFlowAnalyzer::new(chain_query.clone());
    
    // 1. Overall ETF Statistics
    println!("\n📊 ETF STATISTICS");
    println!("{}", "-".repeat(70));
    println!("Total ETF Addresses Tracked: {}", ETF_ADDRESS_COUNT);
    println!("\nProviders by Address Count:");
    
    for (provider, count) in provider_stats().iter().take(10) {
        println!("  {}: {} addresses", provider, count);
    }
    
    // 2. ETH Holdings Analysis
    println!("\n💎 ETF ETH HOLDINGS");
    println!("{}", "-".repeat(70));
    
    let holdings_summary = holdings_tracker.get_all_etf_holdings(None).await?;
    
    println!("Block: {}", holdings_summary.block_number);
    println!("Total ETH in ETFs: {:.2} ETH", holdings_summary.total_etf_eth_formatted);
    println!("Total Value: ${:.2}B (assuming $2000/ETH)", 
        holdings_summary.total_etf_eth_formatted * 2000.0 / 1_000_000_000.0);
    
    println!("\nTop ETF Providers by ETH Holdings:");
    for (i, provider_data) in holdings_summary.providers.iter().take(10).enumerate() {
        println!("{}. {} - {:.2} ETH ({:.1}% market share)", 
            i + 1,
            provider_data.name,
            provider_data.total_eth_formatted,
            provider_data.market_share_percent
        );
        println!("   Addresses: {} | Value: ${:.2}M",
            provider_data.address_count,
            provider_data.total_eth_formatted * 2000.0 / 1_000_000.0
        );
    }
    
    // 3. Provider Comparison
    println!("\n⚔️ PROVIDER COMPARISON: BlackRock vs Grayscale");
    println!("{}", "-".repeat(70));
    
    if let Some((blackrock_eth, grayscale_eth)) = holdings_tracker
        .compare_providers("BlackRock", "Grayscale", None)
        .await? 
    {
        println!("BlackRock Holdings: {:.2} ETH", blackrock_eth);
        println!("Grayscale Holdings: {:.2} ETH", grayscale_eth);
        
        let ratio = blackrock_eth / grayscale_eth;
        if ratio > 1.0 {
            println!("BlackRock holds {:.2}x more ETH than Grayscale", ratio);
        } else {
            println!("Grayscale holds {:.2}x more ETH than BlackRock", 1.0 / ratio);
        }
        
        let combined = blackrock_eth + grayscale_eth;
        let combined_percent = (combined / holdings_summary.total_etf_eth_formatted) * 100.0;
        println!("Combined: {:.2} ETH ({:.1}% of all ETF holdings)", combined, combined_percent);
    }
    
    // 4. Flow Analysis
    println!("\n🔄 ETF FLOW ANALYSIS");
    println!("{}", "-".repeat(70));
    
    // Example addresses (using first address from each provider as example)
    let test_transfers = vec![
        // External to BlackRock (creation)
        ("0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb", "0x0171F896002665C3ea3Ed0c55f21026cA0A734A0"),
        // BlackRock to external (redemption) 
        ("0x0171F896002665C3ea3Ed0c55f21026cA0A734A0", "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb"),
        // Between BlackRock addresses (internal)
        ("0x0171F896002665C3ea3Ed0c55f21026cA0A734A0", "0x0195Bd0E9Dc6F98BD9bB3d0bFF69749a52065FA5"),
    ];
    
    for (from_str, to_str) in test_transfers {
        let from = Address::from_str(from_str)?;
        let to = Address::from_str(to_str)?;
        
        if let Some((provider, direction)) = flow_analyzer.analyze_transfer_flow(from, to) {
            let direction_str = match direction {
                reth_chain_query::entities::common::FlowDirection::Inflow => "📥 CREATION",
                reth_chain_query::entities::common::FlowDirection::Outflow => "📤 REDEMPTION",
                reth_chain_query::entities::common::FlowDirection::Internal => "🔄 INTERNAL",
            };
            println!("{} - {} ETF", direction_str, provider);
        } else if flow_analyzer.involves_etf(from, to) {
            println!("🔀 Transfer involves ETF but couldn't determine direction");
        } else {
            println!("❌ No ETF involvement detected");
        }
    }
    
    // 5. Large Creation/Redemption Detection
    println!("\n🐋 LARGE CREATION/REDEMPTION DETECTION");
    println!("{}", "-".repeat(70));
    
    let test_amounts = vec![
        ("Small", U256::from(100) * U256::from(10).pow(U256::from(18))), // 100 ETH
        ("Medium", U256::from(500) * U256::from(10).pow(U256::from(18))), // 500 ETH
        ("Large", U256::from(2000) * U256::from(10).pow(U256::from(18))), // 2000 ETH
    ];
    
    let blackrock_address = Address::from_str("0x0171F896002665C3ea3Ed0c55f21026cA0A734A0")?;
    
    for (size, amount) in test_amounts {
        println!("\n{} Transfer ({} ETH):", size, format_token_amount(amount, 18));
        
        if flow_analyzer.is_likely_creation(blackrock_address, amount) {
            println!("  🟢 Likely ETF creation event");
        }
        
        if flow_analyzer.is_likely_redemption(blackrock_address, amount) {
            println!("  🔴 Likely ETF redemption event");
        }
        
        if !flow_analyzer.is_likely_creation(blackrock_address, amount) && 
           !flow_analyzer.is_likely_redemption(blackrock_address, amount) {
            println!("  ➡️ Normal size transfer");
        }
    }
    
    // 6. Balance Changes Analysis
    println!("\n📊 BALANCE CHANGES (Last 1000 blocks)");
    println!("{}", "-".repeat(70));
    
    let latest_block = chain_query.get_latest_block()?;
    let from_block = latest_block - 1000;
    
    for provider in ["BlackRock", "Grayscale", "Fidelity"] {
        if let Some(balance_change) = flow_analyzer
            .get_provider_balance_change(provider, from_block, latest_block)
            .await? 
        {
            let change_type = if balance_change.change > 0 {
                "📈 Increased"
            } else if balance_change.change < 0 {
                "📉 Decreased"
            } else {
                "➡️ Unchanged"
            };
            
            println!("{}: {} by {:.2}% ({:+} wei)",
                provider,
                change_type,
                balance_change.percent_change.abs(),
                balance_change.change
            );
        }
    }
    
    // 7. Total ETF Net Flow
    println!("\n🌊 TOTAL ETF NET FLOW");
    println!("{}", "-".repeat(70));
    
    let net_flow = flow_analyzer
        .get_total_etf_net_flow(from_block, latest_block)
        .await?;
    
    let flow_eth = net_flow as f64 / 10_f64.powi(18);
    let flow_direction = if net_flow > 0 {
        "📥 Net Inflow to ETFs"
    } else if net_flow < 0 {
        "📤 Net Outflow from ETFs"
    } else {
        "➡️ No Net Change"
    };
    
    println!("{}: {:.2} ETH", flow_direction, flow_eth.abs());
    println!("Value Change: ${:.2}M", flow_eth.abs() * 2000.0 / 1_000_000.0);
    
    // 8. Address Identification
    println!("\n🔍 ADDRESS IDENTIFICATION");
    println!("{}", "-".repeat(70));
    
    let test_addresses = vec![
        "0x0171F896002665C3ea3Ed0c55f21026cA0A734A0", // BlackRock
        "0x210350289675a16b60Aa6ed0796F12f2Fd6CA45c", // Grayscale
        "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb", // Random
    ];
    
    for addr_str in test_addresses {
        let addr = Address::from_str(addr_str)?;
        if let Some(etf_info) = get_etf_by_address(addr) {
            println!("✅ {} is {} ETF", 
                &addr_str[..10], 
                etf_info.provider
            );
        } else {
            println!("❌ {} is not a known ETF", &addr_str[..10]);
        }
    }
    
    // 9. Top Providers Summary
    println!("\n🏆 TOP 5 ETF PROVIDERS");
    println!("{}", "-".repeat(70));
    
    let top_providers = holdings_tracker.get_top_providers(5, None).await?;
    for (i, (provider, holdings)) in top_providers.iter().enumerate() {
        println!("{}. {} - {:.2} ETH (${:.2}M)",
            i + 1,
            provider,
            holdings,
            holdings * 2000.0 / 1_000_000.0
        );
    }
    
    println!("\n✅ ETF holdings analysis complete!");
    
    Ok(())
}