use std::str::FromStr;
use std::sync::Arc;

use alloy_primitives::{I256, U256};
use eth_price::core::{RawChainPrice, Token};
use eth_price::shared_simulator::get_or_create_simulator_with_provider;
use reth_chain_query::common_addresses::get_address_by_name;
use reth_chain_query::dex::compute_uniswap_v2_pool;
use reth_chain_query::provider_factory_from_datadir;
use reth_provider::AccountReader;
use tx_processor::trade_simulation::{check_can_buy_sell_pool, PoolBuySellParameters, PoolType};
use tx_processor::tx_processor::TxProcessor;
use tx_simulator::tx_builders::uniswap_v4::build_weth_deposit_tx;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Shared provider factory from local reth datadir
    let provider_factory = provider_factory_from_datadir("/home/nima/.local/share/reth/mainnet")?;

    // Shared simulator/tx processor
    let simulator = get_or_create_simulator_with_provider(provider_factory.clone())?;
    let tx_processor = Arc::new(TxProcessor::new());

    // Common addresses
    let weth = get_address_by_name("WETH").expect("WETH address not found");
    let usdc = get_address_by_name("USDC").expect("USDC address not found");
    let buyer =
        get_address_by_name("Nima_eth_1").expect("Configure 'Nima_eth_1' in common_addresses");

    // Target pool & trade size (1 WETH worth of USDC)
    let pool = compute_uniswap_v2_pool(weth, usdc);
    let one_weth = U256::from_str("1000000000000000000")?;

    // Simulate a WETH deposit so the buyer has spendable WETH in this block.
    let block_number = simulator.get_latest_block()?;
    let provider = provider_factory.provider()?;
    let mut chain = simulator.start_simulation_chain(Some(block_number)).await?;

    let mut deposit_tx = build_weth_deposit_tx(buyer, weth, one_weth);
    let buyer_nonce = provider
        .basic_account(&buyer)?
        .map(|account| account.nonce)
        .unwrap_or(0);
    deposit_tx.nonce = Some(buyer_nonce);
    // Slightly bump gas limit for safety
    deposit_tx.gas = Some(150_000);
    let deposit_result = chain.step_with_trace(deposit_tx.clone()).await?;
    if !deposit_result.success {
        println!(
            "WETH deposit failed: {}",
            deposit_result
                .revert_reason
                .as_deref()
                .unwrap_or("Unknown revert")
        );
        return Ok(());
    }

    let deposit_processed = tx_processor
        .process_transaction_from_simulation_result(&deposit_tx, &deposit_result, block_number, 0)
        .await?;

    // Configure buy+sell simulation on Uniswap V2 (WETH → USDC → WETH)
    let params = PoolBuySellParameters::new(usdc, pool, PoolType::UniswapV2)
        .with_test_amount(one_weth)
        .with_buyer(buyer)
        .with_denom_address(weth)
        .with_denom_decimals(18)
        .with_token_decimals(6)
        .with_block(block_number)
        .with_prior_transactions(vec![deposit_processed]);

    let result = check_can_buy_sell_pool(simulator, tx_processor.clone(), params).await?;
    if !result.can_buy {
        println!(
            "Buy leg failed: {}",
            result.failure_reason.as_deref().unwrap_or("Unknown revert")
        );
        return Ok(());
    }

    let base_token = Token::new(weth, "WETH".to_string(), 18);
    let quote_token = Token::new(usdc, "USDC".to_string(), 6);

    // Inspect precise deltas from the processed transactions
    let buy_changes = result.buy_transaction.address_balance_changes.get(&buyer);
    let usdc_acquired = buy_changes
        .and_then(|c| c.currency_net.get("USDC"))
        .filter(|delta| **delta > I256::ZERO)
        .map(|delta| delta.unsigned_abs())
        .unwrap_or(result.tokens_received);
    let eth_spent = buy_changes
        .and_then(|c| c.currency_net.get("ETH"))
        .filter(|delta| **delta < I256::ZERO)
        .map(|delta| delta.unsigned_abs())
        .unwrap_or(one_weth);

    if eth_spent > U256::ZERO {
        let ask = RawChainPrice::new(usdc_acquired, eth_spent);
        println!(
            "Ask price: {}",
            ask.to_human_string(&base_token, &quote_token, 6)
        );
    } else {
        println!("Ask leg executed but ETH spend could not be determined");
    }

    let sell_changes = result.sell_transaction.address_balance_changes.get(&buyer);
    let eth_received = sell_changes
        .and_then(|c| c.currency_net.get("ETH"))
        .filter(|delta| **delta > I256::ZERO)
        .map(|delta| delta.unsigned_abs())
        .unwrap_or(U256::ZERO);

    if result.can_sell && eth_received > U256::ZERO {
        let bid = RawChainPrice::new(eth_received, usdc_acquired);
        println!(
            "Bid price: {}",
            bid.to_human_string(&quote_token, &base_token, 6)
        );
        println!("Sell tax: {:.4}%", result.sell_tax_percent);
    } else if !result.can_sell {
        println!(
            "Sell leg not executable: {}",
            result
                .failure_reason
                .as_deref()
                .unwrap_or("No failure reason recorded")
        );
    } else {
        println!("Sell leg executed but returned zero ETH output");
    }

    println!("Block number: {}", result.block_number);
    println!("Buy tax: {:.4}%", result.buy_tax_percent);
    println!("USDC acquired (raw): {}", usdc_acquired);
    println!("ETH spent (wei): {}", eth_spent);
    println!("ETH received (wei): {}", eth_received);

    Ok(())
}
