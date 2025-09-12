/// Cross-block min/max venue estimator using Uniswap/Sushi V2 reserves.
///
/// For a given start block b and horizon h, and size in ETH:
/// - At block b: compute tokens_out for ETH->stable on {UniswapV2, SushiV2}; pick the max (cheapest buy).
/// - At block b+h: compute ETH_out for that tokens_out on {UniswapV2, SushiV2}; pick the max (richest sell).
/// - Report eth_back, raw PnL (ignoring gas), and the venues chosen.
///
/// Notes:
/// - Uses constant-product formula with 0.3% fee (V2 model). Ignores V3.
/// - Ignores gas and approval cost; this is a price-only heuristic.
/// - Token decimals are assumed: WETH=18, USDC=6, USDT=6.
///
use eyre::Result;
use std::sync::Arc;
use alloy_primitives::{Address, U256};
use reth_chain_query::common_addresses::{get_address_by_name, compute_uniswap_v2_pool, compute_sushiswap_pool};
use reth_chain_query::provider::RethQueryProvider;

const RETH_DB_PATH: &str = "/home/nima/.local/share/reth/mainnet";

#[tokio::main]
async fn main() -> Result<()> {
    // Args: [start_block] [horizon_blocks] [size_eth]
    let args: Vec<String> = std::env::args().collect();
    let start_block_in: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(23_325_279);
    let horizon: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(10);
    let size_eth: f64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1.0);

    let rqp = RethQueryProvider::new(RETH_DB_PATH).expect("init provider");
    let latest = rqp.get_latest_block()?;
    let start_block = if start_block_in == 0 { latest } else { start_block_in };
    let sell_block = (start_block.saturating_add(horizon)).min(latest);

    let weth = get_address_by_name("WETH").expect("WETH");
    let usdc = get_address_by_name("USDC").expect("USDC");
    let usdt = get_address_by_name("USDT").expect("USDT");

    println!("Cross-block min/max (V2-only) | start={} sell={} size={:.6} ETH", start_block, sell_block, size_eth);
    println!("Note: ignores gas; uses 0.3% fee CP-AMM formula; V3 excluded.");

    run_token(&rqp, weth, usdc, "USDC", start_block, sell_block, size_eth).await?;
    run_token(&rqp, weth, usdt, "USDT", start_block, sell_block, size_eth).await?;

    Ok(())
}

async fn run_token(
    rqp: &RethQueryProvider,
    weth: Address,
    token: Address,
    symbol: &str,
    b0: u64,
    b1: u64,
    size_eth: f64,
) -> Result<()> {
    // Pools
    let uni = compute_uniswap_v2_pool(weth, token);
    let sushi = compute_sushiswap_pool(weth, token);

    // Decimals
    let dec_eth = 18u8;
    let dec_tok = 6u8; // USDC/USDT

    // Helpers to scale reserves depending on token ordering in the pair
    let (uni_r0_b0, uni_r1_b0, _) = rqp.uni_v2_get_reserves(uni, Some(b0)).await?; // token0, token1
    let (sushi_r0_b0, sushi_r1_b0, _) = rqp.uni_v2_get_reserves(sushi, Some(b0)).await?;
    let (uni_r0_b1, uni_r1_b1, _) = rqp.uni_v2_get_reserves(uni, Some(b1)).await?;
    let (sushi_r0_b1, sushi_r1_b1, _) = rqp.uni_v2_get_reserves(sushi, Some(b1)).await?;

    // Determine token0/token1 roles via address ordering in V2 pairs
    let token0_is_token = token < weth; // V2 sorts by address

    // ETH->Token at b0, pick max tokens
    let (buy_venue, tokens_out) = {
        let uni_tokens = if token0_is_token {
            // token0 = token, token1 = WETH; in=WETH (r1) out=token (r0)
            v2_amount_out(size_eth, uni_r1_b0, dec_eth, uni_r0_b0, dec_tok)
        } else {
            // token0 = WETH, token1 = token; in=WETH (r0) out=token (r1)
            v2_amount_out(size_eth, uni_r0_b0, dec_eth, uni_r1_b0, dec_tok)
        };
        let sushi_tokens = if token0_is_token {
            v2_amount_out(size_eth, sushi_r1_b0, dec_eth, sushi_r0_b0, dec_tok)
        } else {
            v2_amount_out(size_eth, sushi_r0_b0, dec_eth, sushi_r1_b0, dec_tok)
        };
        if uni_tokens >= sushi_tokens { ("UniswapV2", uni_tokens) } else { ("SushiV2", sushi_tokens) }
    };

    // Token->ETH at b1, pick max ETH
    let (sell_venue, eth_back) = {
        let uni_eth = if token0_is_token {
            // in=token (r0) out=WETH (r1)
            v2_amount_out(tokens_out, uni_r0_b1, dec_tok, uni_r1_b1, dec_eth)
        } else {
            v2_amount_out(tokens_out, uni_r1_b1, dec_tok, uni_r0_b1, dec_eth)
        };
        let sushi_eth = if token0_is_token {
            v2_amount_out(tokens_out, sushi_r0_b1, dec_tok, sushi_r1_b1, dec_eth)
        } else {
            v2_amount_out(tokens_out, sushi_r1_b1, dec_tok, sushi_r0_b1, dec_eth)
        };
        if uni_eth >= sushi_eth { ("UniswapV2", uni_eth) } else { ("SushiV2", sushi_eth) }
    };

    // PnL ignoring gas
    let pnl_eth = eth_back - size_eth;
    let approx_usd_per_eth = tokens_out / size_eth.max(1e-12); // ~ 1 USD per token
    let pnl_usd = pnl_eth * approx_usd_per_eth;

    println!("\n{}:", symbol);
    println!("  buy @{}:   venue={} tokens_out={:.6}", b0, buy_venue, tokens_out);
    println!("  sell @{}:  venue={} eth_back={:.6}", b1, sell_venue, eth_back);
    println!("  pnl_eth(raw, no gas): {:.6}", pnl_eth);
    println!("  pnl_usd(~):          {:.2}", pnl_usd);

    Ok(())
}

fn v2_amount_out(amount_in: f64, reserve_in_raw: U256, dec_in: u8, reserve_out_raw: U256, dec_out: u8) -> f64 {
    let rin = reserve_in_raw.to::<u128>() as f64 / 10f64.powi(dec_in as i32);
    let rout = reserve_out_raw.to::<u128>() as f64 / 10f64.powi(dec_out as i32);
    if rin <= 0.0 || rout <= 0.0 || amount_in <= 0.0 { return 0.0; }
    let fee = 0.003_f64; // 0.3%
    let amount_in_with_fee = amount_in * (1.0 - fee);
    // x*y=k → out = (amount_in_with_fee * reserve_out) / (reserve_in + amount_in_with_fee)
    (amount_in_with_fee * rout) / (rin + amount_in_with_fee)
}
