/// Test Revert Decoder
/// 
/// This example tests that the revert decoder is working properly
/// by simulating transactions that should revert with specific errors.

use eyre::Result;
use tx_simulator::{TxSimulator, CallRequest};
use alloy_primitives::{Address, U256, Bytes};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Testing Revert Decoder");
    println!("=========================\n");
    
    let reth_datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());
    
    let simulator = TxSimulator::new(&reth_datadir)?;
    let latest_block = simulator.get_latest_block()?;
    println!("Using block: {}\n", latest_block);
    
    // Test 1: Call a function that doesn't exist
    println!("Test 1: Calling non-existent function");
    println!("--------------------------------------");
    let invalid_call = CallRequest {
        from: Some("0x0000000000000000000000000000000000000001".parse()?),
        to: Some("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse()?), // WETH
        data: Some(Bytes::from(vec![0x12, 0x34, 0x56, 0x78])), // Invalid selector
        ..Default::default()
    };
    
    let result = simulator.simulate_call(invalid_call).await?;
    println!("Success: {}", result.success);
    println!("Gas used: {}", result.gas_used);
    println!("Revert reason: {:?}\n", result.revert_reason);
    
    // Test 2: Try to transfer more tokens than balance
    println!("Test 2: ERC20 transfer with insufficient balance");
    println!("-------------------------------------------------");
    
    // transfer(address,uint256) selector: 0xa9059cbb
    let mut transfer_data = vec![0xa9, 0x05, 0x9c, 0xbb];
    // to: some random address (32 bytes)
    transfer_data.extend_from_slice(&[0; 12]); // padding
    transfer_data.extend_from_slice(&[0x11; 20]); // address
    // amount: max uint256
    transfer_data.extend_from_slice(&[0xff; 32]);
    
    let transfer_call = CallRequest {
        from: Some("0x0000000000000000000000000000000000000001".parse()?),
        to: Some("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse()?), // USDC
        data: Some(Bytes::from(transfer_data)),
        ..Default::default()
    };
    
    let result = simulator.simulate_call(transfer_call).await?;
    println!("Success: {}", result.success);
    println!("Gas used: {}", result.gas_used);
    println!("Revert reason: {:?}\n", result.revert_reason);
    
    // Test 3: UniswapV3 swap with invalid parameters
    println!("Test 3: UniswapV3 swap with invalid parameters");
    println!("-----------------------------------------------");
    
    // exactInputSingle selector: 0x414bf389
    let mut swap_data = vec![0x41, 0x4b, 0xf3, 0x89];
    // Add some invalid data (not properly encoded struct)
    swap_data.extend_from_slice(&[0x00; 256]); // Invalid struct data
    
    let swap_call = CallRequest {
        from: Some("0x0000000000000000000000000000000000000001".parse()?),
        to: Some("0xE592427A0AEce92De3Edee1F18E0157C05861564".parse()?), // V3 Router
        data: Some(Bytes::from(swap_data)),
        value: Some(U256::from(1000000000000000u64)), // 0.001 ETH
        ..Default::default()
    };
    
    let result = simulator.simulate_call(swap_call).await?;
    println!("Success: {}", result.success);
    println!("Gas used: {}", result.gas_used);
    println!("Revert reason: {:?}\n", result.revert_reason);
    
    println!("✅ Revert decoder test complete!");
    
    Ok(())
}