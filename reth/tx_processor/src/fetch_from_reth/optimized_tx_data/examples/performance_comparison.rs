//! Example: Performance Comparison (All Three Approaches)
//! 
//! This demonstrates the performance differences between all three data retrieval approaches:
//! 1. Basic (database-only) - Fastest ~1ms
//! 2. Smart (conditional simulation) - Intelligent ~1-800ms
//! 3. Complete (always simulate) - Slowest but complete ~80-800ms
//!
//! Perfect for understanding when to use each approach and the trade-offs involved.
//!
//! To run as a binary:
//!   cargo run --bin optimized_tx_comparison [tx_hash]

use anyhow::Result;
use std::env;
use std::str::FromStr;
use std::time::Instant;

use ethers_core::types::H256;
use revm_tx_simulator_lib::fetch_from_reth::optimized_tx_data::{
    get_basic_transaction_data,
    get_smart_transaction_data,
    get_full_transaction_analysis,
    TransactionDataOptions,
};

#[tokio::main]
async fn main() -> Result<()> {
    println!("⚡ Performance Comparison: Basic vs Smart vs Complete");
    println!("====================================================\n");

    // Get transaction hash from args or use default
    let args: Vec<String> = env::args().collect();
    let tx_hash_str = if args.len() > 1 {
        &args[1]
    } else {
        // Use the same audited transaction for consistency
        "0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae"
    };

    let tx_hash = H256::from_str(tx_hash_str)?;
    println!("📥 Transaction: {}", tx_hash);
    println!("🎯 Testing all three optimization levels\n");

    // Test 1: Basic (Database-only)
    println!("🔄 Test 1: Basic Data Retrieval (Database-Only)");
    println!("===============================================");
    
    let basic_start = Instant::now();
    let basic_options = TransactionDataOptions::basic();
    let basic_data = get_basic_transaction_data(tx_hash, basic_options).await?;
    let basic_duration = basic_start.elapsed();
    
    println!("✅ Basic completed in {:.3}ms", basic_duration.as_secs_f64() * 1000.0);
    println!("   Data Source: {}", basic_data.performance.data_source);
    println!("   Optimization Applied: {}", basic_data.performance.optimization_applied);
    println!("   ERC20 Transfers: {}", basic_data.erc20_transfers.len());
    println!("   Event Logs: {}", basic_data.log_count);
    println!("   Internal Transfers: N/A (not included)");

    // Test 2: Smart (Conditional simulation)
    println!("\n🔄 Test 2: Smart Data Retrieval (Auto-Detection)");
    println!("================================================");
    
    let smart_start = Instant::now();
    let smart_options = TransactionDataOptions::smart();
    let smart_data = get_smart_transaction_data(tx_hash, smart_options).await?;
    let smart_duration = smart_start.elapsed();
    
    println!("✅ Smart completed in {:.3}ms", smart_duration.as_secs_f64() * 1000.0);
    println!("   Data Source: {}", smart_data.basic.performance.data_source);
    println!("   Transaction Type: {:?}", smart_data.transaction_type);
    println!("   Simulation Performed: {}", smart_data.simulation_performed);
    println!("   Optimization Applied: {}", smart_data.basic.performance.optimization_applied);
    println!("   ERC20 Transfers: {}", smart_data.basic.erc20_transfers.len());
    println!("   Event Logs: {}", smart_data.basic.log_count);
    if let Some(ref internal) = smart_data.internal_transfers {
        println!("   Internal Transfers: {}", internal.len());
    } else {
        println!("   Internal Transfers: N/A (optimization applied)");
    }

    // Test 3: Complete (Always simulate)
    println!("\n🔄 Test 3: Complete Data Analysis (Always Simulate)");
    println!("==================================================");
    
    let complete_start = Instant::now();
    let complete_options = TransactionDataOptions::complete();
    let complete_data = get_full_transaction_analysis(tx_hash, complete_options).await?;
    let complete_duration = complete_start.elapsed();
    
    println!("✅ Complete completed in {:.3}ms", complete_duration.as_secs_f64() * 1000.0);
    println!("   Data Source: {}", complete_data.smart.basic.performance.data_source);
    println!("   Simulation Success: {}", complete_data.simulation_success);
    println!("   Optimization Applied: {}", complete_data.smart.basic.performance.optimization_applied);
    println!("   ERC20 Transfers: {}", complete_data.smart.basic.erc20_transfers.len());
    println!("   Event Logs: {}", complete_data.smart.basic.log_count);
    println!("   Internal Transfers: {}", complete_data.internal_transfers.len());
    println!("   Addresses Affected: {}", complete_data.addresses_affected);
    println!("   Gas Refunded: {}", complete_data.gas_refunded);

    // Performance Analysis
    println!("\n📊 Performance Analysis");
    println!("======================");
    
    let basic_ms = basic_duration.as_secs_f64() * 1000.0;
    let smart_ms = smart_duration.as_secs_f64() * 1000.0;
    let complete_ms = complete_duration.as_secs_f64() * 1000.0;
    
    println!("   Basic:    {:.3}ms", basic_ms);
    println!("   Smart:    {:.3}ms", smart_ms);
    println!("   Complete: {:.3}ms", complete_ms);
    
    // Speed comparisons
    println!("\n🚀 Speed Comparisons");
    println!("===================");
    
    if smart_ms > 0.0 {
        let basic_vs_smart = smart_ms / basic_ms;
        println!("   Basic vs Smart:    {:.1}x faster", basic_vs_smart);
    }
    
    if complete_ms > 0.0 {
        let basic_vs_complete = complete_ms / basic_ms;
        let smart_vs_complete = complete_ms / smart_ms;
        println!("   Basic vs Complete: {:.1}x faster", basic_vs_complete);
        println!("   Smart vs Complete: {:.1}x faster", smart_vs_complete);
    }

    // Data Completeness Comparison
    println!("\n📋 Data Completeness Comparison");
    println!("===============================");
    
    println!("   Feature                | Basic | Smart | Complete");
    println!("   -----------------------|-------|-------|----------");
    println!("   Transaction Details    |  ✅   |  ✅   |   ✅");
    println!("   Receipt & Logs         |  ✅   |  ✅   |   ✅");
    println!("   ERC20 Transfers        |  ✅   |  ✅   |   ✅");
    println!("   Transaction Type       |  ❌   |  ✅   |   ✅");
    println!("   Internal Transfers     |  ❌   |  {}   |   ✅", 
             if smart_data.simulation_performed { "✅" } else { "❌" });
    println!("   Call Traces            |  ❌   |  ❌   |   ✅");
    println!("   State Changes          |  ❌   |  ❌   |   ✅");
    println!("   Gas Refunds            |  ❌   |  ❌   |   ✅");

    // Use Case Recommendations
    println!("\n💡 Use Case Recommendations");
    println!("===========================");
    
    println!("📱 Basic Mode - Use When:");
    println!("   • Building transaction history displays");
    println!("   • Processing high volumes of transactions");
    println!("   • Only need basic transaction info + ERC20 transfers");
    println!("   • Performance is critical (APIs, dashboards)");
    println!("   • Speed requirement: < 10ms");
    
    println!("\n🧠 Smart Mode - Use When:");
    println!("   • Want optimal performance automatically");
    println!("   • Have mixed transaction types (simple + complex)");
    println!("   • Need internal transfers for some transactions");
    println!("   • Building general-purpose transaction APIs");
    println!("   • Balance speed and completeness");
    
    println!("\n🔬 Complete Mode - Use When:");
    println!("   • Need detailed DeFi analysis");
    println!("   • Debugging complex transactions");
    println!("   • Analyzing MEV or arbitrage");
    println!("   • Building advanced analytics tools");
    println!("   • Completeness more important than speed");

    // Performance Efficiency Analysis
    println!("\n⚡ Efficiency Analysis");
    println!("=====================");
    
    let basic_efficiency = 100.0; // Baseline
    let smart_efficiency = (basic_ms / smart_ms) * 100.0;
    let complete_efficiency = (basic_ms / complete_ms) * 100.0;
    
    println!("   Basic:    {:.1}% efficiency (baseline)", basic_efficiency);
    println!("   Smart:    {:.1}% efficiency", smart_efficiency);
    println!("   Complete: {:.1}% efficiency", complete_efficiency);
    
    // Smart mode analysis
    if smart_data.simulation_performed {
        println!("\n🤖 Smart Mode Decision: Simulation was needed");
        println!("   Reason: {:?} transactions typically have internal transfers", smart_data.transaction_type);
        println!("   Trade-off: Slower but includes complete data");
    } else {
        println!("\n🤖 Smart Mode Decision: Optimization applied");
        println!("   Reason: {:?} transactions typically don't need simulation", smart_data.transaction_type);
        println!("   Benefit: {:.1}x speedup with sufficient data", complete_ms / smart_ms);
    }

    // Final Recommendations
    println!("\n🎯 Final Recommendation for This Transaction");
    println!("============================================");
    
    match smart_data.transaction_type {
        revm_tx_simulator_lib::optimized_tx_data::TransactionType::SimpleTransfer => {
            println!("✅ Recommended: Basic Mode");
            println!("   This simple transfer doesn't need simulation");
            println!("   Gain: {:.1}x speedup with no data loss", complete_ms / basic_ms);
        }
        revm_tx_simulator_lib::optimized_tx_data::TransactionType::ContractCall => {
            if smart_data.simulation_performed {
                println!("🧠 Recommended: Smart Mode (auto-selected simulation)");
                println!("   This contract call had complex patterns");
            } else {
                println!("✅ Recommended: Basic Mode");
                println!("   This simple contract call doesn't need simulation");
            }
        }
        revm_tx_simulator_lib::optimized_tx_data::TransactionType::DexInteraction |
        revm_tx_simulator_lib::optimized_tx_data::TransactionType::ComplexDeFi => {
            println!("🔬 Recommended: Complete Mode");
            println!("   This complex transaction benefits from full analysis");
            println!("   Internal transfers: {}", complete_data.internal_transfers.len());
        }
        revm_tx_simulator_lib::optimized_tx_data::TransactionType::ContractDeployment => {
            println!("✅ Recommended: Basic Mode");
            println!("   Contract deployments rarely need simulation");
        }
    }

    println!("\n✅ Performance comparison completed!");
    println!("💡 Choose the approach that best fits your speed vs completeness requirements.");

    Ok(())
}