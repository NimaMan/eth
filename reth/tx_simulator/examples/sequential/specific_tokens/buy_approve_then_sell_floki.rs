use alloy_primitives::{Address, Bytes, U256};
/// Buy → Approve → Sell FLOKI Token Workflow Example
use eyre::Result;
use std::str::FromStr;
use tx_simulator::{TxSimulator, UnsignedTransaction};

const RETH_DB_PATH: &str = "/home/nima/.local/share/reth/mainnet";

// Contract addresses
const FLOKI_ADDRESS: &str = "0xcf0C122c6b73ff809C693DB761e7BaeBe62b6a2E"; // FLOKI token (9 decimals)
const WETH_ADDRESS: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"; // Wrapped ETH
const UNISWAP_V2_ROUTER: &str = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D"; // Uniswap V2 Router

// Test address with ETH balance
const TEST_BUYER: &str = "0x0C96c602b1b332B8AB2093E5d72D804a24bd5689";

/// Encode balanceOf(owner) function call
fn encode_balance_of(owner: Address) -> Bytes {
    let mut data = vec![0x70, 0xa0, 0x82, 0x31]; // balanceOf selector
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(owner.as_slice());
    Bytes::from(data)
}

/// Decode U256 from contract call output
fn decode_uint256_from_output(output: &Bytes) -> U256 {
    if output.len() >= 32 {
        U256::from_be_slice(&output[..32])
    } else {
        U256::ZERO
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🐕 FLOKI Token Buy → Approve → Sell Workflow Demo");
    println!("=================================================");
    println!("Using FLOKI token with 9 decimals\n");

    // Initialize simulator
    let simulator = TxSimulator::new(RETH_DB_PATH)?;
    println!("✅ Simulator initialized");

    // Use latest block to avoid pruned state issues
    let block = simulator.get_latest_block()?;
    println!("📊 Block: {}", block);
    println!("💰 Investment: 0.1 ETH");
    println!("🏦 Buyer: {}\n", TEST_BUYER);

    let buyer_address = Address::from_str(TEST_BUYER)?;
    let router_address = Address::from_str(UNISWAP_V2_ROUTER)?;

    // Execute FLOKI workflow
    println!("{}", "=".repeat(60));
    println!("FLOKI WORKFLOW");
    println!("{}", "=".repeat(60));

    execute_floki_trading_workflow(&simulator, buyer_address, router_address, block).await?;

    println!("\n✅ FLOKI workflow demonstration complete!");
    Ok(())
}

/// Execute FLOKI trading workflow and show real results
async fn execute_floki_trading_workflow(
    simulator: &TxSimulator,
    buyer: Address,
    router: Address,
    block: u64,
) -> Result<()> {
    println!("\n🚀 Starting FLOKI workflow at block {}", block);

    // Start a simulation chain with trace capability (use latest block)
    let mut chain = simulator.start_simulation_chain(None).await?;
    println!("📍 Chain initialized");

    // Step 1: Buy FLOKI tokens with 0.1 ETH
    println!("\n[Step 1] Buying FLOKI with 0.1 ETH...");
    let buy_tx = create_buy_floki_transaction(
        buyer,
        U256::from(100_000_000_000_000_000u128), // 0.1 ETH
    );

    let buy_result = chain.step_with_trace(buy_tx).await?;

    println!(
        "  Status: {}",
        if buy_result.success {
            "✅ Success"
        } else {
            "❌ Failed"
        }
    );
    println!("  Gas used: {}", buy_result.gas_used);
    println!("  Logs generated: {}", buy_result.call_trace.logs.len());

    // Debug: Check if we received tokens
    if buy_result.success && buy_result.call_trace.logs.is_empty() {
        println!("  ⚠️  WARNING: No logs emitted! FLOKI tokens might not have been transferred.");
        println!("  This could mean the swap didn't actually execute a token transfer.");
    }

    if !buy_result.success {
        println!("  Revert reason: {:?}", buy_result.revert_reason);
        return Ok(());
    }

    // ========================================
    // BALANCE VERIFICATION AFTER BUY
    // ========================================
    println!("\n  🔍 BALANCE VERIFICATION:");
    let floki_addr = Address::from_str(FLOKI_ADDRESS)?;

    // Direct balanceOf() simulation call
    println!("    Making direct balanceOf() simulation call...");
    let balance_call_data = encode_balance_of(buyer);
    let balance_call = UnsignedTransaction {
        from: Some(buyer),
        to: Some(floki_addr),
        value: Some(U256::ZERO),
        data: Some(balance_call_data),
        gas: Some(50_000),
        gas_price: None,
        nonce: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    };

    let balance_result = chain.step_with_trace(balance_call).await?;
    let floki_balance = if balance_result.success {
        if let Some(output) = balance_result.call_trace.output {
            decode_uint256_from_output(&output)
        } else {
            U256::ZERO
        }
    } else {
        println!(
            "    ❌ balanceOf() call failed: {:?}",
            balance_result.revert_reason
        );
        U256::ZERO
    };

    println!("    FLOKI balance after buy: {} raw tokens", floki_balance);

    if floki_balance == U256::ZERO {
        println!("    ❌ CRITICAL: Balance is ZERO despite buy transaction success!");
        println!("    This explains why sells fail - no actual FLOKI tokens received!");
    } else {
        println!(
            "    ✅ SUCCESS: Actual FLOKI tokens received: {}",
            floki_balance
        );
        let floki_formatted = floki_balance / U256::from(10_u64.pow(9)); // FLOKI has 9 decimals
        println!("    Formatted: {} FLOKI", floki_formatted);
    }

    // Step 2: Approve router for FLOKI
    println!("\n[Step 2] Approving router for FLOKI...");
    let approve_tx = create_approve_transaction(buyer, floki_addr, router, U256::MAX);

    let approve_result = chain.step_with_trace(approve_tx).await?;

    println!(
        "  Status: {}",
        if approve_result.success {
            "✅ Success"
        } else {
            "❌ Failed"
        }
    );
    println!("  Gas used: {}", approve_result.gas_used);
    println!("  Logs generated: {}", approve_result.call_trace.logs.len());

    if !approve_result.success {
        println!("  Revert reason: {:?}", approve_result.revert_reason);
        return Ok(());
    }

    // Step 3: Sell FLOKI tokens back to ETH
    println!("\n[Step 3] Selling FLOKI tokens back to ETH...");

    // FLOKI has 9 decimals, so we'll sell 1,000,000 FLOKI
    // 1,000,000 FLOKI = 1000000 * 10^9
    let sell_amount = U256::from(1_000_000u128) * U256::from(10u128).pow(U256::from(9));

    let sell_tx = create_sell_floki_transaction(buyer, sell_amount);
    let sell_result = chain.step_with_trace(sell_tx).await?;

    println!(
        "  Status: {}",
        if sell_result.success {
            "✅ Success"
        } else {
            "❌ Failed"
        }
    );
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
    println!("\n📈 FLOKI Trading Summary:");
    println!("  • Bought FLOKI with 0.1 ETH");
    println!("  • Approved router for unlimited FLOKI");
    println!("  • Attempted to sell 1,000,000 FLOKI");
    println!("  • Note: FLOKI has 9 decimals");

    // Alternative test with mixed approach
    println!("\n{}", "=".repeat(60));
    println!("ALTERNATIVE TEST: Mixed step() and step_with_trace()");
    println!("{}", "=".repeat(60));
    println!("\nTesting with step() for buy/approve, step_with_trace() for sell...");
    println!("This tests if state persistence is the issue.\n");

    // Start a fresh chain
    let mut chain2 = simulator.start_simulation_chain(None).await?;
    println!("📍 Fresh chain initialized");

    // Step 1: Buy with step() (ensures state persistence)
    println!("\n[Step 1] Buying FLOKI with step()...");
    let buy_tx2 = create_buy_floki_transaction(
        buyer,
        U256::from(100_000_000_000_000_000u128), // 0.1 ETH
    );
    let buy_result2 = chain2.step(buy_tx2).await?;
    println!(
        "  Status: {}",
        if buy_result2.success {
            "✅ Success"
        } else {
            "❌ Failed"
        }
    );
    println!("  Gas used: {}", buy_result2.gas_used);

    if !buy_result2.success {
        println!("  Revert reason: {:?}", buy_result2.revert_reason);
        return Ok(());
    }

    // Check FLOKI balance after buy
    println!("\n[Balance Check] Checking FLOKI balance after buy...");
    let balance_call = create_balance_check_transaction(buyer, floki_addr);
    let balance_result = chain2.step(balance_call).await?;
    println!(
        "  Balance check executed: {}",
        if balance_result.success { "✅" } else { "❌" }
    );
    println!("  Note: Balance is stored in contract state, not returned directly");

    // Step 2: Approve with step() (ensures state persistence)
    println!("\n[Step 2] Approving router with step()...");
    let approve_tx2 = create_approve_transaction(buyer, floki_addr, router, U256::MAX);
    let approve_result2 = chain2.step(approve_tx2).await?;
    println!(
        "  Status: {}",
        if approve_result2.success {
            "✅ Success"
        } else {
            "❌ Failed"
        }
    );
    println!("  Gas used: {}", approve_result2.gas_used);

    if !approve_result2.success {
        println!("  Revert reason: {:?}", approve_result2.revert_reason);
        return Ok(());
    }

    // Step 3: Sell with step_with_trace() (to get trace data)
    println!("\n[Step 3] Selling FLOKI with step_with_trace()...");
    let sell_tx2 = create_sell_floki_transaction(buyer, sell_amount);
    let sell_result2 = chain2.step_with_trace(sell_tx2).await?;
    println!(
        "  Status: {}",
        if sell_result2.success {
            "✅ Success"
        } else {
            "❌ Failed"
        }
    );
    println!("  Gas used: {}", sell_result2.gas_used);
    println!("  Logs generated: {}", sell_result2.call_trace.logs.len());

    if !sell_result2.success {
        println!("  Revert reason: {:?}", sell_result2.revert_reason);
    }

    println!("\n📊 Alternative Test Summary:");
    if sell_result2.success {
        println!("  ✅ Mixed approach WORKS! State persistence confirmed as the issue.");
        println!("  • step() properly persists FLOKI's complex state");
        println!("  • step_with_trace() alone fails to persist state between transactions");
    } else {
        println!("  ❌ Mixed approach also failed. Issue might be deeper than state persistence.");
    }

    // Test with reduced amount
    println!("\n{}", "=".repeat(60));
    println!("REDUCED AMOUNT TEST: Selling only 10,000 FLOKI");
    println!("{}", "=".repeat(60));

    let mut chain3 = simulator.start_simulation_chain(None).await?;
    println!("\n📍 Fresh chain initialized for reduced amount test");

    // Buy and approve with step()
    println!("\n[Quick Setup] Buy and Approve with step()...");
    let buy_tx3 = create_buy_floki_transaction(buyer, U256::from(100_000_000_000_000_000u128));
    let buy_result3 = chain3.step(buy_tx3).await?;
    let approve_tx3 = create_approve_transaction(buyer, floki_addr, router, U256::MAX);
    let approve_result3 = chain3.step(approve_tx3).await?;

    if buy_result3.success && approve_result3.success {
        println!("  ✅ Buy and Approve successful");

        // Try selling much smaller amount: 10,000 FLOKI
        println!("\n[Reduced Test] Selling only 10,000 FLOKI...");
        let small_sell_amount = U256::from(10_000u128) * U256::from(10u128).pow(U256::from(9));
        let sell_tx3 = create_sell_floki_transaction(buyer, small_sell_amount);
        let sell_result3 = chain3.step_with_trace(sell_tx3).await?;

        println!(
            "  Status: {}",
            if sell_result3.success {
                "✅ Success"
            } else {
                "❌ Failed"
            }
        );
        println!("  Gas used: {}", sell_result3.gas_used);

        if !sell_result3.success {
            println!("  Revert reason: {:?}", sell_result3.revert_reason);
        } else {
            println!("  ✅ Small amount works! Confirms we do receive FLOKI tokens.");
        }
    }

    Ok(())
}

/// Create a transaction to buy FLOKI with ETH using Uniswap V2
fn create_buy_floki_transaction(buyer: Address, eth_amount: U256) -> UnsignedTransaction {
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

    // FLOKI address
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&hex::decode(&FLOKI_ADDRESS[2..]).unwrap());

    UnsignedTransaction {
        from: Some(buyer),
        to: Some(Address::from_str(UNISWAP_V2_ROUTER).unwrap()),
        value: Some(eth_amount),
        data: Some(Bytes::from(data)),
        gas: Some(300_000),
        gas_price: Some(20_000_000_000), // 20 gwei
        nonce: None,                     // Let UnsignedTxChainSimulation handle nonce
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    }
}

