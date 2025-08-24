use pyo3::prelude::*;
use eth_price_leverage::EthPrices;
use crate::python::price_reader::price_data::PyPriceData;
use std::collections::HashMap;
use std::sync::Arc;
use tx_processor::TxProcessor;

/// Main Python client for Ethereum price data from all sources
#[pyclass]
pub struct PyEthPriceClient {
    inner: EthPrices,
    // Keep reference to shared processor to prevent dropping
    _processor: Option<Arc<TxProcessor>>,
}

impl PyEthPriceClient {
    /// Create from shared TxProcessor instance (used by PyReth)
    pub fn from_shared(processor: Arc<TxProcessor>) -> Self {
        let provider_factory = processor.provider_factory();
        let inner = EthPrices::from_provider(provider_factory);
        
        Self {
            inner,
            _processor: Some(processor),
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
            eprintln!("WARNING: Custom db_path parameter is ignored. Using default mainnet database.");
        }
        
        let client = EthPrices::new(None);
        
        Ok(PyEthPriceClient { 
            inner: client,
            _processor: None,
        })
    }
    
    /// Initialize Uniswap V2 reader
    fn with_uniswap_v2(&mut self) -> PyResult<()> {
        let new_client = std::mem::replace(&mut self.inner, EthPrices::new(None));
        self.inner = new_client.with_uniswap_v2()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to initialize Uniswap V2: {}", e)
            ))?;
        Ok(())
    }
    
    /// Initialize Uniswap V3 reader
    fn with_uniswap_v3(&mut self) -> PyResult<()> {
        let new_client = std::mem::replace(&mut self.inner, EthPrices::new(None));
        self.inner = new_client.with_uniswap_v3()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to initialize Uniswap V3: {}", e)
            ))?;
        Ok(())
    }
    
    /// Initialize Chainlink reader
    fn with_chainlink(&mut self) -> PyResult<()> {
        let new_client = std::mem::replace(&mut self.inner, EthPrices::new(None));
        self.inner = new_client.with_chainlink()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to initialize Chainlink: {}", e)
            ))?;
        Ok(())
    }
    
    /// Initialize SushiSwap reader
    fn with_sushiswap(&mut self) -> PyResult<()> {
        let new_client = std::mem::replace(&mut self.inner, EthPrices::new(None));
        self.inner = new_client.with_sushiswap()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to initialize SushiSwap: {}", e)
            ))?;
        Ok(())
    }
    
    /// Initialize Curve reader
    fn with_curve(&mut self) -> PyResult<()> {
        let new_client = std::mem::replace(&mut self.inner, EthPrices::new(None));
        self.inner = new_client.with_curve()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to initialize Curve: {}", e)
            ))?;
        Ok(())
    }
    
    /// Initialize Balancer reader
    fn with_balancer(&mut self) -> PyResult<()> {
        let new_client = std::mem::replace(&mut self.inner, EthPrices::new(None));
        self.inner = new_client.with_balancer()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to initialize Balancer: {}", e)
            ))?;
        Ok(())
    }
    
    /// Get price from Uniswap V2
    fn get_uniswap_v2_price(&self, pair: String) -> PyResult<PyPriceData> {
        self.inner.get_uniswap_v2_price(&pair)
            .map(|data| data.into())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to get Uniswap V2 price: {}", e)
            ))
    }
    
    /// Get price from Uniswap V3
    fn get_uniswap_v3_price(&self, pair: String) -> PyResult<PyPriceData> {
        self.inner.get_uniswap_v3_price(&pair)
            .map(|data| data.into())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to get Uniswap V3 price: {}", e)
            ))
    }
    
    /// Get price from Chainlink
    fn get_chainlink_price(&self, pair: String) -> PyResult<PyPriceData> {
        self.inner.get_chainlink_price(&pair)
            .map(|data| data.into())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to get Chainlink price: {}", e)
            ))
    }
    
    /// Get price from SushiSwap
    fn get_sushiswap_price(&self, pair: String) -> PyResult<PyPriceData> {
        self.inner.get_sushiswap_price(&pair)
            .map(|data| data.into())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to get SushiSwap price: {}", e)
            ))
    }
    
    /// Get price from Curve
    fn get_curve_price(&self, pair: String) -> PyResult<PyPriceData> {
        self.inner.get_curve_price(&pair)
            .map(|data| data.into())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to get Curve price: {}", e)
            ))
    }
    
    /// Get price from Balancer
    fn get_balancer_price(&self, pair: String) -> PyResult<PyPriceData> {
        self.inner.get_balancer_price(&pair)
            .map(|data| data.into())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to get Balancer price: {}", e)
            ))
    }
    
    /// Get all available prices from all configured sources
    fn get_all_prices(&self) -> HashMap<String, Vec<PyPriceData>> {
        let all_prices = self.inner.get_all_prices();
        let mut result = HashMap::new();
        
        for (source, price_list) in all_prices {
            let py_prices: Vec<PyPriceData> = price_list.into_iter()
                .map(|data| data.into())
                .collect();
            result.insert(source, py_prices);
        }
        
        result
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
    
    /// Get summary of initialized readers
    fn get_summary(&self) -> HashMap<String, String> {
        let mut summary = HashMap::new();
        summary.insert("package".to_string(), "pyreth.price_reader".to_string());
        summary.insert("data_source".to_string(), "Reth Local Database".to_string());
        summary.insert("latency".to_string(), "~0ms (direct storage)".to_string());
        
        let mut sources = Vec::new();
        if self.inner.uniswap_v2.is_some() { sources.push("UniswapV2"); }
        if self.inner.uniswap_v3.is_some() { sources.push("UniswapV3"); }
        if self.inner.chainlink.is_some() { sources.push("Chainlink"); }
        if self.inner.sushiswap.is_some() { sources.push("SushiSwap"); }
        if self.inner.curve.is_some() { sources.push("Curve"); }
        if self.inner.balancer.is_some() { sources.push("Balancer"); }
        
        summary.insert("initialized_sources".to_string(), sources.join(", "));
        summary
    }
    
    fn __repr__(&self) -> String {
        let mut sources = Vec::new();
        if self.inner.uniswap_v2.is_some() { sources.push("UniswapV2"); }
        if self.inner.uniswap_v3.is_some() { sources.push("UniswapV3"); }
        if self.inner.chainlink.is_some() { sources.push("Chainlink"); }
        if self.inner.sushiswap.is_some() { sources.push("SushiSwap"); }
        if self.inner.curve.is_some() { sources.push("Curve"); }
        if self.inner.balancer.is_some() { sources.push("Balancer"); }
        
        format!("PyEthPriceClient(sources=[{}])", sources.join(", "))
    }
}