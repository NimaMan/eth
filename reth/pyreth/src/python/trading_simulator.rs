/// Python bindings for TradingEnabledSimulator
/// 
/// Provides Python interface for trading sequence simulation

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::str::FromStr;
use std::sync::Arc;
use alloy_primitives::Address;
use tx_processor::{TxProcessor, data_models::ProcessedTransaction};

use crate::trading_simulator::{TradingEnabledSimulator, TradingSequenceResult, config::BuySellConfig};
use super::processed_transaction::PyProcessedTransaction;

/// Python wrapper for TradingSequenceResult
#[pyclass(name = "TradingSequenceResult")]
#[derive(Clone)]
pub struct PyTradingSequenceResult {
    #[pyo3(get)]
    pub prior_tx: Option<PyProcessedTransaction>,
    #[pyo3(get)]
    pub buy_tx: PyProcessedTransaction,
    #[pyo3(get)]
    pub approve_tx: PyProcessedTransaction,
    #[pyo3(get)]
    pub sell_tx: PyProcessedTransaction,
    #[pyo3(get)]
    pub trading_enabled: bool,
    #[pyo3(get)]
    pub buy_tax: f64,
    #[pyo3(get)]
    pub sell_tax: f64,
    #[pyo3(get)]
    pub block_number: u64,
}

impl PyTradingSequenceResult {
    fn from_rust_result(result: TradingSequenceResult) -> Self {
        Self {
            prior_tx: result.prior_tx.map(|tx| PyProcessedTransaction::from_processed_transaction(tx)),
            buy_tx: PyProcessedTransaction::from_processed_transaction(result.buy_tx),
            approve_tx: PyProcessedTransaction::from_processed_transaction(result.approve_tx),
            sell_tx: PyProcessedTransaction::from_processed_transaction(result.sell_tx),
            trading_enabled: result.trading_enabled,
            buy_tax: result.buy_tax,
            sell_tax: result.sell_tax,
            block_number: result.block_number,
        }
    }
}

/// Python wrapper for BuySellConfig
#[pyclass(name = "BuySellConfig")]
#[derive(Clone)]
pub struct PyBuySellConfig {
    #[pyo3(get, set)]
    pub test_buy_amount_eth: f64,
    #[pyo3(get, set)]
    pub router_address: String,
    #[pyo3(get, set)]
    pub weth_address: String,
    #[pyo3(get, set)]
    pub gas_limit: u64,
    #[pyo3(get, set)]
    pub gas_price: u64,
    #[pyo3(get, set)]
    pub buyer_address: String,
}

#[pymethods]
impl PyBuySellConfig {
    #[new]
    fn new() -> Self {
        let config = BuySellConfig::default();
        Self {
            test_buy_amount_eth: 0.01, // Convert from wei to ETH
            router_address: format!("{:?}", config.router_address),
            weth_address: format!("{:?}", config.weth_address),
            gas_limit: config.gas_limit,
            gas_price: (config.gas_price / 1_000_000_000) as u64, // Convert to gwei
            buyer_address: format!("{:?}", config.buyer_address),
        }
    }
    
    /// Create config with custom buy amount in ETH
    #[staticmethod]
    fn with_buy_amount(amount_eth: f64) -> Self {
        let mut config = Self::new();
        config.test_buy_amount_eth = amount_eth;
        config
    }
}

impl PyBuySellConfig {
    fn to_rust_config(&self) -> PyResult<BuySellConfig> {
        use alloy_primitives::U256;
        
        let test_buy_amount = U256::from((self.test_buy_amount_eth * 1e18) as u128);
        let router = Address::from_str(&self.router_address.trim_start_matches("0x"))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid router address: {}", e)))?;
        let weth = Address::from_str(&self.weth_address.trim_start_matches("0x"))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid WETH address: {}", e)))?;
        let buyer = Address::from_str(&self.buyer_address.trim_start_matches("0x"))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid buyer address: {}", e)))?;
        
        Ok(BuySellConfig {
            test_buy_amount,
            router_address: router,
            weth_address: weth,
            gas_limit: self.gas_limit,
            gas_price: self.gas_price as u128 * 1_000_000_000, // Convert from gwei to wei
            buyer_address: buyer,
            deadline_seconds: 300,
        })
    }
}

/// Python wrapper for TradingEnabledSimulator
#[pyclass(name = "TradingSimulator")]
pub struct PyTradingSimulator {
    inner: Arc<TradingEnabledSimulator>,
    runtime: Arc<tokio::runtime::Runtime>,
}

impl PyTradingSimulator {
    /// Create from shared TxProcessor instance (used by PyReth)
    pub fn from_shared(processor: Arc<TxProcessor>) -> PyResult<Self> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        
        let simulator = TradingEnabledSimulator::new(processor)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        
        Ok(Self {
            inner: Arc::new(simulator),
            runtime: Arc::new(runtime),
        })
    }
}

