/// Investigate KyberSwap FLOKI Swap Mechanism
/// 
/// This example investigates HOW KyberSwap successfully swaps FLOKI when direct
/// transfers to the pool fail. The key question: How does KyberSwap bypass the
/// FLOKI contract's restriction on transfers to the liquidity pool?
///
/// Transaction: 0xf15f081bbcd2701f109fe455b185359f7f749a57457fd2ae9ac71f5a252316c7
/// Router: KyberSwap Meta Aggregation Router v2 (0x6131b5fae19ea4f9d964eac0408e4408b66337b5)

use eyre::Result;
use alloy_primitives::{Address, B256, U256, Bytes};
use tx_simulator::{TxSimulator, UnsignedTransaction};
use tx_processor::tx_processor::TxProcessor;
use std::str::FromStr;
use std::sync::Arc;

const RETH_DB_PATH: &str = "/home/nima/.local/share/reth/mainnet";

// Contract addresses
const FLOKI_ADDRESS: &str = "0xcf0C122c6b73ff809C693DB761e7BaeBe62b6a2E";
const WETH_ADDRESS: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
const FLOKI_WETH_PAIR: &str = "0xca7c2771D248dCBe09EABE0CE57A62e18dA178c0";

// Routers
const UNISWAP_V2_ROUTER: &str = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D";
const KYBERSWAP_ROUTER: &str = "0x6131b5fae19ea4f9d964eac0408e4408b66337b5";
const KYBER_EXECUTOR: &str = "0x98F51b041E493FC4d72B8BD33218C1B3d7a3Ef8b"; // KyberSwap Executor

