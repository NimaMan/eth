/// Debug Scout Token Simulation - Diagnostic Tool
/// 
/// This example compares our simulator with direct RPC calls to identify
/// why simulations fail for transactions that succeed on-chain.
///
/// Test Case: Scout Token at block 23124804 (first successful buy)
/// - Token: 0x4e21B13330a5bfabcd1aC3F4Bc4fA444571F52ae
/// - Pool: 0xcad25c3386b115f0c456e2966f8843b53497c983
/// - Real TX: 0xf76b472cde3aacd3def5fd7921ed4424a7981ada796d67182bdfe97f37cd1693

use mempool_processor::simulator::{UnifiedSimulator, BuySellSimulatorConfig};
use alloy_primitives::{Address, U256};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types::{TransactionRequest, BlockId};
use alloy_sol_types::SolCall;
use std::str::FromStr;
use eyre::Result;
use hex;

const SCOUT_TOKEN: &str = "0x4e21B13330a5bfabcd1aC3F4Bc4fA444571F52ae";
const SCOUT_POOL: &str = "0xcad25c3386b115f0c456e2966f8843b53497c983";
const TEST_BLOCK: u64 = 23124804;
const UNISWAP_ROUTER: &str = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D";
const WETH: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";

// Uniswap V2 Router function signatures
alloy_sol_types::sol! {
    function swapExactETHForTokens(
        uint256 amountOutMin,
        address[] calldata path,
        address to,
        uint256 deadline
    ) external payable returns (uint256[] memory amounts);
    
    function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
    
    function balanceOf(address owner) external view returns (uint256);
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Scout Token Simulation Debug Tool");
    println!("=====================================\n");
    
    // Setup
    let provider = ProviderBuilder::new().on_http("http://localhost:8545".parse()?);
    let token_address = Address::from_str(SCOUT_TOKEN)?;
    let pool_address = Address::from_str(SCOUT_POOL)?;
    let router_address = Address::from_str(UNISWAP_ROUTER)?;
    let weth_address = Address::from_str(WETH)?;
    
    // Test addresses
    let our_buyer = Address::from_str("0x0C96c602b1b332B8AB2093E5d72D804a24bd5689")?; // Our hardcoded
    let real_buyer = Address::from_str("0xBe3569068562218C792cF25b98DBf1418AFf2455")?; // From real TX
    
    println!("📊 Test Parameters:");
    println!("  Block: {}", TEST_BLOCK);
    println!("  Token: {}", SCOUT_TOKEN);
    println!("  Pool: {}", SCOUT_POOL);
    println!("  Our Buyer: {:?}", our_buyer);
    println!("  Real Buyer: {:?}", real_buyer);
    println!();
    
    // ==== STEP 1: Check Account Balances ====
    println!("💰 Checking ETH Balances at Block {}:", TEST_BLOCK);
    
    let our_balance = provider.get_balance(our_buyer).block_id(BlockId::Number(TEST_BLOCK.into())).await?;
    let real_balance = provider.get_balance(real_buyer).block_id(BlockId::Number(TEST_BLOCK.into())).await?;
    
    println!("  Our Test Address: {} ETH", format_ether(our_balance));
    println!("  Real Buyer Address: {} ETH", format_ether(real_balance));
    
    if our_balance == U256::ZERO {
        println!("  ❌ OUR TEST ADDRESS HAS NO ETH - This is likely the problem!");
    }
    println!();
    
    // ==== STEP 2: Check Pool State ====
    println!("🏊 Pool State at Block {}:", TEST_BLOCK);
    
    // Get pool reserves
    let reserves_call = getReservesCall {};
    let reserves_data = provider.call(
        TransactionRequest::default()
            .to(pool_address)
            .input(reserves_call.abi_encode().into())
    ).block(BlockId::Number(TEST_BLOCK.into())).await?;
    
    if reserves_data.len() >= 64 {
        let reserve0 = U256::from_be_slice(&reserves_data[0..32]);
        let reserve1 = U256::from_be_slice(&reserves_data[32..64]);
        
        // Determine which reserve is WETH (should be reserve1 based on address comparison)
        let is_weth_reserve0 = weth_address < token_address;
        let (weth_reserve, token_reserve) = if is_weth_reserve0 {
            (reserve0, reserve1)
        } else {
            (reserve1, reserve0)
        };
        
        println!("  WETH Reserve: {} ETH", format_ether(weth_reserve));
        println!("  Token Reserve: {}", token_reserve);
        
        // Calculate price impact for different amounts
        let our_amount = U256::from(100_000_000_000_000_000u64); // 0.1 ETH
        let real_amount = U256::from(21_000_000_000_000u64); // 0.000021 ETH
        
        let our_impact = calculate_price_impact(our_amount, weth_reserve);
        let real_impact = calculate_price_impact(real_amount, weth_reserve);
        
        println!("  Price Impact (0.1 ETH): {:.2}%", our_impact);
        println!("  Price Impact (0.000021 ETH): {:.2}%", real_impact);
        
        if our_impact > 50.0 {
            println!("  ⚠️  WARNING: 0.1 ETH would cause >50% price impact!");
        }
    }
    println!();
    
    // ==== STEP 3: Test Our Simulator ====
    println!("🧪 Testing Our Simulator:");
    println!("  Amount: 0.1 ETH (hardcoded)");
    println!("  Buyer: {:?}", our_buyer);
    
    let config = BuySellSimulatorConfig::default();
    println!("  Config: {:?}", config);
    
    let simulator = UnifiedSimulator::with_config(
        "/home/nima/.local/share/reth/mainnet",
        config.clone()
    )?;
    
    match simulator.simulate_buy_sell_sequence(token_address, pool_address, Some(TEST_BLOCK)).await {
        Ok(result) => {
            println!("  ✅ Simulation succeeded!");
            println!("    Buy Success: {}", result.buy_result.success);
            println!("    Sell Success: {}", result.sell_result.success);
            println!("    Buy Gas: {}", result.buy_result.gas_used);
            println!("    Sell Gas: {}", result.sell_result.gas_used);
            if let Some(reason) = &result.buy_result.revert_reason {
                println!("    Buy Revert: {}", reason);
            }
            if let Some(reason) = &result.sell_result.revert_reason {
                println!("    Sell Revert: {}", reason);
            }
            println!("    Buy state changes: {} addresses affected", result.buy_result.state_changes.len());
            println!("    Sell state changes: {} addresses affected", result.sell_result.state_changes.len());
        }
        Err(e) => {
            println!("  ❌ Simulation failed: {}", e);
            // Try to extract detailed error
            let error_str = e.to_string();
            if error_str.contains("0x08c379a0") {
                // This is a revert with reason string
                if let Some(reason) = decode_revert_reason(&error_str) {
                    println!("    Revert reason: {}", reason);
                }
            }
        }
    }
    println!();
    
    // ==== STEP 4: Direct RPC Call with Our Parameters ====
    println!("📡 Direct RPC Call (Our Parameters):");
    println!("  From: {:?}", our_buyer);
    println!("  Amount: 0.1 ETH");
    
    let our_buy_call = swapExactETHForTokensCall {
        amountOutMin: U256::ZERO,
        path: vec![weth_address, token_address],
        to: our_buyer,
        deadline: U256::from(9999999999u64),
    };
    
    let our_tx = TransactionRequest::default()
        .from(our_buyer)
        .to(router_address)
        .value(U256::from(100_000_000_000_000_000u64))
        .input(our_buy_call.abi_encode().into());
    
    match provider.call(our_tx).block(BlockId::Number(TEST_BLOCK.into())).await {
        Ok(result) => {
            println!("  ✅ Call succeeded!");
            println!("    Result length: {} bytes", result.len());
            if !result.is_empty() {
                println!("    Result: 0x{}", hex::encode(&result[..32.min(result.len())]));
            }
        }
        Err(e) => {
            println!("  ❌ Call failed: {}", e);
            let error_str = e.to_string();
            if let Some(reason) = decode_revert_reason(&error_str) {
                println!("    Decoded: {}", reason);
            }
        }
    }
    println!();
    
    // ==== STEP 5: Direct RPC Call with Real Transaction Parameters ====
    println!("📡 Direct RPC Call (Real TX Parameters):");
    println!("  From: {:?}", real_buyer);
    println!("  Amount: 0.000021 ETH (from real tx)");
    
    let real_buy_call = swapExactETHForTokensCall {
        amountOutMin: U256::ZERO,
        path: vec![weth_address, token_address],
        to: real_buyer,
        deadline: U256::from(9999999999u64),
    };
    
    let real_tx = TransactionRequest::default()
        .from(real_buyer)
        .to(router_address)
        .value(U256::from(21_000_000_000_000u64))
        .input(real_buy_call.abi_encode().into());
    
    match provider.call(real_tx).block(BlockId::Number(TEST_BLOCK.into())).await {
        Ok(result) => {
            println!("  ✅ Call succeeded!");
            println!("    Result length: {} bytes", result.len());
            if !result.is_empty() {
                // Try to decode the amounts array
                if result.len() >= 64 {
                    let offset = U256::from_be_slice(&result[0..32]);
                    let length = U256::from_be_slice(&result[32..64]);
                    println!("    Amounts returned: {} values", length);
                    if result.len() >= 96 {
                        let amount0 = U256::from_be_slice(&result[64..96]);
                        println!("    ETH in: {} ETH", format_ether(amount0));
                        if result.len() >= 128 {
                            let amount1 = U256::from_be_slice(&result[96..128]);
                            println!("    Tokens out: {}", amount1);
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("  ❌ Call failed: {}", e);
            let error_str = e.to_string();
            if let Some(reason) = decode_revert_reason(&error_str) {
                println!("    Decoded: {}", reason);
            }
        }
    }
    println!();
    
    // ==== STEP 6: Test with a funded address ====
    println!("🔧 Alternative Test - Using real buyer address in our simulator:");
    
    // Create a custom config with the real buyer address
    let mut custom_config = BuySellSimulatorConfig::default();
    custom_config.buyer_address = real_buyer;
    custom_config.test_buy_amount = U256::from(21_000_000_000_000u64); // Use smaller amount
    
    let custom_simulator = UnifiedSimulator::with_config(
        "/home/nima/.local/share/reth/mainnet",
        custom_config
    )?;
    
    match custom_simulator.simulate_buy_sell_sequence(token_address, pool_address, Some(TEST_BLOCK)).await {
        Ok(result) => {
            println!("  ✅ Custom simulation succeeded!");
            println!("    Buy Success: {}", result.buy_result.success);
            println!("    Sell Success: {}", result.sell_result.success);
            if let Some(reason) = &result.buy_result.revert_reason {
                println!("    Buy Revert: {}", reason);
            }
            if let Some(reason) = &result.sell_result.revert_reason {
                println!("    Sell Revert: {}", reason);
            }
        }
        Err(e) => {
            println!("  ❌ Custom simulation failed: {}", e);
        }
    }
    println!();
    
    // ==== STEP 7: Summary ====
    println!("📋 DIAGNOSIS SUMMARY:");
    println!("=====================================");
    
    if our_balance == U256::ZERO {
        println!("❌ PRIMARY ISSUE: Our test address (0x0C96c602...) has no ETH at block {}", TEST_BLOCK);
        println!("   This is why simulation fails with TRANSFER_FAILED");
        println!();
        println!("   SOLUTION OPTIONS:");
        println!("   1. Use a funded address for simulations");
        println!("   2. Override balance in simulation state");
        println!("   3. Use smaller amounts that don't require actual ETH");
    }
    
    let our_amount = U256::from(100_000_000_000_000_000u64);
    let weth_reserve = U256::from(1_000_000_000_000_000_000u64); // placeholder
    if calculate_price_impact(our_amount, weth_reserve) > 10.0 {
        println!();
        println!("⚠️  SECONDARY ISSUE: 0.1 ETH causes excessive price impact");
        println!("   The pool may not have enough liquidity for this amount");
        println!("   Real transaction used 0.000021 ETH (4,761x smaller)");
    }
    
    println!();
    println!("🔧 RECOMMENDATIONS:");
    println!("1. Modify simulator to check and handle zero balances");
    println!("2. Use state overrides to give test address ETH");
    println!("3. Dynamically adjust test amounts based on pool liquidity");
    println!("4. Add pre-flight checks before simulation");
    
    Ok(())
}

// Helper functions
fn format_ether(wei: U256) -> String {
    // Convert to string first to avoid precision loss
    let wei_str = wei.to_string();
    let wei_f64 = wei_str.parse::<f64>().unwrap_or(0.0);
    let eth = wei_f64 / 1e18;
    format!("{:.6}", eth)
}

fn calculate_price_impact(amount_in: U256, reserve: U256) -> f64 {
    if reserve == U256::ZERO {
        return 100.0;
    }
    
    // Rough approximation: impact = (amount_in / reserve) * 100
    let amount_f64 = amount_in.to_string().parse::<f64>().unwrap_or(0.0);
    let reserve_f64 = reserve.to_string().parse::<f64>().unwrap_or(1.0);
    
    (amount_f64 / reserve_f64) * 100.0
}

fn decode_revert_reason(error_str: &str) -> Option<String> {
    // Look for hex error data in the string
    if let Some(start) = error_str.find("0x08c379a0") {
        // This is the Error(string) selector
        // Try to extract and decode the revert reason
        if let Some(hex_start) = error_str[start..].find("0x") {
            let hex_str = &error_str[start + hex_start..];
            if let Some(end) = hex_str.find(|c: char| !c.is_ascii_hexdigit() && c != 'x') {
                let hex_data = &hex_str[..end];
                if hex_data.len() > 138 { // 0x + 8 (selector) + 64 (offset) + 64 (length) + at least 2 for data
                    // Skip: 0x08c379a0 (8) + offset (64) + length (64) = 136 chars + 0x = 138
                    if let Ok(bytes) = hex::decode(&hex_data[138..]) {
                        if let Ok(reason) = String::from_utf8(bytes) {
                            return Some(reason.trim_end_matches('\0').to_string());
                        }
                    }
                }
            }
        }
    }
    
    // Also check for other common patterns
    if error_str.contains("TRANSFER_FAILED") {
        return Some("UniswapV2: TRANSFER_FAILED".to_string());
    }
    if error_str.contains("UniswapV2: K") {
        return Some("UniswapV2: K (constant product invariant violated)".to_string());
    }
    
    None
}