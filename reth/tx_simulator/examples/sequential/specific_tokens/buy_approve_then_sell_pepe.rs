/// Buy → Approve → Sell PEPE Token Workflow Example
/// 
/// This example demonstrates a complete PEPE token trading workflow using UnsignedTxChainSimulation,
/// where each step depends on the results of the previous one:
/// 
/// 1. Buy PEPE tokens with ETH
/// 2. Approve router for PEPE spending
/// 3. Sell PEPE tokens back to ETH
/// 
/// PEPE is a popular meme token with 18 decimals (unlike USDC/USDT which have 6).
/// This workflow REQUIRES state preservation between steps.

use eyre::Result;
use alloy_primitives::{Address, U256, Bytes};
use tx_simulator::{TxSimulator, UnsignedTransaction};
use std::str::FromStr;

const RETH_DB_PATH: &str = "/home/nima/.local/share/reth/mainnet";

// Contract addresses
const PEPE_ADDRESS: &str = "0x6982508145454Ce325dDbE47a25d4ec3d2311933";     // PEPE token (18 decimals)
const WETH_ADDRESS: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";     // Wrapped ETH
const UNISWAP_V2_ROUTER: &str = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D"; // Uniswap V2 Router

// Test address with ETH balance (~5.38 ETH)
const TEST_BUYER: &str = "0x0C96c602b1b332B8AB2093E5d72D804a24bd5689";

#[tokio::main]
async fn main() -> Result<()> {
    println!("🐸 PEPE Token Buy → Approve → Sell Workflow Demo");
    println!("=================================================");
    println!("Using PEPE token with 18 decimals\n");
    
    // Initialize simulator
    let simulator = TxSimulator::new(RETH_DB_PATH)?;
    println!("✅ Simulator initialized");
    
    // Get latest block
    let latest_block = simulator.get_latest_block()?;
    println!("📊 Block: {}", latest_block);
    println!("💰 Investment: 0.1 ETH");
    println!("🏦 Buyer: {}\n", TEST_BUYER);
    
    let buyer_address = Address::from_str(TEST_BUYER)?;
    let router_address = Address::from_str(UNISWAP_V2_ROUTER)?;
    
    // Execute PEPE workflow
    println!("{}", "=".repeat(60));
    println!("PEPE WORKFLOW");
    println!("{}", "=".repeat(60));
    
    execute_pepe_trading_workflow(
        &simulator,
        buyer_address,
        router_address,
        latest_block,
    ).await?;
    
    println!("\n✅ PEPE workflow demonstration complete!");
    Ok(())
}

/// Execute PEPE trading workflow and show real results
async fn execute_pepe_trading_workflow(
    simulator: &TxSimulator,
    buyer: Address,
    router: Address,
    block: u64,
) -> Result<()> {
    println!("\n🚀 Starting PEPE workflow at block {}", block);
    
    // Start a simulation chain with trace capability (use latest block)
    let mut chain = simulator.start_simulation_chain(None).await?;
    println!("📍 Chain initialized");
    
    // Step 1: Buy PEPE tokens with 0.1 ETH
    println!("\n[Step 1] Buying PEPE with 0.1 ETH...");
    let buy_tx = create_buy_pepe_transaction(
        buyer, 
        U256::from(100_000_000_000_000_000u128) // 0.1 ETH
    );
    
    let buy_result = chain.step_with_trace(buy_tx).await?;
    
    println!("  Status: {}", if buy_result.success { "✅ Success" } else { "❌ Failed" });
    println!("  Gas used: {}", buy_result.gas_used);
    println!("  Logs generated: {}", buy_result.call_trace.logs.len());
    
    if !buy_result.success {
        println!("  Revert reason: {:?}", buy_result.revert_reason);
        return Ok(());
    }
    
    // Step 2: Approve router for PEPE
    println!("\n[Step 2] Approving router for PEPE...");
    let pepe_addr = Address::from_str(PEPE_ADDRESS)?;
    let approve_tx = create_approve_transaction(buyer, pepe_addr, router, U256::MAX);
    
    let approve_result = chain.step_with_trace(approve_tx).await?;
    
    println!("  Status: {}", if approve_result.success { "✅ Success" } else { "❌ Failed" });
    println!("  Gas used: {}", approve_result.gas_used);
    println!("  Logs generated: {}", approve_result.call_trace.logs.len());
    
    if !approve_result.success {
        println!("  Revert reason: {:?}", approve_result.revert_reason);
        return Ok(());
    }
    
    // Step 3: Sell PEPE tokens back to ETH
    println!("\n[Step 3] Selling PEPE tokens back to ETH...");
    
    // PEPE has 18 decimals, so we'll sell 1,000,000 PEPE
    // 1,000,000 PEPE = 1000000 * 10^18
    let sell_amount = U256::from(1_000_000u128) * U256::from(10u128).pow(U256::from(18));
    
    let sell_tx = create_sell_pepe_transaction(buyer, sell_amount);
    let sell_result = chain.step_with_trace(sell_tx).await?;
    
    println!("  Status: {}", if sell_result.success { "✅ Success" } else { "❌ Failed" });
    println!("  Gas used: {}", sell_result.gas_used);
    println!("  Logs generated: {}", sell_result.call_trace.logs.len());
    
    if !sell_result.success {
        println!("  Revert reason: {:?}", sell_result.revert_reason);
    }
    
    // Final state
    let state = chain.current_state();
    println!("\n📊 Final Chain State:");
    println!("  • Transactions executed: {}", state.transaction_count);
    println!("  • Total gas used: {}", state.total_gas_used);
    
    // Summary
    println!("\n📈 PEPE Trading Summary:");
    println!("  • Bought PEPE with 0.1 ETH");
    println!("  • Approved router for unlimited PEPE");
    println!("  • Attempted to sell 1,000,000 PEPE");
    println!("  • Note: PEPE has 18 decimals (like ETH)");
    
    Ok(())
}

