use alloy_primitives::U256;
use eyre::Result;
use reth_chain_query::common_addresses::uniswap_v4_pools;
use std::sync::Arc;
use tx_processor::simulator::types::UniswapV4PoolConfig;
use tx_processor::simulator::{check_can_buy_sell_pool, PoolBuySellParameters, PoolType};
use tx_processor::tx_processor::TxProcessor;
use tx_simulator::TxSimulator;

const TEST_AMOUNT_WEI: u128 = 10_000_000_000_000_000; // 0.01 ETH

#[tokio::main]
async fn main() -> Result<()> {
    println!("🦄 Uniswap V4 Buy Swap Simulation");
    println!("=================================\n");

    let pools = uniswap_v4_pools();
    if pools.is_empty() {
        return Err(eyre::eyre!(
            "No canonical Uniswap V4 pools configured in dex_token_sets"
        ));
    }

    let pool = &pools[0];
    println!("Target pool : {} / {}", pool.symbol, pool.denom_symbol);

    let reth_datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());
    println!("Reth datadir: {}", reth_datadir);

    let simulator = Arc::new(TxSimulator::new(&reth_datadir)?);
    let tx_processor = Arc::new(TxProcessor::new());

    let resolved_block = match pool.block_hint {
        Some(block) => {
            println!("Using block hint : {}", block);
            block
        }
        None => {
            let latest = simulator.get_latest_block()?;
            println!("Latest snapshot : {}", latest);
            latest
        }
    };

    let v4_config = UniswapV4PoolConfig {
        pool_manager: pool.pool_manager,
        pool_id: pool.pool_id,
        currency0: pool.token_address,
        currency1: pool.denom_address,
        fee: pool.fee,
        tick_spacing: pool.tick_spacing,
        hooks: pool.hooks,
        hook_data: Vec::new(),
    };

    let params = PoolBuySellParameters::new(pool.token_address, pool.pool_manager, PoolType::UniswapV4)
        .with_test_amount(U256::from(TEST_AMOUNT_WEI))
        .with_denom_address(pool.denom_address)
        .with_denom_decimals(pool.denom_decimals)
        .with_token_decimals(pool.token_decimals)
        .with_block(resolved_block)
        .with_uniswap_v4_config(v4_config);

    let result =
        check_can_buy_sell_pool(simulator.clone(), tx_processor.clone(), params).await;

    match result {
        Ok(res) => {
            println!("\nSimulation block : {}", res.block_number);
            println!(
                "Buy outcome      : {}",
                if res.can_buy { "✅ success" } else { "❌ failed" }
            );
            println!(
                "Tokens received  : {} {}",
                format_amount(res.tokens_received, pool.token_decimals),
                pool.symbol
            );
            println!(
                "Denom spent      : {} {}",
                format_amount(res.denom_spent, pool.denom_decimals),
                pool.denom_symbol
            );

            if let Some(reason) = res.failure_reason {
                println!("\n⚠️ Failure reason: {reason}");
            } else {
                println!("\nNo failure reason recorded (buy leg succeeded).");
            }

            println!("\nRaw buy transaction hash: {:#x}", res.buy_transaction.hash);
        }
        Err(err) => {
            println!("\n❌ Simulation error: {err:?}");
        }
    }

    Ok(())
}

fn format_amount(amount: U256, decimals: u8) -> String {
    if amount.is_zero() {
        return "0".to_string();
    }

    let digits = amount.to_string();
    let decimals = decimals as usize;

    if decimals == 0 {
        return digits;
    }

    if digits.len() <= decimals {
        let padded = format!("{:0>width$}", digits, width = decimals + 1);
        let split = padded.len() - decimals;
        let (whole, frac) = padded.split_at(split);
        let frac_trimmed = frac.trim_end_matches('0');
        if frac_trimmed.is_empty() {
            whole.to_string()
        } else {
            format!("{whole}.{frac_trimmed}")
        }
    } else {
        let split = digits.len() - decimals;
        let (whole, frac) = digits.split_at(split);
        let frac_trimmed = frac.trim_end_matches('0');
        if frac_trimmed.is_empty() {
            whole.to_string()
        } else {
            format!("{whole}.{frac_trimmed}")
        }
    }
}