/// Create an approve transaction for ERC20 tokens
fn create_approve_transaction(
    from: Address,
    token: Address,
    spender: Address,
    amount: U256,
) -> UnsignedTransaction {
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

/// Create a balance check transaction for ERC20 tokens
fn create_balance_check_transaction(owner: Address, token: Address) -> UnsignedTransaction {
    // balanceOf(address owner)
    let mut data = vec![0x70, 0xa0, 0x82, 0x31]; // balanceOf selector

    // owner address
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(owner.as_slice());

    UnsignedTransaction {
        from: Some(owner),
        to: Some(token),
        value: Some(U256::ZERO),
        data: Some(Bytes::from(data)),
        gas: Some(50_000),
        gas_price: Some(20_000_000_000),
        nonce: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    }
}

/// Create a transaction to sell FLOKI for ETH using Uniswap V2
fn create_sell_floki_transaction(seller: Address, floki_amount: U256) -> UnsignedTransaction {
    // swapExactTokensForETH(uint256 amountIn, uint256 amountOutMin, address[] path, address to, uint256 deadline)
    let mut data = vec![0x18, 0xcb, 0xaf, 0xe5]; // Function selector

    // amountIn (amount of FLOKI to sell)
    data.extend_from_slice(&floki_amount.to_be_bytes::<32>());

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

    // FLOKI address
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&hex::decode(&FLOKI_ADDRESS[2..]).unwrap());

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
