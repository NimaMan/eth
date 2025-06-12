//! Example: Smart Auto-Detection (Intelligent Optimization)
//! 
//! This demonstrates the smart mode that automatically chooses between database-only
//! and simulation based on transaction characteristics. It provides the best balance
//! of speed and completeness.
//!
//! The system uses heuristics to detect:
//! - Simple transfers (database-only)
//! - DEX interactions (simulation needed)
//! - Complex DeFi (simulation needed)
//! - Contract deployments (database-only)
//!
//! To run as a binary:
//!   cargo run --bin optimized_tx_smart [tx_hash]

use anyhow::Result;
use std::env;
use std::str::FromStr;

use ethers_core::types::H256;
use revm_tx_simulator_lib::fetch_from_reth::optimized_tx_data::{
    get_smart_transaction_data,
    TransactionDataOptions,
    get_transaction_optimization_advice,
};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧠 Smart Auto-Detection (Intelligent Optimization)");
    println!("=================================================\n");

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

    // First, get optimization advice
    println!("🔍 Analyzing transaction for optimization...");
    let advice = get_transaction_optimization_advice(tx_hash, "http://127.0.0.1:8545").await?;
    println!("💡 Optimization Advice: {}\n", advice);

    // Create options for smart data retrieval
    let options = TransactionDataOptions::smart();
    
    println!("🔧 Mode: Smart auto-detection");
    println!("🎯 Goal: Optimal speed vs completeness trade-off\n");

    // Fetch smart transaction data
    println!("🧠 Analyzing transaction and choosing optimal approach...");
    let smart_data = get_smart_transaction_data(tx_hash, options).await?;

    // Display optimization decision
    println!("\n🤖 Smart Decision Analysis");
    println!("=========================");
    println!("   Transaction Type: {:?}", smart_data.transaction_type);
    println!("   Simulation Performed: {}", smart_data.simulation_performed);
    println!("   Optimization Applied: {}", smart_data.basic.performance.optimization_applied);
    println!("   Data Source: {}", smart_data.basic.performance.data_source);

    // Display performance metrics
    println!("\n📈 Performance Results");
    println!("=====================");
    println!("   Total Time: {:.3}ms", smart_data.basic.performance.retrieval_time_ms);
    if let Some(db_time) = smart_data.basic.performance.database_time_ms {
        println!("   Database Time: {:.3}ms", db_time);
    }
    if let Some(sim_time) = smart_data.basic.performance.simulation_time_ms {
        println!("   Simulation Time: {:.3}ms", sim_time);
    }
    
    let speed_class = if smart_data.basic.performance.retrieval_time_ms < 10.0 {
        "ULTRA-FAST 🚀"
    } else if smart_data.basic.performance.retrieval_time_ms < 100.0 {
        "FAST ⚡"
    } else {
        "COMPLETE 🔄"
    };
    println!("   Speed Class: {}", speed_class);

    // Display transaction details (basic)
    println!("\n📊 Transaction Details");
    println!("====================");
    println!("   Hash: {:x}", smart_data.basic.hash);
    println!("   From: {:x}", smart_data.basic.from);
    println!("   To: {}", smart_data.basic.to.map_or("Contract Creation".to_string(), |addr| format!("{:x}", addr)));
    println!("   Value: {} wei", smart_data.basic.value);
    println!("   Gas Used: {} ({:.1}% of limit)", smart_data.basic.gas_used, 
             (smart_data.basic.gas_used as f64 / smart_data.basic.gas_limit as f64) * 100.0);
    println!("   Status: {}", if smart_data.basic.status { "✅ Success" } else { "❌ Failed" });

    // Display smart analysis
    println!("\n🧠 Smart Analysis");
    println!("================");
    println!("   Transaction Type: {:?}", smart_data.transaction_type);
    println!("   Complex Interactions: {}", smart_data.complex_interactions);
    println!("   DEX Swaps Detected: {}", smart_data.dex_swaps_detected);
    println!("   Multi-hop Detected: {}", smart_data.multi_hop_detected);
    println!("   ERC20 Transfers: {}", smart_data.basic.erc20_transfers.len());
    println!("   Event Logs: {}", smart_data.basic.log_count);

    // Show internal transfers if simulation was performed
    if smart_data.simulation_performed {
        if let Some(ref internal_transfers) = smart_data.internal_transfers {
            println!("\n💸 Internal Transfers ({}):", internal_transfers.len());
            for (i, transfer) in internal_transfers.iter().take(5).enumerate() {
                println!("   {}: {:x} → {:x} = {} wei", 
                    i + 1, transfer.from, transfer.to, transfer.value);
            }
            if internal_transfers.len() > 5 {
                println!("   ... and {} more", internal_transfers.len() - 5);
            }
        }
    } else {
        println!("\n💸 Internal Transfers: Not needed (optimization applied)");
    }

    // Show ERC20 transfers
    if !smart_data.basic.erc20_transfers.is_empty() {
        println!("\n💰 ERC20 Transfers ({}):", smart_data.basic.erc20_transfers.len());
        for (i, transfer) in smart_data.basic.erc20_transfers.iter().take(3).enumerate() {
            println!("   {}: {:x} → {:x} = {} (token: {:x})", 
                i + 1, transfer.from, transfer.to, transfer.amount, transfer.token_address);
        }
        if smart_data.basic.erc20_transfers.len() > 3 {
            println!("   ... and {} more", smart_data.basic.erc20_transfers.len() - 3);
        }
    }

    // Decision explanation
    println!("\n🎯 Optimization Decision");
    println!("=======================");
    match smart_data.transaction_type {
        revm_tx_simulator_lib::optimized_tx_data::TransactionType::SimpleTransfer => {
            println!("✅ Database-only: Simple ETH transfer, no complex interactions expected");
        }
        revm_tx_simulator_lib::optimized_tx_data::TransactionType::ContractCall => {
            if smart_data.simulation_performed {
                println!("🔄 Simulation: Contract call with complex patterns detected");
            } else {
                println!("✅ Database-only: Simple contract call, no internal transfers expected");
            }
        }
        revm_tx_simulator_lib::optimized_tx_data::TransactionType::DexInteraction => {
            println!("🔄 Simulation: DEX interaction likely to have internal transfers");
        }
        revm_tx_simulator_lib::optimized_tx_data::TransactionType::ComplexDeFi => {
            println!("🔄 Simulation: Complex DeFi operation requires full analysis");
        }
        revm_tx_simulator_lib::optimized_tx_data::TransactionType::ContractDeployment => {
            println!("✅ Database-only: Contract deployment, no internal transfers expected");
        }
    }

    // Performance comparison
    println!("\n⚡ Performance Impact");
    println!("===================");
    if smart_data.simulation_performed {
        println!("📊 Complete data retrieved with simulation");
        println!("🔄 Trade-off: Slower but includes internal transfers");
        println!("💡 Alternative: Use basic mode for 80-800x speedup (without internal transfers)");
    } else {
        println!("🚀 Optimized: Database-only access applied");
        println!("⚡ Speed gain: 80-800x faster than simulation");
        println!("💡 Alternative: Use complete mode to force simulation");
    }

    // Usage recommendations
    println!("\n💡 Smart Mode Benefits");
    println!("=====================");
    println!("✅ Automatic optimization based on transaction type");
    println!("✅ Best balance of speed and completeness");
    println!("✅ Intelligent detection of when simulation is needed");
    println!("✅ Detailed transaction type classification");
    println!("✅ Perfect for production APIs with mixed workloads");

    println!("\n🎯 Use Smart Mode When:");
    println!("======================");
    println!("• You want optimal performance automatically");
    println!("• You have mixed transaction types (transfers + DeFi)");
    println!("• You need internal transfers for some transactions but not all");
    println!("• You want to minimize unnecessary simulation overhead");
    println!("• You're building APIs with variable transaction complexity");

    println!("\n✅ Smart analysis completed in {:.3}ms with {} optimization!", 
             smart_data.basic.performance.retrieval_time_ms,
             if smart_data.basic.performance.optimization_applied { "automatic" } else { "complete" });

    Ok(())
}