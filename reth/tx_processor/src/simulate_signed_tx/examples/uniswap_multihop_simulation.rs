//! Example: Analyzing Uniswap multi-hop swaps
//! 
//! This example demonstrates how to analyze complex DeFi transactions
//! that involve multiple Uniswap swaps across different versions.
//!
//! To run as a binary:
//!   cargo run --bin uniswap_multihop_simulation

use anyhow::Result;
use ethers_core::types::H256;
use std::str::FromStr;
use std::collections::HashMap;

// Import the main simulation function
use revm_tx_simulator_lib::simulate_signed_tx::simulate_signed_tx;

const RPC_URL: &str = "http://127.0.0.1:8545";

// Known Uniswap contracts
const UNISWAP_V2_ROUTER: &str = "0x7a250d5630b4cf539739df2c5dacb4c659f2488d";
const UNISWAP_V3_ROUTER: &str = "0xe592427a0aece92de3edee1f18e0157c05861564";
const UNISWAP_V4_POOL_MANAGER: &str = "0x000000000004444c5dc75cb358380d2e3de08a90";
const UNIVERSAL_ROUTER: &str = "0xfbd4cdb413e45a52e2c8312f670e9ce67e794c37";

// Common tokens
const WETH: &str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";
const USDC: &str = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
const USDT: &str = "0xdac17f958d2ee523a2206206994597c13d831ec7";

#[tokio::main]
async fn main() -> Result<()> {
    println!("🦄 Uniswap Multi-Hop Swap Analysis");
    println!("==================================\n");
    
    // Transaction with Uniswap swaps
    let tx_hash = H256::from_str("0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae")?;
    
    println!("📋 Analyzing transaction: {:?}", tx_hash);
    println!("⏳ Simulating transaction...\n");
    
    let output = simulate_signed_tx(tx_hash, RPC_URL).await?;
    
    // Basic results
    println!("📊 Simulation Results:");
    println!("  • Status: {:?}", output.result_type);
    println!("  • Gas Used: {}", output.gas_used);
    println!("  • Total Events: {}", output.logs.len());
    
    // Identify Uniswap interactions
    println!("\n🔍 Uniswap Protocol Detection:");
    let uniswap_logs = analyze_uniswap_logs(&output.logs);
    
    if uniswap_logs.v2_swaps > 0 {
        println!("  • Uniswap V2 Swaps: {}", uniswap_logs.v2_swaps);
    }
    if uniswap_logs.v3_swaps > 0 {
        println!("  • Uniswap V3 Swaps: {}", uniswap_logs.v3_swaps);
    }
    if uniswap_logs.v4_events > 0 {
        println!("  • Uniswap V4 Events: {}", uniswap_logs.v4_events);
    }
    
    // Analyze swap routing
    println!("\n📍 Swap Routing Analysis:");
    let swaps = extract_swap_info(&output.logs);
    
    if swaps.is_empty() {
        println!("  No Uniswap swaps detected");
    } else {
        println!("  Total Swaps: {}", swaps.len());
        
        // Show swap details
        for (i, swap) in swaps.iter().enumerate() {
            println!("\n  Swap #{}:", i + 1);
            println!("    Protocol: {}", swap.protocol);
            println!("    Pool: {}", format_address(&swap.pool));
            if let Some(amount) = swap.amount_info {
                println!("    Amount: {} units", amount);
            }
        }
    }
    
    // Token flow analysis
    println!("\n💰 Token Flow Analysis:");
    let token_transfers = analyze_token_flows(&output.logs);
    
    for (token, info) in token_transfers.iter() {
        println!("  {}:", identify_token(token));
        println!("    • Transfer Count: {}", info.transfer_count);
        println!("    • Unique Addresses: {}", info.unique_addresses.len());
    }
    
    // Multi-hop detection
    if swaps.len() > 1 {
        println!("\n🛤️  Multi-Hop Swap Detected!");
        println!("  • {} hops through different pools", swaps.len());
        println!("  • Gas per hop: ~{}", output.gas_used / swaps.len() as u64);
    }
    
    // Router analysis
    println!("\n🔧 Router Analysis:");
    let router_events = analyze_router_usage(&output.logs);
    
    for (router, count) in router_events.iter() {
        println!("  • {}: {} events", identify_router(router), count);
    }
    
    // MEV indicators
    println!("\n⚡ MEV Indicators:");
    if swaps.len() > 2 {
        println!("  • Complex routing detected - potential arbitrage");
    }
    
    let has_weth_operations = output.logs.iter().any(|log| {
        format!("{:x}", log.address).to_lowercase() == WETH
    });
    
    if has_weth_operations {
        println!("  • WETH wrapping/unwrapping detected");
    }
    
    // Price impact estimation
    if swaps.len() > 0 {
        println!("\n📈 Trading Insights:");
        println!("  • Average gas per swap: {}", output.gas_used / swaps.len() as u64);
        println!("  • Execution efficiency: {:.2}%", 
            (output.gas_refunded as f64 / output.gas_used as f64) * 100.0
        );
    }
    
    Ok(())
}

