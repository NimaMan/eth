//! Scan cross-venue buy→approve→sell across the last N blocks and write CSV.
//! Produces best net PnL (USD-approx) per block for USDC and USDT.
//!
//! Usage:
//!   cargo run -p tx_processor --example cross_venue_usd_series -- [end_block] [n_blocks] [size_eth]
//! Defaults: end_block = latest in DB, n_blocks = 100, size_eth = 1.0

use alloy_primitives::{Address, U256};
use eyre::Result;
use reth_chain_query::common_addresses::{
    compute_sushiswap_pool, compute_uniswap_v2_pool, compute_uniswap_v3_pool, get_address_by_name,
};
use reth_chain_query::tx_builders::amm_swap_route::AmmSwapRoute;
use std::env;
use std::fs;
use std::io::Write;
use std::sync::Arc;
use tx_processor::simulator::simulate_cross_venue_buy_approve_sell;
use tx_processor::tx_processor::TxProcessor;
use tx_processor::ProcessedTxProvider;
use tx_simulator::TxSimulator;

const RETH_DB_PATH: &str = "/home/nima/.local/share/reth/mainnet";

#[tokio::main]
async fn main() -> Result<()> {
    let simulator = Arc::new(TxSimulator::new(RETH_DB_PATH)?);
    let txp = Arc::new(TxProcessor::new());
    let processed = Arc::new(ProcessedTxProvider::with_provider_factory(
        simulator.provider_factory().clone(),
    )?);

    // CLI: [end_block] [n_blocks] [size_eth]
    let args: Vec<String> = env::args().collect();
    let end_block: u64 = if let Some(s) = args.get(1) {
        s.parse().unwrap_or(0)
    } else {
        0
    };
    let n_blocks: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(100);
    let size_eth: f64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1.0);

    // If end_block = 0, use latest
    let end_block = if end_block == 0 {
        processed.get_latest_block().await?
    } else {
        end_block
    };
    let start_block = end_block.saturating_sub(n_blocks.saturating_sub(1));
    let eth_in_wei = U256::from(((size_eth.max(0.0)) * 1e18).round() as u128);

    // Addresses
    let weth = get_address_by_name("WETH").expect("WETH address");
    let usdc = get_address_by_name("USDC").expect("USDC address");
    let usdt = get_address_by_name("USDT").expect("USDT address");

    // Routes to consider
    let routes = vec![
        AmmSwapRoute::UniswapV2 {
            pool: compute_uniswap_v2_pool(weth, usdc),
        },
        AmmSwapRoute::SushiswapV2 {
            pool: compute_sushiswap_pool(weth, usdc),
        },
        AmmSwapRoute::UniswapV3 {
            pool: compute_uniswap_v3_pool(weth, usdc, 500),
            fee_tier: 500,
        },
        AmmSwapRoute::UniswapV3 {
            pool: compute_uniswap_v3_pool(weth, usdc, 3000),
            fee_tier: 3000,
        },
    ];
    let routes_usdt = vec![
        AmmSwapRoute::UniswapV2 {
            pool: compute_uniswap_v2_pool(weth, usdt),
        },
        AmmSwapRoute::SushiswapV2 {
            pool: compute_sushiswap_pool(weth, usdt),
        },
        AmmSwapRoute::UniswapV3 {
            pool: compute_uniswap_v3_pool(weth, usdt, 500),
            fee_tier: 500,
        },
        AmmSwapRoute::UniswapV3 {
            pool: compute_uniswap_v3_pool(weth, usdt, 3000),
            fee_tier: 3000,
        },
    ];

    // Output CSV
    fs::create_dir_all("artifacts").ok();
    let out_path = format!(
        "artifacts/cross_venue_usd_series_{}_{}.csv",
        end_block, n_blocks
    );
    let mut f = fs::File::create(&out_path)?;
    writeln!(f, "block,symbol,base_fee_gwei,size_eth,buy_route,sell_route,pnl_eth_raw,pnl_usd_raw,pnl_eth_exec,pnl_usd_exec,gas_total,eth_back")?;

    println!(
        "Scanning blocks {}..={} (size_eth = {:.6})",
        start_block, end_block, size_eth
    );
    for b in start_block..=end_block {
        let base_fee_wei = processed.get_base_fee_at_block(b).await?;
        let tip_gwei = 0.0_f64; // base fee only
        let gas_price_gwei = (base_fee_wei as f64) / 1e9 + tip_gwei;

        // USDC best
        if let Some(row) = scan_best_for_token(
            simulator.clone(),
            txp.clone(),
            usdc,
            &routes,
            b,
            eth_in_wei,
            size_eth,
            gas_price_gwei,
        )
        .await?
        {
            writeln!(
                f,
                "{b},USDC,{:.6},{:.6},{},{},{:.9},{:.2},{:.9},{:.2},{},{}",
                gas_price_gwei,
                size_eth,
                row.buy,
                row.sell,
                row.pnl_eth_raw,
                row.pnl_usd_raw,
                row.pnl_eth_exec,
                row.pnl_usd_exec,
                row.gas_total,
                row.eth_back
            )?;
        }
        // USDT best
        if let Some(row) = scan_best_for_token(
            simulator.clone(),
            txp.clone(),
            usdt,
            &routes_usdt,
            b,
            eth_in_wei,
            size_eth,
            gas_price_gwei,
        )
        .await?
        {
            writeln!(
                f,
                "{b},USDT,{:.6},{:.6},{},{},{:.9},{:.2},{:.9},{:.2},{},{}",
                gas_price_gwei,
                size_eth,
                row.buy,
                row.sell,
                row.pnl_eth_raw,
                row.pnl_usd_raw,
                row.pnl_eth_exec,
                row.pnl_usd_exec,
                row.gas_total,
                row.eth_back
            )?;
        }
    }
    println!("Wrote {}", out_path);
    Ok(())
}

