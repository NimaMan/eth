/// Alternative FLOKI selling methods to bypass UniswapV2: K error
///
/// This example demonstrates multiple approaches to sell FLOKI tokens:
/// 1. WETH intermediate swap (FLOKI -> WETH -> ETH)
/// 2. Direct pair interaction (bypass router)
/// 3. Smaller chunk swaps
/// 4. Add/Remove liquidity method
use alloy_primitives::{Address, Bytes, I256, U256};
use eyre::Result;
use std::str::FromStr;
use tracing::{error, info};
use tx_processor::tx_processor::TxProcessor;
use tx_simulator::{TxSimulator, UnsignedTransaction, UnsignedTxChainSimulation};

// Token addresses
const FLOKI_ADDRESS: &str = "0xcf0C122c6b73ff809C693DB761e7BaeBe62b6a2E";
const WETH_ADDRESS: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
const UNISWAP_V2_ROUTER: &str = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D";
const SUCCESSFUL_ROUTER: &str = "0xBEE3211ab312a8D065c4FeF0247448e17A8da000"; // Router that successfully sold FLOKI
const FLOKI_WETH_PAIR: &str = "0xca7c2771D248dCBe09EABE0CE57A62e18dA178c0";

const FLOKI_DECIMALS: u8 = 9;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("tx_processor=info".parse()?)
                .add_directive("tx_simulator=info".parse()?),
        )
        .init();

    info!("🚀 Starting FLOKI alternative selling methods test");

    // Initialize components
    let reth_datadir = "/home/nima/.local/share/reth/mainnet";
    let tx_processor = TxProcessor::new();
    let simulator = TxSimulator::new(reth_datadir)?;

    // Test configuration - use latest block
    let block = simulator.get_latest_block()?;
    let buyer = Address::from_str("0x0C96c602b1b332B8AB2093E5d72D804a24bd5689")?;
    let router = Address::from_str(UNISWAP_V2_ROUTER)?;

    // Start simulation chain
    let mut chain = simulator.start_simulation_chain(Some(block)).await?;
    info!("📍 Chain initialized at block {}", block);

    // Get current nonce to avoid nonce errors
    let chain_state = chain.current_state();
    info!(
        "📍 Current nonce for buyer: {}",
        chain_state.nonces.get(&buyer).unwrap_or(&0)
    );

    // Step 1: Buy FLOKI with 1 ETH
    info!("\n[Step 1] Buying FLOKI with 1 ETH...");
    let buy_amount = U256::from(1_000_000_000_000_000_000u128); // 1 ETH
    let buy_tx = create_buy_floki_transaction(buyer, buy_amount);

    let buy_result = chain.step_with_trace(buy_tx.clone()).await?;

    if !buy_result.success {
        error!("Failed to buy FLOKI: {:?}", buy_result.revert_reason);
        return Ok(());
    }

    // Process transaction to get exact amounts
    let buy_processed = tx_processor
        .process_transaction_from_simulation_result(&buy_tx, &buy_result, block, 0)
        .await?;

    // Extract FLOKI received from balance changes
    let floki_address = Address::from_str(FLOKI_ADDRESS)?;
    let floki_received_signed = buy_processed
        .address_balance_changes
        .get(&buyer)
        .and_then(|changes| changes.token_net.get(&format!("{:?}", floki_address)))
        .copied()
        .unwrap_or(I256::ZERO);
    let floki_received = floki_received_signed.unsigned_abs();

    let eth_spent_signed = buy_processed
        .address_balance_changes
        .get(&buyer)
        .and_then(|changes| changes.currency_net.get("ETH"))
        .copied()
        .unwrap_or(I256::ZERO);
    let eth_spent = eth_spent_signed.unsigned_abs();

    info!("✅ Successfully bought FLOKI");
    info!("  Gas used: {}", buy_result.gas_used);
    info!("\n  📊 FLOKI RECEIVED FROM 1 ETH BUY:");
    info!("  ═══════════════════════════════════");
    info!("    Raw amount: {} wei", floki_received);
    info!("    Formatted: {}", format_floki_amount(floki_received));
    info!(
        "    ETH spent: {}",
        format_eth_amount(eth_spent)
    );

    let exchange_rate = if eth_spent > U256::ZERO {
        let floki_per_eth = (floki_received * U256::from(10u64).pow(U256::from(18)))
            / eth_spent;
        floki_per_eth / U256::from(10u64).pow(U256::from(FLOKI_DECIMALS))
    } else {
        U256::ZERO
    };
    info!("    Exchange rate: {} FLOKI per ETH", exchange_rate);
    info!("    Token decimals: {} (not standard 18!)", FLOKI_DECIMALS);

    // CRITICAL CHECK: Verify FLOKI balance using balanceOf
    info!("\n🔍 CRITICAL BALANCE VERIFICATION:");
    info!("   Checking actual FLOKI balance in contract...");

    // Method 1: Our helper function
    let balance = get_floki_balance(&mut chain, buyer).await?;
    info!(
        "   Balance from helper: {} raw ({} FLOKI)",
        balance,
        format_floki_amount(balance)
    );

    // Method 2: Direct balanceOf call with detailed logging
    let balance_check = check_floki_balance_detailed(&mut chain, buyer).await?;
    info!("   Balance from direct call: {} raw", balance_check);

    // Check if balances match
    if balance != balance_check {
        error!(
            "   ⚠️ BALANCE MISMATCH! Helper: {} vs Direct: {}",
            balance, balance_check
        );
    }

    if balance == U256::ZERO {
        error!("   ❌ CRITICAL: Balance is ZERO despite successful buy!");
        error!("   This explains why transfers fail - FLOKI's _balances[buyer] was never updated!");

        // Let's also check if the buyer address had any balance before
        info!("\n   Checking if buyer had pre-existing balance...");
        let original_balance = get_original_balance(&mut chain, buyer).await?;
        info!(
            "   Original balance at block start: {} raw",
            original_balance
        );
    } else {
        info!(
            "   ✅ Balance exists: {} FLOKI",
            format_floki_amount(balance)
        );
    }

    // ========================================
    // EARLY EXIT FOR BALANCE VERIFICATION
    // ========================================
    info!("\n🎯 BALANCE VERIFICATION COMPLETE!");
    info!("   Stopping here to see balance comparison results.");
    info!("   This avoids nonce errors from later transfer tests.");
    return Ok(());

    // Step 2: Approve BOTH routers (standard and successful one)
    info!("\n[Step 2] Approving routers to spend FLOKI...");

    // Approve standard Uniswap router
    let approve_uniswap_tx =
        create_approve_transaction(buyer, Address::from_str(FLOKI_ADDRESS)?, router, U256::MAX);

    let approve_result = chain.step_with_trace(approve_uniswap_tx).await?;

    if !approve_result.success {
        error!(
            "Failed to approve Uniswap router: {:?}",
            approve_result.revert_reason
        );
        return Ok(());
    }

    info!("✅ Uniswap router approved");

    // Approve the successful router
    let approve_successful_tx = create_approve_transaction(
        buyer,
        Address::from_str(FLOKI_ADDRESS)?,
        Address::from_str(SUCCESSFUL_ROUTER)?,
        U256::MAX,
    );

    let approve_successful_result = chain.step_with_trace(approve_successful_tx).await?;

    if !approve_successful_result.success {
        error!(
            "Failed to approve successful router: {:?}",
            approve_successful_result.revert_reason
        );
        return Ok(());
    }

    info!("✅ Both routers approved");

    // First, let's debug the treasury handler issue
    info!("\n=== DEBUGGING TREASURY HANDLER ===\n");
    info!("Reading treasury handler address from FLOKI contract...");

    // Read treasuryHandler address from FLOKI (it's a public variable)
    let treasury_handler_address = get_treasury_handler(&mut chain).await?;
    info!("Treasury Handler: {:?}", treasury_handler_address);

    // Test beforeTransferHandler directly
    if treasury_handler_address != Address::ZERO {
        info!("\nTesting beforeTransferHandler directly...");
        test_treasury_handler(
            &mut chain,
            buyer,
            treasury_handler_address,
            balance / U256::from(100),
        )
        .await?;
    } else {
        info!("Treasury handler is ZERO address - essentially disabled");
    }

    // Now check tax handler
    info!("\n=== CHECKING TAX HANDLER ===");
    let tax_handler_address = get_tax_handler(&mut chain).await?;
    info!("Tax Handler: {:?}", tax_handler_address);

    if tax_handler_address != Address::ZERO {
        info!("Testing getTax directly...");
        test_tax_handler(
            &mut chain,
            buyer,
            tax_handler_address,
            balance / U256::from(100),
        )
        .await?;
    } else {
        info!("Tax handler is ZERO address - no taxes");
    }

    // CRITICAL TEST: Try transferring to a regular address instead of pool
    info!("\n=== TESTING TRANSFER TO DIFFERENT ADDRESSES ===");
    info!(
        "We have {} FLOKI, let's test different transfers",
        format_floki_amount(balance)
    );

    // Test 1: Transfer to a random address
    let random_address = Address::from_str("0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0")?;
    info!(
        "\n1. Testing transfer to random address: {:?}",
        random_address
    );
    test_direct_transfer(
        &mut chain,
        buyer,
        random_address,
        U256::from(100_000_000_000u128),
    )
    .await?;

    // Test 2: Transfer to pool address
    let pool_address = Address::from_str(FLOKI_WETH_PAIR)?;
    info!("\n2. Testing transfer to pool address: {:?}", pool_address);
    test_direct_transfer(
        &mut chain,
        buyer,
        pool_address,
        U256::from(100_000_000_000u128),
    )
    .await?;

    // Test 3: Transfer to router address
    let router_address = Address::from_str(UNISWAP_V2_ROUTER)?;
    info!(
        "\n3. Testing transfer to router address: {:?}",
        router_address
    );
    test_direct_transfer(
        &mut chain,
        buyer,
        router_address,
        U256::from(100_000_000_000u128),
    )
    .await?;

    // Now test direct pool interaction - bypass ALL routers
    info!("\n=== TESTING DIRECT POOL INTERACTION ===\n");
    info!("Bypassing all routers - direct interaction with Uniswap V2 pool");
    info!("Pool: 0xca7c2771D248dCBe09EABE0CE57A62e18dA178c0");
    info!("{}", "-".repeat(70));

    // Test different amounts with direct pool interaction
    let test_amounts = vec![
        (
            "100 FLOKI",
            U256::from(100u64) * U256::from(10u64).pow(U256::from(FLOKI_DECIMALS)),
        ),
        (
            "1000 FLOKI",
            U256::from(1000u64) * U256::from(10u64).pow(U256::from(FLOKI_DECIMALS)),
        ),
        (
            "10000 FLOKI",
            U256::from(10000u64) * U256::from(10u64).pow(U256::from(FLOKI_DECIMALS)),
        ),
        ("1% of balance", balance / U256::from(100)),
        ("10% of balance", balance / U256::from(10)),
    ];

    for (label, amount) in test_amounts {
        if amount > balance {
            info!("⏭️  Skipping {} - exceeds balance", label);
            continue;
        }

        info!("\n🔄 Testing direct pool swap: {}", label);
        info!("   Amount: {} FLOKI", format_floki_amount(amount));

        let success = test_direct_pool_interaction(&mut chain, buyer, amount).await?;

        if success {
            info!("✅ SUCCESS! Direct pool interaction worked with {}!", label);
            info!("   This proves the issue is router complexity, not pool liquidity!");
            break;
        } else {
            info!("❌ Failed with {}", label);
        }
    }

    Ok(())
}