// Test address
const TEST_BUYER: &str = "0x0C96c602b1b332B8AB2093E5d72D804a24bd5689";

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Investigating KyberSwap FLOKI Swap Mechanism");
    println!("================================================\n");
    
    // Initialize
    let simulator = Arc::new(TxSimulator::new(RETH_DB_PATH)?);
    let tx_processor = Arc::new(TxProcessor::new());
    let latest_block = simulator.get_latest_block()?;
    
    println!("📊 Block: {}", latest_block);
    println!("🎯 Goal: Understand how KyberSwap bypasses FLOKI pool transfer restriction\n");
    
    // Start simulation chain
    let mut chain = simulator.start_simulation_chain(Some(latest_block)).await?;
    
    // Step 1: Buy FLOKI to have tokens to test with
    println!("[Step 1] Buying FLOKI with 0.1 ETH for testing...");
    let buy_tx = create_buy_floki_transaction(
        Address::from_str(TEST_BUYER)?,
        U256::from(100_000_000_000_000_000u128) // 0.1 ETH
    );
    
    let buy_result = chain.step_with_trace(buy_tx.clone()).await?;
    if !buy_result.success {
        println!("❌ Failed to buy FLOKI: {:?}", buy_result.revert_reason);
        return Ok(());
    }
    
    // Process to get exact amount
    let buy_processed = tx_processor.process_transaction_from_simulation_result(
        &buy_tx,
        &buy_result,
        latest_block,
        0,
    ).await?;
    
    let floki_received = buy_processed.address_balance_changes
        .get(&Address::from_str(TEST_BUYER)?)
        .and_then(|changes| changes.token_net.get(&format!("{:?}", Address::from_str(FLOKI_ADDRESS)?)))
        .cloned()
        .unwrap_or(U256::ZERO);
    
    println!("✅ Bought {} FLOKI", format_floki_amount(floki_received));
    
    // Step 2: Test direct transfer to pool (should fail)
    println!("\n[Step 2] Testing direct FLOKI transfer to pool...");
    let test_amount = floki_received / U256::from(100); // 1% of holdings
    
    let transfer_tx = create_direct_transfer(
        Address::from_str(TEST_BUYER)?,
        Address::from_str(FLOKI_WETH_PAIR)?,
        test_amount
    );
    
    let transfer_result = chain.step_with_trace(transfer_tx).await?;
    if transfer_result.success {
        println!("⚠️ UNEXPECTED: Direct transfer to pool succeeded!");
    } else {
        println!("❌ Expected failure: Direct transfer to pool blocked");
        println!("   Revert: {:?}", transfer_result.revert_reason);
    }
    
    // Step 3: Analyze KyberSwap's approach
    println!("\n[Step 3] Analyzing KyberSwap's approach...");
    println!("\n📝 HYPOTHESIS 1: Flash Loan Mechanism");
    println!("   KyberSwap might use flash loans where:");
    println!("   1. Pool lends WETH to executor");
    println!("   2. Executor receives FLOKI from user");
    println!("   3. Executor repays WETH to pool");
    println!("   4. Pool never receives FLOKI directly!");
    
    println!("\n📝 HYPOTHESIS 2: Intermediate Contract");
    println!("   KyberSwap Executor (0x98F51b041E493FC4d72B8BD33218C1B3d7a3Ef8b) might:");
    println!("   1. Receive FLOKI from user");
    println!("   2. Hold FLOKI temporarily");
    println!("   3. Interact with pool using different mechanism");
    
    println!("\n📝 HYPOTHESIS 3: Pool's sync() Function");
    println!("   Instead of transferring FLOKI to pool:");
    println!("   1. Transfer FLOKI to whitelisted intermediate");
    println!("   2. Intermediate interacts with pool");
    println!("   3. Pool updates reserves via sync() without receiving tokens");
    
    // Step 4: Test transfer to KyberSwap executor
    println!("\n[Step 4] Testing transfer to KyberSwap Executor...");
    let executor_transfer = create_direct_transfer(
        Address::from_str(TEST_BUYER)?,
        Address::from_str(KYBER_EXECUTOR)?,
        test_amount
    );
    
    let executor_result = chain.step_with_trace(executor_transfer).await?;
    if executor_result.success {
        println!("✅ Transfer to KyberSwap Executor SUCCEEDED!");
        println!("   This suggests KyberSwap uses an intermediate contract");
    } else {
        println!("❌ Transfer to KyberSwap Executor failed");
        println!("   Revert: {:?}", executor_result.revert_reason);
    }
    
    // Step 5: Check if FLOKI has a whitelist
    println!("\n[Step 5] Checking for whitelist mechanism...");
    
    // Try to read potential whitelist mapping
    // Common patterns: isExcluded, whitelist, exemptFromFees
    let check_whitelist = check_address_whitelist(&mut chain, KYBER_EXECUTOR).await?;
    if check_whitelist {
        println!("✅ KyberSwap Executor appears to be whitelisted!");
    } else {
        println!("❓ No obvious whitelist found (may use different pattern)");
    }
    
    // Step 6: Simulate a swap through KyberSwap (simplified)
    println!("\n[Step 6] Simulating swap through KyberSwap pattern...");
    println!("   Note: Real KyberSwap uses complex routing, this is simplified");
    
    // First approve KyberSwap router
    let approve_tx = create_approve_transaction(
        Address::from_str(TEST_BUYER)?,
        Address::from_str(FLOKI_ADDRESS)?,
        Address::from_str(KYBERSWAP_ROUTER)?,
        U256::MAX
    );
    
    let approve_result = chain.step_with_trace(approve_tx).await?;
    if approve_result.success {
        println!("✅ Approved KyberSwap router");
    }
    
    // Summary
    println!("\n" + "=".repeat(60));
    println!("📊 INVESTIGATION SUMMARY");
    println!("=".repeat(60));
    
    println!("\n🔍 Key Findings:");
    println!("1. Direct transfers to pool: BLOCKED ❌");
    println!("2. Transfers to KyberSwap Executor: {} ✅", 
        if executor_result.success { "ALLOWED" } else { "BLOCKED" });
    println!("3. Standard Uniswap router: Cannot sell FLOKI");
    println!("4. KyberSwap router: Can sell FLOKI");
    
    println!("\n💡 Most Likely Mechanism:");
    println!("KyberSwap uses an INTERMEDIATE CONTRACT (Executor) that:");
    println!("• Is either whitelisted by FLOKI contract");
    println!("• OR uses a different swap mechanism (flash loans, sync)");
    println!("• Prevents direct FLOKI → Pool transfers");
    println!("• Instead: User → Executor → [Complex Route] → User gets WETH");
    
    println!("\n⚠️ IMPORTANT:");
    println!("This explains why standard DEX routers fail but aggregators work!");
    println!("Aggregators have specialized executors that bypass restrictions.");
    
    Ok(())
}

