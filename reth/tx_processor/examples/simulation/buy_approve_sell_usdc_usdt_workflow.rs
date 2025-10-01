use alloy_primitives::{Address, Bytes, U256};
/// USDC and USDT Buy → Approve → Sell Workflow with Balance Analysis
///
/// This example demonstrates a complete token trading workflow for USDC and USDT
/// using UnsignedTxChainSimulation with proper state preservation between transactions.
///
/// Key features:
/// 1. Uses simulation chain for proper state preservation between transactions
/// 2. Shows detailed balance changes for the buyer after buy and sell
/// 3. Compares USDC vs USDT trading workflows
/// 4. Uses ProcessedTransaction to extract token amounts and fees
///
/// This shows how to analyze the actual token amounts received and ETH spent.
use eyre::Result;
use std::str::FromStr;
use std::sync::Arc;
use tx_processor::tx_processor::TxProcessor;
use tx_simulator::{TxSimulator, UnsignedTransaction};

const RETH_DB_PATH: &str = "/home/nima/.local/share/reth/mainnet";

// Well-known contract addresses
const USDC_ADDRESS: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"; // USDC token
const USDT_ADDRESS: &str = "0xdAC17F958D2ee523a2206206994597C13D831ec7"; // USDT token
const WETH_ADDRESS: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"; // Wrapped ETH
const UNISWAP_V2_ROUTER: &str = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D"; // Uniswap V2 Router

// Test address with ETH balance
const TEST_BUYER: &str = "0x0C96c602b1b332B8AB2093E5d72D804a24bd5689"; // Test buyer (NOT a fee recipient)

#[tokio::main]
async fn main() -> Result<()> {
    println!("💱 Buy → Approve → Sell Workflow Demo");
    println!("=====================================");
    println!("Using simulation chain for proper state preservation.\n");

    // Initialize simulator and processor
    let simulator = Arc::new(TxSimulator::new(RETH_DB_PATH)?);
    let tx_processor = Arc::new(TxProcessor::new());
    println!("✅ Simulator and processor initialized");

    // Get latest block
    let latest_block = simulator.get_latest_block()?;
    println!("📊 Block: {}", latest_block);
    println!("💰 Investment: 1 ETH per token\n");

    let buyer_address = Address::from_str(TEST_BUYER)?;
    let router_address = Address::from_str(UNISWAP_V2_ROUTER)?;

    // Test USDC workflow
    println!("{}", "=".repeat(60));
    println!("USDC WORKFLOW");
    println!("{}", "=".repeat(60));
    execute_trading_workflow(
        simulator.clone(),
        tx_processor.clone(),
        buyer_address,
        USDC_ADDRESS,
        "USDC",
        router_address,
        latest_block,
    )
    .await?;

    println!("\n{}", "=".repeat(60));
    println!("USDT WORKFLOW");
    println!("{}", "=".repeat(60));
    execute_trading_workflow(
        simulator.clone(),
        tx_processor.clone(),
        buyer_address,
        USDT_ADDRESS,
        "USDT",
        router_address,
        latest_block,
    )
    .await?;

    println!("\n✅ Workflow demonstration complete!");
    Ok(())
}

