//! Fund flow analysis example - demonstrates analyzing fund flows

use qarqa_core_types::*;
use qarqa_core_types::utils::*;
use qarqa_tx_simulation::{FundFlowAnalyzer};
use alloy_primitives::{Address, U256, B256};
use std::str::FromStr;
use chrono;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("=== Fund Flow Analysis Example ===\n");
    
    if let Err(e) = run_example().await {
        eprintln!("Example failed: {}", e);
        std::process::exit(1);
    }
}

async fn run_example() -> QarqaResult<()> {
    // Create fund flow analyzer
    let analyzer = FundFlowAnalyzer::new()
        .with_min_value(U256::from(1000)) // 1000 wei minimum
        .with_gas_inclusion(true)
        .with_weth_as_eth(true);
    
    println!("✓ Fund flow analyzer configured");
    println!("  Min value threshold: 1000 wei");
    println!("  Include gas payments: Yes");
    println!("  Treat WETH as ETH: Yes\n");
    
    // Create test fund flows data
    let complete_flows = create_test_fund_flows()?;
    
    println!("1. Analyzing {} sets of complete fund flows:", complete_flows.len());
    for (i, flows) in complete_flows.iter().enumerate() {
        println!("   {}. Block {}: {} ETH movements, {} token movements", 
                 i + 1, 
                 flows.block_number,
                 flows.eth_movements.len(),
                 flows.token_movements.len());
    }
    println!();
    
    // Analyze fund flows
    let fund_flows = analyzer.analyze_fund_flows(&complete_flows)?;
    println!("2. Extracted {} unique fund flows:", fund_flows.len());
    
    for (i, flow) in fund_flows.iter().enumerate() {
        println!("   {}. {} → {}", 
                 i + 1,
                 format_address(flow.from),
                 format_address(flow.to));
        println!("      ETH: {} | Transactions: {} | Type: {:?}",
                 flow.amount_eth,
                 flow.transaction_count,
                 flow.flow_type);
    }
    println!();
    
    // Calculate net balances
    let net_balances = analyzer.calculate_net_balances(&fund_flows);
    println!("3. Net balance analysis for {} addresses:", net_balances.len());
    
    for (address, balance) in net_balances.iter() {
        println!("   {}: Net ETH: {:.6} (In: {:.6}, Out: {:.6})", 
                 format_address(*address),
                 balance.net_eth,
                 balance.eth_in,
                 balance.eth_out);
    }
    println!();
    
    // Demonstrate different analyzer configurations
    demonstrate_analyzer_configurations(&complete_flows).await?;
    
    // Show analysis insights
    demonstrate_analysis_insights(&fund_flows, &net_balances);
    
    println!("\n=== Fund flow analysis completed! ===");
    
    Ok(())
}

fn create_test_fund_flows() -> QarqaResult<Vec<CompleteFundFlows>> {
    let mut flows = Vec::new();
    
    // Transaction 1: Simple ETH transfer
    flows.push(CompleteFundFlows {
        transaction_hash: B256::from_str("0x1111111111111111111111111111111111111111111111111111111111111111")?,
        block_number: 18500000,
        timestamp: chrono::Utc::now(),
        eth_movements: vec![
            EthMovement {
                from: Address::from_str("0x1111111111111111111111111111111111111111")?,
                to: Address::from_str("0x2222222222222222222222222222222222222222")?,
                amount: U256::from_str("1000000000000000000")?, // 1 ETH
                movement_type: EthMovementType::Direct,
            },
            EthMovement {
                from: Address::from_str("0x1111111111111111111111111111111111111111")?,
                to: Address::ZERO, // Miners
                amount: U256::from_str("420000000000000")?, // Gas payment
                movement_type: EthMovementType::Gas,
            }
        ],
        token_movements: Vec::new(),
        gas_used: 21000,
        status: true,
    });
    
    // Transaction 2: Multiple movements
    flows.push(CompleteFundFlows {
        transaction_hash: B256::from_str("0x2222222222222222222222222222222222222222222222222222222222222222")?,
        block_number: 18500001,
        timestamp: chrono::Utc::now(),
        eth_movements: vec![
            EthMovement {
                from: Address::from_str("0x3333333333333333333333333333333333333333")?,
                to: Address::from_str("0x4444444444444444444444444444444444444444")?,
                amount: U256::from_str("500000000000000000")?, // 0.5 ETH
                movement_type: EthMovementType::Direct,
            },
            EthMovement {
                from: Address::from_str("0x4444444444444444444444444444444444444444")?,
                to: Address::from_str("0x5555555555555555555555555555555555555555")?,
                amount: U256::from_str("200000000000000000")?, // 0.2 ETH internal
                movement_type: EthMovementType::Internal,
            },
            EthMovement {
                from: Address::from_str("0x3333333333333333333333333333333333333333")?,
                to: Address::ZERO,
                amount: U256::from_str("1500000000000000")?, // Gas payment
                movement_type: EthMovementType::Gas,
            }
        ],
        token_movements: vec![
            TokenMovement {
                token_address: Address::from_str("0x6666666666666666666666666666666666666666")?,
                from: Address::from_str("0x3333333333333333333333333333333333333333")?,
                to: Address::from_str("0x4444444444444444444444444444444444444444")?,
                amount: U256::from_str("1000000")?, // 1 USDC (6 decimals)
                symbol: Some("USDC".to_string()),
                decimals: Some(6),
            }
        ],
        gas_used: 75000,
        status: true,
    });
    
    // Transaction 3: Same addresses (should be aggregated)
    flows.push(CompleteFundFlows {
        transaction_hash: B256::from_str("0x3333333333333333333333333333333333333333333333333333333333333333")?,
        block_number: 18500002,
        timestamp: chrono::Utc::now(),
        eth_movements: vec![
            EthMovement {
                from: Address::from_str("0x1111111111111111111111111111111111111111")?,
                to: Address::from_str("0x2222222222222222222222222222222222222222")?,
                amount: U256::from_str("2000000000000000000")?, // 2 ETH
                movement_type: EthMovementType::Direct,
            },
            EthMovement {
                from: Address::from_str("0x1111111111111111111111111111111111111111")?,
                to: Address::ZERO,
                amount: U256::from_str("630000000000000")?, // Gas payment
                movement_type: EthMovementType::Gas,
            }
        ],
        token_movements: Vec::new(),
        gas_used: 31500,
        status: true,
    });
    
    Ok(flows)
}

