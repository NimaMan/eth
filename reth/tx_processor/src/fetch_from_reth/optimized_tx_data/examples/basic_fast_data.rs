//! Example: Basic Transaction Data (Fastest ~1ms)
//! 
//! This demonstrates the fastest way to get transaction data using database-only access.
//! Perfect for when you don't need internal transfers or complex simulation data.
//!
//! Use cases:
//! - Transaction history display
//! - ERC20 transfer tracking
//! - Gas usage analysis
//! - Event log processing
//!
//! To run as a binary:
//!   cargo run --bin optimized_tx_basic [tx_hash]

use anyhow::Result;
use std::env;
use std::str::FromStr;

use ethers_core::types::H256;
use revm_tx_simulator_lib::fetch_from_reth::optimized_tx_data::{
    get_basic_transaction_data,
    TransactionDataOptions,
};

#[tokio::main]
async fn main() -> Result<()> {
    println!("⚡ Basic Transaction Data (Database-Only - Fastest)");
    println!("==================================================\n");

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

    // Create options for basic data retrieval (fastest)
    let options = TransactionDataOptions::basic();
    
    println!("🔧 Mode: Database-only (no simulation)");
    println!("🎯 Target: Sub-millisecond retrieval\n");

    // Fetch basic transaction data
    println!("⚡ Fetching basic transaction data...");
    let basic_data = get_basic_transaction_data(tx_hash, options).await?;

    // Display performance metrics
    println!("\n📈 Performance Results");
    println!("=====================");
    println!("   Data Source: {}", basic_data.performance.data_source);
    println!("   Total Time: {:.3}ms", basic_data.performance.retrieval_time_ms);
    println!("   Database Time: {:.3}ms", basic_data.performance.database_time_ms.unwrap_or(0.0));
    println!("   Optimization Applied: {}", basic_data.performance.optimization_applied);
    println!("   Speed Class: {} 🚀", if basic_data.performance.retrieval_time_ms < 10.0 { "ULTRA-FAST" } else { "FAST" });

    // Display transaction details
    println!("\n📊 Transaction Details");
    println!("====================");
    println!("   Hash: {:x}", basic_data.hash);
    println!("   From: {:x}", basic_data.from);
    println!("   To: {}", basic_data.to.map_or("Contract Creation".to_string(), |addr| format!("{:x}", addr)));
    println!("   Value: {} wei", basic_data.value);
    println!("   Gas Limit: {}", basic_data.gas_limit);
    println!("   Gas Used: {} ({:.1}% of limit)", basic_data.gas_used, 
             (basic_data.gas_used as f64 / basic_data.gas_limit as f64) * 100.0);
    println!("   Gas Price: {} wei", basic_data.gas_price);
    println!("   Nonce: {}", basic_data.nonce);
    println!("   Status: {}", if basic_data.status { "✅ Success" } else { "❌ Failed" });

    // Display block context
    println!("\n🧱 Block Context");
    println!("===============");
    println!("   Block Number: {}", basic_data.block_number);
    println!("   Block Hash: {:x}", basic_data.block_hash);
    println!("   Transaction Index: {}", basic_data.transaction_index);

    // Display transaction analysis
    println!("\n🔍 Transaction Analysis");
    println!("======================");
    println!("   Is Contract Call: {}", if basic_data.is_contract_call { "Yes" } else { "No" });
    println!("   Input Data Size: {} bytes", basic_data.input_data.len());
    println!("   Event Logs: {}", basic_data.log_count);
    println!("   ERC20 Transfers: {}", basic_data.erc20_transfers.len());

    // Show ERC20 transfers if any
    if !basic_data.erc20_transfers.is_empty() {
        println!("\n💰 ERC20 Transfers ({}):", basic_data.erc20_transfers.len());
        for (i, transfer) in basic_data.erc20_transfers.iter().take(5).enumerate() {
            println!("   {}: {:x} → {:x} = {} (token: {:x})", 
                i + 1, transfer.from, transfer.to, transfer.amount, transfer.token_address);
        }
        if basic_data.erc20_transfers.len() > 5 {
            println!("   ... and {} more", basic_data.erc20_transfers.len() - 5);
        }
    }

    // Show some event logs
    if !basic_data.logs.is_empty() {
        println!("\n📋 Sample Event Logs:");
        for (i, log) in basic_data.logs.iter().take(3).enumerate() {
            println!("   {}: Contract {:x}, {} topics", 
                i + 1, log.address, log.topics().len());
            if !log.topics().is_empty() {
                println!("      Topic[0]: {:x}", log.topics()[0]);
            }
        }
        if basic_data.logs.len() > 3 {
            println!("   ... and {} more", basic_data.logs.len() - 3);
        }
    }

    // Performance advantages
    println!("\n🎯 Performance Advantages");
    println!("========================");
    println!("✅ Direct database access (no RPC overhead)");
    println!("✅ No simulation execution (80-800x faster)");
    println!("✅ Immediate transaction and receipt data");
    println!("✅ All event logs included");
    println!("✅ ERC20 transfer analysis");
    println!("✅ Perfect for transaction history and basic analysis");

    // Use case recommendations
    println!("\n💡 Perfect For:");
    println!("===============");
    println!("• Transaction history displays");
    println!("• ERC20 transfer tracking");
    println!("• Gas usage analysis");
    println!("• Event log processing");
    println!("• Basic transaction validation");
    println!("• High-volume data processing");

    println!("\n⚠️  Not Included (use Smart/Complete mode for these):");
    println!("=====================================================");
    println!("• Internal ETH transfers");
    println!("• Detailed call traces");
    println!("• State change analysis");
    println!("• Complex DeFi interaction analysis");

    println!("\n✅ Basic transaction data retrieved in {:.3}ms!", 
             basic_data.performance.retrieval_time_ms);

    Ok(())
}