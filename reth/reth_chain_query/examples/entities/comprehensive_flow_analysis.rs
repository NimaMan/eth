/// Comprehensive flow analysis example
/// 
/// Demonstrates the complete flow analysis system with time conversion:
/// - ETF flows with time aggregation
/// - CEX flows with concentration analysis
/// - Stablecoin supply changes over time
/// - Cross-entity correlations

use reth_chain_query::{
    ChainQuery, 
    EtfFlowAnalyzer, 
    CexFlowAnalyzer,
    StablecoinSupplyTracker,
    PeriodType,
};
use chrono::{Utc, Duration};
use std::sync::Arc;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Initialize ChainQuery
    let reth_datadir = "/home/nima/.local/share/reth/mainnet";
    let chain_query = Arc::new(ChainQuery::new(reth_datadir)?);
    
    println!("📊 Comprehensive Entity Flow Analysis");
    println!("=" .repeat(60));
    
    // Get time boundaries for analysis
    let now = Utc::now();
    let analysis_periods = vec![
        ("1 Hour", Duration::hours(1)),
        ("24 Hours", Duration::days(1)),
        ("7 Days", Duration::weeks(1)),
    ];
    
    for (period_name, duration) in analysis_periods {
        println!("\n🕐 Analysis Period: {}", period_name);
        println!("-" .repeat(50));
        
        let start_time = now - duration;
        let (from_block, to_block) = chain_query.time_converter
            .get_blocks_for_period(start_time, now)
            .await?;
        
        println!("Time range: {} to {}", 
            start_time.format("%Y-%m-%d %H:%M UTC"),
            now.format("%Y-%m-%d %H:%M UTC")
        );
        println!("Block range: {} to {} ({} blocks)\n",
            from_block,
            to_block,
            to_block - from_block
        );
        
        // 1. ETF Flow Analysis
        analyze_etf_flows(&chain_query, from_block, to_block).await?;
        
        // 2. CEX Flow Analysis
        analyze_cex_flows(&chain_query, from_block, to_block).await?;
        
        // 3. Stablecoin Supply Analysis
        analyze_stablecoin_supply(&chain_query, from_block, to_block).await?;
    }
    
    // Cross-entity correlation analysis
    println!("\n🔄 Cross-Entity Correlation Analysis");
    println!("=" .repeat(60));
    
    analyze_cross_entity_flows(&chain_query).await?;
    
    println!("\n✅ Comprehensive Analysis Complete!");
    
    Ok(())
}

async fn analyze_etf_flows(
    chain_query: &Arc<ChainQuery>,
    from_block: u64,
    to_block: u64,
) -> eyre::Result<()> {
    println!("📈 ETF Flow Analysis:");
    
    let etf_analyzer = EtfFlowAnalyzer::new(chain_query.clone());
    let flows = etf_analyzer.calculate_flows_between_blocks(from_block, to_block).await?;
    
    // Top providers by flow
    let mut provider_flows: Vec<_> = flows.provider_flows.iter().collect();
    provider_flows.sort_by(|a, b| b.1.net_flow_eth.partial_cmp(&a.1.net_flow_eth).unwrap());
    
    println!("  Top ETF Providers by Net Flow:");
    for (provider, data) in provider_flows.iter().take(5) {
        if data.net_flow_eth.abs() > 0.01 {
            let flow_type = if data.net_flow_eth > 0.0 { "↗️ IN" } else { "↘️ OUT" };
            println!("    {} {}: {:.2} ETH (in: {:.2}, out: {:.2})",
                flow_type,
                provider,
                data.net_flow_eth,
                data.inflow_eth,
                data.outflow_eth
            );
        }
    }
    
    println!("  Summary:");
    println!("    Total Inflow:  {:.2} ETH", flows.total_inflow);
    println!("    Total Outflow: {:.2} ETH", flows.total_outflow);
    println!("    Net Flow:      {:.2} ETH", flows.net_flow);
    
    Ok(())
}

async fn analyze_cex_flows(
    chain_query: &Arc<ChainQuery>,
    from_block: u64,
    to_block: u64,
) -> eyre::Result<()> {
    println!("\n💱 CEX Flow Analysis:");
    
    let cex_analyzer = CexFlowAnalyzer::new(chain_query.clone());
    let flows = cex_analyzer.calculate_flows_between_blocks(from_block, to_block).await?;
    
    // Top exchanges by flow
    let mut exchange_flows: Vec<_> = flows.exchange_flows.iter().collect();
    exchange_flows.sort_by(|a, b| b.1.net_flow_eth.partial_cmp(&a.1.net_flow_eth).unwrap());
    
    println!("  Top Exchanges by Net Flow:");
    for (exchange, data) in exchange_flows.iter().take(5) {
        if data.net_flow_eth.abs() > 0.01 {
            let flow_type = if data.net_flow_eth > 0.0 { "↗️ IN" } else { "↘️ OUT" };
            println!("    {} {}: {:.2} ETH (in: {:.2}, out: {:.2})",
                flow_type,
                exchange,
                data.net_flow_eth,
                data.inflow_eth,
                data.outflow_eth
            );
        }
    }
    
    println!("  Summary:");
    println!("    Total Inflow:  {:.2} ETH", flows.total_inflow);
    println!("    Total Outflow: {:.2} ETH", flows.total_outflow);
    println!("    Net Flow:      {:.2} ETH", flows.net_flow);
    
    // Concentration metrics
    let balances = cex_analyzer.get_all_exchange_balances(Some(to_block)).await?;
    let total_cex_eth: f64 = balances.values().sum();
    let top_3_eth: f64 = {
        let mut sorted: Vec<_> = balances.values().cloned().collect();
        sorted.sort_by(|a, b| b.partial_cmp(a).unwrap());
        sorted.iter().take(3).sum()
    };
    
    if total_cex_eth > 0.0 {
        let concentration = (top_3_eth / total_cex_eth) * 100.0;
        println!("  Market Concentration:");
        println!("    Total CEX Holdings: {:.2} ETH", total_cex_eth);
        println!("    Top 3 Concentration: {:.1}%", concentration);
    }
    
    Ok(())
}

