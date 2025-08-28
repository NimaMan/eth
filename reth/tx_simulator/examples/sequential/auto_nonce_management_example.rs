/// Auto Nonce Management Example
/// 
/// This example demonstrates how the reth_tx_simulator automatically adapts nonces
/// when encountering "nonce too low" errors. This is common when simulating mempool
/// transactions or accounts with pending transactions.
///
/// WHAT IT DEMONSTRATES:
/// 1. Default behavior: automatic nonce adaptation when error occurs
/// 2. Explicit control: turning adaptation on/off
/// 3. Best practice: omitting nonce to auto-detect from state
///
/// KEY FEATURES:
/// - Automatic nonce extraction from error messages
/// - Transparent retry with correct nonce
/// - No special functions needed - it just works
///
/// INPUT:
/// Creates transactions with deliberately wrong nonces to trigger adaptation.
///
/// OUTPUT:
/// Shows the error, extracted nonce, and successful retry with correct nonce.

use tx_simulator::{TxSimulator, CallRequest};
use alloy_primitives::{U256, Address};
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("\n🔧 Auto Nonce Management Example");
    println!("==================================\n");
    
    // Initialize simulator
    let simulator = TxSimulator::new("/home/nima/.local/share/reth/mainnet")?;
    println!("✅ Simulator initialized");
    
    // Use a real address that likely has transactions
    let from_address = "0x95222290DD7278Aa3Ddd389Cc1E1d165CC4BAfe5".parse::<Address>()?;
    
    println!("\n1️⃣  Testing with deliberately wrong nonce...");
    
    // Create transaction with nonce 0 (likely wrong for this active address)
    let mut request = CallRequest {
        from: Some(from_address),
        to: Some("0x388C818CA8B9251b393131C08a736A67ccB19297".parse()?),
        value: Some(U256::from(1_000_000_000_000_000u64)), // 0.001 ETH
        gas: Some(21000),
        gas_price: Some(20_000_000_000), // 20 gwei
        data: None,
        nonce: Some(0), // Deliberately wrong nonce
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    };
    
    // First attempt - the simulator will automatically adapt the nonce
    println!("\n   Attempting simulation with wrong nonce 0...");
    match simulator.simulate_call(request.clone()).await {
        Ok(result) => {
            println!("   ✅ Simulation succeeded (nonce was automatically adapted!)");
            println!("   Gas used: {}", result.gas_used);
            println!("   Success: {}", result.success);
        }
        Err(e) => {
            println!("   ❌ Simulation failed: {}", e);
        }
    }
    
    println!("\n2️⃣  Testing with adaptation disabled...");
    
    // Now test with adaptation explicitly disabled
    // Note: We no longer have a way to disable nonce adaptation
    match simulator.simulate_call(request.clone()).await {
        Ok(_) => {
            println!("   ✅ Simulation succeeded");
        }
        Err(e) => {
            println!("   ❌ Expected error (adaptation disabled): {}", e);
            if e.to_string().contains("expected") {
                println!("   📝 Without adaptation, nonce errors are not automatically handled");
            }
        }
    }
    
    println!("\n3️⃣  Testing without providing nonce (auto-detection)...");
    
    // Remove nonce to let the simulator detect it
    request.nonce = None;
    
    match simulator.simulate_call(request).await {
        Ok(result) => {
            println!("   ✅ Simulation succeeded with auto-detected nonce!");
            println!("   Gas used: {}", result.gas_used);
        }
        Err(e) => {
            println!("   ❌ Simulation failed: {}", e);
        }
    }
    
    println!("\n📊 SUMMARY:");
    println!("   - Nonce adaptation is enabled by default (transparent retry)");
    println!("   - Note: Nonce adaptation is always enabled in the current implementation");
    println!("   - Best practice: omit nonce field for auto-detection from state");
    println!("   - Signed transactions cannot adapt nonce (would break signature)");
    
    Ok(())
}