struct UniswapStats {
    v2_swaps: usize,
    v3_swaps: usize,
    v4_events: usize,
}

fn analyze_uniswap_logs(logs: &[alloy_primitives::Log]) -> UniswapStats {
    let mut stats = UniswapStats {
        v2_swaps: 0,
        v3_swaps: 0,
        v4_events: 0,
    };
    
    for log in logs {
        if let Some(topic) = log.topics().first() {
            let sig = format!("{:x}", topic);
            
            // V2 Swap event
            if sig == "d78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822" {
                stats.v2_swaps += 1;
            }
            // V3 Swap event
            else if sig == "c42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67" {
                stats.v3_swaps += 1;
            }
        }
        
        // V4 events come from pool manager
        let addr = format!("{:x}", log.address).to_lowercase();
        if addr == UNISWAP_V4_POOL_MANAGER {
            stats.v4_events += 1;
        }
    }
    
    stats
}

struct SwapInfo {
    protocol: String,
    pool: alloy_primitives::Address,
    amount_info: Option<u128>,
}

fn extract_swap_info(logs: &[alloy_primitives::Log]) -> Vec<SwapInfo> {
    let mut swaps = Vec::new();
    
    for log in logs {
        if let Some(topic) = log.topics().first() {
            let sig = format!("{:x}", topic);
            
            // V2 Swap
            if sig == "d78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822" {
                swaps.push(SwapInfo {
                    protocol: "Uniswap V2".to_string(),
                    pool: log.address,
                    amount_info: None, // Would need to decode data
                });
            }
            // V3 Swap
            else if sig == "c42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67" {
                swaps.push(SwapInfo {
                    protocol: "Uniswap V3".to_string(),
                    pool: log.address,
                    amount_info: None,
                });
            }
        }
    }
    
    swaps
}

struct TokenFlowInfo {
    transfer_count: usize,
    unique_addresses: std::collections::HashSet<alloy_primitives::Address>,
}

fn analyze_token_flows(logs: &[alloy_primitives::Log]) -> HashMap<alloy_primitives::Address, TokenFlowInfo> {
    let mut flows = HashMap::new();
    
    for log in logs {
        if let Some(topic) = log.topics().first() {
            let sig = format!("{:x}", topic);
            
            // ERC20 Transfer event
            if sig == "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef" {
                let info = flows.entry(log.address).or_insert(TokenFlowInfo {
                    transfer_count: 0,
                    unique_addresses: std::collections::HashSet::new(),
                });
                
                info.transfer_count += 1;
                
                // Extract from/to addresses from topics
                if log.topics().len() >= 3 {
                    // Topics[1] is from, topics[2] is to
                    info.unique_addresses.insert(alloy_primitives::Address::default());
                }
            }
        }
    }
    
    flows
}

fn analyze_router_usage(logs: &[alloy_primitives::Log]) -> HashMap<String, usize> {
    let mut routers = HashMap::new();
    
    let known_routers = vec![
        (UNISWAP_V2_ROUTER, "V2 Router"),
        (UNISWAP_V3_ROUTER, "V3 Router"),
        (UNIVERSAL_ROUTER, "Universal Router"),
        (UNISWAP_V4_POOL_MANAGER, "V4 Pool Manager"),
    ];
    
    for log in logs {
        let addr = format!("{:x}", log.address).to_lowercase();
        
        for (router_addr, name) in &known_routers {
            if addr == *router_addr {
                *routers.entry(name.to_string()).or_insert(0) += 1;
            }
        }
    }
    
    routers
}

fn identify_token(addr: &alloy_primitives::Address) -> String {
    let addr_str = format!("{:x}", addr).to_lowercase();
    
    match addr_str.as_str() {
        s if s == WETH => "WETH".to_string(),
        s if s == USDC => "USDC".to_string(),
        s if s == USDT => "USDT".to_string(),
        _ => format_address(addr),
    }
}

fn identify_router(name: &str) -> &str {
    name
}

fn format_address(addr: &alloy_primitives::Address) -> String {
    let s = format!("{:x}", addr);
    if s.len() > 10 {
        format!("0x{}...{}", &s[..4], &s[s.len()-4..])
    } else {
        format!("0x{}", s)
    }
}