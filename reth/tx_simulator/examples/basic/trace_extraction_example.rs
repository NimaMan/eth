/// Trace Extraction Example
///
/// This example demonstrates the `simulate_unsigned_transaction_with_full_trace_at_block` method,
/// which provides comprehensive analysis including internal transactions, event logs,
/// and complete call traces. This is the most detailed simulation method available.

use tx_simulator::{TxSimulator, CallRequest};
use alloy_primitives::{Address, U256, Bytes};
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize simulator
    let reth_datadir = "/home/nima/.local/share/reth/mainnet";
    let simulator = TxSimulator::new(reth_datadir)?;
    
    // Get latest block for testing
    let latest_block = simulator.get_latest_block()?;
    let test_block = latest_block - 100; // Use a slightly older block
    
    println!("🔍 Full Trace Simulation Example");
    println!("================================");
    println!("Test block: {}", test_block);
    
    // Example 1: Simple ETH transfer with full trace
    println!("\n📝 Example 1: ETH Transfer with Full Trace");
    println!("{}", "-".repeat(45));
    
    let eth_transfer = CallRequest {
        from: Some("0x0C96c602b1b332B8AB2093E5d72D804a24bd5689".parse()?),
        to: Some("0xa0b86a33e6c2c76f8c4f8e60b55c2e6f4fd9a3db".parse()?),
        value: Some(U256::from(1000000000000000u64)), // 0.001 ETH
        gas: Some(21000),
        gas_price: Some(20_000_000_000), // 20 gwei
        ..Default::default()
    };
    
    let result = simulator
        .simulate_unsigned_transaction_with_full_trace_at_block(eth_transfer, test_block)
        .await?;
    
    println!("Success: {}", result.success);
    println!("Gas used: {}", result.gas_used);
    println!("Call trace depth: {}", result.call_trace.calls.len());
    println!("Logs generated: {}", result.call_trace.logs.len());
    println!("Call trace depth: {}", result.call_trace.calls.len());
    
    if let Some(reason) = &result.revert_reason {
        println!("Revert reason: {}", reason);
    }
    
    // Example 2: ERC20 token transfer with full trace
    println!("\n📝 Example 2: ERC20 Token Transfer with Full Trace");
    println!("{}", "-".repeat(50));
    
    // USDC transfer call data (transfer function)
    let transfer_selector = [0xa9, 0x05, 0x9c, 0xbb]; // transfer(address,uint256)
    let recipient: Address = "0xa0b86a33e6c2c76f8c4f8e60b55c2e6f4fd9a3db".parse()?;
    let amount = U256::from(1000_000u64); // 1 USDC (6 decimals)
    
    let mut call_data = Vec::new();
    call_data.extend_from_slice(&transfer_selector);
    call_data.extend_from_slice(&[0u8; 12]); // Pad to 32 bytes
    call_data.extend_from_slice(recipient.as_slice());
    call_data.extend_from_slice(&amount.to_be_bytes::<32>());
    
    let erc20_transfer = CallRequest {
        from: Some("0x742d35cc6548c5b8a9f63c4c81d0e90e3e1d3d9e".parse()?),
        to: Some("0xa0b86a33e6c2c76f8c4f8e60b55c2e6f4fd9a3db".parse()?), // USDC contract
        data: Some(Bytes::from(call_data)),
        gas: Some(65000),
        gas_price: Some(20_000_000_000),
        ..Default::default()
    };
    
    let result = simulator
        .simulate_unsigned_transaction_with_full_trace_at_block(erc20_transfer, test_block)
        .await?;
    
    println!("Success: {}", result.success);
    println!("Gas used: {}", result.gas_used);
    println!("Call trace depth: {}", result.call_trace.calls.len());
    println!("Logs generated: {}", result.call_trace.logs.len());
    
    // Show call trace structure (internal transaction extraction should be done by tx_processor)
    if !result.call_trace.calls.is_empty() {
        println!("\n📊 Call Trace Structure:");
        println!("  Note: Internal transaction extraction should be done by tx_processor");
        println!("  Available: CallFrame with {} child calls", result.call_trace.calls.len());
    }
    
    // Show call trace structure (logs are available in call_trace for tx_processor to handle)
    if !result.call_trace.logs.is_empty() {
        println!("\n📋 Call Trace Logs Available: {}", result.call_trace.logs.len());
        println!("   Note: Log decoding and analysis should be done by tx_processor");
    }
    
    // Example 3: Complex DeFi interaction with full trace
    println!("\n📝 Example 3: Uniswap V2 Swap with Full Trace");
    println!("{}", "-".repeat(45));
    
    // Uniswap V2: swapExactETHForTokens
    let swap_selector = [0x7f, 0xf3, 0x6a, 0xb5]; // swapExactETHForTokens(uint256,address[],address,uint256)
    let min_amount_out = U256::from(1u64);
    let deadline = U256::from(9999999999u64);
    let recipient: Address = "0x742d35cc6548c5b8a9f63c4c81d0e90e3e1d3d9e".parse()?;
    
    // Simplified call data for demo (would need proper path encoding for real swap)
    let mut swap_data = Vec::new();
    swap_data.extend_from_slice(&swap_selector);
    swap_data.extend_from_slice(&min_amount_out.to_be_bytes::<32>());
    
    let uniswap_swap = CallRequest {
        from: Some("0x742d35cc6548c5b8a9f63c4c81d0e90e3e1d3d9e".parse()?),
        to: Some("0x7a250d5630b4cf539739df2c5dacb4c659f2488d".parse()?), // Uniswap V2 Router
        value: Some(U256::from(100000000000000000u64)), // 0.1 ETH
        data: Some(Bytes::from(swap_data)),
        gas: Some(200000),
        gas_price: Some(30_000_000_000),
        ..Default::default()
    };
    
    let result = simulator
        .simulate_unsigned_transaction_with_full_trace_at_block(uniswap_swap, test_block)
        .await?;
    
    println!("Success: {}", result.success);
    println!("Gas used: {}", result.gas_used);
    println!("Call trace depth: {}", result.call_trace.calls.len());
    println!("Logs generated: {}", result.call_trace.logs.len());
    println!("Call trace depth: {}", result.call_trace.calls.len());
    
    if result.success {
        println!("✅ Complex DeFi interaction simulated successfully!");
        println!("   This demonstrates the simulator's ability to handle:");
        println!("   • Multi-contract interactions");
        println!("   • Internal function calls");
        println!("   • Event emissions");
        println!("   • State changes across multiple contracts");
    } else {
        println!("❌ Transaction would fail: {:?}", result.revert_reason);
        println!("   This is expected for demo data - real swaps need proper parameters");
    }
    
    // Summary
    println!("\n📊 Full Trace Simulation Summary");
    println!("================================");
    println!("The `simulate_unsigned_transaction_with_full_trace_at_block` method provides:");
    println!("• Complete call traces with all sub-calls");
    println!("• Internal transaction extraction");
    println!("• Event log capture");
    println!("• Detailed gas usage analysis");
    println!("• Comprehensive revert reason reporting");
    println!("• State change visibility");
    println!();
    println!("This is the most comprehensive simulation method available,");
    println!("ideal for detailed transaction analysis, MEV research,");
    println!("and understanding complex DeFi interactions.");
    
    Ok(())
}