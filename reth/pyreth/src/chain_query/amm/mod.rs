//! AMM liquidity helpers for ChainQuery
//!
//! Exposes a Python `PoolLiquidityInfo` data class and methods from `PyChainQuery`
//! to fetch pool reserves/liquidity without RPC (direct from Reth).

use pyo3::prelude::*;
use reth_chain_query::tx_builders::amm_swap_route::AmmSwapRoute;
use reth_chain_query::RethQueryProvider;
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
    pub reserve0: Option<String>,
    #[pyo3(get)]
    pub reserve1: Option<String>,
    #[pyo3(get)]
    pub reserve0_scale_adjusted: Option<String>,
    #[pyo3(get)]
    pub reserve1_scale_adjusted: Option<String>,
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
) -> PyResult<PyPoolLiquidityInfo> {
    let info = runtime
        .block_on(async move { provider.get_route_liquidity(&route, block).await })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

    Ok(PyPoolLiquidityInfo {
        protocol: info.protocol.to_string(),
        pool: format!("0x{}", hex::encode(info.pool)),
        pool_id: info.pool_id.map(|b| format!("0x{}", hex::encode(b))),
        token0: info.token0.map(|a| format!("0x{}", hex::encode(a))),
        token1: info.token1.map(|a| format!("0x{}", hex::encode(a))),
        token0_symbol: info.token0_symbol,
        token1_symbol: info.token1_symbol,
        token0_decimals: info.token0_decimals,
        token1_decimals: info.token1_decimals,
        reserve0: info.reserve0.map(|v| v.to_string()),
        reserve1: info.reserve1.map(|v| v.to_string()),
        reserve0_scale_adjusted: info.reserve0_scale_adjusted,
        reserve1_scale_adjusted: info.reserve1_scale_adjusted,
        v3_liquidity: info.v3_liquidity.map(|v| v.to_string()),
        tick: info.tick,
        sqrt_price_x96: info.sqrt_price_x96.map(|v| v.to_string()),
        price_1e18: info.price_1e18.map(|v| v.to_string()),
        block_number: info.block_number,
    })
}