/// Test direct pool interaction - bypass all routers
async fn test_direct_pool_interaction(
    chain: &mut UnsignedTxChainSimulation,
    seller: Address,
    floki_amount: U256,
) -> Result<bool> {
    info!("   Step 1: Getting pool reserves...");

    // Get current reserves from pool
    let (reserve0, reserve1) = get_pool_reserves(chain).await?;
    info!(
        "   Pool reserves - Reserve0 (WETH): {}, Reserve1 (FLOKI): {}",
        reserve0, reserve1
    );

    // Calculate how much WETH we'll get for our FLOKI
    // FLOKI is token1, WETH is token0 in this pool
    let amount_out = calculate_amount_out(floki_amount, reserve1, reserve0);
    info!(
        "   Expected output: {} wei WETH for {} FLOKI",
        amount_out,
        format_floki_amount(floki_amount)
    );

    if amount_out == U256::ZERO {
        error!("   ❌ Calculated output is zero - insufficient liquidity or reserves issue");
        return Ok(false);
    }

    info!("   Step 2: Transferring FLOKI to pool...");
    // Transfer FLOKI directly to the pool
    let transfer_tx = create_transfer_to_pool_transaction(seller, floki_amount);

    info!("   🔍 DEBUG: Transfer transaction details:");
    info!("      From: {:?}", transfer_tx.from);
    info!("      To: {:?}", transfer_tx.to);
    info!("      Value: {:?}", transfer_tx.value);
    info!("      Gas: {:?}", transfer_tx.gas);
    info!("      Data: {:?}", transfer_tx.data);

    let transfer_result = chain.step_with_trace(transfer_tx).await?;

    info!("   🔍 DEBUG: Transfer result:");
    info!("      Success: {}", transfer_result.success);
    info!("      Gas used: {}", transfer_result.gas_used);
    info!("      Revert reason: {:?}", transfer_result.revert_reason);

    // Print the FULL call trace to see exactly where it fails
    if let Some(trace) = &transfer_result.call_trace.output {
        info!("   🔍 DEBUG: Call trace output length: {}", trace.len());
        if trace.len() > 0 {
            info!("   🔍 DEBUG: Output: 0x{}", hex::encode(trace));
        }
    }

    // Print execution trace if available
    info!("   🔍 DEBUG: Call trace details:");
    info!("      Gas used: {}", transfer_result.call_trace.gas_used);
    info!("      Error: {:?}", transfer_result.call_trace.error);

    if !transfer_result.success {
        error!("   ❌ EXACT FAILURE: FLOKI.transfer() reverted at contract level");
        error!("      This means the FLOKI contract's transfer function rejected the call");
        error!("      Contract address: {}", FLOKI_ADDRESS);
        error!("      Function called: transfer(address,uint256)");
        error!(
            "      Parameters: to={}, amount={}",
            FLOKI_WETH_PAIR, floki_amount
        );
        return Ok(false);
    }

    info!("   ✅ FLOKI transferred to pool successfully");

    info!("   Step 3: Calling pool.swap()...");
    // Call swap on pool directly
    let swap_tx = create_pool_swap_transaction(seller, amount_out, U256::ZERO, seller);
    let swap_result = chain.step_with_trace(swap_tx).await?;

    if swap_result.success {
        info!("   ✅ Direct pool swap successful! Got {} WETH", amount_out);
        info!("   Gas used: {}", swap_result.gas_used);
        return Ok(true);
    } else {
        error!(
            "   ❌ Direct pool swap failed: {:?}",
            swap_result.revert_reason
        );
        return Ok(false);
    }
}

