use alloy_primitives::{Address, U256};
use pyo3::prelude::*;
use std::str::FromStr;

use eth_price_leverage::stablecoin_agent::rl::{
    action::{space::DiscreteActionSpace as RsDiscreteSpace, Action as RustAction},
    env::{BaygusStablecoinEnv, Env as RustEnv, StepOutput as RustStepOutput},
    types::{ExecMode as RustExec, RouteId as RustRouteId, Side as RustSide, Token as RustToken},
};

fn parse_address(s: &str) -> PyResult<Address> {
    s.parse::<Address>().map_err(|_| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid address: {}", s))
    })
}

#[pyclass]
#[derive(Clone)]
pub struct PyChainSnapshot {
    #[pyo3(get)]
    pub block: u64,
    #[pyo3(get)]
    pub base_fee_wei: u128,
}

#[pyclass]
#[derive(Clone)]
pub struct PyPortfolioState {
    #[pyo3(get)]
    pub eth_wei: String,
    #[pyo3(get)]
    pub usdc_raw: String,
    #[pyo3(get)]
    pub usdt_raw: String,
    #[pyo3(get)]
    pub dai_raw: String,
}

fn conv_state(
    s: &eth_price_leverage::stablecoin_agent::rl::state::State,
) -> (PyChainSnapshot, PyPortfolioState) {
    let chain = PyChainSnapshot {
        block: s.chain.block,
        base_fee_wei: s.chain.base_fee_wei,
    };
    let pf = PyPortfolioState {
        eth_wei: s.portfolio.eth_wei.to_string(),
        usdc_raw: s
            .portfolio
            .currencies_raw
            .get("USDC")
            .map(|v| v.to_string())
            .unwrap_or_else(|| "0".into()),
        usdt_raw: s
            .portfolio
            .currencies_raw
            .get("USDT")
            .map(|v| v.to_string())
            .unwrap_or_else(|| "0".into()),
        dai_raw: s
            .portfolio
            .currencies_raw
            .get("DAI")
            .map(|v| v.to_string())
            .unwrap_or_else(|| "0".into()),
    };
    (chain, pf)
}

#[pyclass]
pub struct PyStepOutput {
    #[pyo3(get)]
    pub chain: PyChainSnapshot,
    #[pyo3(get)]
    pub portfolio: PyPortfolioState,
    #[pyo3(get)]
    pub reward: f64,
    #[pyo3(get)]
    pub info: String,
}

impl From<RustStepOutput> for PyStepOutput {
    fn from(o: RustStepOutput) -> Self {
        let (chain, pf) = conv_state(&o.state);
        PyStepOutput {
            chain,
            portfolio: pf,
            reward: o.reward,
            info: o.info,
        }
    }
}

#[pyclass]
#[derive(Clone)]
pub struct PyStablecoinAction {
    inner: RustAction,
}

#[pymethods]
impl PyStablecoinAction {
    #[new]
    #[pyo3(signature = (route_kind, pool, side, token, size_raw, fee=None, exec_mode=None))]
    fn new(
        route_kind: &str,
        pool: &str,
        side: &str,
        token: &str,
        size_raw: &pyo3::types::PyAny,
        fee: Option<u32>,
        exec_mode: Option<&str>,
    ) -> PyResult<Self> {
        let route = match route_kind.to_lowercase().as_str() {
            "univ2" => RustRouteId::UniswapV2 {
                pool: parse_address(pool)?,
            },
            "sushiv2" => RustRouteId::SushiswapV2 {
                pool: parse_address(pool)?,
            },
            "univ3" => {
                let f = fee.ok_or_else(|| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>("fee required for univ3")
                })?;
                RustRouteId::UniswapV3 {
                    pool: parse_address(pool)?,
                    fee: f,
                }
            }
            _ => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "route_kind must be one of: univ2, sushiv2, univ3",
                ))
            }
        };
        let side = match side.to_lowercase().as_str() {
            "buy" => RustSide::Buy,
            "sell" => RustSide::Sell,
            _ => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "side must be Buy or Sell",
                ))
            }
        };
        let token = match token.to_uppercase().as_str() {
            "ETH" => RustToken::ETH,
            "USDC" => RustToken::USDC,
            "USDT" => RustToken::USDT,
            "DAI" => RustToken::DAI,
            _ => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "token must be one of ETH,USDC,USDT,DAI",
                ))
            }
        };
        // size_raw from Python int or string
        let size_raw_u256: U256 = if let Ok(i) = size_raw.extract::<u128>() {
            U256::from(i)
        } else if let Ok(s) = size_raw.extract::<String>() {
            U256::from_str(&s).map_err(|_| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>("invalid size_raw string")
            })?
        } else {
            // last resort: format the object to string and parse
            let s = size_raw.str()?.to_string_lossy().to_string();
            U256::from_str(&s)
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("invalid size_raw"))?
        };
        let exec = match exec_mode.map(|s| s.to_lowercase()) {
            Some(s) if s == "withpermit" => RustExec::WithPermit,
            _ => RustExec::Default,
        };
        Ok(Self {
            inner: RustAction {
                route_id: route,
                side,
                token,
                size_raw: size_raw_u256,
                exec,
            },
        })
    }
}

