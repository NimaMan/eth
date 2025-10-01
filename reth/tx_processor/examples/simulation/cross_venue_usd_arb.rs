use alloy_primitives::{Address, U256};
/// Cross-venue ETH↔Stable two-leg arb simulation at a fixed block.
///
/// For a given stable (USDC/USDT), this example:
/// - Buys token on Uniswap V3 0.05% (ETH->Token)
/// - Approves spender for Sushi V2 router
/// - Sells token on Sushi V2 (Token->ETH)
/// - Reports tokens_out, ETH_back, gas per leg, gas cost, and net PnL (ETH & USD)
use eyre::Result;
use reth_chain_query::common_addresses::{
    compute_sushiswap_pool, compute_uniswap_v3_pool, get_address_by_name,
};
use reth_chain_query::tx_builders::amm_swap_route::AmmSwapRoute;
use std::env;
use std::str::FromStr;
use std::sync::Arc;
use tx_processor::simulator::simulate_cross_venue_buy_approve_sell;
use tx_processor::tx_processor::TxProcessor;
use tx_processor::ProcessedTxProvider;
use tx_simulator::TxSimulator;

const RETH_DB_PATH: &str = "/home/nima/.local/share/reth/mainnet";
const DEFAULT_BLOCK: u64 = 23_325_279; // from prior run
const DEFAULT_SIZE_ETH: f64 = 1.0;
const DEFAULT_THRESHOLD_USD: f64 = 5.0; // only show ≥ $5 by default

#[tokio::main]
async fn main() -> Result<()> {
    let simulator = Arc::new(TxSimulator::new(RETH_DB_PATH)?);
    let txp = Arc::new(TxProcessor::new());

    // CLI args: [block] [size_eth] [threshold_usd]
    let args: Vec<String> = env::args().collect();
    let block: u64 = args
        .get(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_BLOCK);
    let size_eth: f64 = args
        .get(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_SIZE_ETH);
    let threshold_usd: f64 = args
        .get(3)
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_THRESHOLD_USD);

    // Shared provider for base_fee
    let processed = Arc::new(ProcessedTxProvider::with_provider_factory(
        simulator.provider_factory().clone(),
    )?);
    let base_fee_wei = processed.get_base_fee_at_block(block).await?; // wei per gas
                                                                      // Gas pricing: use on-chain base fee only (tip = 0) per request
    let tip_gwei = 0.0_f64;

    // ETH and token addresses
    let weth = get_address_by_name("WETH").expect("WETH address");
    let usdc = get_address_by_name("USDC").expect("USDC address");
    let usdt = get_address_by_name("USDT").expect("USDT address");

    // Build routes
    let v3_500_usdc_pool = compute_uniswap_v3_pool(weth, usdc, 500);
    let buy_usdc = AmmSwapRoute::UniswapV3 {
        pool: v3_500_usdc_pool,
        fee_tier: 500,
    };
    let sell_usdc = AmmSwapRoute::SushiswapV2 {
        pool: compute_sushiswap_pool(weth, usdc),
    };

    let v3_500_usdt_pool = compute_uniswap_v3_pool(weth, usdt, 500);
    let buy_usdt = AmmSwapRoute::UniswapV3 {
        pool: v3_500_usdt_pool,
        fee_tier: 500,
    };
    let sell_usdt = AmmSwapRoute::SushiswapV2 {
        pool: compute_sushiswap_pool(weth, usdt),
    };

    // Convert size_eth → wei (rounded)
    let eth_in_wei = U256::from(((size_eth.max(0.0)) * 1e18).round() as u128);

    println!(
        "Cross-venue two-leg arb @ block {} (size = {:.6} ETH)",
        block, size_eth
    );
    println!(
        "Base fee = {:.3} gwei, Tip = {:.3} gwei",
        base_fee_wei as f64 / 1e9,
        tip_gwei
    );

    // Single pair demo (V3 500 -> Sushi V2)
    let r_usdc = simulate_cross_venue_buy_approve_sell(
        simulator.clone(),
        txp.clone(),
        usdc,
        buy_usdc,
        sell_usdc,
        eth_in_wei,
        block,
    )
    .await?;
    report(
        "USDC (V3_500 -> SushiV2)",
        r_usdc,
        6,
        base_fee_wei,
        tip_gwei,
        size_eth,
    );

    let r_usdt = simulate_cross_venue_buy_approve_sell(
        simulator.clone(),
        txp.clone(),
        usdt,
        buy_usdt,
        sell_usdt,
        eth_in_wei,
        block,
    )
    .await?;
    report(
        "USDT (V3_500 -> SushiV2)",
        r_usdt,
        6,
        base_fee_wei,
        tip_gwei,
        size_eth,
    );

    // Scan all venue pairs for USDC and USDT at this block
    println!(
        "\nScanning all venue pairs (V2, SushiV2, V3[500,3000]) at block {}...",
        block
    );
    scan_token(
        simulator.clone(),
        txp.clone(),
        usdc,
        "USDC",
        weth,
        base_fee_wei,
        tip_gwei,
        block,
        eth_in_wei,
        size_eth,
        threshold_usd,
    )
    .await?;
    scan_token(
        simulator.clone(),
        txp.clone(),
        usdt,
        "USDT",
        weth,
        base_fee_wei,
        tip_gwei,
        block,
        eth_in_wei,
        size_eth,
        threshold_usd,
    )
    .await?;

    Ok(())
}

