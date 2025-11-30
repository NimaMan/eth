use crate::price_reader::price_data::PyPriceData;
use alloy_primitives::U256;
use eth_prices::price_readers::snapshot::ChainlinkReader;
use eth_prices::price_readers::snapshot::UniswapV2Reader;
use eth_prices::{price_readers::MultiVenuePriceReader, EthPrices};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use reth_chain_query::common_addresses::get_address_by_name;
use reth_chain_query::dex::compute_uniswap_v2_pool;
use reth_chain_query::provider::RethQueryProvider;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tx_simulator::TxSimulator;

/// Main Python client for Ethereum price data from all sources
#[pyclass]
pub struct PyEthPriceClient {
    inner: EthPrices,
    multi_venue_reader: Option<Arc<MultiVenuePriceReader>>,
}

impl PyEthPriceClient {
    /// Create from shared TxSimulator instance (used by PyReth)
    pub fn from_simulator(simulator: Arc<TxSimulator>) -> Self {
        // Wrap the cloned ProviderFactory in Arc to match expected type
        let provider_factory = Arc::new(simulator.provider_factory().clone());
        let inner = match EthPrices::from_provider(provider_factory.clone()).with_chainlink() {
            Ok(initialized) => initialized,
            Err(err) => {
                eprintln!(
                    "WARNING: failed to initialize Chainlink reader in PyEthPriceClient::from_simulator: {}",
                    err
                );
                EthPrices::from_provider(provider_factory.clone())
            }
        };

        // Also create multi-venue reader
        let aggregated = MultiVenuePriceReader::new(provider_factory.clone())
            .with_all_amms(provider_factory.clone())
            .unwrap_or_else(|_| MultiVenuePriceReader::new(provider_factory.clone()))
            .with_all_oracles(provider_factory.clone())
            .unwrap_or_else(|_| MultiVenuePriceReader::new(provider_factory.clone()));

        Self {
            inner,
            multi_venue_reader: Some(Arc::new(aggregated)),
        }
    }
}

#[pymethods]
impl PyEthPriceClient {
    /// Create new client with default mainnet configuration
    /// DEPRECATED: Use PyReth().price_client() instead to avoid multiple database connections
    #[new]
    #[pyo3(signature = (db_path=None))]
    fn new(db_path: Option<String>) -> PyResult<Self> {
        eprintln!("WARNING: Creating standalone PyEthPriceClient is deprecated. Use PyReth().price_client() instead.");

        if db_path.is_some() {
            eprintln!(
                "WARNING: Custom db_path parameter is ignored. Using default mainnet database."
            );
        }

        let client = EthPrices::new(None);

        Ok(PyEthPriceClient {
            inner: client,
            multi_venue_reader: None,
        })
    }