/// Get reserves from Uniswap V2 pool
async fn get_pool_reserves(chain: &mut UnsignedTxChainSimulation) -> Result<(U256, U256)> {
    // getReserves() selector: 0x0902f1ac
    let data = vec![0x09, 0x02, 0xf1, 0xac];

    let call = UnsignedTransaction {
        from: None,
        to: Some(Address::from_str(FLOKI_WETH_PAIR)?),
        value: Some(U256::ZERO),
        data: Some(Bytes::from(data)),
        gas: Some(50_000),
        gas_price: None,
        nonce: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    };

    let result = chain.step_with_trace(call).await?;

    if let Some(output) = result.call_trace.output {
        if output.len() >= 64 {
            let reserve0 = U256::from_be_slice(&output[0..32]); // WETH
            let reserve1 = U256::from_be_slice(&output[32..64]); // FLOKI
            return Ok((reserve0, reserve1));
        }
    }

    Ok((U256::ZERO, U256::ZERO))
}

/// Calculate amount out using Uniswap V2 formula
fn calculate_amount_out(amount_in: U256, reserve_in: U256, reserve_out: U256) -> U256 {
    if amount_in == U256::ZERO || reserve_in == U256::ZERO || reserve_out == U256::ZERO {
        return U256::ZERO;
    }

    // Uniswap V2 formula with 0.3% fee
    // amountOut = (amountIn * 997 * reserveOut) / (reserveIn * 1000 + amountIn * 997)
    let amount_in_with_fee = amount_in * U256::from(997);
    let numerator = amount_in_with_fee * reserve_out;
    let denominator = reserve_in * U256::from(1000) + amount_in_with_fee;

    if denominator == U256::ZERO {
        return U256::ZERO;
    }

    numerator / denominator
}