#[pyclass]
pub struct PyStablecoinEnv {
    inner: BaygusStablecoinEnv,
}

#[pymethods]
impl PyStablecoinEnv {
    #[new]
    #[pyo3(signature = (agent, start_block, reth_datadir=None, tip_gwei=1, slippage_bps=50, block_step=1))]
    fn new(
        agent: &str,
        start_block: u64,
        reth_datadir: Option<String>,
        tip_gwei: u64,
        slippage_bps: u32,
        block_step: u64,
    ) -> PyResult<Self> {
        let agent_addr = parse_address(agent)?;
        let default_path = std::env::var("RETH_DATADIR")
            .ok()
            .unwrap_or("/home/nima/.local/share/reth/mainnet".to_string());
        let path = reth_datadir.as_deref().unwrap_or(&default_path);
        let env = BaygusStablecoinEnv::new(
            path,
            agent_addr,
            start_block,
            tip_gwei,
            slippage_bps,
            block_step,
        )
        .map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("failed to init env: {}", e))
        })?;
        Ok(Self { inner: env })
    }

    /// Get default discrete action labels (routes x Buy or Buy+Sell), with optional NoOp at index 0
    #[pyo3(signature = (include_sells=false, include_noop=false))]
    fn default_discrete_labels(
        &self,
        include_sells: bool,
        include_noop: bool,
    ) -> PyResult<Vec<String>> {
        let mut space = if include_sells {
            RsDiscreteSpace::default_buy_sell()
        } else {
            RsDiscreteSpace::default_buy_only()
        };
        if include_noop {
            space.include_noop = true;
        }
        Ok(space.labels())
    }

    /// Build concrete default discrete actions for current state.
    /// - Buy actions are fixed to `fixed_buy_wei` (default 1 ETH)
    /// - Sell actions are sized to 1 ETH equivalent in token, clamped to balance
    #[pyo3(signature = (include_sells=false, include_noop=false, fixed_buy_wei=None))]
    fn build_default_discrete_actions(
        &self,
        include_sells: bool,
        include_noop: bool,
        fixed_buy_wei: Option<u128>,
    ) -> PyResult<Vec<PyStablecoinAction>> {
        let mut space = if include_sells {
            RsDiscreteSpace::default_buy_sell()
        } else {
            RsDiscreteSpace::default_buy_only()
        };
        if include_noop {
            space.include_noop = true;
        }
        if let Some(v) = fixed_buy_wei {
            space.fixed_buy_wei = U256::from(v);
        }
        // Use Chainlink ETH/USD for current block
        let b = self.inner.state.chain.block;
        let ethusd = self
            .inner
            .rt
            .block_on(self.inner.cl.get_price_at_block("ETH/USD", b))
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("oracle error: {}", e))
            })?
            .price;
        let acts = space.build_actions(&self.inner.state, ethusd);
        Ok(acts
            .into_iter()
            .map(|inner| PyStablecoinAction { inner })
            .collect())
    }

    /// Take a step with the given action and return (state, reward, info)
    fn step(&mut self, action: &PyStablecoinAction) -> PyResult<PyStepOutput> {
        let out = self.inner.step(action.inner.clone()).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("step error: {}", e))
        })?;
        Ok(out.into())
    }

    /// Get current state snapshot
    fn state(&self) -> PyResult<(PyChainSnapshot, PyPortfolioState)> {
        let st = self.inner.state().clone();
        Ok(conv_state(&st))
    }

    /// Enable per-block state cache inside the env with a default route set.
    fn enable_cache_default_routes(&mut self, capacity: usize) -> PyResult<()> {
        self.inner
            .enable_state_cache_default_routes(capacity)
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "failed to enable cache: {}",
                    e
                ))
            })
    }

    /// Prefetch blocks into the env cache (inclusive range).
    fn prefetch_blocks(&mut self, start_block: u64, end_block: u64) -> PyResult<()> {
        self.inner
            .prefetch_blocks(start_block, end_block)
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("prefetch error: {}", e))
            })
    }

    /// Get cached means/medians for a block if available: (mean_all, median_all, chainlink_eth_usd)
    fn get_cached_mean_median(
        &self,
        block: u64,
    ) -> PyResult<(Option<f64>, Option<f64>, Option<f64>)> {
        if let Some(bp) = self.inner.get_cached_block(block) {
            Ok((
                bp.mean_usd_per_eth_including_oracle(),
                bp.median_usd_per_eth_including_oracle(),
                bp.chainlink_eth_usd,
            ))
        } else {
            Ok((None, None, None))
        }
    }
}
