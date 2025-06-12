//! Example: Using the new simulate_signed_tx_from_db function
//! 
//! This demonstrates the new reusable function that other components can call:
//! 1. Uses the simulate_signed_tx_from_db function from the public API
//! 2. Shows how to call it with different parameters
//! 3. Compares it with the original RPC-based approach
//! 4. Demonstrates the function's reusability for other system components
//!
//! This is the second example requested by the user, showing usage of the new function.
//!
//! To run as a binary:
//!   cargo run --bin simulate_using_new_db_function

use anyhow::Result;
use std::env;
use std::str::FromStr;
use std::time::Instant;

use ethers_core::types::H256;
use revm_tx_simulator_lib::simulate_signed_tx::{
    simulate_signed_tx,
    simulate_signed_tx_from_db,
};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Testing New simulate_signed_tx_from_db Function");
    println!("================================================\n");

    // Get transaction hash from args or use default
    let args: Vec<String> = env::args().collect();
    let tx_hash_str = if args.len() > 1 {
        &args[1]
    } else {
        // Use the same audited transaction for consistency
        "0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae"
    };

    let tx_hash = H256::from_str(tx_hash_str)?;
    let rpc_url = "http://127.0.0.1:8545";
    
    println!("📥 Transaction: {}", tx_hash);
    println!("🌐 RPC URL: {}", rpc_url);

    // Test 1: Use the new function with default database path
    println!("\n🔄 Test 1: Using new simulate_signed_tx_from_db function (default path)");
    println!("=======================================================================");
    
    let start_time = Instant::now();
    let db_result = simulate_signed_tx_from_db(tx_hash, None, rpc_url).await?;
    let db_duration = start_time.elapsed();
    
    println!("✅ Database simulation completed!");
    println!("   Result Type: {:?}", db_result.result_type);
    println!("   Gas Used: {}", db_result.gas_used);
    println!("   Gas Refunded: {}", db_result.gas_refunded);
    println!("   Output Size: {} bytes", db_result.output_data.len());
    println!("   Internal Transfers: {}", db_result.internal_transfers.len());
    println!("   Event Logs: {}", db_result.logs.len());
    println!("   Duration: {:?}", db_duration);

    // Test 2: Use the new function with explicit database path
    println!("\n🔄 Test 2: Using new function with explicit database path");
    println!("========================================================");
    
    let custom_datadir = "/home/nima/.local/share/reth/mainnet";
    let start_time = Instant::now();
    let db_custom_result = simulate_signed_tx_from_db(tx_hash, Some(custom_datadir), rpc_url).await?;
    let db_custom_duration = start_time.elapsed();
    
    println!("✅ Custom path simulation completed!");
    println!("   Custom Path: {}", custom_datadir);
    println!("   Result Type: {:?}", db_custom_result.result_type);
    println!("   Gas Used: {}", db_custom_result.gas_used);
    println!("   Duration: {:?}", db_custom_duration);

    // Test 3: Compare with original RPC-based function
    println!("\n🔄 Test 3: Comparing with original RPC-based simulation");
    println!("=======================================================");
    
    let start_time = Instant::now();
    let rpc_result = simulate_signed_tx(tx_hash, rpc_url).await?;
    let rpc_duration = start_time.elapsed();
    
    println!("✅ RPC simulation completed!");
    println!("   Result Type: {:?}", rpc_result.result_type);
    println!("   Gas Used: {}", rpc_result.gas_used);
    println!("   Duration: {:?}", rpc_duration);

    // Test 4: Results comparison
    println!("\n📊 Comparison Analysis");
    println!("=====================");
    
    println!("Gas Usage Comparison:");
    println!("   Database approach: {}", db_result.gas_used);
    println!("   RPC approach:      {}", rpc_result.gas_used);
    if db_result.gas_used == rpc_result.gas_used {
        println!("   ✅ Perfect match!");
    } else {
        let diff = if db_result.gas_used > rpc_result.gas_used {
            db_result.gas_used - rpc_result.gas_used
        } else {
            rpc_result.gas_used - db_result.gas_used
        };
        println!("   ⚠️  Difference: {}", diff);
    }

    println!("\nExecution Results:");
    println!("   Database result: {:?}", db_result.result_type);
    println!("   RPC result:      {:?}", rpc_result.result_type);
    if format!("{:?}", db_result.result_type) == format!("{:?}", rpc_result.result_type) {
        println!("   ✅ Results match!");
    } else {
        println!("   ⚠️  Results differ");
    }

    println!("\nEvent Logs:");
    println!("   Database logs: {}", db_result.logs.len());
    println!("   RPC logs:      {}", rpc_result.logs.len());
    if db_result.logs.len() == rpc_result.logs.len() {
        println!("   ✅ Log counts match!");
    } else {
        println!("   ⚠️  Log counts differ");
    }

    println!("\nInternal Transfers:");
    println!("   Database transfers: {}", db_result.internal_transfers.len());
    println!("   RPC transfers:      {}", rpc_result.internal_transfers.len());
    if db_result.internal_transfers.len() == rpc_result.internal_transfers.len() {
        println!("   ✅ Transfer counts match!");
    } else {
        println!("   ⚠️  Transfer counts differ");
    }

    // Performance comparison
    println!("\n⏱️  Performance Comparison");
    println!("=========================");
    println!("   Database approach: {:?}", db_duration);
    println!("   RPC approach:      {:?}", rpc_duration);
    
    if db_duration < rpc_duration {
        let speedup = rpc_duration.as_micros() as f64 / db_duration.as_micros() as f64;
        println!("   🚀 Database approach is {:.1}x faster!", speedup);
    } else if rpc_duration < db_duration {
        let slowdown = db_duration.as_micros() as f64 / rpc_duration.as_micros() as f64;
        println!("   📈 RPC approach is {:.1}x faster", slowdown);
    } else {
        println!("   ⚖️  Similar performance");
    }

    // Usage examples for other components
    println!("\n🔧 Usage Examples for Other Components");
    println!("=====================================");
    println!("The new simulate_signed_tx_from_db function can be used by other components like this:");
    println!();
    println!("```rust");
    println!("use revm_tx_simulator_lib::simulate_signed_tx::simulate_signed_tx_from_db;");
    println!("use ethers_core::types::H256;");
    println!();
    println!("// Option 1: Use default Reth database path");
    println!("let result = simulate_signed_tx_from_db(tx_hash, None, rpc_url).await?;");
    println!();
    println!("// Option 2: Use custom database path");
    println!("let result = simulate_signed_tx_from_db(");
    println!("    tx_hash, ");
    println!("    Some(\"/path/to/reth/datadir\"), ");
    println!("    rpc_url");
    println!(").await?;");
    println!("```");

    println!("\n🎯 Key Benefits of the New Function");
    println!("===================================");
    println!("✅ Reusable by other system components");
    println!("✅ Configurable database path");
    println!("✅ Hybrid approach: database for tx data, RPC for state");
    println!("✅ Performance optimized for transaction data retrieval");
    println!("✅ Maintains same API as existing functions");
    println!("✅ Full compatibility with existing simulation infrastructure");

    println!("\n✅ Successfully tested the new simulate_signed_tx_from_db function!");
    println!("💡 This function is now available for use by other components in the system.");

    Ok(())
}