// ===== Helper Functions =====

/// Get FLOKI balance
async fn get_floki_balance(chain: &mut UnsignedTxChainSimulation, owner: Address) -> Result<U256> {
    get_token_balance(chain, owner, Address::from_str(FLOKI_ADDRESS)?).await
}

/// Check FLOKI balance with detailed logging
async fn check_floki_balance_detailed(
    chain: &mut UnsignedTxChainSimulation,
    owner: Address,
) -> Result<U256> {
    info!("      Calling FLOKI.balanceOf({:?})", owner);

    let mut data = vec![0x70, 0xa0, 0x82, 0x31]; // balanceOf selector
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(owner.as_slice());

    let call = UnsignedTransaction {
        from: Some(owner),
        to: Some(Address::from_str(FLOKI_ADDRESS)?),
        value: Some(U256::ZERO),
        data: Some(Bytes::from(data)),
        gas: Some(50_000),
        gas_price: None,
        nonce: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    };

    info!("      Call data: {:?}", call.data);
    let result = chain.step_with_trace(call).await?;

    info!("      Call success: {}", result.success);
    info!("      Gas used: {}", result.gas_used);

    if let Some(output) = result.call_trace.output {
        info!("      Output length: {} bytes", output.len());
        if output.len() >= 32 {
            let balance = U256::from_be_slice(&output[0..32]);
            info!("      Raw output: 0x{}", hex::encode(&output[0..32]));
            info!("      Decoded balance: {}", balance);
            return Ok(balance);
        } else {
            error!("      Invalid output length!");
        }
    } else {
        error!("      No output from balanceOf!");
    }

    Ok(U256::ZERO)
}