fn report(
    symbol: &str,
    r: tx_processor::simulator::CrossVenueArbResult,
    token_decimals: u8,
    base_fee_wei: u128,
    tip_gwei: f64,
    size_eth: f64,
) {
    let scale = 10f64.powi(token_decimals as i32);
    let tokens = r.tokens_bought.to_string().parse::<f64>().unwrap_or(0.0) / scale;
    let eth_back = r.eth_back.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
    let gas_total = r.gas_total as f64;
    let gas_price_gwei = (base_fee_wei as f64) / 1e9 + tip_gwei;
    let gas_cost_eth = gas_total * gas_price_gwei / 1e9; // gas * gwei → eth
    let eth_pnl_raw = eth_back - 1.0 - gas_cost_eth;
    let usd_per_eth = if size_eth > 0.0 {
        tokens / size_eth
    } else {
        tokens
    }; // ~1 USD per token
    let usd_pnl_raw = eth_pnl_raw * usd_per_eth;
    let eth_pnl_exec = eth_pnl_raw.max(0.0);
    let usd_pnl_exec = usd_pnl_raw.max(0.0);

    println!("\n{}:", symbol);
    println!("  tokens_bought: {:.6}", tokens);
    println!("  eth_back:      {:.6}", eth_back);
    println!(
        "  gas: buy={} approve={} sell={} total={}",
        r.gas_buy, r.gas_approve, r.gas_sell, r.gas_total
    );
    println!("  gas_price:     {:.3} gwei (base+tip)", gas_price_gwei);
    println!("  gas_cost_eth:  {:.6}", gas_cost_eth);
    println!("  pnl_eth_raw:   {:.6}", eth_pnl_raw);
    println!("  pnl_usd_raw(~):{:.2}", usd_pnl_raw);
    println!("  pnl_eth_exec:  {:.6}", eth_pnl_exec);
    println!("  pnl_usd_exec(~):{:.2}", usd_pnl_exec);
    let decision = if eth_pnl_raw >= 0.0 {
        "EXECUTE"
    } else {
        "SKIP"
    };
    println!("  decision:      {} (pnl_eth >= 0)", decision);
}