/// Execute trading workflow and show real results
async fn execute_trading_workflow(
    simulator: Arc<TxSimulator>,
    tx_processor: Arc<TxProcessor>,
    buyer: Address,
    token_address: &str,
    token_symbol: &str,
    router: Address,
    block: u64,
) -> Result<()> {
    println!("\n🚀 Starting {} workflow at block {}", token_symbol, block);

    // Start a simulation chain with trace capability (use latest block)
    let mut chain = simulator.start_simulation_chain(None).await?;
    println!("📍 Chain initialized");

    // Step 1: Buy tokens with 1 ETH
    println!("\n[Step 1] Buying {} with 1 ETH...", token_symbol);
    let buy_tx = create_buy_token_transaction(
        buyer,
        token_address,
        U256::from(1_000_000_000_000_000_000u128), // 1 ETH
    );

    let buy_result = chain.step_with_trace(buy_tx.clone()).await?;

    println!(
        "  Status: {}",
        if buy_result.success {
            "✅ Success"
        } else {
            "❌ Failed"
        }
    );
    println!("  Gas used: {}", buy_result.gas_used);

    if !buy_result.success {
        println!("  Revert reason: {:?}", buy_result.revert_reason);
        return Ok(());
    }

    // Process the buy transaction to get balance changes
    let buy_processed = tx_processor
        .process_transaction_from_simulation_result(&buy_tx, &buy_result, block, 0)
        .await?;

    // Extract buyer's balance changes from buy
    println!("\n  💰 Buyer Balance Changes After Buy:");
    if let Some(buyer_changes) = buy_processed.address_balance_changes.get(&buyer) {
        // ETH spent
        if let Some(eth_change) = buyer_changes.currency_net.get("ETH") {
            let eth_wei = eth_change.to_string();
            println!("    ETH spent: {} wei", eth_wei);
        }
        // Tokens received
        for (token, amount) in &buyer_changes.token_net {
            println!("    Token received: {} = {} (raw amount)", token, amount);
        }
    }

    // Count ERC20 transfers
    println!(
        "    ERC20 transfers detected: {}",
        buy_processed.erc20_transfers.len()
    );
    for transfer in &buy_processed.erc20_transfers {
        if transfer.to_address == buyer {
            println!(
                "    Received {} tokens (wei): {}",
                token_symbol, transfer.amount
            );
        }
    }

    // Step 2: Approve router
    println!("\n[Step 2] Approving router...");
    let token_addr = Address::from_str(token_address)?;
    let approve_tx = create_approve_transaction(buyer, token_addr, router, U256::MAX);

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

    // Step 3: Sell a small amount of tokens
    println!("\n[Step 3] Selling tokens back to ETH...");

    // We don't know exact balance, so try selling a small amount that should work
    let sell_amount = if token_symbol == "USDC" {
        U256::from(100_000_000u128) // 100 USDC (6 decimals)
    } else {
        U256::from(100_000_000u128) // 100 USDT (6 decimals)
    };

    let sell_tx = create_sell_token_transaction(buyer, token_address, sell_amount);
    let sell_result = chain.step_with_trace(sell_tx.clone()).await?;

    println!(
        "  Status: {}",
        if sell_result.success {
            "✅ Success"
        } else {
            "❌ Failed"
        }
    );
    println!("  Gas used: {}", sell_result.gas_used);

    if !sell_result.success {
        println!("  Revert reason: {:?}", sell_result.revert_reason);
    } else {
        // Process the sell transaction to get balance changes
        let sell_processed = tx_processor
            .process_transaction_from_simulation_result(
                &sell_tx,
                &sell_result,
                block,
                2, // tx index
            )
            .await?;

        println!("\n  💰 Buyer Balance Changes After Sell:");
        if let Some(buyer_changes) = sell_processed.address_balance_changes.get(&buyer) {
            // ETH received
            if let Some(eth_change) = buyer_changes.currency_net.get("ETH") {
                let eth_wei = eth_change.to_string();
                println!("    ETH received: {} wei", eth_wei);
            }
            // Tokens sold
            for (token, amount) in &buyer_changes.token_net {
                println!("    Token sent: {} = {} (raw amount)", token, amount);
            }
        }

        // Count ERC20 transfers
        println!(
            "    ERC20 transfers detected: {}",
            sell_processed.erc20_transfers.len()
        );
        for transfer in &sell_processed.erc20_transfers {
            if transfer.from_address == buyer {
                println!(
                    "    Sent {} tokens (wei): {}",
                    token_symbol, transfer.amount
                );
            }
        }
    }

    // Final state
    let state = chain.current_state();
    println!("\n📊 Final Chain State:");
    println!("  • Transactions executed: {}", state.transaction_count);
    println!("  • Total gas used: {}", state.total_gas_used);

    Ok(())
}

/// Create a transaction to buy a token with ETH using Uniswap V2
fn create_buy_token_transaction(
    buyer: Address,
    token_address: &str,
    eth_amount: U256,
) -> UnsignedTransaction {
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

    // Target token address
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&hex::decode(&token_address[2..]).unwrap());

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

/// Create a transaction to sell tokens for ETH using Uniswap V2
fn create_sell_token_transaction(
    seller: Address,
    token_address: &str,
    token_amount: U256,
) -> UnsignedTransaction {
    // swapExactTokensForETH(uint256 amountIn, uint256 amountOutMin, address[] path, address to, uint256 deadline)
    let mut data = vec![0x18, 0xcb, 0xaf, 0xe5]; // Function selector

    // amountIn
    data.extend_from_slice(&token_amount.to_be_bytes::<32>());

    // amountOutMin (1 = accept any amount)
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

    // Source token
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&hex::decode(&token_address[2..]).unwrap());

    // WETH
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
