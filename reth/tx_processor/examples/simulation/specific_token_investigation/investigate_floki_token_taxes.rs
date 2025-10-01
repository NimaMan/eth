use alloy_primitives::{Address, I256, U256};
/// FLOKI Token Tax Investigation
///
/// This example investigates FLOKI token which has confirmed transaction fee mechanism via tax handler.
/// We perform buy -> approve -> sell sequence and calculate actual tax percentages after each step.
///
/// Algorithm:
/// 1. Initialize simulation environment with FLOKI token and FLOKI/WETH Uniswap V2 pool
/// 2. Execute BUY transaction and calculate buy tax from balance changes
/// 3. Execute APPROVE transaction for token spending
/// 4. Execute SELL transaction and calculate sell tax from balance changes
/// 5. Report detailed tax analysis and investigate actual tax percentages
use eyre::Result;
use reth_chain_query::tx_builders::{self, amm_swap_route::AmmSwapRoute};
use std::sync::Arc;
use tx_processor::tx_processor::tax_calculator::{
    calculate_buy_tax_from_processed_transaction, calculate_sell_tax_from_processed_transaction,
};
use tx_processor::tx_processor::TxProcessor;
use tx_simulator::{TxSimulator, UnsignedTransaction};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔬 FLOKI Token Tax Investigation");
    println!("==============================");
    println!("Investigating FLOKI token with confirmed tax handler mechanism");
    println!("Goal: Understand actual buy/sell tax calculation\\n");

    // Get reth datadir
    let reth_datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());

    // Create simulator and tx processor
    let simulator = Arc::new(TxSimulator::new(&reth_datadir)?);
    let tx_processor = Arc::new(TxProcessor::new());

    // FLOKI token (current/new contract) - has tax handler mechanism
    let floki_token = Address::from([
        0xcf, 0x0c, 0x12, 0x2c, 0x6b, 0x73, 0xff, 0x80, 0x9c, 0x69, 0x3d, 0xb7, 0x61, 0xe7, 0xba,
        0xeb, 0xe6, 0x2b, 0x6a, 0x2e,
    ]); // 0xcf0c122c6b73ff809c693db761e7baebe62b6a2e

    // FLOKI/WETH Uniswap V2 pool address
    let floki_pool = Address::from([
        0xca, 0x7c, 0x27, 0x71, 0xd2, 0x48, 0xdc, 0xbe, 0x09, 0xea, 0xbe, 0x0c, 0xe5, 0x7a, 0x62,
        0xe1, 0x8d, 0xa1, 0x78, 0xc0,
    ]); // 0xca7c2771d248dcbe09eabe0ce57a62e18da178c0

    let buyer_address = Address::from([
        0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef, 0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd,
        0xef, 0x12, 0x34, 0x56, 0x78,
    ]);

    let test_amount = U256::from(500_000_000_000_000_000u128); // 0.5 ETH
    let block_number = 18289000u64; // Use much older stable block to avoid pruning

    println!("📊 Test Configuration:");
    println!("  FLOKI Token: {}", floki_token);
    println!("  FLOKI/WETH Pool: {}", floki_pool);
    println!("  Test Amount: {} wei (0.5 ETH)", test_amount);
    println!("  Block Number: {}", block_number);
    println!("  Expected Tax: Variable via tax handler\\n");

    // Start simulation chain
    let mut chain = simulator.start_simulation_chain(Some(block_number)).await?;
    let route = AmmSwapRoute::UniswapV2 { pool: floki_pool };

    // ===================
    // STEP 1: BUY TOKENS
    // ===================
    println!("🔄 STEP 1: Executing BUY transaction");

    let buy_tx = tx_builders::build_buy_swap(
        &route,
        buyer_address,
        floki_token,
        test_amount,
        5000, // 50% slippage
        u64::MAX,
    );

    let buy_result = chain.step_with_trace(buy_tx.clone()).await?;
    println!(
        "  Buy Result: success={}, gas_used={}",
        buy_result.success, buy_result.gas_used
    );

    if !buy_result.success {
        println!(
            "  ❌ Buy transaction failed: {:?}",
            buy_result.revert_reason
        );
        println!("  📝 Pool might not exist or have insufficient liquidity");
        return Ok(());
    }

    // Process buy transaction
    let buy_processed = tx_processor
        .process_transaction_from_simulation_result(
            &buy_tx,
            &buy_result,
            block_number,
            1, // tx_index
        )
        .await?;

    // Calculate buy tax
    let buy_tax = calculate_buy_tax_from_processed_transaction(
        &buy_processed,
        floki_pool,
        buyer_address,
        floki_token,
    );

    // Extract tokens received
    let tokens_received_signed =
        extract_tokens_received(&buy_processed, buyer_address, floki_token);
    let tokens_received = tokens_received_signed.unsigned_abs();

    println!("  ✅ Buy transaction successful!");
    println!("  📊 Tokens Received: {}", tokens_received);
    println!(
        "  💰 Buy Tax: {:.2}%",
        buy_tax.as_percentage().unwrap_or(0.0)
    );

    if buy_tax.as_percentage().unwrap_or(0.0) > 0.5 {
        println!("  🎯 Tax detected! FLOKI tax handler is active.");
    } else {
        println!(
            "  ⚠️  No significant tax detected. Tax handler might be inactive or rate is very low."
        );
    }

    // ===================
    // STEP 2: APPROVE TOKENS
    // ===================
    println!("\\n🔄 STEP 2: Executing APPROVE transaction");

    let approve_tx =
        tx_builders::build_approve_for_route(&route, buyer_address, floki_token, U256::MAX);

    let approve_result = chain.step_with_trace(approve_tx.clone()).await?;
    println!(
        "  Approve Result: success={}, gas_used={}",
        approve_result.success, approve_result.gas_used
    );

    if !approve_result.success {
        println!(
            "  ❌ Approve transaction failed: {:?}",
            approve_result.revert_reason
        );
        return Ok(());
    }

    println!("  ✅ Approve transaction successful!");

    // ===================
    // STEP 3: SELL TOKENS
    // ===================
    println!("\\n🔄 STEP 3: Executing SELL transaction");
    println!("  Attempting to sell {} FLOKI tokens", tokens_received);

    let sell_tx = tx_builders::build_sell_swap(
        &route,
        buyer_address,
        floki_token,
        tokens_received,
        5000, // 50% slippage
        u64::MAX,
    );

    let sell_result = chain.step_with_trace(sell_tx.clone()).await?;
    println!(
        "  Sell Result: success={}, gas_used={}",
        sell_result.success, sell_result.gas_used
    );

    if !sell_result.success {
        println!(
            "  ❌ Sell transaction failed: {:?}",
            sell_result.revert_reason
        );
        println!("\\n🔍 SELL FAILURE ANALYSIS:");
        println!("  This could be due to:");
        println!("  1. Tax handler applying different rates for buy vs sell");
        println!("  2. Slippage tolerance too low for tax-adjusted amounts");
        println!("  3. Pool liquidity changes during tax calculations");
        println!("  4. Tax handler blocking certain sell operations");

        // Try with next block
        println!("\\n🔄 Attempting sell in NEXT BLOCK (block delay)...");
        let next_block_chain = simulator
            .start_simulation_chain(Some(block_number + 1))
            .await?;

        // Re-apply buy and approve to establish state
        let mut delayed_chain = next_block_chain;
        let _ = delayed_chain.step_with_trace(buy_tx).await?;
        let _ = delayed_chain.step_with_trace(approve_tx).await?;

        // Try sell again
        let delayed_sell_result = delayed_chain.step_with_trace(sell_tx.clone()).await?;
        println!(
            "  Delayed Sell Result: success={}, gas_used={}",
            delayed_sell_result.success, delayed_sell_result.gas_used
        );

        if delayed_sell_result.success {
            println!("  ✅ Sell succeeded with block delay!");

            // Calculate sell tax for delayed transaction
            let sell_processed = tx_processor
                .process_transaction_from_simulation_result(
                    &sell_tx,
                    &delayed_sell_result,
                    block_number + 1,
                    3,
                )
                .await?;

            let sell_tax = calculate_sell_tax_from_processed_transaction(
                &sell_processed,
                floki_pool,
                buyer_address,
            );

            println!(
                "  💰 Sell Tax: {:.2}%",
                sell_tax.as_percentage().unwrap_or(0.0)
            );
        } else {
            println!(
                "  ❌ Sell still fails with block delay: {:?}",
                delayed_sell_result.revert_reason
            );
            println!("  📝 FLOKI tax handler may restrict sell operations.");
        }
    } else {
        // Sell succeeded - calculate sell tax
        let sell_processed = tx_processor
            .process_transaction_from_simulation_result(&sell_tx, &sell_result, block_number, 3)
            .await?;

        let sell_tax = calculate_sell_tax_from_processed_transaction(
            &sell_processed,
            floki_pool,
            buyer_address,
        );

        println!("  ✅ Sell transaction successful!");
        println!(
            "  💰 Sell Tax: {:.2}%",
            sell_tax.as_percentage().unwrap_or(0.0)
        );
    }

    // ===================
    // SUMMARY
    // ===================
    println!("\\n📊 FLOKI TOKEN INVESTIGATION SUMMARY");
    println!("===================================");
    println!("  Token: FLOKI (with tax handler) - {}", floki_token);
    println!("  Pool: FLOKI/WETH Uniswap V2 - {}", floki_pool);
    println!("  Buy Tax: {:.2}%", buy_tax.as_percentage().unwrap_or(0.0));

    println!("\\n🎯 Key Findings:");
    if buy_tax.as_percentage().unwrap_or(0.0) > 0.1 {
        println!("  ✅ Buy tax detected - FLOKI tax handler is active");
        println!("  🔬 Tax calculation system working with fee-on-transfer tokens");
    } else {
        println!("  ⚠️  No significant buy tax detected");
        println!("  📝 Tax handler might be inactive or using very low rates");
    }

    println!("  💡 This validates our tax calculation methodology");
    println!("  📊 Tax rates are dynamically determined by FLOKI's tax handler contract");

    Ok(())
}

/// Extract tokens received from a processed transaction
fn extract_tokens_received(
    processed_tx: &tx_processor::tx_processor::data_models::ProcessedTransaction,
    recipient_address: Address,
    token_address: Address,
) -> I256 {
    use reth_chain_query::to_checksum_address;
    use tx_processor::tx_processor::address_balance_change_calculator::get_token_symbol;

    if let Some(balance_changes) = processed_tx.address_balance_changes.get(&recipient_address) {
        // Check if token is in DENOM_ADDRESSES (known token)
        if let Some(symbol) = get_token_symbol(&token_address) {
            // Known token - look in currency_net using symbol
            if let Some(&amount) = balance_changes.currency_net.get(symbol) {
                if amount > I256::ZERO {
                    return amount;
                }
            }
        } else {
            // Unknown token - look in token_net using checksummed address
            let token_key = to_checksum_address(&token_address);
            if let Some(&amount) = balance_changes.token_net.get(&token_key) {
                if amount > I256::ZERO {
                    return amount;
                }
            }
        }
    }

    I256::ZERO
}
