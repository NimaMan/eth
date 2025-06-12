//! Example: Advanced transaction analysis
//! 
//! This example demonstrates advanced analysis of transaction simulation results,
//! including log parsing, gas analysis, and pattern detection.
//!
//! To run as a binary:
//!   cargo run --bin advanced_tracing

use anyhow::Result;
use ethers_core::types::H256;
use std::str::FromStr;
use std::collections::HashMap;

// Import the main simulation function
use revm_tx_simulator_lib::simulate_signed_tx::simulate_signed_tx;

const RPC_URL: &str = "http://127.0.0.1:8545";

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Advanced Transaction Analysis Example");
    println!("=======================================\n");
    
    // Analyze a complex DeFi transaction
    let tx_hash = H256::from_str("0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae")?;
    
    println!("📋 Analyzing transaction: {:?}", tx_hash);
    println!("⏳ Simulating transaction...\n");
    
    // Simulate the transaction
    let output = simulate_signed_tx(tx_hash, RPC_URL).await?;
    
    // Basic results
    println!("📊 Simulation Results:");
    println!("  • Status: {:?}", output.result_type);
    println!("  • Gas Used: {}", output.gas_used);
    println!("  • Gas Refunded: {} ({:.2}% of used)", 
        output.gas_refunded,
        if output.gas_used > 0 { 
            (output.gas_refunded as f64 / output.gas_used as f64) * 100.0 
        } else { 
            0.0 
        }
    );
    
    // Analyze logs
    println!("\n📜 Log Analysis:");
    let log_stats = analyze_logs(&output.logs);
    println!("  • Total Events: {}", output.logs.len());
    println!("  • Unique Contracts: {}", log_stats.unique_contracts.len());
    println!("  • Event Types: {}", log_stats.event_types.len());
    
    // Detect patterns
    println!("\n🔎 Pattern Detection:");
    
    // Check for ERC20 transfers
    let transfer_count = count_erc20_transfers(&output.logs);
    if transfer_count > 0 {
        println!("  • ERC20 Transfers: {}", transfer_count);
    }
    
    // Check for swaps
    let swap_count = count_swap_events(&output.logs);
    if swap_count > 0 {
        println!("  • DEX Swaps: {}", swap_count);
    }
    
    // Check for approvals
    let approval_count = count_approval_events(&output.logs);
    if approval_count > 0 {
        println!("  • Token Approvals: {}", approval_count);
    }
    
    // Detailed event breakdown
    println!("\n📊 Event Breakdown:");
    for (event_sig, count) in log_stats.event_types.iter().take(5) {
        println!("  • {}: {} occurrences", 
            get_event_name(event_sig).unwrap_or(&format!("0x{}...", &event_sig[..8])),
            count
        );
    }
    
    // Contract interaction analysis
    println!("\n🏭 Top Contract Interactions:");
    let mut contracts: Vec<_> = log_stats.unique_contracts.iter().collect();
    contracts.sort_by(|a, b| b.1.cmp(a.1));
    
    for (addr, count) in contracts.iter().take(5) {
        println!("  • {}: {} events", format_address(addr), count);
    }
    
    // Gas efficiency analysis
    println!("\n⛽ Gas Efficiency Analysis:");
    let gas_per_event = output.gas_used as f64 / output.logs.len() as f64;
    println!("  • Average gas per event: {:.0}", gas_per_event);
    println!("  • Gas refund ratio: {:.2}%", 
        (output.gas_refunded as f64 / output.gas_used as f64) * 100.0
    );
    
    // MEV indicator heuristics
    println!("\n⚡ MEV Indicators:");
    if swap_count > 1 {
        println!("  • Multiple swaps detected - possible arbitrage");
    }
    if has_sandwich_pattern(&output.logs) {
        println!("  • Sandwich attack pattern detected");
    }
    if has_flash_loan_pattern(&output.logs) {
        println!("  • Flash loan usage detected");
    }
    
    Ok(())
}

struct LogStats {
    unique_contracts: HashMap<alloy_primitives::Address, usize>,
    event_types: HashMap<String, usize>,
}

fn analyze_logs(logs: &[alloy_primitives::Log]) -> LogStats {
    let mut stats = LogStats {
        unique_contracts: HashMap::new(),
        event_types: HashMap::new(),
    };
    
    for log in logs {
        // Count contract interactions
        *stats.unique_contracts.entry(log.address).or_insert(0) += 1;
        
        // Count event types
        if let Some(topic) = log.topics().first() {
            let sig = format!("{:x}", topic);
            *stats.event_types.entry(sig).or_insert(0) += 1;
        }
    }
    
    stats
}

fn count_erc20_transfers(logs: &[alloy_primitives::Log]) -> usize {
    // ERC20 Transfer event signature
    const TRANSFER_SIG: &str = "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";
    
    logs.iter()
        .filter(|log| {
            log.topics().first()
                .map(|t| format!("{:x}", t) == TRANSFER_SIG)
                .unwrap_or(false)
        })
        .count()
}

fn count_swap_events(logs: &[alloy_primitives::Log]) -> usize {
    // Common swap event signatures
    const SWAP_V2: &str = "d78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822";
    const SWAP_V3: &str = "c42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67";
    
    logs.iter()
        .filter(|log| {
            log.topics().first()
                .map(|t| {
                    let sig = format!("{:x}", t);
                    sig == SWAP_V2 || sig == SWAP_V3
                })
                .unwrap_or(false)
        })
        .count()
}

fn count_approval_events(logs: &[alloy_primitives::Log]) -> usize {
    // ERC20 Approval event signature
    const APPROVAL_SIG: &str = "8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925";
    
    logs.iter()
        .filter(|log| {
            log.topics().first()
                .map(|t| format!("{:x}", t) == APPROVAL_SIG)
                .unwrap_or(false)
        })
        .count()
}

fn get_event_name(sig: &str) -> Option<&'static str> {
    match sig {
        "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef" => Some("Transfer"),
        "8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925" => Some("Approval"),
        "d78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822" => Some("Swap (V2)"),
        "c42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67" => Some("Swap (V3)"),
        "7fcf532c15f0a6db0bd6d0e038bea71d30d808c7d98cb3bf7268a95bf5081b65" => Some("Withdrawal"),
        _ => None,
    }
}

fn has_sandwich_pattern(logs: &[alloy_primitives::Log]) -> bool {
    // Simple heuristic: look for swap-swap-swap pattern with same token pair
    let swaps: Vec<_> = logs.iter()
        .filter(|log| {
            log.topics().first()
                .map(|t| {
                    let sig = format!("{:x}", t);
                    sig == "d78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822" ||
                    sig == "c42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67"
                })
                .unwrap_or(false)
        })
        .collect();
    
    swaps.len() >= 3
}

fn has_flash_loan_pattern(logs: &[alloy_primitives::Log]) -> bool {
    // Look for large transfers that are returned in the same transaction
    // This is a simplified heuristic
    let transfer_count = count_erc20_transfers(logs);
    transfer_count >= 4 // At least 2 borrow + 2 repay
}

fn format_address(addr: &alloy_primitives::Address) -> String {
    let s = format!("{:x}", addr);
    if s.len() > 10 {
        format!("0x{}...{}", &s[..4], &s[s.len()-4..])
    } else {
        format!("0x{}", s)
    }
}