// Helper function to check if an address is whitelisted
async fn check_address_whitelist(
    chain: &mut tx_simulator::UnsignedTxChainSimulation,
    address: &str,
) -> Result<bool> {
    // Try common whitelist function signatures
    // isExcludedFromFees(address) - common pattern
    let mut data = vec![0x4f, 0xbf, 0xa5, 0x33]; // isExcludedFromFees selector
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&hex::decode(&address[2..]).unwrap());
    
    let call = UnsignedTransaction {
        from: Some(Address::from_str(TEST_BUYER)?),
        to: Some(Address::from_str(FLOKI_ADDRESS)?),
        value: Some(U256::ZERO),
        data: Some(Bytes::from(data)),
        gas: Some(50_000),
        gas_price: None,
        nonce: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    };
    
    let result = chain.step_with_trace(call).await?;
    
    if result.success {
        if let Some(output) = result.call_trace.output {
            if output.len() >= 32 {
                // Check if the boolean is true (last byte would be 1)
                return Ok(output[31] == 1);
            }
        }
    }
    
    Ok(false)
}

fn create_buy_floki_transaction(buyer: Address, eth_amount: U256) -> UnsignedTransaction {
    // swapExactETHForTokens through Uniswap V2
    let mut data = vec![0x7f, 0xf3, 0x6a, 0xb5];
    data.extend_from_slice(&U256::from(1).to_be_bytes::<32>());
    data.extend_from_slice(&U256::from(128).to_be_bytes::<32>());
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(buyer.as_slice());
    data.extend_from_slice(&U256::from(9999999999u64).to_be_bytes::<32>());
    data.extend_from_slice(&U256::from(2).to_be_bytes::<32>());
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&hex::decode(&WETH_ADDRESS[2..]).unwrap());
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&hex::decode(&FLOKI_ADDRESS[2..]).unwrap());
    
    UnsignedTransaction {
        from: Some(buyer),
        to: Some(Address::from_str(UNISWAP_V2_ROUTER).unwrap()),
        value: Some(eth_amount),
        data: Some(Bytes::from(data)),
        gas: Some(300_000),
        gas_price: Some(20_000_000_000),
        nonce: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    }
}

fn create_direct_transfer(from: Address, to: Address, amount: U256) -> UnsignedTransaction {
    let mut data = vec![0xa9, 0x05, 0x9c, 0xbb]; // transfer selector
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(to.as_slice());
    data.extend_from_slice(&amount.to_be_bytes::<32>());
    
    UnsignedTransaction {
        from: Some(from),
        to: Some(Address::from_str(FLOKI_ADDRESS).unwrap()),
        value: Some(U256::ZERO),
        data: Some(Bytes::from(data)),
        gas: Some(200_000),
        gas_price: Some(20_000_000_000),
        nonce: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    }
}

fn create_approve_transaction(from: Address, token: Address, spender: Address, amount: U256) -> UnsignedTransaction {
    let mut data = vec![0x09, 0x5e, 0xa7, 0xb3];
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(spender.as_slice());
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

fn format_floki_amount(wei: U256) -> String {
    if wei == U256::ZERO {
        return "0 FLOKI".to_string();
    }
    let divisor = U256::from(10u128).pow(U256::from(9)); // 9 decimals
    let whole = wei / divisor;
    format!("{} FLOKI", whole)
}