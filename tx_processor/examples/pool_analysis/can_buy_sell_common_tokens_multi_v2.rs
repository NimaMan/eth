use alloy_primitives::{address, Address, U256};
/// Multi-DEX V2 Token Trading Viability Simulation
///
/// Tests buy/sell viability for tokens on PancakeSwap V2, ShibaSwap V2,
/// and Fraxswap V2 using deterministic pool address computation.
use eyre::Result;
use reth_chain_query::dex::{
    compute_fraxswap_v2_pool, compute_pancakeswap_v2_pool, compute_shibaswap_v2_pool,
};
use std::sync::Arc;
use tx_processor::simulator::{
    check_can_buy_sell_pool, PoolBuySellParameters, PoolBuySellSimulationResult, PoolType,
};
use tx_processor::tx_processor::TxProcessor;
use tx_simulator::TxSimulator;

#[derive(Debug, Clone)]
pub struct TokenConfig {
    symbol: &'static str,
    token_address: Address,
    denom_address: Address,
    pool_type: PoolType,
    decimals: u8,
}

#[derive(Debug)]
struct TokenTestResult {
    config: TokenConfig,
    pool_address: Address,
    result: Option<PoolBuySellSimulationResult>,
    error: Option<String>,
    test_duration: std::time::Duration,
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🎯 Multi-DEX V2 Trading Viability Analysis");
    println!("============================================\n");

    let reth_datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());
    println!("Using Reth datadir: {}", reth_datadir);

    let simulator = Arc::new(TxSimulator::new(&reth_datadir)?);
    let tx_processor = Arc::new(TxProcessor::new());
    let latest_block = simulator.get_latest_block()?;
    println!("Latest block: {}\n", latest_block);

    let token_configs = token_configs();
    let total_tokens = token_configs.len();
    let mut all_results = Vec::new();

    println!("📋 Testing {} tokens across PancakeSwap, ShibaSwap, and Fraxswap:", total_tokens);
    for config in &token_configs {
        println!("  {} ({:?})", config.symbol, config.pool_type);
    }
    println!();

    for (idx, config) in token_configs.into_iter().enumerate() {
        println!("{}", "=".repeat(80));
        println!(
            "🔍 TESTING [{}/{}]: {} ({:?})",
            idx + 1,
            total_tokens,
            config.symbol,
            config.pool_type
        );
        println!("{}", "=".repeat(80));

        let test_result = test_single_token(&config, &simulator, &tx_processor, latest_block).await;
        all_results.push(test_result);

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    print_summary(&all_results);
    Ok(())
}

async fn test_single_token(
    config: &TokenConfig,
    simulator: &Arc<TxSimulator>,
    tx_processor: &Arc<TxProcessor>,
    latest_block: u64,
) -> TokenTestResult {
    let start = std::time::Instant::now();

    let pool_address = match resolve_pool_address(config) {
        Ok(addr) => addr,
        Err(e) => {
            return TokenTestResult {
                config: config.clone(),
                pool_address: Address::ZERO,
                result: None,
                error: Some(e),
                test_duration: start.elapsed(),
            };
        }
    };

    println!("  Token: {}", config.token_address);
    println!("  Denom: {}", config.denom_address);
    println!("  Pool:  {}", pool_address);
    println!("  Type:  {:?}", config.pool_type);

    let pool_config = PoolBuySellParameters::new(
        config.token_address,
        pool_address,
        config.pool_type.clone(),
    )
    .with_test_amount(U256::from(1_000_000_000_000_000_000u64))
    .with_denom_address(config.denom_address)
    .with_denom_decimals(18)
    .with_token_decimals(config.decimals)
    .with_block(latest_block);

    match check_can_buy_sell_pool(simulator.clone(), tx_processor.clone(), pool_config).await {
        Ok(result) => {
            let duration = start.elapsed();
            println!(
                "  Result: buy={}, approve={}, sell={}, tradeable={}",
                result.can_buy, result.can_approve, result.can_sell, result.is_tradeable
            );
            if let Some(reason) = &result.failure_reason {
                println!("  Failure: {}", reason);
            }
            TokenTestResult {
                config: config.clone(),
                pool_address,
                result: Some(result),
                error: None,
                test_duration: duration,
            }
        }
        Err(e) => {
            let msg = format!("Analysis failed: {}", e);
            println!("  ❌ {}", msg);
            TokenTestResult {
                config: config.clone(),
                pool_address,
                result: None,
                error: Some(msg),
                test_duration: start.elapsed(),
            }
        }
    }
}