    /// Initialize Uniswap V2 reader
    fn with_uniswap_v2(&mut self) -> PyResult<()> {
        let new_client = std::mem::replace(&mut self.inner, EthPrices::new(None));
        self.inner = new_client.with_uniswap_v2().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to initialize Uniswap V2: {}",
                e
            ))
        })?;
        Ok(())
    }

    /// Initialize Uniswap V3 reader
    fn with_uniswap_v3(&mut self) -> PyResult<()> {
        let new_client = std::mem::replace(&mut self.inner, EthPrices::new(None));
        self.inner = new_client.with_uniswap_v3().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to initialize Uniswap V3: {}",
                e
            ))
        })?;
        Ok(())
    }

    /// Initialize Chainlink reader
    fn with_chainlink(&mut self) -> PyResult<()> {
        let new_client = std::mem::replace(&mut self.inner, EthPrices::new(None));
        self.inner = new_client.with_chainlink().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to initialize Chainlink: {}",
                e
            ))
        })?;
        Ok(())
    }

    /// Initialize SushiSwap reader
    fn with_sushiswap(&mut self) -> PyResult<()> {
        let new_client = std::mem::replace(&mut self.inner, EthPrices::new(None));
        self.inner = new_client.with_sushiswap().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to initialize SushiSwap: {}",
                e
            ))
        })?;
        Ok(())
    }

    /// Initialize Curve reader
    fn with_curve(&mut self) -> PyResult<()> {
        let new_client = std::mem::replace(&mut self.inner, EthPrices::new(None));
        self.inner = new_client.with_curve().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to initialize Curve: {}",
                e
            ))
        })?;
        Ok(())
    }

    /// Initialize Balancer reader
    fn with_balancer(&mut self) -> PyResult<()> {
        let new_client = std::mem::replace(&mut self.inner, EthPrices::new(None));
        self.inner = new_client.with_balancer().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to initialize Balancer: {}",
                e
            ))
        })?;
        Ok(())
    }

    /// Get price from Uniswap V2
    fn get_uniswap_v2_price(&self, py: Python, pair: String) -> PyResult<PyPriceData> {
        let rust_data = self.inner.get_uniswap_v2_price(&pair).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to get Uniswap V2 price: {}",
                e
            ))
        })?;

        PyPriceData::from_rust_data(rust_data, py)
    }

    /// Get price from Uniswap V3
    fn get_uniswap_v3_price(&self, py: Python, pair: String) -> PyResult<PyPriceData> {
        let rust_data = self.inner.get_uniswap_v3_price(&pair).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to get Uniswap V3 price: {}",
                e
            ))
        })?;

        PyPriceData::from_rust_data(rust_data, py)
    }

    /// Get price from Chainlink (latest block)
    fn get_chainlink_price(&self, pair: String) -> PyResult<PyPriceData> {
        let rt = Runtime::new().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to create async runtime: {}",
                e
            ))
        })?;

        let rust_data = rt
            .block_on(self.inner.get_chainlink_price(&pair))
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to get Chainlink price: {}",
                    e
                ))
            })?;

        Python::with_gil(|py| PyPriceData::from_rust_data(rust_data, py))
    }

    /// Get Chainlink price at a specific block
    #[pyo3(signature = (pair, block_number))]
    fn get_chainlink_price_at_block(
        &self,
        pair: String,
        block_number: u64,
    ) -> PyResult<PyPriceData> {
        let rt = Runtime::new().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to create async runtime: {}",
                e
            ))
        })?;

        let rust_data = rt
            .block_on(self.inner.get_chainlink_price_at_block(&pair, block_number))
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to get Chainlink price at block {}: {}",
                    block_number, e
                ))
            })?;

        Python::with_gil(|py| PyPriceData::from_rust_data(rust_data, py))
    }

    /// Get price from SushiSwap
    fn get_sushiswap_price(&self, py: Python, pair: String) -> PyResult<PyPriceData> {
        let rust_data = self.inner.get_sushiswap_price(&pair).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to get SushiSwap price: {}",
                e
            ))
        })?;

        PyPriceData::from_rust_data(rust_data, py)
    }

    /// Get price from Curve
    fn get_curve_price(&self, py: Python, pair: String) -> PyResult<PyPriceData> {
        // Create a runtime to run the async function
        let rt = Runtime::new().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to create async runtime: {}",
                e
            ))
        })?;

        let rust_data = rt
            .block_on(self.inner.get_curve_price(&pair))
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to get Curve price: {}",
                    e
                ))
            })?;

        PyPriceData::from_rust_data(rust_data, py)
    }

    /// Get price from Balancer
    fn get_balancer_price(&self, py: Python, pair: String) -> PyResult<PyPriceData> {
        // Create a runtime to run the async function
        let rt = Runtime::new().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to create async runtime: {}",
                e
            ))
        })?;

        let rust_data = rt
            .block_on(self.inner.get_balancer_price(&pair))
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to get Balancer price: {}",
                    e
                ))
            })?;

        PyPriceData::from_rust_data(rust_data, py)
    }

    /// Initialize all DEX readers at once
    fn with_all_dex(&mut self) -> PyResult<()> {
        self.with_uniswap_v2()?;
        self.with_uniswap_v3()?;
        self.with_chainlink()?;
        self.with_sushiswap()?;
        self.with_curve()?;
        self.with_balancer()?;
        Ok(())
    }

    /// Get all prices from all available sources
    fn get_all_prices(&self, py: Python<'_>, pair: &str) -> PyResult<HashMap<String, PyObject>> {
        if let Some(aggregated) = &self.multi_venue_reader {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let all_prices = rt.block_on(async { aggregated.get_all_prices_async(pair).await });

            let mut result = HashMap::new();
            for (source, price_result) in all_prices {
                match price_result {
                    Ok(price_data) => {
                        let py_price = PyPriceData::from_rust_data(price_data, py)?;
                        result.insert(source, py_price.into_py(py));
                    }
                    Err(e) => {
                        result.insert(
                            source,
                            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                                "Error: {}",
                                e
                            ))
                            .to_object(py),
                        );
                    }
                }
            }
            Ok(result)
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "Aggregated reader not initialized. Use PyReth().price_client() for full functionality"
            ))
        }
    }

    /// Get median price across all sources
    fn get_median_price(&self, pair: &str) -> PyResult<f64> {
        if let Some(aggregated) = &self.multi_venue_reader {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let median = rt.block_on(async { aggregated.get_median_price(pair).await });

            median.map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to get median price: {}",
                    e
                ))
            })
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "Aggregated reader not initialized. Use PyReth().price_client() for full functionality"
            ))
        }
    }

    /// Get weighted average price with optional weights
    fn get_weighted_average(
        &self,
        pair: &str,
        weights: Option<HashMap<String, f64>>,
    ) -> PyResult<f64> {
        if let Some(aggregated) = &self.multi_venue_reader {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let avg = rt.block_on(async { aggregated.get_weighted_average(pair, weights).await });

            avg.map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to get weighted average: {}",
                    e
                ))
            })
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "Aggregated reader not initialized. Use PyReth().price_client() for full functionality"
            ))
        }
    }

    fn get_summary(&self) -> HashMap<String, String> {
        let mut summary = HashMap::new();
        summary.insert("package".to_string(), "pyreth.price_reader".to_string());
        summary.insert("data_source".to_string(), "Reth Local Database".to_string());
        summary.insert("latency".to_string(), "~0ms (direct storage)".to_string());

        let mut sources = Vec::new();
        if self.inner.uniswap_v2.is_some() {
            sources.push("UniswapV2");
        }
        if self.inner.uniswap_v3.is_some() {
            sources.push("UniswapV3");
        }
        if self.inner.chainlink.is_some() {
            sources.push("Chainlink");
        }
        if self.inner.sushiswap.is_some() {
            sources.push("SushiSwap");
        }
        if self.inner.curve.is_some() {
            sources.push("Curve");
        }
        if self.inner.balancer.is_some() {
            sources.push("Balancer");
        }

        summary.insert("initialized_sources".to_string(), sources.join(", "));

        if self.multi_venue_reader.is_some() {
            summary.insert("multi_venue_reader".to_string(), "Enabled".to_string());
        }

        summary
    }

    /// Get ETH price from specific protocol and stablecoin
    fn get_eth_price(&self, py: Python, protocol: &str, stablecoin: &str) -> PyResult<PyPriceData> {
        let pair = match stablecoin.to_uppercase().as_str() {
            "USDC" => match protocol.to_lowercase().as_str() {
                "uniswapv2" => "ETH/USD".to_string(),
                "uniswapv3_500" => "ETH/USD_V3_500".to_string(),
                "uniswapv3_3000" => "ETH/USD_V3_3000".to_string(),
                "sushiswap" => "ETH/USD".to_string(),
                _ => {
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Unsupported protocol '{}' for USDC",
                        protocol
                    )))
                }
            },
            "USDT" => match protocol.to_lowercase().as_str() {
                "uniswapv3" => "ETH/USDT_V3".to_string(),
                "sushiswap" => "ETH/USDT".to_string(),
                "curve" => "ETH/USDT_CURVE".to_string(),
                _ => {
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Unsupported protocol '{}' for USDT",
                        protocol
                    )))
                }
            },
            "DAI" => match protocol.to_lowercase().as_str() {
                "sushiswap" => "ETH/DAI".to_string(),
                _ => {
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Unsupported protocol '{}' for DAI",
                        protocol
                    )))
                }
            },
            _ => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Unsupported stablecoin: {}",
                    stablecoin
                )))
            }
        };

        // Route to appropriate protocol method
        match protocol.to_lowercase().as_str() {
            "uniswapv2" => self.get_uniswap_v2_price(py, pair),
            "uniswapv3_500" | "uniswapv3_3000" | "uniswapv3" => self.get_uniswap_v3_price(py, pair),
            "sushiswap" => self.get_sushiswap_price(py, pair),
            "curve" => self.get_curve_price(py, pair),
            _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Unknown protocol: {}",
                protocol
            ))),
        }
    }

    /// Get comprehensive ETH prices from all protocols and stablecoins
    fn get_all_eth_prices(&self, py: Python) -> PyResult<PyObject> {
        let main_dict = PyDict::new_bound(py);
        let eth_dict = PyDict::new_bound(py);

        // USDC prices
        let usdc_dict = PyDict::new_bound(py);

        if self.inner.uniswap_v2.is_some() {
            match self.get_eth_price(py, "UniswapV2", "USDC") {
                Ok(price_data) => {
                    let py_obj = Py::new(py, price_data)?;
                    usdc_dict.set_item("UniswapV2", py_obj)?;
                }
                Err(_) => {}
            }
        }

        if self.inner.uniswap_v3.is_some() {
            match self.get_eth_price(py, "UniswapV3_500", "USDC") {
                Ok(price_data) => {
                    let py_obj = Py::new(py, price_data)?;
                    usdc_dict.set_item("UniswapV3_500", py_obj)?;
                }
                Err(_) => {}
            }
            match self.get_eth_price(py, "UniswapV3_3000", "USDC") {
                Ok(price_data) => {
                    let py_obj = Py::new(py, price_data)?;
                    usdc_dict.set_item("UniswapV3_3000", py_obj)?;
                }
                Err(_) => {}
            }
        }

        if self.inner.sushiswap.is_some() {
            match self.get_eth_price(py, "SushiSwap", "USDC") {
                Ok(price_data) => {
                    let py_obj = Py::new(py, price_data)?;
                    usdc_dict.set_item("SushiSwap", py_obj)?;
                }
                Err(_) => {}
            }
        }

        if usdc_dict.len() > 0 {
            eth_dict.set_item("USDC", usdc_dict)?;
        }

        // USDT prices
        let usdt_dict = PyDict::new_bound(py);

        if self.inner.uniswap_v3.is_some() {
            match self.get_eth_price(py, "UniswapV3", "USDT") {
                Ok(price_data) => {
                    let py_obj = Py::new(py, price_data)?;
                    usdt_dict.set_item("UniswapV3", py_obj)?;
                }
                Err(_) => {}
            }
        }

        if self.inner.sushiswap.is_some() {
            match self.get_eth_price(py, "SushiSwap", "USDT") {
                Ok(price_data) => {
                    let py_obj = Py::new(py, price_data)?;
                    usdt_dict.set_item("SushiSwap", py_obj)?;
                }
                Err(_) => {}
            }
        }

        if self.inner.curve.is_some() {
            match self.get_eth_price(py, "Curve", "USDT") {
                Ok(price_data) => {
                    let py_obj = Py::new(py, price_data)?;
                    usdt_dict.set_item("Curve", py_obj)?;
                }
                Err(_) => {}
            }
        }

        if usdt_dict.len() > 0 {
            eth_dict.set_item("USDT", usdt_dict)?;
        }

        // DAI prices
        let dai_dict = PyDict::new_bound(py);

        if self.inner.sushiswap.is_some() {
            match self.get_eth_price(py, "SushiSwap", "DAI") {
                Ok(price_data) => {
                    let py_obj = Py::new(py, price_data)?;
                    dai_dict.set_item("SushiSwap", py_obj)?;
                }
                Err(_) => {}
            }
        }

        if dai_dict.len() > 0 {
            eth_dict.set_item("DAI", dai_dict)?;
        }

        main_dict.set_item("ETH", eth_dict)?;
        Ok(main_dict.into())
    }

    /// Get available protocol-stablecoin combinations
    fn get_available_sources(&self, py: Python) -> PyResult<PyObject> {
        let sources_dict = PyDict::new_bound(py);

        let mut usdc_protocols = Vec::new();
        let mut usdt_protocols = Vec::new();
        let mut dai_protocols = Vec::new();

        if self.inner.uniswap_v2.is_some() {
            usdc_protocols.push("UniswapV2");
        }

        if self.inner.uniswap_v3.is_some() {
            usdc_protocols.push("UniswapV3_500");
            usdc_protocols.push("UniswapV3_3000");
            usdt_protocols.push("UniswapV3");
        }

        if self.inner.sushiswap.is_some() {
            usdc_protocols.push("SushiSwap");
            usdt_protocols.push("SushiSwap");
            dai_protocols.push("SushiSwap");
        }

        if self.inner.curve.is_some() {
            usdt_protocols.push("Curve");
        }

        if !usdc_protocols.is_empty() {
            sources_dict.set_item("USDC", usdc_protocols)?;
        }
        if !usdt_protocols.is_empty() {
            sources_dict.set_item("USDT", usdt_protocols)?;
        }
        if !dai_protocols.is_empty() {
            sources_dict.set_item("DAI", dai_protocols)?;
        }

        Ok(sources_dict.into())
    }

    fn __repr__(&self) -> String {
        let mut sources = Vec::new();
        if self.inner.uniswap_v2.is_some() {
            sources.push("UniswapV2");
        }
        if self.inner.uniswap_v3.is_some() {
            sources.push("UniswapV3");
        }
        if self.inner.chainlink.is_some() {
            sources.push("Chainlink");
        }
        if self.inner.sushiswap.is_some() {
            sources.push("SushiSwap");
        }
        if self.inner.curve.is_some() {
            sources.push("Curve");
        }
        if self.inner.balancer.is_some() {
            sources.push("Balancer");
        }

        format!("PyEthPriceClient(sources=[{}])", sources.join(", "))
    }

    /// Get ETH/USD time series for the last N hours, sampled every `step_secs` seconds.
    /// Returns a list of dicts: {timestamp, block, chainlink, univ2_usdc, univ2_usdt}
    #[pyo3(signature = (hours=24*7, step_secs=3600))]
    fn get_eth_usd_timeseries(
        &self,
        py: Python,
        hours: u64,
        step_secs: u64,
    ) -> PyResult<Vec<PyObject>> {
        // Derive a shared ProviderFactory through any initialized reader under multi_venue_reader
        let provider_factory = if let Some(agg) = &self.multi_venue_reader {
            if let Some(v2r) = &agg.uniswap_v2 {
                Arc::new(v2r.provider_factory.clone())
            } else if let Some(v3r) = &agg.uniswap_v3 {
                Arc::new(v3r.provider_factory.clone())
            } else if let Some(sr) = &agg.sushiswap {
                Arc::new(sr.provider_factory.clone())
            } else if let Some(cr) = &agg.curve {
                Arc::new(cr.provider_factory.clone())
            } else if let Some(br) = &agg.balancer {
                Arc::new(br.provider_factory.clone())
            } else {
                return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    "Aggregated reader not initialized; call PyReth().price_client() and initialize at least one AMM reader"
                ));
            }
        } else {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "Aggregated reader not available; use PyReth().price_client() to avoid extra DB connections"
            ));
        };

        // Readers on shared provider
        let v2 = UniswapV2Reader::from_provider(provider_factory.clone())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        let chainlink = ChainlinkReader::from_provider(provider_factory.clone())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        // Helper provider for token metadata and block timestamps
        let rqp = RethQueryProvider::with_provider_factory(provider_factory.clone())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        // Latest block and timestamps
        let latest_block = rqp
            .get_latest_block()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        let latest_ts = rqp
            .get_block_timestamp(latest_block)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        let start_ts = latest_ts.saturating_sub(hours * step_secs);

        // Estimate avg block time to bound binary-search window
        let back = std::cmp::min(1000, latest_block) as u64;
        let t_prev = rqp
            .get_block_timestamp(latest_block.saturating_sub(back))
            .unwrap_or(latest_ts.saturating_sub(back * 12));
        let avg_bt = ((latest_ts.saturating_sub(t_prev)) as f64 / back.max(1) as f64).max(1.0);
        let approx_blocks = ((hours * step_secs) as f64 / avg_bt) as u64;
        let min_block = latest_block.saturating_sub(approx_blocks + 10_000);

        // Resolve pools and token ordering
        let weth = get_address_by_name("WETH")
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Missing WETH"))?;
        let usdc = get_address_by_name("USDC")
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Missing USDC"))?;
        let usdt = get_address_by_name("USDT")
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Missing USDT"))?;
        let pool_usdc = compute_uniswap_v2_pool(weth, usdc);
        let pool_usdt = compute_uniswap_v2_pool(weth, usdt);
        let (usdc_t0, _usdc_t1) = py
            .allow_threads(|| {
                tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(async { rqp.uni_v2_get_tokens(pool_usdc, Some(latest_block)).await })
            })
            .unwrap_or((usdc, weth));
        let (usdt_t0, _usdt_t1) = py
            .allow_threads(|| {
                tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(async { rqp.uni_v2_get_tokens(pool_usdt, Some(latest_block)).await })
            })
            .unwrap_or((usdt, weth));

        // Binary search helper for timestamp -> block
        let block_for_ts = |ts: u64| -> u64 {
            let mut lo = min_block;
            let mut hi = latest_block;
            let mut ans = lo;
            while lo <= hi {
                let mid = lo + (hi - lo) / 2;
                let ts_mid = rqp.get_block_timestamp(mid).unwrap_or(0);
                if ts_mid >= ts {
                    ans = mid;
                    if mid == 0 {
                        break;
                    }
                    hi = mid.saturating_sub(1);
                } else {
                    lo = mid.saturating_add(1);
                }
            }
            ans
        };

        let mut out: Vec<PyObject> = Vec::new();
        // Create a small async runtime for chainlink at-block calls
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        for i in 0..=hours {
            let ts = start_ts + i * step_secs;
            let b = block_for_ts(ts);

            // Chainlink at block
            let cl = rt.block_on(chainlink.get_price_at_block("ETH/USD", b)).ok();
            let cl_p = cl.as_ref().map(|p| p.price_as_f64()).unwrap_or(0.0);

            // Uniswap V2 USDC per ETH
            let (r0u, r1u) = v2
                .get_reserves_at_block_raw(pool_usdc, b)
                .unwrap_or((U256::ZERO, U256::ZERO));
            let (dec0u, dec1u, base_is_t0u) = if usdc_t0 == weth {
                (18u8, 6u8, true)
            } else {
                (6u8, 18u8, false)
            };
            let usdc_p = v2.compute_price_from_reserves(r0u, r1u, dec0u, dec1u, base_is_t0u);

            // Uniswap V2 USDT per ETH
            let (r0t, r1t) = v2
                .get_reserves_at_block_raw(pool_usdt, b)
                .unwrap_or((U256::ZERO, U256::ZERO));
            let (dec0t, dec1t, base_is_t0t) = if usdt_t0 == weth {
                (18u8, 6u8, true)
            } else {
                (6u8, 18u8, false)
            };
            let usdt_p = v2.compute_price_from_reserves(r0t, r1t, dec0t, dec1t, base_is_t0t);

            let row = PyDict::new_bound(py);
            row.set_item("timestamp", ts)?;
            row.set_item("block", b)?;
            row.set_item("chainlink_eth_usd", cl_p)?;
            row.set_item("univ2_usdc", usdc_p)?;
            row.set_item("univ2_usdt", usdt_p)?;
            out.push(row.into());
        }
        Ok(out)
    }

    /// Get ETH/USD series over the last `window_blocks` blocks, sampled every `step_blocks`.
    /// Returns list of dicts: {block, timestamp, chainlink_eth_usd, univ2_usdc, univ2_usdt,
    /// and if include_reserves: usdc_weth_reserve, usdc_stable_reserve, usdt_weth_reserve, usdt_stable_reserve}
    #[pyo3(signature = (window_blocks=10_000, step_blocks=100, include_reserves=false))]
    fn get_eth_usd_last_n_blocks(
        &self,
        py: Python,
        window_blocks: u64,
        step_blocks: u64,
        include_reserves: bool,
    ) -> PyResult<Vec<PyObject>> {
        // Derive a shared ProviderFactory through any initialized reader under multi_venue_reader
        let provider_factory = if let Some(agg) = &self.multi_venue_reader {
            if let Some(v2r) = &agg.uniswap_v2 {
                Arc::new(v2r.provider_factory.clone())
            } else if let Some(v3r) = &agg.uniswap_v3 {
                Arc::new(v3r.provider_factory.clone())
            } else if let Some(sr) = &agg.sushiswap {
                Arc::new(sr.provider_factory.clone())
            } else if let Some(cr) = &agg.curve {
                Arc::new(cr.provider_factory.clone())
            } else if let Some(br) = &agg.balancer {
                Arc::new(br.provider_factory.clone())
            } else {
                return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    "Aggregated reader not initialized; call PyReth().price_client() and initialize at least one AMM reader"
                ));
            }
        } else {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "Aggregated reader not available; use PyReth().price_client() to avoid extra DB connections"
            ));
        };

        // Readers on shared provider
        let v2 = UniswapV2Reader::from_provider(provider_factory.clone())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        let chainlink = ChainlinkReader::from_provider(provider_factory.clone())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        // Helper provider for token metadata and block timestamps
        let rqp = RethQueryProvider::with_provider_factory(provider_factory.clone())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        // Latest and start blocks
        let latest_block = rqp
            .get_latest_block()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        let start_block = latest_block.saturating_sub(window_blocks.saturating_sub(1));

        // Resolve pools and token ordering (assume stable over window)
        let weth = get_address_by_name("WETH")
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Missing WETH"))?;
        let usdc = get_address_by_name("USDC")
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Missing USDC"))?;
        let usdt = get_address_by_name("USDT")
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Missing USDT"))?;
        let pool_usdc = compute_uniswap_v2_pool(weth, usdc);
        let pool_usdt = compute_uniswap_v2_pool(weth, usdt);
        let (usdc_t0, _usdc_t1) = py
            .allow_threads(|| {
                tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(async { rqp.uni_v2_get_tokens(pool_usdc, Some(latest_block)).await })
            })
            .unwrap_or((usdc, weth));
        let (usdt_t0, _usdt_t1) = py
            .allow_threads(|| {
                tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(async { rqp.uni_v2_get_tokens(pool_usdt, Some(latest_block)).await })
            })
            .unwrap_or((usdt, weth));

        let mut out: Vec<PyObject> = Vec::new();
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        let mut b = start_block;
        while b <= latest_block {
            let ts = rqp.get_block_timestamp(b).unwrap_or(0);

            // Chainlink at block
            let cl = rt.block_on(chainlink.get_price_at_block("ETH/USD", b)).ok();
            let cl_p = cl.as_ref().map(|p| p.price_as_f64()).unwrap_or(0.0);

            // USDC pool reserves
            let (r0u, r1u) = v2
                .get_reserves_at_block_raw(pool_usdc, b)
                .unwrap_or((U256::ZERO, U256::ZERO));
            let (dec0u, dec1u, base_is_t0u) = if usdc_t0 == weth {
                (18u8, 6u8, true)
            } else {
                (6u8, 18u8, false)
            };
            let usdc_p = v2.compute_price_from_reserves(r0u, r1u, dec0u, dec1u, base_is_t0u);

            // USDT pool reserves
            let (r0t, r1t) = v2
                .get_reserves_at_block_raw(pool_usdt, b)
                .unwrap_or((U256::ZERO, U256::ZERO));
            let (dec0t, dec1t, base_is_t0t) = if usdt_t0 == weth {
                (18u8, 6u8, true)
            } else {
                (6u8, 18u8, false)
            };
            let usdt_p = v2.compute_price_from_reserves(r0t, r1t, dec0t, dec1t, base_is_t0t);

            let row = PyDict::new_bound(py);
            row.set_item("block", b)?;
            row.set_item("timestamp", ts)?;
            row.set_item("chainlink_eth_usd", cl_p)?;
            row.set_item("univ2_usdc", usdc_p)?;
            row.set_item("univ2_usdt", usdt_p)?;

            if include_reserves {
                let (usdc_weth_r, usdc_stable_r) = if usdc_t0 == weth {
                    (r0u, r1u)
                } else {
                    (r1u, r0u)
                };
                let (usdt_weth_r, usdt_stable_r) = if usdt_t0 == weth {
                    (r0t, r1t)
                } else {
                    (r1t, r0t)
                };
                let usdc_weth_f = usdc_weth_r.to::<u128>() as f64 / 1e18f64;
                let usdc_stable_f = usdc_stable_r.to::<u128>() as f64 / 1e6f64;
                let usdt_weth_f = usdt_weth_r.to::<u128>() as f64 / 1e18f64;
                let usdt_stable_f = usdt_stable_r.to::<u128>() as f64 / 1e6f64;
                row.set_item("usdc_weth_reserve", usdc_weth_f)?;
                row.set_item("usdc_stable_reserve", usdc_stable_f)?;
                row.set_item("usdt_weth_reserve", usdt_weth_f)?;
                row.set_item("usdt_stable_reserve", usdt_stable_f)?;
            }

            out.push(row.into());

            if latest_block.saturating_sub(b) < step_blocks {
                break;
            }
            b = b.saturating_add(step_blocks);
        }

        Ok(out)
    }
}
