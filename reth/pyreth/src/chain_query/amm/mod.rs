//! AMM liquidity helpers for ChainQuery
//!
//! Exposes a Python `PoolLiquidityInfo` data class and methods from `PyChainQuery`
//! to fetch pool reserves/liquidity without RPC (direct from Reth).

use alloy_primitives::U256;
use pyo3::prelude::*;
use reth_chain_query::tx_builders::amm_swap_route::AmmSwapRoute;
use reth_chain_query::{provider::PoolLiquidityInfo, RethQueryProvider};
use reth_primitives::SealedHeader;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Python wrapper for AMM pool liquidity info
#[pyclass(name = "PoolLiquidityInfo")]
#[derive(Clone)]
pub struct PyPoolLiquidityInfo {
    #[pyo3(get)]
    pub protocol: String,
    #[pyo3(get)]
    pub pool: String,
    #[pyo3(get)]
    pub pool_id: Option<String>,
    #[pyo3(get)]
    pub token0: Option<String>,
    #[pyo3(get)]
    pub token1: Option<String>,
    #[pyo3(get)]
    pub token0_symbol: Option<String>,
    #[pyo3(get)]
    pub token1_symbol: Option<String>,
    #[pyo3(get)]
    pub token0_decimals: Option<u8>,
    #[pyo3(get)]
    pub token1_decimals: Option<u8>,
    #[pyo3(get)]
    pub reserve0_raw: Option<String>,
    #[pyo3(get)]
    pub reserve1_raw: Option<String>,
    #[pyo3(get)]
    pub reserve0_scaled: Option<String>,
    #[pyo3(get)]
    pub reserve1_scaled: Option<String>,
    #[pyo3(get)]
    pub v3_liquidity: Option<String>,
    #[pyo3(get)]
    pub tick: Option<i32>,
    #[pyo3(get)]
    pub sqrt_price_x96: Option<String>,
    #[pyo3(get)]
    pub price_1e18: Option<String>,
    #[pyo3(get)]
    pub block_number: u64,
}

/// Internal helper to fetch liquidity for a given AMM route and convert to Python struct
pub fn get_pool_liquidity(
    runtime: Arc<Runtime>,
    provider: Arc<RethQueryProvider>,
    route: AmmSwapRoute,
    block: Option<u64>,
    header: Option<SealedHeader>,
) -> PyResult<PyPoolLiquidityInfo> {
    let header_clone = header.clone();
    let info = runtime
        .block_on(async move { provider.get_route_liquidity(&route, block, header_clone).await })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
    let PoolLiquidityInfo {
        protocol,
        pool,
        pool_id,
        token0,
        token1,
        token0_symbol,
        token1_symbol,
        token0_decimals,
        token1_decimals,
        reserve0,
        reserve1,
        v3_liquidity,
        tick,
        sqrt_price_x96,
        price_1e18,
        block_number,
    } = info;

    let reserve0_scaled =
        reserve0.and_then(|value| token0_decimals.map(|dec| format_scaled(value, dec)));
    let reserve1_scaled =
        reserve1.and_then(|value| token1_decimals.map(|dec| format_scaled(value, dec)));

    Ok(PyPoolLiquidityInfo {
        protocol: protocol.to_string(),
        pool: format!("0x{}", hex::encode(pool)),
        pool_id: pool_id.map(|b| format!("0x{}", hex::encode(b))),
        token0: token0.map(|a| format!("0x{}", hex::encode(a))),
        token1: token1.map(|a| format!("0x{}", hex::encode(a))),
        token0_symbol,
        token1_symbol,
        token0_decimals,
        token1_decimals,
        reserve0_raw: reserve0.map(|v| v.to_string()),
        reserve1_raw: reserve1.map(|v| v.to_string()),
        reserve0_scaled,
        reserve1_scaled,
        v3_liquidity: v3_liquidity.map(|v| v.to_string()),
        tick,
        sqrt_price_x96: sqrt_price_x96.map(|v| v.to_string()),
        price_1e18: price_1e18.map(|v| v.to_string()),
        block_number,
    })
}

fn format_scaled(value: U256, decimals: u8) -> String {
    if decimals == 0 {
        return value.to_string();
    }

    let scale = U256::from(10u8).pow(U256::from(decimals));
    if scale.is_zero() {
        return value.to_string();
    }

    let whole = value / scale;
    let frac = value % scale;

    if frac.is_zero() {
        return whole.to_string();
    }

    let mut frac_str = frac.to_string();
    let decimals_len = decimals as usize;
    if frac_str.len() < decimals_len {
        let mut padded = String::with_capacity(decimals_len);
        for _ in 0..(decimals_len - frac_str.len()) {
            padded.push('0');
        }
        padded.push_str(&frac_str);
        frac_str = padded;
    }

    let frac_trimmed = frac_str.trim_end_matches('0');
    if frac_trimmed.is_empty() {
        whole.to_string()
    } else {
        format!("{}.{}", whole, frac_trimmed)
    }
}