async fn analyze_stablecoin_supply(
    chain_query: &Arc<ChainQuery>,
    from_block: u64,
    to_block: u64,
) -> eyre::Result<()> {
    println!("\n💵 Stablecoin Supply Analysis:");
    
    let supply_tracker = StablecoinSupplyTracker::new(chain_query.clone());
    let supply_changes = supply_tracker
        .calculate_supply_changes_between_blocks(from_block, to_block)
        .await?;
    
    println!("  Top Supply Changes:");
    for (token, data) in supply_changes.token_changes.iter().take(5) {
        if data.net_change.abs() > 1000.0 {
            let change_type = if data.net_change > 0.0 { "📈" } else { "📉" };
            println!("    {} {}: ${:.0} ({:+.2}%)",
                change_type,
                token,
                data.net_change,
                data.percent_change
            );
            if data.minted > 0.0 {
                println!("      Minted: ${:.0}", data.minted);
            }
            if data.burned > 0.0 {
                println!("      Burned: ${:.0}", data.burned);
            }
        }
    }
    
    println!("  Summary:");
    println!("    Total Minted: ${:.0}", supply_changes.total_minted);
    println!("    Total Burned: ${:.0}", supply_changes.total_burned);
    println!("    Net Change:   ${:.0}", supply_changes.total_net_change);
    
    Ok(())
}

async fn analyze_cross_entity_flows(chain_query: &Arc<ChainQuery>) -> eyre::Result<()> {
    // Analyze correlations between different entity types
    let latest_block = chain_query.get_latest_block()?;
    let one_day_blocks = 7200;
    let from_block = latest_block - one_day_blocks;
    
    println!("\n🔄 24-Hour Cross-Entity Analysis:");
    println!("-" .repeat(50));
    
    // Get all flows
    let etf_analyzer = EtfFlowAnalyzer::new(chain_query.clone());
    let cex_analyzer = CexFlowAnalyzer::new(chain_query.clone());
    let supply_tracker = StablecoinSupplyTracker::new(chain_query.clone());
    
    let etf_flows = etf_analyzer.calculate_flows_between_blocks(from_block, latest_block).await?;
    let cex_flows = cex_analyzer.calculate_flows_between_blocks(from_block, latest_block).await?;
    let supply_changes = supply_tracker.calculate_supply_changes_between_blocks(from_block, latest_block).await?;
    
    // Calculate correlations
    let total_institutional_inflow = etf_flows.total_inflow + cex_flows.total_inflow;
    let total_institutional_outflow = etf_flows.total_outflow + cex_flows.total_outflow;
    let net_institutional = total_institutional_inflow - total_institutional_outflow;
    
    println!("  Institutional Flow Summary:");
    println!("    ETF Net Flow:        {:+.2} ETH", etf_flows.net_flow);
    println!("    CEX Net Flow:        {:+.2} ETH", cex_flows.net_flow);
    println!("    Combined Net Flow:   {:+.2} ETH", net_institutional);
    
    println!("\n  Stablecoin Response:");
    println!("    Net Supply Change:   ${:+.0}", supply_changes.total_net_change);
    
    // Simple correlation indicator
    let correlation = if (net_institutional > 0.0 && supply_changes.total_net_change > 0.0) ||
                        (net_institutional < 0.0 && supply_changes.total_net_change < 0.0) {
        "Positive (same direction)"
    } else if net_institutional.abs() < 10.0 || supply_changes.total_net_change.abs() < 10000.0 {
        "Neutral (low activity)"
    } else {
        "Negative (opposite direction)"
    };
    
    println!("\n  Correlation: {}", correlation);
    
    // Market sentiment indicator
    let sentiment = if net_institutional > 100.0 && supply_changes.total_net_change > 1_000_000.0 {
        "🟢 Bullish (institutional accumulation + stablecoin minting)"
    } else if net_institutional < -100.0 && supply_changes.total_net_change < -1_000_000.0 {
        "🔴 Bearish (institutional distribution + stablecoin burning)"
    } else {
        "🟡 Neutral"
    };
    
    println!("  Market Sentiment: {}", sentiment);
    
    Ok(())
}