struct Row {
    buy: String,
    sell: String,
    pnl_eth_raw: f64,
    pnl_usd_raw: f64,
    pnl_eth_exec: f64,
    pnl_usd_exec: f64,
    gas_total: u64,
    eth_back: f64,
}

async fn scan_best_for_token(
    simulator: Arc<TxSimulator>,
    txp: Arc<TxProcessor>,
    token: Address,
    routes: &Vec<AmmSwapRoute>,
    block: u64,
    amount_in_eth: U256,
    size_eth: f64,
    gas_price_gwei: f64,
) -> Result<Option<Row>> {
    let mut best: Option<Row> = None;
    for buy_route in routes.iter() {
        for sell_route in routes.iter() {
            let res = simulate_cross_venue_buy_approve_sell(
                simulator.clone(),
                txp.clone(),
                token,
                buy_route.clone(),
                sell_route.clone(),
                amount_in_eth,
                block,
            )
            .await?;

            let tokens = res.tokens_bought.to_string().parse::<f64>().unwrap_or(0.0) / 1e6;
            let eth_back = res.eth_back.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
            let gas_total = res.gas_total as f64;
            let gas_cost_eth = gas_total * gas_price_gwei / 1e9;
            let pnl_eth_raw = eth_back - 1.0 - gas_cost_eth;
            let usd_per_eth = if size_eth > 0.0 {
                tokens / size_eth
            } else {
                tokens
            };
            let pnl_usd_raw = pnl_eth_raw * usd_per_eth;
            let pnl_eth_exec = pnl_eth_raw.max(0.0);
            let pnl_usd_exec = pnl_usd_raw.max(0.0);

            if best
                .as_ref()
                .map(|b| pnl_usd_exec > b.pnl_usd_exec)
                .unwrap_or(true)
            {
                best = Some(Row {
                    buy: format!("{:?}", buy_route),
                    sell: format!("{:?}", sell_route),
                    pnl_eth_raw,
                    pnl_usd_raw,
                    pnl_eth_exec,
                    pnl_usd_exec,
                    gas_total: res.gas_total,
                    eth_back,
                });
            }
        }
    }
    Ok(best)
}