/// Get original balance at block start (before any transactions)
async fn get_original_balance(
    chain: &mut UnsignedTxChainSimulation,
    owner: Address,
) -> Result<U256> {
    // This checks the balance without any of our transactions
    // It should be the balance at the start of the block
    get_token_balance(chain, owner, Address::from_str(FLOKI_ADDRESS)?).await
}

/// Test direct transfer to any address
async fn test_direct_transfer(
    chain: &mut UnsignedTxChainSimulation,
    from: Address,
    to: Address,
    amount: U256,
) -> Result<()> {
    info!(
        "   Attempting transfer: {} FLOKI from {:?} to {:?}",
        format_floki_amount(amount),
        from,
        to
    );

    // Identify the target address type
    let target_type = if to == Address::from_str(FLOKI_WETH_PAIR).unwrap_or(Address::ZERO) {
        "LIQUIDITY POOL"
    } else if to == Address::from_str(UNISWAP_V2_ROUTER).unwrap_or(Address::ZERO) {
        "ROUTER"
    } else {
        "REGULAR ADDRESS"
    };
    info!("   Target type: {}", target_type);

    // Create transfer transaction
    let mut data = vec![0xa9, 0x05, 0x9c, 0xbb]; // transfer selector
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(to.as_slice());
    data.extend_from_slice(&amount.to_be_bytes::<32>());

    let transfer_tx = UnsignedTransaction {
        from: Some(from),
        to: Some(Address::from_str(FLOKI_ADDRESS)?),
        value: Some(U256::ZERO),
        data: Some(Bytes::from(data)),
        gas: Some(200_000),
        gas_price: Some(20_000_000_000),
        nonce: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    };

    let result = chain.step_with_trace(transfer_tx).await?;

    if result.success {
        info!("   ✅ TRANSFER SUCCEEDED! Gas used: {}", result.gas_used);
        info!("   This address CAN receive FLOKI transfers!");
    } else {
        error!("   ❌ TRANSFER FAILED!");
        error!("   Gas used: {}", result.gas_used);
        error!("   Revert reason: {:?}", result.revert_reason);

        // Explain why it failed based on target type
        if target_type == "LIQUIDITY POOL" {
            error!("   📝 EXPLANATION: FLOKI contract BLOCKS transfers to its liquidity pool!");
            error!("                   This is why you can't sell FLOKI through DEX routers.");
            error!("                   Router needs to send FLOKI to pool, but that's BLOCKED!");
        } else if target_type == "ROUTER" {
            error!("   📝 EXPLANATION: Transfer to router might work, but router can't forward to pool");
        }
        error!("   This address CANNOT receive FLOKI transfers!");
    }

    Ok(())
}

/// Get any ERC20 token balance
async fn get_token_balance(
    chain: &mut UnsignedTxChainSimulation,
    owner: Address,
    token: Address,
) -> Result<U256> {
    let mut data = vec![0x70, 0xa0, 0x82, 0x31]; // balanceOf selector
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(owner.as_slice());

    let call = UnsignedTransaction {
        from: Some(owner),
        to: Some(token),
        value: Some(U256::ZERO),
        data: Some(Bytes::from(data)),
        gas: Some(50_000),
        gas_price: None,
        nonce: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    };

    let result = chain.step_with_trace(call).await?;

    if let Some(output) = result.call_trace.output {
        if output.len() >= 32 {
            return Ok(U256::from_be_slice(&output[0..32]));
        }
    }

    Ok(U256::ZERO)
}

