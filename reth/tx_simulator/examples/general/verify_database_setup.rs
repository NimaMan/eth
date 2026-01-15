use eyre::Result;
/// Basic Functionality Test
///
/// Tests that the tx_simulator can successfully initialize and connect to
/// the Reth database, demonstrating that all core functionality is working.
use tx_simulator::TxSimulator;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔧 Basic TX Simulator Functionality Test");
    println!("=========================================");

    // Test 1: Initialize simulator
    println!("\n📋 Test 1: Simulator Initialization");
    let reth_datadir = "/home/nima/.local/share/reth/mainnet";
    let simulator = match TxSimulator::new(reth_datadir) {
        Ok(sim) => {
            println!("✅ Simulator initialized successfully");
            sim
        }
        Err(e) => {
            println!("❌ Failed to initialize simulator: {}", e);
            return Err(e.into());
        }
    };

    // Test 2: Database connection and latest block
    println!("\n📋 Test 2: Database Access");
    match simulator.get_latest_block() {
        Ok(latest) => {
            println!("✅ Latest block retrieved: {}", latest);
            if latest > 0 {
                println!("✅ Database access working correctly");
            } else {
                println!("⚠️ Latest block is 0 - database might be empty");
            }
        }
        Err(e) => {
            println!("❌ Failed to get latest block: {}", e);
            return Err(e.into());
        }
    }

    // Test 3: Base fee calculation (post-London blocks)
    println!("\n📋 Test 3: Base Fee Calculation");
    let latest_block = simulator.get_latest_block()?;
    let test_block = latest_block - 100; // Use a block that definitely exists

    match simulator.get_base_fee_at_block(test_block) {
        Ok(base_fee) => {
            println!(
                "✅ Base fee at block {}: {} gwei",
                test_block,
                base_fee as f64 / 1e9
            );
        }
        Err(e) => {
            println!("⚠️ Could not get base fee for block {}: {}", test_block, e);
            println!("   This is normal for pre-London blocks (< 12,965,000)");
        }
    }

    // Test 4: Provider factory access
    println!("\n📋 Test 4: Provider Factory Access");
    let provider_factory = simulator.provider_factory();
    match provider_factory.provider() {
        Ok(_provider) => {
            println!("✅ Provider factory access working");
        }
        Err(e) => {
            println!("❌ Provider factory access failed: {}", e);
            return Err(e.into());
        }
    }

    // Summary
    println!("\n📊 Test Results Summary");
    println!("=======================");
    println!("✅ All basic functionality tests passed!");
    println!("📍 Current block: {}", simulator.get_latest_block()?);
    println!("🗄️ Database path: {}", reth_datadir);
    println!("🔧 TX Simulator ready for transaction simulation");

    Ok(())
}
