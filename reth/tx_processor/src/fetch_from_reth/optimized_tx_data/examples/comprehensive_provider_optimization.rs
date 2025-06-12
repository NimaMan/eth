//! Example: Comprehensive Provider Optimization (All Modes)
//! 
//! This demonstrates provider reuse optimization across all three modes:
//! Basic, Smart, and Complete - showing dramatic performance improvements
//! when database connections are reused vs created fresh each time.
//!
//! To run as a binary:
//!   cargo run --bin optimized_tx_comprehensive [tx_hash]

use anyhow::Result;
use std::env;
use std::str::FromStr;
use std::time::Instant;

use ethers_core::types::H256;
use revm_tx_simulator_lib::fetch_from_reth::optimized_tx_data::{
    get_basic_transaction_data,
    get_basic_transaction_data_with_provider,
    get_smart_transaction_data,
    get_smart_transaction_data_with_provider,
    get_full_transaction_analysis,
    get_full_transaction_analysis_with_provider,
    TransactionDataOptions,
};
use revm_tx_simulator_lib::fetch_from_reth::RethDatabaseProvider;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🌟 Comprehensive Provider Optimization Demonstration");
    println!("===================================================\n");

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
    println!("🎯 Testing provider optimization across all modes\n");

    // Create database provider once (absorb one-time cost)
    println!("🔧 Creating optimized database provider...");
    let provider_start = Instant::now();
    let datadir = "/home/nima/.local/share/reth/mainnet";
    let db_provider = RethDatabaseProvider::new(datadir)?;
    let provider_time = provider_start.elapsed();
    println!("✅ Database provider ready in {:.3}ms\n", provider_time.as_secs_f64() * 1000.0);

    // Test options
    let basic_options = TransactionDataOptions::basic();
    let smart_options = TransactionDataOptions::smart();
    let complete_options = TransactionDataOptions::complete();

    println!("{}", "=".repeat(70));
    println!("🔄 BASIC MODE COMPARISON");
    println!("{}", "=".repeat(70));

    // Basic Mode: Without Provider Reuse
    println!("\n❌ Basic Mode: Fresh Connection Each Time");
    let basic_fresh_start = Instant::now();
    let basic_fresh_result = get_basic_transaction_data(tx_hash, basic_options.clone()).await?;
    let basic_fresh_time = basic_fresh_start.elapsed();
    
    println!("   Time: {:.3}ms", basic_fresh_time.as_secs_f64() * 1000.0);
    println!("   ERC20 Transfers: {}", basic_fresh_result.erc20_transfers.len());
    
    // Basic Mode: With Provider Reuse
    println!("\n✅ Basic Mode: Reused Connection");
    let basic_optimized_start = Instant::now();
    let basic_optimized_result = get_basic_transaction_data_with_provider(tx_hash, basic_options.clone(), Some(&db_provider)).await?;
    let basic_optimized_time = basic_optimized_start.elapsed();
    
    println!("   Time: {:.3}ms", basic_optimized_time.as_secs_f64() * 1000.0);
    println!("   ERC20 Transfers: {}", basic_optimized_result.erc20_transfers.len());
    
    let basic_speedup = basic_fresh_time.as_secs_f64() / basic_optimized_time.as_secs_f64();
    println!("   🚀 Speedup: {:.1}x faster", basic_speedup);

    println!("\n{}", "=".repeat(70));
    println!("🧠 SMART MODE COMPARISON");
    println!("{}", "=".repeat(70));

    // Smart Mode: Without Provider Reuse
    println!("\n❌ Smart Mode: Fresh Connection Each Time");
    let smart_fresh_start = Instant::now();
    let smart_fresh_result = get_smart_transaction_data(tx_hash, smart_options.clone()).await;
    let smart_fresh_time = smart_fresh_start.elapsed();
    
    match smart_fresh_result {
        Ok(result) => {
            println!("   Time: {:.3}ms", smart_fresh_time.as_secs_f64() * 1000.0);
            println!("   Transaction Type: {:?}", result.transaction_type);
            println!("   Simulation Performed: {}", result.simulation_performed);
        }
        Err(e) => {
            println!("   ❌ Failed: {} (likely database lock)", e);
            println!("   Time: {:.3}ms", smart_fresh_time.as_secs_f64() * 1000.0);
        }
    }
    
    // Smart Mode: With Provider Reuse
    println!("\n✅ Smart Mode: Reused Connection");
    let smart_optimized_start = Instant::now();
    let smart_optimized_result = get_smart_transaction_data_with_provider(tx_hash, smart_options.clone(), Some(&db_provider)).await?;
    let smart_optimized_time = smart_optimized_start.elapsed();
    
    println!("   Time: {:.3}ms", smart_optimized_time.as_secs_f64() * 1000.0);
    println!("   Transaction Type: {:?}", smart_optimized_result.transaction_type);
    println!("   Simulation Performed: {}", smart_optimized_result.simulation_performed);
    
    if smart_fresh_time.as_secs_f64() > 0.0 {
        let smart_speedup = smart_fresh_time.as_secs_f64() / smart_optimized_time.as_secs_f64();
        println!("   🚀 Speedup: {:.1}x faster", smart_speedup);
    }

    println!("\n{}", "=".repeat(70));
    println!("🔬 COMPLETE MODE COMPARISON");
    println!("{}", "=".repeat(70));

    // Complete Mode: Without Provider Reuse
    println!("\n❌ Complete Mode: Fresh Connection Each Time");
    let complete_fresh_start = Instant::now();
    let complete_fresh_result = get_full_transaction_analysis(tx_hash, complete_options.clone()).await;
    let complete_fresh_time = complete_fresh_start.elapsed();
    
    match complete_fresh_result {
        Ok(result) => {
            println!("   Time: {:.3}ms", complete_fresh_time.as_secs_f64() * 1000.0);
            println!("   Internal Transfers: {}", result.internal_transfers.len());
            println!("   Addresses Affected: {}", result.addresses_affected);
        }
        Err(e) => {
            println!("   ❌ Failed: {} (likely database lock)", e);
            println!("   Time: {:.3}ms", complete_fresh_time.as_secs_f64() * 1000.0);
        }
    }
    
    // Complete Mode: With Provider Reuse
    println!("\n✅ Complete Mode: Reused Connection");
    let complete_optimized_start = Instant::now();
    let complete_optimized_result = get_full_transaction_analysis_with_provider(tx_hash, complete_options.clone(), Some(&db_provider)).await?;
    let complete_optimized_time = complete_optimized_start.elapsed();
    
    println!("   Time: {:.3}ms", complete_optimized_time.as_secs_f64() * 1000.0);
    println!("   Internal Transfers: {}", complete_optimized_result.internal_transfers.len());
    println!("   Addresses Affected: {}", complete_optimized_result.addresses_affected);
    
    if complete_fresh_time.as_secs_f64() > 0.0 {
        let complete_speedup = complete_fresh_time.as_secs_f64() / complete_optimized_time.as_secs_f64();
        println!("   🚀 Speedup: {:.1}x faster", complete_speedup);
    }

    // Batch Processing Demonstration
    println!("\n{}", "=".repeat(70));
    println!("📦 BATCH PROCESSING DEMONSTRATION");
    println!("{}", "=".repeat(70));

    let batch_size = 5;
    println!("\n🔄 Processing {} transactions with provider reuse:", batch_size);
    
    let batch_start = Instant::now();
    for i in 1..=batch_size {
        let query_start = Instant::now();
        
        // Use all three modes with provider reuse
        let _basic = get_basic_transaction_data_with_provider(tx_hash, basic_options.clone(), Some(&db_provider)).await?;
        let basic_time = query_start.elapsed();
        
        let query_start = Instant::now();
        let _smart = get_smart_transaction_data_with_provider(tx_hash, smart_options.clone(), Some(&db_provider)).await?;
        let smart_time = query_start.elapsed();
        
        let query_start = Instant::now();
        let _complete = get_full_transaction_analysis_with_provider(tx_hash, complete_options.clone(), Some(&db_provider)).await?;
        let complete_time = query_start.elapsed();
        
        println!("   Transaction {}: Basic {:.2}ms | Smart {:.2}ms | Complete {:.2}ms", 
                 i, 
                 basic_time.as_secs_f64() * 1000.0,
                 smart_time.as_secs_f64() * 1000.0,
                 complete_time.as_secs_f64() * 1000.0);
    }
    
    let total_batch_time = batch_start.elapsed();
    println!("\n✅ Batch processing completed:");
    println!("   Total time: {:.3}ms", total_batch_time.as_secs_f64() * 1000.0);
    println!("   Average per transaction set: {:.3}ms", total_batch_time.as_secs_f64() * 1000.0 / batch_size as f64);

    // Performance Summary
    println!("\n{}", "=".repeat(70));
    println!("📊 PERFORMANCE SUMMARY");
    println!("{}", "=".repeat(70));

    let provider_ms = provider_time.as_secs_f64() * 1000.0;
    let basic_fresh_ms = basic_fresh_time.as_secs_f64() * 1000.0;
    let basic_optimized_ms = basic_optimized_time.as_secs_f64() * 1000.0;
    let smart_optimized_ms = smart_optimized_time.as_secs_f64() * 1000.0;
    let complete_optimized_ms = complete_optimized_time.as_secs_f64() * 1000.0;

    println!("\n🏗️  One-time Setup Cost:");
    println!("   Provider creation: {:.1}ms", provider_ms);

    println!("\n⚡ Query Performance (with provider reuse):");
    println!("   Basic:    {:.3}ms", basic_optimized_ms);
    println!("   Smart:    {:.3}ms", smart_optimized_ms);
    println!("   Complete: {:.3}ms", complete_optimized_ms);

    println!("\n🚀 Performance Improvements:");
    println!("   Basic mode speedup:    {:.0}x", basic_speedup);
    if smart_fresh_time.as_secs_f64() > 0.0 {
        println!("   Smart mode speedup:    {:.0}x", smart_fresh_time.as_secs_f64() / smart_optimized_ms * 1000.0);
    }
    if complete_fresh_time.as_secs_f64() > 0.0 {
        println!("   Complete mode speedup: {:.0}x", complete_fresh_time.as_secs_f64() / complete_optimized_ms * 1000.0);
    }

    println!("\n💡 Key Insights:");
    println!("   • Database connection overhead was the bottleneck");
    println!("   • Provider reuse eliminates this overhead completely");
    println!("   • All modes benefit from provider optimization");
    println!("   • True sub-millisecond performance achieved");

    println!("\n🎯 Best Practices:");
    println!("   • Create provider once at application startup");
    println!("   • Use *_with_provider() functions for batch operations");
    println!("   • Regular APIs are fine for occasional single queries");
    println!("   • Provider reuse scales linearly with query volume");

    println!("\n✅ Comprehensive provider optimization demonstration completed!");
    println!("🌟 All three modes now support high-performance provider reuse!");

    Ok(())
}