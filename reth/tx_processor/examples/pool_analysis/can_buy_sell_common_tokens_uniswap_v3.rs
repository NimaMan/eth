use alloy_primitives::{Address, Bytes, U256};
/// UniswapV3 Token Trading Viability Analysis
///
/// Tests multiple popular tokens on UniswapV3 pools with different fee tiers
/// to verify buy→approve→sell sequences work correctly with concentrated liquidity.
use eyre::{eyre, Result};
use reth_chain_query::common_addresses::dex_pools::{
    compute_uniswap_v3_pool, UNISWAP_V3_FACTORY,
};
use std::sync::Arc;
use tx_processor::simulator::{
    check_can_buy_sell_pool, PoolBuySellParameters, PoolBuySellSimulationResult, PoolType,
};
use tx_processor::tx_processor::TxProcessor;
use tx_simulator::TxSimulator;

#[derive(Debug, Clone)]
pub struct TokenConfig {
    pub symbol: &'static str,
    pub token_address: Address,
    pub denom_address: Address,
    pub fee_tier: u32, // V3 fee tier in basis points (500, 3000, 10000)
    pub expected_behavior: ExpectedBehavior,
    pub decimals: u8,
}

#[derive(Debug, Clone)]
pub enum ExpectedBehavior {
    ShouldWork,
    MightFail(&'static str),
}

impl TokenConfig {
    fn get_fee_tier_string(&self) -> &'static str {
        match self.fee_tier {
            500 => "0.05%",
            3000 => "0.30%",
            10000 => "1.00%",
            _ => "Unknown",
        }
    }
}

#[path = "token_sets/uniswap_v3.rs"]
mod uniswap_v3_tokens;

async fn test_token(
    simulator: Arc<TxSimulator>,
    tx_processor: Arc<TxProcessor>,
    token: &TokenConfig,
    pool_address: Address,
    block_number: u64,
) -> Result<PoolBuySellSimulationResult> {
    let config = PoolBuySellParameters::new(
        token.token_address,
        pool_address,
        PoolType::UniswapV3 {
            fee_tier: token.fee_tier,
        },
    )
    .with_test_amount(U256::from(10_000_000_000_000_000u64)) // 0.01 ETH
    .with_denom_address(token.denom_address)
    .with_token_decimals(token.decimals)
    .with_block(block_number);

    check_can_buy_sell_pool(simulator, tx_processor, config).await
}

fn format_eth_amount(wei: U256) -> String {
    if wei == U256::ZERO {
        return "-".to_string();
    }

    let eth_str = wei.to_string();
    if eth_str.len() <= 18 {
        let padded = format!("{:0>18}", eth_str);
        let whole = "0";
        let decimal = &padded[0..4];
        return format!("{}.{}E", whole, decimal);
    }

    let (whole, decimal) = eth_str.split_at(eth_str.len() - 18);
    format!("{}.{}E", whole, &decimal[0..4.min(decimal.len())])
}

