use alloy_primitives::{Address, Bytes, U256};
use eyre::{eyre, Result, WrapErr};
use serde::Deserialize;
use std::{collections::HashMap, env, str::FromStr, sync::Arc, time::Instant};
use tx_processor::{
    simulator::{check_can_buy_sell_pool, PoolBuySellParameters, PoolType},
    tx_processor::TxProcessor,
};
use tx_simulator::TxSimulator;

const DEFAULT_TOKEN_SERVER_URL: &str = "http://127.0.0.1:8765";
const DEFAULT_RETH_DATADIR: &str = "/home/nima/.local/share/reth/mainnet";
const WETH: Address = alloy_primitives::address!("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
const USDC: Address = alloy_primitives::address!("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
const DECIMALS_SELECTOR: [u8; 4] = [0x31, 0x3c, 0xe5, 0x67];

#[derive(Debug, Deserialize)]
struct ActiveRun {
    id: String,
    current_block: u64,
}

#[derive(Debug, Deserialize)]
struct PoolListResponse {
    pools: Vec<TokenServerPool>,
}

#[derive(Debug, Deserialize)]
struct TokenListResponse {
    tokens: Vec<TokenServerToken>,
}

#[derive(Debug, Deserialize)]
struct TokenServerToken {
    contract_address: String,
    decimals: u8,
}

#[derive(Clone, Debug, Deserialize)]
struct TokenServerPool {
    token_address: String,
    token_symbol: String,
    pool_address: String,
    protocol: String,
    fee_tier: Option<u32>,
    denom_address: String,
    denom_symbol: Option<String>,
    latest_block_number: Option<u64>,
    stage: String,
    can_buy: bool,
    can_sell: bool,
    trading_enabled: bool,
    last_trading_failure_reason: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let token_server_url =
        env::var("TOKEN_SERVER_URL").unwrap_or_else(|_| DEFAULT_TOKEN_SERVER_URL.to_string());
    let run_id = match env::var("RUN_ID") {
        Ok(run_id) => run_id,
        Err(_) => fetch_active_run(&token_server_url).await?.id,
    };
    let reth_datadir =
        env::var("RETH_DATADIR").unwrap_or_else(|_| DEFAULT_RETH_DATADIR.to_string());
    let max_pools = env::var("MAX_POOLS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok());

    let active_block = fetch_active_run(&token_server_url).await?.current_block;
    let pools = fetch_v3_pools(&token_server_url, &run_id).await?;
    let tokens = fetch_tokens(&token_server_url, &run_id).await?;

    let simulator = Arc::new(TxSimulator::new(&reth_datadir)?);
    let tx_processor = Arc::new(TxProcessor::new());

    println!(
        "run_id={run_id} active_block={active_block} v3_pools={}",
        pools.len()
    );
    println!("reth_datadir={reth_datadir}");
    println!(
        "{:<4} {:<12} {:<42} {:<7} {:<7} {:<8} {:<13} {:<17} {:<11} {:<11} {:<12} {}",
        "#",
        "symbol",
        "pool",
        "fee",
        "denom",
        "block",
        "stage",
        "server",
        "sim_buy",
        "sim_sell",
        "elapsed_ms",
        "failure"
    );

    let mut ok = 0usize;
    let mut mismatches = 0usize;
    let mut errors = 0usize;
    let selected = max_pools.map_or(pools.len(), |limit| limit.min(pools.len()));

    for (idx, pool) in pools.into_iter().take(selected).enumerate() {
        let started = Instant::now();
        let result = simulate_pool(
            simulator.clone(),
            tx_processor.clone(),
            &tokens,
            &pool,
            active_block,
        )
        .await;
        let elapsed_ms = started.elapsed().as_millis();

        match result {
            Ok(result) => {
                let matches_server =
                    result.can_buy == pool.can_buy && result.can_sell == pool.can_sell;
                if matches_server {
                    ok += 1;
                } else {
                    mismatches += 1;
                }
                println!(
                    "{:<4} {:<12} {:<42} {:<7} {:<7} {:<8} {:<13} {:<17} {:<11} {:<11} {:<12} {}",
                    idx + 1,
                    trim(&pool.token_symbol, 12),
                    pool.pool_address,
                    pool.fee_tier.unwrap_or_default(),
                    trim(pool.denom_symbol.as_deref().unwrap_or("?"), 7),
                    simulation_block(&pool, active_block),
                    trim(&pool.stage, 13),
                    server_status(&pool),
                    result.can_buy,
                    result.can_sell,
                    elapsed_ms,
                    trim(
                        result
                            .failure_reason
                            .as_deref()
                            .or(pool.last_trading_failure_reason.as_deref())
                            .unwrap_or("-"),
                        160,
                    ),
                );
            }
            Err(err) => {
                errors += 1;
                println!(
                    "{:<4} {:<12} {:<42} {:<7} {:<7} {:<8} {:<13} {:<17} {:<11} {:<11} {:<12} {}",
                    idx + 1,
                    trim(&pool.token_symbol, 12),
                    pool.pool_address,
                    pool.fee_tier.unwrap_or_default(),
                    trim(pool.denom_symbol.as_deref().unwrap_or("?"), 7),
                    simulation_block(&pool, active_block),
                    trim(&pool.stage, 13),
                    server_status(&pool),
                    "error",
                    "error",
                    elapsed_ms,
                    trim(&err.to_string(), 160),
                );
            }
        }
    }

    println!(
        "summary checked={} matched={} mismatched={} errors={}",
        selected, ok, mismatches, errors
    );

    Ok(())
}

fn server_status(pool: &TokenServerPool) -> String {
    format!(
        "{} / {} / {}",
        pool.can_buy, pool.can_sell, pool.trading_enabled
    )
}

async fn fetch_active_run(token_server_url: &str) -> Result<ActiveRun> {
    reqwest::get(format!("{token_server_url}/runs/active"))
        .await
        .wrap_err("failed to request active run")?
        .error_for_status()
        .wrap_err("active run request failed")?
        .json()
        .await
        .wrap_err("failed to decode active run")
}

async fn fetch_v3_pools(token_server_url: &str, run_id: &str) -> Result<Vec<TokenServerPool>> {
    let response: PoolListResponse =
        reqwest::get(format!("{token_server_url}/runs/{run_id}/pools"))
            .await
            .wrap_err("failed to request run pools")?
            .error_for_status()
            .wrap_err("run pools request failed")?
            .json()
            .await
            .wrap_err("failed to decode run pools")?;

    Ok(response
        .pools
        .into_iter()
        .filter(|pool| pool.protocol.eq_ignore_ascii_case("UNISWAP-V3"))
        .collect())
}

async fn fetch_tokens(token_server_url: &str, run_id: &str) -> Result<HashMap<String, u8>> {
    let response: TokenListResponse =
        reqwest::get(format!("{token_server_url}/runs/{run_id}/tokens"))
            .await
            .wrap_err("failed to request run tokens")?
            .error_for_status()
            .wrap_err("run tokens request failed")?
            .json()
            .await
            .wrap_err("failed to decode run tokens")?;

    Ok(response
        .tokens
        .into_iter()
        .map(|token| (token.contract_address.to_ascii_lowercase(), token.decimals))
        .collect())
}

async fn simulate_pool(
    simulator: Arc<TxSimulator>,
    tx_processor: Arc<TxProcessor>,
    tokens: &HashMap<String, u8>,
    pool: &TokenServerPool,
    active_block: u64,
) -> Result<tx_processor::simulator::PoolBuySellSimulationResult> {
    let token_address = parse_address(&pool.token_address)?;
    let pool_address = parse_address(&pool.pool_address)?;
    let denom_address = parse_address(&pool.denom_address)?;
    let token_decimals = *tokens
        .get(&pool.token_address.to_ascii_lowercase())
        .ok_or_else(|| eyre!("missing token decimals for {}", pool.token_address))?;
    let block_number = simulation_block(pool, active_block);
    let denom_decimals = denom_decimals(simulator.as_ref(), denom_address, block_number).await?;
    let fee_tier = pool
        .fee_tier
        .ok_or_else(|| eyre!("missing fee tier for pool {}", pool.pool_address))?;
    let test_amount = scaled_decimal_amount(0.01, denom_decimals)?;

    let pool_type = match pool.protocol.to_ascii_uppercase().as_str() {
        "SUSHISWAP-V3" | "SUSHISWAP_V3" | "SUSHI-V3" | "SUSHI_V3" => {
            PoolType::SushiSwapV3 { fee_tier }
        }
        _ => PoolType::UniswapV3 { fee_tier },
    };

    let config = PoolBuySellParameters::new(token_address, pool_address, pool_type)
        .with_test_amount(test_amount)
        .with_denom_address(denom_address)
        .with_denom_decimals(denom_decimals)
        .with_token_decimals(token_decimals)
        .with_block(block_number);

    check_can_buy_sell_pool(simulator, tx_processor, config).await
}

async fn denom_decimals(
    simulator: &TxSimulator,
    denom_address: Address,
    block_number: u64,
) -> Result<u8> {
    if denom_address == WETH {
        return Ok(18);
    }
    if denom_address == USDC {
        return Ok(6);
    }

    let result = simulator
        .simulate_view_function(
            denom_address,
            Bytes::from(DECIMALS_SELECTOR.to_vec()),
            Some(block_number),
        )
        .await
        .wrap_err_with(|| format!("failed to read decimals for denom {denom_address:#x}"))?;

    if !result.success || result.output.len() < 32 {
        return Err(eyre!(
            "decimals() failed for denom {denom_address:#x} at block {block_number}"
        ));
    }

    let value = U256::from_be_slice(&result.output[0..32]);
    let decimals: u8 = value
        .try_into()
        .map_err(|_| eyre!("denom decimals do not fit u8 for {denom_address:#x}: {value}"))?;
    Ok(decimals)
}

fn simulation_block(pool: &TokenServerPool, active_block: u64) -> u64 {
    pool.latest_block_number.unwrap_or(active_block)
}

fn parse_address(value: &str) -> Result<Address> {
    Address::from_str(value).map_err(|err| eyre!("invalid address {value}: {err}"))
}

fn scaled_decimal_amount(amount: f64, decimals: u8) -> Result<U256> {
    let scale = 10u128
        .checked_pow(decimals as u32)
        .ok_or_else(|| eyre!("invalid decimals {decimals}"))?;
    let raw = (amount * scale as f64).round();
    if !raw.is_finite() || raw <= 0.0 {
        return Err(eyre!("invalid scaled amount {raw}"));
    }
    Ok(U256::from(raw as u128))
}

fn trim(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    let mut output = value
        .chars()
        .take(max.saturating_sub(1))
        .collect::<String>();
    output.push('~');
    output
}
