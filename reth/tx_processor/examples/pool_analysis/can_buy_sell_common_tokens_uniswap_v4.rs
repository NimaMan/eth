use alloy_primitives::U256;
/// Uniswap V4 Trading Viability Analysis
///
/// Iterates over the canonical Uniswap V4 pool catalog defined in
/// `reth_chain_query::common_addresses::dex_token_sets` and validates that
/// each pool can execute a buy → approve → sell sequence using the Baygus
/// router configuration shipped with the project.
use eyre::Result;
use reth_chain_query::common_addresses::{uniswap_v4_pools, UniswapV4PoolInfo};
use std::sync::Arc;
use tx_processor::simulator::types::UniswapV4PoolConfig;
use tx_processor::simulator::{
    check_can_buy_sell_pool, PoolBuySellParameters, PoolBuySellSimulationResult, PoolType,
};
use tx_processor::tx_processor::TxProcessor;
use tx_simulator::TxSimulator;

const TEST_AMOUNT_WEI: u128 = 10_000_000_000_000_000; // 0.01 ETH

#[derive(Debug, Clone)]
struct PoolTestCase {
    info: UniswapV4PoolInfo,
}

impl PoolTestCase {
    fn new(info: &UniswapV4PoolInfo) -> Self {
        Self { info: info.clone() }
    }

    fn pool_label(&self) -> String {
        format!(
            "{} / {} (fee: {} bps, tick spacing: {})",
            self.info.symbol, "WETH", self.info.fee, self.info.tick_spacing
        )
    }

    fn build_simulation_params(&self, block_number: u64) -> PoolBuySellParameters {
        let v4_config = UniswapV4PoolConfig {
            pool_manager: self.info.pool_manager,
            pool_id: self.info.pool_id,
            currency0: self.info.token_address,
            currency1: self.info.denom_address,
            fee: self.info.fee,
            tick_spacing: self.info.tick_spacing,
            hooks: self.info.hooks,
            hook_data: Vec::new(),
        };

        PoolBuySellParameters::new(
            self.info.token_address,
            self.info.pool_manager,
            PoolType::UniswapV4,
        )
        .with_test_amount(U256::from(TEST_AMOUNT_WEI))
        .with_denom_address(self.info.denom_address)
        .with_denom_decimals(self.info.denom_decimals)
        .with_token_decimals(self.info.token_decimals)
        .with_block(block_number)
        .with_uniswap_v4_config(v4_config)
    }
}

fn format_eth_amount(wei: U256) -> String {
    if wei == U256::ZERO {
        return "-".into();
    }

    let eth_str = wei.to_string();
    if eth_str.len() <= 18 {
        let padded = format!("{:0>18}", eth_str);
        return format!("0.{}E", &padded[..4]);
    }

    let (whole, decimal) = eth_str.split_at(eth_str.len() - 18);
    format!("{}.{}E", whole, &decimal[..4.min(decimal.len())])
}

fn print_summary_table(results: &[(PoolTestCase, Result<PoolBuySellSimulationResult>)]) {
    println!("\n📊 Summary Table");
    println!("================================================================================");
    println!(
        "Pool                               | Tradeable | Buy ✓ | Approve ✓ | Sell ✓ | Buy Tax | Sell Tax | ETH Back | Failure"
    );
    println!("--------------------------------------------------------------------------------");

    for (case, result) in results {
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
                    .take(24)
                    .collect::<String>();

                println!(
                    "{:<34} | {:<9} | {:<5} | {:<9} | {:<6} | {:<7} | {:<8} | {:<8} | {:<24}",
                    case.pool_label(),
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
            Err(err) => {
                println!(
                    "{:<34} | ❌        | ❌    | ❌        | ❌     | -       | -        | -        | Error: {}",
                    case.pool_label(),
                    err.to_string()
                        .chars()
                        .take(24)
                        .collect::<String>()
                );
            }
        }
    }

    println!("================================================================================");
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🦄 Uniswap V4 Multi-Pool Trading Viability Analysis");
    println!("===================================================");
    println!("Validating canonical V4 pools with a 0.01 ETH test amount\n");

    let pools = uniswap_v4_pools();
    if pools.is_empty() {
        return Err(eyre::eyre!(
            "No canonical Uniswap V4 pools configured in dex_token_sets"
        ));
    }

    let pool_cases: Vec<PoolTestCase> = pools.iter().map(PoolTestCase::new).collect();

    let reth_datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());
    println!("🔧 Using Reth datadir: {}", reth_datadir);

    let simulator = Arc::new(TxSimulator::new(&reth_datadir)?);
    let tx_processor = Arc::new(TxProcessor::new());

    let latest_block = simulator.get_latest_block()?;
    println!("🧱 Block height: {}", latest_block);
    println!("📦 Pools to test: {}", pool_cases.len());
    println!("💰 Test amount: 0.01 ETH\n");

    let mut results = Vec::new();

    for (idx, case) in pool_cases.iter().enumerate() {
        println!(
            "[{}/{}] Testing {}",
            idx + 1,
            pool_cases.len(),
            case.pool_label()
        );

        let params = case.build_simulation_params(latest_block);
        let result = check_can_buy_sell_pool(simulator.clone(), tx_processor.clone(), params).await;

        match &result {
            Ok(res) => {
                println!(
                    "    ✅ tradeable={}, buy={}, approve={}, sell={}, buy_tax={:.2}%, sell_tax={:.2}%",
                    res.is_tradeable, res.can_buy, res.can_approve, res.can_sell, res.buy_tax_percent, res.sell_tax_percent
                );
            }
            Err(err) => {
                println!("    ❌ simulation error: {err}");
            }
        }

        results.push((case.clone(), result));
        println!();
    }

    print_summary_table(&results);
    Ok(())
}