/// Create a transaction to buy PEPE with ETH using Uniswap V2
fn create_buy_pepe_transaction(buyer: Address, eth_amount: U256) -> UnsignedTransaction {
    // swapExactETHForTokens(uint256 amountOutMin, address[] path, address to, uint256 deadline)
    let mut data = vec![0x7f, 0xf3, 0x6a, 0xb5]; // Function selector
    
    // amountOutMin (1 = accept any amount)
    data.extend_from_slice(&U256::from(1).to_be_bytes::<32>());
    
    // path offset (points to dynamic array)
    data.extend_from_slice(&U256::from(128).to_be_bytes::<32>());
    
    // to address (buyer)
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(buyer.as_slice());
    
    // deadline (far future)
    data.extend_from_slice(&U256::from(9999999999u64).to_be_bytes::<32>());
    
    // path array
    data.extend_from_slice(&U256::from(2).to_be_bytes::<32>()); // length = 2
    
    // WETH address
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&hex::decode(&WETH_ADDRESS[2..]).unwrap());
    
    // PEPE address
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&hex::decode(&PEPE_ADDRESS[2..]).unwrap());
    
    UnsignedTransaction {
        from: Some(buyer),
        to: Some(Address::from_str(UNISWAP_V2_ROUTER).unwrap()),
        value: Some(eth_amount),
        data: Some(Bytes::from(data)),
        gas: Some(300_000),
        gas_price: Some(20_000_000_000), // 20 gwei
        nonce: None, // Let UnsignedTxChainSimulation handle nonce
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    }
}

/// Create an approve transaction for ERC20 tokens
fn create_approve_transaction(from: Address, token: Address, spender: Address, amount: U256) -> UnsignedTransaction {
    // approve(address spender, uint256 amount)
    let mut data = vec![0x09, 0x5e, 0xa7, 0xb3]; // approve selector
    
    // spender address
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(spender.as_slice());
    
    // amount
    data.extend_from_slice(&amount.to_be_bytes::<32>());
    
    UnsignedTransaction {
        from: Some(from),
        to: Some(token),
        value: Some(U256::ZERO),
        data: Some(Bytes::from(data)),
        gas: Some(100_000),
        gas_price: Some(20_000_000_000),
        nonce: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    }
}

/// Create a transaction to sell PEPE for ETH using Uniswap V2
fn create_sell_pepe_transaction(seller: Address, pepe_amount: U256) -> UnsignedTransaction {
    // swapExactTokensForETH(uint256 amountIn, uint256 amountOutMin, address[] path, address to, uint256 deadline)
    let mut data = vec![0x18, 0xcb, 0xaf, 0xe5]; // Function selector
    
    // amountIn (amount of PEPE to sell)
    data.extend_from_slice(&pepe_amount.to_be_bytes::<32>());
    
    // amountOutMin (1 = accept any amount of ETH)
    data.extend_from_slice(&U256::from(1).to_be_bytes::<32>());
    
    // path offset
    data.extend_from_slice(&U256::from(160).to_be_bytes::<32>());
    
    // to address (seller)
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(seller.as_slice());
    
    // deadline
    data.extend_from_slice(&U256::from(9999999999u64).to_be_bytes::<32>());
    
    // path array
    data.extend_from_slice(&U256::from(2).to_be_bytes::<32>()); // length = 2
    
    // PEPE address
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&hex::decode(&PEPE_ADDRESS[2..]).unwrap());
    
    // WETH address
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&hex::decode(&WETH_ADDRESS[2..]).unwrap());
    
    UnsignedTransaction {
        from: Some(seller),
        to: Some(Address::from_str(UNISWAP_V2_ROUTER).unwrap()),
        value: Some(U256::ZERO),
        data: Some(Bytes::from(data)),
        gas: Some(300_000),
        gas_price: Some(20_000_000_000),
        nonce: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    }
}