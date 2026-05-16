use alloy_primitives::{address, Address, U256};
/// PancakeSwap V3 Token Trading Viability Analysis
///
/// Tests buy→approve→sell sequences on PancakeSwap V3 pools using
/// deterministic pool address computation with the Ethereum deployer.
use eyre::Result;
use reth_chain_query::dex::compute_pancakeswap_v3_pool;
use std::sync::Arc;
use tx_processor::trade_simulation::{
    check_can_buy_sell_pool, PoolBuySellParameters, PoolBuySellSimulationResult, PoolType,
};
use tx_processor::tx_processor::TxProcessor;
use tx_simulator::TxSimulator;

#[derive(Debug, Clone)]
pub struct TokenConfig {
    pub symbol: &'static str,
    pub token_address: Address,
    pub denom_address: Address,
    pub fee_tier: u32,
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
            100 => "0.01%",
            500 => "0.05%",
            2500 => "0.25%",
            10000 => "1.00%",
            _ => "Unknown",
        }
    }
}

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
        PoolType::PancakeSwapV3 {
            fee_tier: token.fee_tier,
        },
    )
    .with_test_amount(U256::from(10_000_000_000_000_000u64)) // 0.01 ETH
    .with_denom_address(token.denom_address)
    .with_denom_decimals(18)
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
        return format!("0.{}E", &padded[0..4]);
    }
    let (whole, decimal) = eth_str.split_at(eth_str.len() - 18);
    format!("{}.{}E", whole, &decimal[0..4.min(decimal.len())])
}

fn print_summary_table(results: &[(TokenConfig, Address, Result<PoolBuySellSimulationResult>)]) {
    println!("\n📊 PancakeSwap V3 Summary Table:");
    println!("================================================================================");
    println!(
        "Token     | Fee Tier | Tradeable | Buy ✓ | Approve ✓ | Sell ✓ | Buy Tax | Sell Tax | ETH Back | Failure"
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
                let failure = res.failure_reason.as_deref().unwrap_or("-");
                println!(
                    "{:<9} | {:>8} | {:>9} | {:>5} | {:>9} | {:>6} | {:>7} | {:>8} | {:>8} | {}",
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
                    "{:<9} | {:>8} | {:>9} | {:>5} | {:>9} | {:>6} | {:>7} | {:>8} | {:>8} | Error: {}",
                    token.symbol,
                    token.get_fee_tier_string(),
                    "💥",
                    "-",
                    "-",
                    "-",
                    "-",
                    "-",
                    "-",
                    e
                );
            }
        }
    }
    println!("================================================================================");
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🥞 PancakeSwap V3 Trading Viability Analysis");
    println!("=============================================\n");

    let reth_datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());
    println!("Using Reth datadir: {}", reth_datadir);

    let simulator = Arc::new(TxSimulator::new(&reth_datadir)?);
    let tx_processor = Arc::new(TxProcessor::new());
    let latest_block = simulator.get_latest_block()?;
    println!("Latest block: {}\n", latest_block);

    let tokens = vec![
        // Verified on-chain: https://dexscreener.com/ethereum/0x1ac1a8feaaea1900c4166deeed0c11cc10669d36
        TokenConfig {
            symbol: "WETH-USDC",
            token_address: address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),
            denom_address: address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"),
            fee_tier: 500,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 6,
        },
        // Verified on-chain: https://dexscreener.com/ethereum/0x1645ca2363ff04fddc6c8b0e8be1c3f773fe6a0d
        TokenConfig {
            symbol: "HASH-USDT",
            token_address: address!("AC7b5d06fa1e77D08aea40d46cb7C5923A87A0cc"),
            denom_address: address!("dAC17F958D2ee523a2206206994597C13D831ec7"),
            fee_tier: 10000,
            expected_behavior: ExpectedBehavior::MightFail("meme/low liquidity"),
            decimals: 18,
        },
    ];

    let mut results = Vec::new();

    for token in &tokens {
        let pool_address =
            compute_pancakeswap_v3_pool(token.token_address, token.denom_address, token.fee_tier);

        println!(
            "🔍 Testing {} @ {} (pool: {})",
            token.symbol,
            token.get_fee_tier_string(),
            pool_address
        );

        match test_token(
            simulator.clone(),
            tx_processor.clone(),
            token,
            pool_address,
            latest_block,
        )
        .await
        {
            Ok(result) => {
                println!(
                    "   → tradeable={} buy={} approve={} sell={} | buy_tax={:.2}% sell_tax={:.2}%",
                    result.is_tradeable,
                    result.can_buy,
                    result.can_approve,
                    result.can_sell,
                    result.buy_tax_percent,
                    result.sell_tax_percent
                );
                results.push((token.clone(), pool_address, Ok(result)));
            }
            Err(e) => {
                println!("   → ERROR: {}", e);
                results.push((token.clone(), pool_address, Err(e)));
            }
        }

        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    }

    print_summary_table(&results);
    Ok(())
}