#[pymethods]
impl PyTradingSimulator {
    /// Create new TradingSimulator instance
    /// 
    /// DEPRECATED: Use PyReth().trading_simulator() instead to avoid multiple database connections
    #[new]
    fn new() -> PyResult<Self> {
        eprintln!("WARNING: Creating standalone TradingSimulator is deprecated. Use PyReth().trading_simulator() instead.");
        
        // Create TxProcessor for standalone use
        let processor = Arc::new(tx_processor::TxProcessor::new("/home/nima/.local/share/reth/mainnet")
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?);
        
        Self::from_shared(processor)
    }
    
    /// Simulate transaction with buy/sell sequence
    /// 
    /// Args:
    ///     prior_tx: Optional ProcessedTransaction to apply before testing (could be any tx that affects trading)
    ///     token_address: Token contract address as string
    ///     pool_address: Pool contract address as string
    ///     block_number: Optional block number (default: latest)
    /// 
    /// Returns:
    ///     TradingSequenceResult with all transaction results and tax calculations
    #[pyo3(signature = (prior_tx, token_address, pool_address, block_number=None))]
    fn simulate_tx_with_buy_sell_seq(
        &self,
        _py: Python,
        prior_tx: Option<&PyProcessedTransaction>,
        token_address: &str,
        pool_address: &str,
        block_number: Option<u64>,
    ) -> PyResult<PyTradingSequenceResult> {
        let token_addr = Address::from_str(token_address.trim_start_matches("0x"))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Invalid token address: {}", e)
            ))?;
        
        let pool_addr = Address::from_str(pool_address.trim_start_matches("0x"))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Invalid pool address: {}", e)
            ))?;
        
        // Convert PyProcessedTransaction to ProcessedTransaction if provided
        let prior_processed = prior_tx.map(|tx| tx.to_processed_transaction());
        
        let simulator = self.inner.clone();
        let result = self.runtime.block_on(async move {
            simulator.simulate_trading_sequence(
                prior_processed.as_ref(),
                token_addr,
                pool_addr,
                block_number,
            ).await
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Simulation failed: {}", e)
        ))?;
        
        Ok(PyTradingSequenceResult::from_rust_result(result))
    }
    
    /// Simulate with custom configuration
    /// 
    /// Args:
    ///     prior_tx: Optional ProcessedTransaction to apply first
    ///     token_address: Token contract address
    ///     pool_address: Pool contract address
    ///     config: Custom BuySellConfig
    ///     block_number: Optional block number
    /// 
    /// Returns:
    ///     TradingSequenceResult
    #[pyo3(signature = (prior_tx, token_address, pool_address, config, block_number=None))]
    fn simulate_with_config(
        &self,
        _py: Python,
        prior_tx: Option<&PyProcessedTransaction>,
        token_address: &str,
        pool_address: &str,
        config: &PyBuySellConfig,
        block_number: Option<u64>,
    ) -> PyResult<PyTradingSequenceResult> {
        // ACTUAL FIX: Python is using default 0.001 ETH, but we need 0.1 ETH
        // The issue is that Python and Rust use different default amounts
        let token_addr = Address::from_str(token_address.trim_start_matches("0x"))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Invalid token address: {}", e)
            ))?;
        let pool_addr = Address::from_str(pool_address.trim_start_matches("0x"))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Invalid pool address: {}", e)
            ))?;
        
        // Convert PyBuySellConfig to BuySellConfig
        let rust_config = config.to_rust_config()?;
        
        // Create a new simulator with the custom config
        let custom_simulator = Arc::new(TradingEnabledSimulator::with_config(
            self.inner.get_tx_processor().clone(),
            rust_config
        ).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Failed to create custom simulator: {}", e)
        ))?);
        
        // Convert PyProcessedTransaction to ProcessedTransaction if provided
        let prior_processed = prior_tx.map(|tx| tx.to_processed_transaction());
        let result = self.runtime.block_on(async move {
            custom_simulator.simulate_trading_sequence(
                prior_processed.as_ref(),
                token_addr,
                pool_addr,
                block_number,
            ).await
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Simulation failed: {}", e)
        ))?;
        
        Ok(PyTradingSequenceResult::from_rust_result(result))
    }
    
    /// Get default configuration
    #[staticmethod]
    fn default_config() -> PyBuySellConfig {
        PyBuySellConfig::new()
    }
    
    /// Get simulator information
    fn get_info(&self, py: Python) -> PyResult<Py<pyo3::types::PyDict>> {
        let dict = pyo3::types::PyDict::new(py);
        dict.set_item("version", "0.1.0")?;
        dict.set_item("type", "TradingEnabledSimulator")?;
        dict.set_item("default_buy_amount", "0.01 ETH")?;
        dict.set_item("router", "Uniswap V2")?;
        dict.set_item("supports_tax_calculation", true)?;
        Ok(dict.into())
    }
    
    fn __repr__(&self) -> String {
        "TradingSimulator(type='sequential', version='0.1.0')".to_string()
    }
}