fn resolve_pool_address(config: &TokenConfig) -> Result<Address, String> {
    match config.pool_type {
        PoolType::PancakeSwapV2 => Ok(compute_pancakeswap_v2_pool(
            config.token_address,
            config.denom_address,
        )),
        PoolType::ShibaSwapV2 => Ok(compute_shibaswap_v2_pool(
            config.token_address,
            config.denom_address,
        )),
        PoolType::FraxswapV2 => Ok(compute_fraxswap_v2_pool(
            config.token_address,
            config.denom_address,
        )),
        other => Err(format!(
            "Pool resolution not implemented for pool type {:?}",
            other
        )),
    }
}

fn token_configs() -> Vec<TokenConfig> {
    let weth = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
    let usdc = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");

    vec![
        // PancakeSwap V2 — verified against on-chain pool
        TokenConfig {
            symbol: "CHAIN-PCS",
            token_address: address!("0f9f5E9b76AA03e9Ab1dbd76223FC70A322b55Ad"),
            denom_address: weth,
            pool_type: PoolType::PancakeSwapV2,
            decimals: 18,
        },
        // ShibaSwap V2 — verified against on-chain pool
        TokenConfig {
            symbol: "WETH-USDC-SHIBA",
            token_address: usdc,
            denom_address: weth,
            pool_type: PoolType::ShibaSwapV2,
            decimals: 6,
        },
        // Fraxswap V2 — verified against on-chain pool
        TokenConfig {
            symbol: "WETH-USDC-FRAX",
            token_address: usdc,
            denom_address: weth,
            pool_type: PoolType::FraxswapV2,
            decimals: 6,
        },
    ]
}

fn print_summary(results: &[TokenTestResult]) {
    println!("\n{}", "=".repeat(80));
    println!("📊 RESULTS SUMMARY");
    println!("{}", "=".repeat(80));

    let mut successful = 0;
    let mut failed = 0;
    let mut errors = 0;

    for r in results {
        if let Some(res) = &r.result {
            if res.is_tradeable {
                successful += 1;
            } else {
                failed += 1;
            }
        } else {
            errors += 1;
        }
    }

    println!("  ✅ Tradeable: {}", successful);
    println!("  ❌ Failed:    {}", failed);
    println!("  💥 Errors:    {}", errors);
    println!("  📊 Total:     {}", results.len());

    println!("\n{:<20} | {:<10} | {:<8} | {:<8} | {:<8} | {:<20}",
        "Symbol", "Tradeable", "Buy", "Approve", "Sell", "Error");
    println!("{}", "-".repeat(90));

    for r in results {
        let symbol = r.config.symbol;
        if let Some(res) = &r.result {
            let tradeable = if res.is_tradeable { "✅" } else { "❌" };
            let buy = if res.can_buy { "✅" } else { "❌" };
            let approve = if res.can_approve { "✅" } else { "❌" };
            let sell = if res.can_sell { "✅" } else { "❌" };
            let reason = res.failure_reason.as_deref().unwrap_or("-");
            println!(
                "{:<20} | {:<10} | {:<8} | {:<8} | {:<8} | {}",
                symbol, tradeable, buy, approve, sell, reason
            );
        } else {
            let err = r.error.as_deref().unwrap_or("Unknown");
            println!("{:<20} | {:<10} | {:<8} | {:<8} | {:<8} | {}",
                symbol, "💥 ERROR", "-", "-", "-", err);
        }
    }
}
