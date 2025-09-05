/// Test the Mempool Simulator to verify it solves the database lock issue
///
/// This example demonstrates that MempoolSimulator uses a single database 
/// connection to avoid "error code: 11" (EAGAIN) database lock issues.
///
/// Tests performed:
/// 1. Pool buy/sell simulation on different pools
/// 2. Multiple rapid simulations (stress test)
/// All using the same database connection!

use mempool_processor::simulator::MempoolSimulator;
use alloy_primitives::Address;
use std::str::FromStr;
use std::time::Instant;
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter("info")
        .init();
    
    println!("🚀 Testing Mempool Simulator - Database Lock Prevention\n");
    println!("This test verifies that using a single MempoolSimulator instance");
    println!("avoids database lock issues (error code: 11) when running multiple simulations.\n");
    
    // Create mempool simulator - single database connection!
    let datadir = "/home/nima/.local/share/reth/mainnet";
    let simulator = MempoolSimulator::new(datadir)?;
    
    let latest_block = simulator.get_latest_block()?;
    println!("📦 Latest block: {}", latest_block);
    
    // Test parameters - known working token/pool
    let token_address = Address::from_str("0xaAd355C579d85432D0a31A3801D267dEaC111071")?;
    let pool_address = Address::from_str("0xDbDCBee13F7a0af5F5dD233553C055f5f0627F98")?;
    let block_number = 23046144u64;
    
    // Track if we encounter any database lock errors
    let mut db_lock_errors = 0;
    
    // ========== Test 1: Pool Buy/Sell Simulation ==========
    println!("\n🧪 Test 1: Pool buy/sell simulation");
    println!("Token: {}", token_address);
    println!("Pool: {}", pool_address);
    println!("Block: {}", block_number);
    
    let start = Instant::now();
    match simulator.simulate_pool_buy_sell_simple(
        token_address,
        pool_address,
        Some(block_number)
    ).await {
        Ok(result) => {
            let elapsed = start.elapsed();
            println!("✅ SUCCESS in {:.2}ms", elapsed.as_secs_f64() * 1000.0);
            println!("  Can buy: {}", result.can_buy);
            println!("  Can sell: {}", result.can_sell);
            println!("  Buy tax: {:.2}%", result.buy_tax_percent);
            println!("  Sell tax: {:.2}%", result.sell_tax_percent);
        }
        Err(e) => {
            println!("❌ FAILED: {}", e);
            if e.to_string().contains("error code: 11") {
                println!("⚠️  DATABASE LOCK ERROR DETECTED!");
                db_lock_errors += 1;
            }
        }
    }
    
    // ========== Test 2: Rapid Multiple Simulations (Stress Test) ==========
    println!("\n🧪 Test 2: Rapid multiple simulations (stress test for database locks)");
    
    // Test with multiple different pools/tokens
    let test_cases = vec![
        // (token, pool, name)
        (
            "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", // USDC
            "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc", // USDC/WETH V2
            "USDC"
        ),
        (
            "0xdAC17F958D2ee523a2206206994597C13D831ec7", // USDT
            "0x0d4a11d5EEaaC28EC3F61d100daF4d40471f1852", // USDT/WETH V2
            "USDT"
        ),
        (
            "0x6B175474E89094C44Da98b954EedeAC495271d0F", // DAI
            "0xA478c2975Ab1Ea89e8196811F51A7B7Ade33eB11", // DAI/WETH V2
            "DAI"
        ),
    ];
    
    let mut successes = 0;
    let mut failures = 0;
    
    for (token_str, pool_str, name) in test_cases {
        let token = Address::from_str(token_str)?;
        let pool = Address::from_str(pool_str)?;
        
        print!("  Testing {}... ", name);
        let start = Instant::now();
        
        match simulator.simulate_pool_buy_sell_simple(token, pool, None).await {
            Ok(_) => {
                successes += 1;
                println!("✅ ({:.2}ms)", start.elapsed().as_secs_f64() * 1000.0);
            }
            Err(e) => {
                failures += 1;
                println!("❌");
                if e.to_string().contains("error code: 11") {
                    println!("    ⚠️  DATABASE LOCK ERROR!");
                    db_lock_errors += 1;
                }
            }
        }
    }
    
    // ========== Test 3: Sequential Simulations on Same Pool ==========
    println!("\n🧪 Test 3: Sequential simulations on same pool");
    println!("Running 5 simulations on the same pool in quick succession...");
    
    let mut sequential_successes = 0;
    for i in 1..=5 {
        print!("  Simulation {}... ", i);
        let start = Instant::now();
        
        match simulator.simulate_pool_buy_sell_simple(
            token_address,
            pool_address,
            Some(block_number)
        ).await {
            Ok(_) => {
                sequential_successes += 1;
                println!("✅ ({:.2}ms)", start.elapsed().as_secs_f64() * 1000.0);
            }
            Err(e) => {
                println!("❌");
                if e.to_string().contains("error code: 11") {
                    println!("    ⚠️  DATABASE LOCK ERROR!");
                    db_lock_errors += 1;
                }
            }
        }
    }
    
    // ========== Summary ==========
    println!("\n📊 TEST SUMMARY");
    println!("================");
    println!("Stress test: {} successes, {} failures", successes, failures);
    println!("Sequential test: {} successes out of 5", sequential_successes);
    
    if db_lock_errors == 0 {
        println!("\n✅ NO DATABASE LOCK ERRORS DETECTED!");
        println!("The MempoolSimulator successfully uses a single database connection.");
    } else {
        println!("\n⚠️  WARNING: {} database lock errors detected!", db_lock_errors);
        println!("This indicates a problem with database connection management.");
    }
    
    println!("\n✨ Mempool simulator test complete!");
    
    Ok(())
}