use alloy_primitives::U256;
/// Uniswap V4 Trading Viability Analysis
///
/// Iterates over the canonical Uniswap V4 pool catalog defined in
/// `reth_chain_query::common_addresses::dex_token_denom_pairs` and validates that
/// each pool can execute a buy → Permit2 approve → sell sequence using the
/// deployed Uniswap Universal Router.
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
            self.info.symbol, self.info.denom_symbol, self.info.fee, self.info.tick_spacing
        )
    }

    fn selected_block(&self, latest_block: u64) -> u64 {
        self.info.block_hint.unwrap_or(latest_block)
    }

    fn build_simulation_params(&self, block_number: u64) -> PoolBuySellParameters {
        let v4_config = UniswapV4PoolConfig {
            pool_manager: self.info.pool_manager,
            pool_id: self.info.pool_id,
            currency0: self.info.currency0,
            currency1: self.info.currency1,
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

fn format_token_amount(amount: U256, decimals: u8) -> String {
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

fn print_summary_table(results: &[(PoolTestCase, Result<PoolBuySellSimulationResult>)]) {
    println!("\n📊 Summary Table");
    println!("================================================================================");
    println!(
        "Pool                               | Block    | Tradeable | Buy ✓ | Approve ✓ | Sell ✓ | Buy Tax | Sell Tax | Denom Back | Failure"
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
                let denom_back = format_token_amount(res.denom_received, case.info.denom_decimals);
                let failure = res
                    .failure_reason
                    .as_deref()
                    .unwrap_or("-")
                    .chars()
                    .take(24)
                    .collect::<String>();

                println!(
                    "{:<34} | {:<8} | {:<9} | {:<5} | {:<9} | {:<6} | {:<7} | {:<8} | {:<10} | {:<24}",
                    case.pool_label(),
                    res.block_number,
                    tradeable,
                    can_buy,
                    can_approve,
                    can_sell,
                    buy_tax,
                    sell_tax,
                    denom_back,
                    failure
                );
            }
            Err(err) => {
                println!(
                    "{:<34} | -        | ❌        | ❌    | ❌        | ❌     | -       | -        | -          | Error: {}",
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
            "No canonical Uniswap V4 pools configured in dex_token_denom_pairs"
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
            "[{}/{}] Testing {} at block {}",
            idx + 1,
            pool_cases.len(),
            case.pool_label(),
            case.selected_block(latest_block)
        );

        let params = case.build_simulation_params(case.selected_block(latest_block));
        let result = check_can_buy_sell_pool(simulator.clone(), tx_processor.clone(), params).await;

        match &result {
            Ok(res) => {
                println!(
                    "    ✅ tradeable={}, buy={}, approve={}, sell={}, buy_tax={:.2}%, sell_tax={:.2}%",
                    res.is_tradeable, res.can_buy, res.can_approve, res.can_sell, res.buy_tax_percent, res.sell_tax_percent
                );
                println!(
                    "    ℹ️ block={}, tokens_received={} {}, denom_spent={} {}, denom_received={} {}",
                    res.block_number,
                    format_token_amount(res.tokens_received, case.info.token_decimals),
                    case.info.symbol,
                    format_token_amount(res.denom_spent, case.info.denom_decimals),
                    case.info.denom_symbol,
                    format_token_amount(res.denom_received, case.info.denom_decimals),
                    case.info.denom_symbol
                );
                if let Some(reason) = &res.failure_reason {
                    println!("    ⚠️ failure_reason: {reason}");
                }
                println!(
                    "    ℹ️ buy_tx.to={:?}, approve_tx.to={:?}, sell_tx.to={:?}",
                    res.buy_transaction.to_address,
                    res.approve_transaction.to_address,
                    res.sell_transaction.to_address
                );
                println!(
                    "    ℹ️ prior_tx_count={}, prior_targets={:?}",
                    res.prior_transactions.len(),
                    res.prior_transactions
                        .iter()
                        .map(|tx| tx.to_address)
                        .collect::<Vec<_>>()
                );
                println!(
                    "    ℹ️ prior_statuses={:?}",
                    res.prior_transactions
                        .iter()
                        .map(|tx| tx.status)
                        .collect::<Vec<_>>()
                );
                println!(
                    "    ℹ️ prior_contracts={:?}",
                    res.prior_transactions
                        .iter()
                        .map(|tx| tx.contract_address)
                        .collect::<Vec<_>>()
                );
                println!(
                    "    ℹ️ buy_struct_logs_present={}",
                    res.buy_transaction.struct_logs.is_some()
                );
                println!(
                    "    ℹ️ buy_internal_calls={:?}",
                    res.buy_transaction
                        .internal_transactions
                        .iter()
                        .map(|it| (it.from_address, it.to_address, it.error.clone()))
                        .collect::<Vec<_>>()
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