// Removed unused helper functions

// ===== Transaction Building Functions =====

fn create_buy_floki_transaction(buyer: Address, eth_amount: U256) -> UnsignedTransaction {
    let mut data = vec![0x7f, 0xf3, 0x6a, 0xb5]; // swapExactETHForTokens
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

fn create_approve_transaction(
    from: Address,
    token: Address,
    spender: Address,
    amount: U256,
) -> UnsignedTransaction {
    let mut data = vec![0x09, 0x5e, 0xa7, 0xb3]; // approve
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

/// Transfer FLOKI tokens directly to the pool
fn create_transfer_to_pool_transaction(from: Address, amount: U256) -> UnsignedTransaction {
    // transfer(address,uint256) - ERC20 transfer
    let mut data = vec![0xa9, 0x05, 0x9c, 0xbb]; // transfer selector

    // Pool address as recipient
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&hex::decode(&FLOKI_WETH_PAIR[2..]).unwrap());

    // Amount to transfer
    data.extend_from_slice(&amount.to_be_bytes::<32>());

    UnsignedTransaction {
        from: Some(from),
        to: Some(Address::from_str(FLOKI_ADDRESS).unwrap()),
        value: Some(U256::ZERO),
        data: Some(Bytes::from(data)),
        gas: Some(100_000),
        gas_price: Some(20_000_000_000),
        nonce: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    }
}

/// Call swap directly on Uniswap V2 pool
fn create_pool_swap_transaction(
    sender: Address,
    amount0_out: U256, // WETH out
    amount1_out: U256, // FLOKI out (should be 0 for our case)
    to: Address,
) -> UnsignedTransaction {
    // swap(uint256,uint256,address,bytes) - Uniswap V2 pool swap
    let mut data = vec![0x02, 0x2c, 0x0d, 0x9f]; // swap selector

    // amount0Out (WETH we want to receive)
    data.extend_from_slice(&amount0_out.to_be_bytes::<32>());

    // amount1Out (FLOKI we want to receive - 0 in our case)
    data.extend_from_slice(&amount1_out.to_be_bytes::<32>());

    // to address (where to send the output tokens)
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(to.as_slice());

    // data offset (for callback data)
    data.extend_from_slice(&U256::from(128).to_be_bytes::<32>());

    // data length (empty callback data)
    data.extend_from_slice(&U256::from(0).to_be_bytes::<32>());

    UnsignedTransaction {
        from: Some(sender),
        to: Some(Address::from_str(FLOKI_WETH_PAIR).unwrap()),
        value: Some(U256::ZERO),
        data: Some(Bytes::from(data)),
        gas: Some(200_000), // Direct pool swap needs less gas than router
        gas_price: Some(20_000_000_000),
        nonce: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    }
}

// Removed unused transaction creation functions

/// Get tax handler address from FLOKI contract
async fn get_tax_handler(chain: &mut UnsignedTxChainSimulation) -> Result<Address> {
    // taxHandler() selector: 0xc57f6661
    let data = vec![0xc5, 0x7f, 0x66, 0x61];

    let call = UnsignedTransaction {
        from: None,
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

    if let Some(output) = result.call_trace.output {
        if output.len() >= 32 {
            // Last 20 bytes are the address
            let mut address_bytes = [0u8; 20];
            address_bytes.copy_from_slice(&output[12..32]);
            return Ok(Address::from(address_bytes));
        }
    }

    Ok(Address::ZERO)
}

/// Test tax handler to see if it's causing the issue
async fn test_tax_handler(
    chain: &mut UnsignedTxChainSimulation,
    from: Address,
    tax_handler: Address,
    amount: U256,
) -> Result<()> {
    let to = Address::from_str(FLOKI_WETH_PAIR)?; // Pool address

    info!("Calling getTax({:?}, {:?}, {})", from, to, amount);

    // getTax(address,address,uint256) selector: 0x89cd9855
    let mut data = vec![0x89, 0xcd, 0x98, 0x55];

    // from address
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(from.as_slice());

    // to address
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(to.as_slice());

    // amount
    data.extend_from_slice(&amount.to_be_bytes::<32>());

    let call = UnsignedTransaction {
        from: Some(from),
        to: Some(tax_handler),
        value: Some(U256::ZERO),
        data: Some(Bytes::from(data)),
        gas: Some(200_000),
        gas_price: Some(20_000_000_000),
        nonce: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    };

    let result = chain.step_with_trace(call).await?;

    if result.success {
        info!("✅ getTax succeeded! Gas used: {}", result.gas_used);
        if let Some(output) = result.call_trace.output {
            if output.len() >= 32 {
                let tax = U256::from_be_slice(&output[0..32]);
                info!(
                    "   Tax amount: {} ({}% of transfer)",
                    tax,
                    (tax * U256::from(100)) / amount
                );
            }
        }
    } else {
        error!("❌ getTax FAILED!");
        error!("   Gas used: {}", result.gas_used);
        error!("   Revert reason: {:?}", result.revert_reason);
        error!("   Error: {:?}", result.call_trace.error);
        error!("   THIS COULD BE THE FAILURE POINT!");
    }

    Ok(())
}

/// Get treasury handler address from FLOKI contract
async fn get_treasury_handler(chain: &mut UnsignedTxChainSimulation) -> Result<Address> {
    // treasuryHandler() selector: 0x3e72a434
    let data = vec![0x3e, 0x72, 0xa4, 0x34];

    let call = UnsignedTransaction {
        from: None,
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

    if let Some(output) = result.call_trace.output {
        if output.len() >= 32 {
            // Last 20 bytes are the address
            let mut address_bytes = [0u8; 20];
            address_bytes.copy_from_slice(&output[12..32]);
            return Ok(Address::from(address_bytes));
        }
    }

    Ok(Address::ZERO)
}

/// Test treasury handler directly to find exact failure
async fn test_treasury_handler(
    chain: &mut UnsignedTxChainSimulation,
    from: Address,
    treasury_handler: Address,
    amount: U256,
) -> Result<()> {
    let to = Address::from_str(FLOKI_WETH_PAIR)?; // Pool address

    info!(
        "Calling beforeTransferHandler({:?}, {:?}, {})",
        from, to, amount
    );

    // beforeTransferHandler(address,address,uint256) selector: 0x2ef0c85e
    let mut data = vec![0x2e, 0xf0, 0xc8, 0x5e];

    // from address
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(from.as_slice());

    // to address
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(to.as_slice());

    // amount
    data.extend_from_slice(&amount.to_be_bytes::<32>());

    let call = UnsignedTransaction {
        from: Some(from),
        to: Some(treasury_handler),
        value: Some(U256::ZERO),
        data: Some(Bytes::from(data)),
        gas: Some(200_000),
        gas_price: Some(20_000_000_000),
        nonce: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    };

    let result = chain.step_with_trace(call).await?;

    if result.success {
        info!(
            "✅ beforeTransferHandler succeeded! Gas used: {}",
            result.gas_used
        );
        info!("   This means treasury handler accepts the transfer");
    } else {
        error!("❌ beforeTransferHandler FAILED!");
        error!("   Gas used: {}", result.gas_used);
        error!("   Revert reason: {:?}", result.revert_reason);
        error!("   Error: {:?}", result.call_trace.error);
        error!("   THIS IS THE EXACT FAILURE POINT!");
    }

    Ok(())
}

// ===== Formatting Functions =====

fn format_floki_amount(wei: U256) -> String {
    if wei == U256::ZERO {
        return "0 FLOKI".to_string();
    }

    let divisor = U256::from(10u128).pow(U256::from(FLOKI_DECIMALS));
    let whole = wei / divisor;
    let remainder = wei % divisor;

    if remainder == U256::ZERO {
        format!("{} FLOKI", whole)
    } else {
        let decimal_part = (remainder * U256::from(100)) / divisor;
        format!("{}.{:02} FLOKI", whole, decimal_part)
    }
}

fn format_eth_amount(wei: U256) -> String {
    if wei == U256::ZERO {
        return "0 ETH".to_string();
    }

    let divisor = U256::from(10u128).pow(U256::from(18));
    let whole = wei / divisor;
    let remainder = wei % divisor;

    let decimal_part = (remainder * U256::from(1_000_000u64)) / divisor;
    format!("{}.{:06} ETH", whole, decimal_part)
}