fn print_summary_table(results: &[(TokenConfig, Address, Result<PoolBuySellSimulationResult>)]) {
    println!("\n📊 Summary Table:");
    println!("================================================================================");
    println!(
        "Token  | Fee Tier | Tradeable | Buy ✓ | Approve ✓ | Sell ✓ | Buy Tax | Sell Tax | ETH Back | Failure"
    );
    println!("--------------------------------------------------------------------------------");

    for (token, _pool, result) in results {
        match result {
            Ok(res) => {
                let tradeable = if res.is_tradeable { "✅" } else { "❌" };
                let can_buy = if res.can_buy { "✅" } else { "❌" };
                let can_approve = if res.can_approve { "✅" } else { "❌" };
                let can_sell = if res.can_sell { "✅" } else { "❌" };
                let buy_tax = format!("{:.1}%", res.buy_tax_percent);
                let sell_tax = format!("{:.1}%", res.sell_tax_percent);
                let eth_back = format_eth_amount(res.denom_received);
                let failure = res
                    .failure_reason
                    .as_deref()
                    .unwrap_or("-")
                    .chars()
                    .take(20)
                    .collect::<String>();

                println!(
                    "{:<6} | {:<8} | {:<9} | {:<5} | {:<9} | {:<6} | {:<7} | {:<8} | {:<8} | {:<20}",
                    token.symbol,
                    token.get_fee_tier_string(),
                    tradeable,
                    can_buy,
                    can_approve,
                    can_sell,
                    buy_tax,
                    sell_tax,
                    eth_back,
                    failure
                );
            }
            Err(e) => {
                println!(
                    "{:<6} | {:<8} | ❌        | ❌    | ❌        | ❌     | -       | -        | -        | Error: {}",
                    token.symbol,
                    token.get_fee_tier_string(),
                    e.to_string().chars().take(20).collect::<String>()
                );
            }
        }
    }

    println!("================================================================================");
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🦄 Uniswap V3 Multi-Token Trading Viability Analysis");
    println!("====================================================");
    println!("Testing popular tokens across different V3 fee tiers");
    println!();

    let tokens = uniswap_v3_tokens::token_configs();

    // Get reth datadir
    let reth_datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());

    println!("🔧 Initializing components...");
    println!("  Data directory: {}", reth_datadir);

    let simulator = Arc::new(TxSimulator::new(&reth_datadir)?);
    let tx_processor = Arc::new(TxProcessor::new());

    let latest_block = simulator.get_latest_block()?;
    println!("  Using block: {}", latest_block);
    println!();

    println!("🧪 Testing {} tokens on Uniswap V3...", tokens.len());
    println!("  Test amount: 0.01 ETH");
    println!("  Fee tiers: 0.05% (500), 0.30% (3000), 1.00% (10000)");
    println!();

    let start_time = std::time::Instant::now();
    let mut results = Vec::new();

    for (i, token) in tokens.iter().enumerate() {
        print!(
            "  [{}/{}] Testing {} ({} tier)... ",
            i + 1,
            tokens.len(),
            token.symbol,
            token.get_fee_tier_string()
        );

        let pool_address =
            compute_uniswap_v3_pool(token.token_address, token.denom_address, token.fee_tier);

        if let Err(err) =
            ensure_v3_pool_exists(simulator.clone(), token, pool_address, latest_block).await
        {
            println!("❌ {}", err);
            results.push((token.clone(), pool_address, Err(err)));
            continue;
        }

        let result = test_token(
            simulator.clone(),
            tx_processor.clone(),
            token,
            pool_address,
            latest_block,
        )
        .await;

        match &result {
            Ok(res) if res.is_tradeable => println!("✅ Tradeable"),
            Ok(res) => {
                println!("❌ Not tradeable");
                if let Some(ref reason) = res.failure_reason {
                    println!("    Full error: {}", reason);
                }
            }
            Err(e) => println!("❌ Error: {}", e),
        }

        println!("    Pool address: {}", pool_address);
        println!("    Denom: {}", token.denom_address);

        results.push((token.clone(), pool_address, result));
    }

    let elapsed = start_time.elapsed();

    print_summary_table(&results);

    println!("\n📈 Statistics by Fee Tier:");
    println!("================================");

    for fee_tier in [500, 3000, 10000] {
        let tier_results: Vec<_> = results
            .iter()
            .filter(|(t, _, _)| t.fee_tier == fee_tier)
            .collect();

        if tier_results.is_empty() {
            continue;
        }

        let tradeable_count = tier_results
            .iter()
            .filter(|(_, _, r)| r.as_ref().map(|res| res.is_tradeable).unwrap_or(false))
            .count();

        let tier_name = match fee_tier {
            500 => "0.05%",
            3000 => "0.30%",
            10000 => "1.00%",
            _ => "Unknown",
        };

        println!(
            "  {} tier: {}/{} tradeable ({:.1}%)",
            tier_name,
            tradeable_count,
            tier_results.len(),
            (tradeable_count as f64 / tier_results.len() as f64) * 100.0
        );
    }

    println!("\n⏱️  Performance Summary:");
    println!("  Total Testing Time: {:?}", elapsed);
    println!("  Average Per Token: {:?}", elapsed / tokens.len() as u32);

    let mut success_count = 0;
    let mut failure_count = 0;
    let mut error_count = 0;

    for (token, _, result) in &results {
        match result {
            Ok(res) if res.is_tradeable => success_count += 1,
            Ok(res) => {
                failure_count += 1;
                if let Some(reason) = &res.failure_reason {
                    println!("  ⚠️  {} flagged non-tradeable: {}", token.symbol, reason);
                }
            }
            Err(err) => {
                error_count += 1;
                println!("  💥 {} errored: {}", token.symbol, err);
            }
        }
    }

    println!("\n🔧 Component Validation Status:");
    if success_count == results.len() {
        println!("  ✅ V3 pool adapter working correctly");
        println!("  ✅ Fee tier handling validated");
        println!("  ✅ Concentrated liquidity simulation functional");
    } else {
        println!("  ✅ V3 pool adapter exercised across tiers");
        if failure_count > 0 {
            println!("  📝 {} tokens reported non-tradeable", failure_count);
        }
        if error_count > 0 {
            println!("  📝 {} tokens returned execution errors", error_count);
        }
    }

    println!("\n🎉 Uniswap V3 multi-token analysis complete!");

    Ok(())
}

async fn ensure_v3_pool_exists(
    simulator: Arc<TxSimulator>,
    token: &TokenConfig,
    pool_address: Address,
    block_number: u64,
) -> Result<()> {
    if pool_address.is_zero() {
        return Err(eyre!(
            "No Uniswap V3 pool found for {} at fee tier {}",
            token.symbol,
            token.get_fee_tier_string()
        ));
    }

    let view_data =
        encode_get_pool_call(token.token_address, token.denom_address, token.fee_tier);
    let response = simulator
        .simulate_view_function(UNISWAP_V3_FACTORY, view_data, Some(block_number), None)
        .await?;

    if !response.success || response.output.len() < 32 {
        return Err(eyre!(
            "No Uniswap V3 pool found for {} at fee tier {} (factory returned zero address)",
            token.symbol,
            token.get_fee_tier_string()
        ));
    }

    let mut buf = [0u8; 20];
    buf.copy_from_slice(&response.output[12..32]);
    let resolved = Address::from(buf);

    if resolved.is_zero() {
        return Err(eyre!(
            "No Uniswap V3 pool found for {} at fee tier {} (factory returned zero address)",
            token.symbol,
            token.get_fee_tier_string()
        ));
    }

    if resolved != pool_address {
        return Err(eyre!(
            "Computed pool address {} does not match factory getPool result {} for {} at fee tier {}",
            pool_address,
            resolved,
            token.symbol,
            token.get_fee_tier_string()
        ));
    }

    Ok(())
}

fn encode_get_pool_call(token_a: Address, token_b: Address, fee: u32) -> Bytes {
    let mut data = vec![0x16, 0x98, 0xee, 0x82];
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(token_a.as_slice());
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(token_b.as_slice());
    data.extend_from_slice(&[0u8; 29]);
    let fee_bytes = fee.to_be_bytes();
    data.extend_from_slice(&fee_bytes[1..]);
    Bytes::from(data)
}