async fn demonstrate_analyzer_configurations(complete_flows: &[CompleteFundFlows]) -> QarqaResult<()> {
    println!("4. Testing different analyzer configurations:");
    
    // Configuration 1: Exclude gas payments
    let analyzer_no_gas = FundFlowAnalyzer::new()
        .with_gas_inclusion(false);
    
    let flows_no_gas = analyzer_no_gas.analyze_fund_flows(complete_flows)?;
    println!("   Without gas payments: {} fund flows", flows_no_gas.len());
    
    // Configuration 2: Higher minimum value
    let analyzer_high_min = FundFlowAnalyzer::new()
        .with_min_value(U256::from_str("500000000000000000")?); // 0.5 ETH minimum
    
    let flows_high_min = analyzer_high_min.analyze_fund_flows(complete_flows)?;
    println!("   High minimum (0.5 ETH): {} fund flows", flows_high_min.len());
    
    // Configuration 3: No WETH conversion
    let analyzer_no_weth = FundFlowAnalyzer::new()
        .with_weth_as_eth(false);
    
    let flows_no_weth = analyzer_no_weth.analyze_fund_flows(complete_flows)?;
    println!("   No WETH conversion: {} fund flows", flows_no_weth.len());
    
    println!();
    
    Ok(())
}

fn demonstrate_analysis_insights(fund_flows: &[FundFlow], net_balances: &std::collections::HashMap<Address, qarqa_tx_simulation::NetBalance>) {
    println!("5. Analysis Insights:");
    
    // Find most active addresses
    let mut active_addresses: Vec<_> = net_balances.iter()
        .map(|(addr, balance)| (*addr, balance.tx_count))
        .collect();
    active_addresses.sort_by(|a, b| b.1.cmp(&a.1));
    
    println!("   Most active addresses:");
    for (i, (addr, tx_count)) in active_addresses.iter().take(3).enumerate() {
        println!("     {}. {}: {} transactions", 
                 i + 1, 
                 format_address(*addr), 
                 tx_count);
    }
    
    // Find largest flows
    let mut largest_flows = fund_flows.to_vec();
    largest_flows.sort_by(|a, b| b.amount_eth.partial_cmp(&a.amount_eth).unwrap_or(std::cmp::Ordering::Equal));
    
    println!("   Largest fund flows:");
    for (i, flow) in largest_flows.iter().take(3).enumerate() {
        println!("     {}. {} → {}: {} ETH", 
                 i + 1,
                 format_address(flow.from),
                 format_address(flow.to),
                 flow.amount_eth);
    }
    
    // Summary statistics
    let total_eth_volume: f64 = fund_flows.iter()
        .filter(|f| !matches!(f.flow_type, FlowType::GasPayment))
        .map(|f| f.amount_eth)
        .sum();
    
    let total_gas_paid: f64 = fund_flows.iter()
        .filter(|f| matches!(f.flow_type, FlowType::GasPayment))
        .map(|f| f.amount_eth)
        .sum();
    
    println!("   Summary:");
    println!("     Total ETH volume: {} ETH", total_eth_volume);
    println!("     Total gas paid: {} ETH", total_gas_paid);
    println!("     Average flow size: {} ETH", 
             if fund_flows.is_empty() { 0.0 } else { total_eth_volume / fund_flows.len() as f64 });
    
    // Flow type distribution
    let mut flow_type_counts = std::collections::HashMap::new();
    for flow in fund_flows {
        *flow_type_counts.entry(&flow.flow_type).or_insert(0) += 1;
    }
    
    println!("   Flow type distribution:");
    for (flow_type, count) in flow_type_counts.iter() {
        println!("     {:?}: {}", flow_type, count);
    }
}