async fn scan_token(
    simulator: Arc<TxSimulator>,
    txp: Arc<TxProcessor>,
    token: Address,
    symbol: &str,
    weth: Address,
    base_fee_wei: u128,
    tip_gwei: f64,
    block: u64,
    amount_in_eth: U256,
    size_eth: f64,
    threshold_usd: f64,
) -> Result<()> {
    use reth_chain_query::common_addresses::{
        compute_sushiswap_pool, compute_uniswap_v2_pool, compute_uniswap_v3_pool,
    };
    use reth_chain_query::tx_builders::amm_swap_route::AmmSwapRoute;

    let mut routes: Vec<AmmSwapRoute> = vec![
        AmmSwapRoute::UniswapV2 {
            pool: compute_uniswap_v2_pool(weth, token),
        },
        AmmSwapRoute::SushiswapV2 {
            pool: compute_sushiswap_pool(weth, token),
        },
        AmmSwapRoute::UniswapV3 {
            pool: compute_uniswap_v3_pool(weth, token, 500),
            fee_tier: 500,
        },
        AmmSwapRoute::UniswapV3 {
            pool: compute_uniswap_v3_pool(weth, token, 3000),
            fee_tier: 3000,
        },
    ];

    let mut best: Option<(String, String, f64, f64, u64)> = None; // (buy,sell,pnl_eth_exec,usd_pnl_exec,gas_total)
    let mut rows: Vec<(String, String, f64, f64, f64, u64)> = Vec::new(); // (buy,sell,pnl_eth_exec,usd_pnl_exec,eth_back,gas)

    println!("\n{} venue pairs:", symbol);
    for (i, buy_route) in routes.iter().enumerate() {
        for (j, sell_route) in routes.iter().enumerate() {
            // Allow same routes; cross-venue profit will reveal itself
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
            let tokens =
                res.tokens_bought.to_string().parse::<f64>().unwrap_or(0.0) / 10f64.powi(6);
            let eth_back = res.eth_back.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
            let gas_total = res.gas_total as f64;
            let gas_price_gwei = (base_fee_wei as f64) / 1e9 + tip_gwei;
            let gas_cost_eth = gas_total * gas_price_gwei / 1e9;
            let pnl_eth_raw = eth_back - 1.0 - gas_cost_eth;
            // Approximate USD pnl using tokens per ETH implied by this run
            let usd_per_eth = if size_eth > 0.0 {
                tokens / size_eth
            } else {
                tokens
            };
            let usd_pnl_raw = pnl_eth_raw * usd_per_eth;
            let pnl_eth_exec = pnl_eth_raw.max(0.0);
            let usd_pnl_exec = usd_pnl_raw.max(0.0);

            rows.push((
                format!("{:?}", buy_route),
                format!("{:?}", sell_route),
                pnl_eth_exec,
                usd_pnl_exec,
                eth_back,
                res.gas_total,
            ));

            if best.as_ref().map(|b| pnl_eth_exec > b.2).unwrap_or(true) {
                best = Some((
                    format!("{:?}", buy_route),
                    format!("{:?}", sell_route),
                    pnl_eth_exec,
                    usd_pnl_exec,
                    res.gas_total,
                ));
            }
        }
    }

    // Rank by USD pnl desc
    rows.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));

    // Print only rows meeting threshold (≥ $threshold) or, if threshold <= 0, those with pnl_eth_exec ≥ 0
    println!(
        "\n{} profitable pairs (threshold ≥ ${:.2}):",
        symbol, threshold_usd
    );
    let mut shown = 0usize;
    for (buy, sell, pnl_eth, usd_pnl, eth_back, gas) in rows.into_iter() {
        let pass = if threshold_usd > 0.0 {
            usd_pnl >= threshold_usd
        } else {
            pnl_eth >= 0.0
        };
        if pass {
            println!(
                "  buy={} -> sell={} | pnl_eth={:.6} pnl_usd≈{:.2} eth_back={:.6} gas={}",
                buy, sell, pnl_eth, usd_pnl, eth_back, gas
            );
            shown += 1;
        }
    }
    if shown == 0 {
        println!("  None found meeting threshold.");
    }

    if let Some((b, s, pe, pu, g)) = best {
        // BEST is shown after decision clamping (min 0)
        println!("\n{} BEST: buy={} sell={} | pnl_eth_exec={:.6} pnl_usd_exec≈{:.2} gas={} (decision: {})",
            symbol, b, s, pe, pu, g, if pe > 0.0 { "EXECUTE" } else { "SKIP" });
    }
    Ok(())
}
