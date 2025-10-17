use super::super::tx_processor::py_processed_transaction::PyProcessedTransaction;
use crate::header_utils::parse_sealed_header_from_json;
use alloy_primitives::Address;
/// Python bindings for Pool Buy Sell Simulator
///
/// Provides Python interface for pool trading viability analysis and simulation
use pyo3::prelude::*;
use pyo3::types::PyAny;
use reth_primitives::SealedHeader;
use std::{convert::TryFrom, str::FromStr, sync::Arc};
use tx_processor::simulator::types::{
    DEFAULT_APPROVE_GAS_LIMIT, DEFAULT_BUY_GAS_LIMIT, DEFAULT_GAS_LIMIT_NO_PRIOR,
    DEFAULT_SELL_GAS_LIMIT,
};
use tx_processor::ProcessedTransaction as RustProcessedTransaction;
use tx_processor::{
    check_can_buy_sell_pool, tx_processor::TxProcessor, PoolBuySellParameters,
    PoolBuySellSimulationResult, PoolType,
};
use tx_simulator::TxSimulator;

/// Python wrapper for PoolBuySellSimulationResult
#[pyclass(name = "PoolBuySellSimulationResult")]
#[derive(Clone)]
pub struct PyPoolBuySellSimulationResult {
    #[pyo3(get)]
    pub can_buy: bool,
    #[pyo3(get)]
    pub can_approve: bool,
    #[pyo3(get)]
    pub can_sell: bool,
    #[pyo3(get)]
    pub buy_tax_percentage: f64,
    #[pyo3(get)]
    pub sell_tax_percentage: f64,
    #[pyo3(get)]
    pub pool_type: String,
    #[pyo3(get)]
    pub block_number: u64,
    #[pyo3(get)]
    pub tokens_received_raw: String,
    #[pyo3(get)]
    pub eth_spent_raw: String,
    #[pyo3(get)]
    pub eth_received_raw: String,
    #[pyo3(get)]
    pub error_message: Option<String>,
    #[pyo3(get)]
    pub buy_transaction: PyProcessedTransaction,
    #[pyo3(get)]
    pub approve_transaction: PyProcessedTransaction,
    #[pyo3(get)]
    pub sell_transaction: PyProcessedTransaction,
    #[pyo3(get)]
    pub prior_transaction: Option<PyProcessedTransaction>,
}

impl PyPoolBuySellSimulationResult {
    fn from_rust_result(result: PoolBuySellSimulationResult) -> Self {
        let pool_type_str = match result.pool_type {
            PoolType::UniswapV2 => "UNISWAP-V2".to_string(),
            PoolType::SushiSwap => "SUSHI-SWAP".to_string(),
            PoolType::UniswapV3 { fee_tier } => format!("UNISWAP-V3({})", fee_tier),
            PoolType::UniswapV4 => "UNISWAP-V4".to_string(),
            _ => "UNKNOWN".to_string(),
        };

        Self {
            can_buy: result.can_buy,
            can_approve: result.can_approve,
            can_sell: result.can_sell,
            buy_tax_percentage: result.buy_tax_percent,
            sell_tax_percentage: result.sell_tax_percent,
            pool_type: pool_type_str,
            block_number: result.block_number,
            tokens_received_raw: result.tokens_received.to_string(),
            eth_spent_raw: result.eth_spent.to_string(),
            eth_received_raw: result.eth_received.to_string(),
            error_message: result.failure_reason,
            buy_transaction: PyProcessedTransaction::from_processed_transaction(
                result.buy_transaction.clone(),
            ),
            approve_transaction: PyProcessedTransaction::from_processed_transaction(
                result.approve_transaction.clone(),
            ),
            sell_transaction: PyProcessedTransaction::from_processed_transaction(
                result.sell_transaction.clone(),
            ),
            prior_transaction: result
                .prior_transaction
                .as_ref()
                .map(|tx| PyProcessedTransaction::from_processed_transaction(tx.clone())),
        }
    }
}

/// Python wrapper for PoolBuySellParameters
#[pyclass(name = "PoolBuySellParameters")]
#[derive(Clone)]
pub struct PyPoolBuySellParameters {
    #[pyo3(get, set)]
    pub test_amount_eth: f64,
    #[pyo3(get, set)]
    pub buyer_address: String,
    #[pyo3(get, set)]
    pub buy_gas_limit: u64,
    #[pyo3(get, set)]
    pub approve_gas_limit: u64,
    #[pyo3(get, set)]
    pub sell_gas_limit: u64,
    #[pyo3(get, set)]
    pub gas_price_gwei: u64,
    #[pyo3(get, set)]
    pub max_fee_per_gas_gwei: Option<f64>,
    #[pyo3(get, set)]
    pub max_priority_fee_gwei: Option<f64>,
    #[pyo3(get, set)]
    pub block_number: Option<u64>,
    #[pyo3(get, set)]
    pub slippage_tolerance: f64,
    #[pyo3(get, set)]
    pub block_delay: u64,
    pub token_decimals: Option<u8>,
    pub prior_gas_limit: Option<u64>,
    pub prior_max_fee_per_gas_wei: Option<u128>,
    pub prior_max_priority_fee_per_gas_wei: Option<u128>,
    // Optional prior transaction to execute before buy/approve/sell
    // Set via helper methods below
    pub(crate) prior_tx: Option<RustProcessedTransaction>,
    pub(crate) block_header: Option<SealedHeader>,
}

#[pymethods]
impl PyPoolBuySellParameters {
    #[new]
    fn new() -> Self {
        Self {
            test_amount_eth: 0.01, // Default 0.01 ETH
            buyer_address: "0x0C96c602b1b332B8AB2093E5d72D804a24bd5689".to_string(),
            buy_gas_limit: DEFAULT_BUY_GAS_LIMIT,
            approve_gas_limit: DEFAULT_APPROVE_GAS_LIMIT,
            sell_gas_limit: DEFAULT_SELL_GAS_LIMIT,
            gas_price_gwei: 0,
            max_fee_per_gas_gwei: None,
            max_priority_fee_gwei: None,
            block_number: None,
            slippage_tolerance: 5.0,
            block_delay: 0,
            token_decimals: None,
            prior_gas_limit: None,
            prior_max_fee_per_gas_wei: None,
            prior_max_priority_fee_per_gas_wei: None,
            prior_tx: None,
            block_header: None,
        }
    }

    /// Create config with custom buy amount in ETH
    #[staticmethod]
    fn with_buy_amount(amount_eth: f64) -> Self {
        let mut config = Self::new();
        config.test_amount_eth = amount_eth;
        config
    }

    /// Set buyer address
    fn with_buyer<'a>(mut slf: PyRefMut<'a, Self>, address: &str) -> PyRefMut<'a, Self> {
        slf.buyer_address = address.to_string();
        slf
    }

    /// Set a prior transaction from a processed transaction
    /// This transaction executes before buy/approve/sell.
    fn set_prior_tx_from_processed(&mut self, prior: &PyProcessedTransaction) {
        let processed = prior.to_processed_transaction();
        self.apply_prior_processed(processed);
    }

    /// Set a minimal prior transaction from unsigned parameters
    /// Useful for setup calls (e.g., enabling trading) before viability checks.
    #[pyo3(signature = (
        from_address,
        to_address=None,
        value_hex=None,
        data_hex=None,
        nonce=None,
        gas_limit=None,
        max_fee_per_gas_wei=None,
        max_priority_fee_per_gas_wei=None
    ))]
    fn set_prior_tx_from_unsigned(
        &mut self,
        from_address: &str,
        to_address: Option<&str>,
        value_hex: Option<&str>,
        data_hex: Option<&str>,
        nonce: Option<u64>,
        gas_limit: Option<u64>,
        max_fee_per_gas_wei: Option<u128>,
        max_priority_fee_per_gas_wei: Option<u128>,
    ) -> PyResult<()> {
        use alloy_primitives::{Address, B256, U256};
        // Parse inputs
        let from = Address::from_str(from_address.trim_start_matches("0x")).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid from address: {}", e))
        })?;
        let to = if let Some(t) = to_address {
            Some(Address::from_str(t.trim_start_matches("0x")).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid to address: {}",
                    e
                ))
            })?)
        } else {
            None
        };
        let value = if let Some(vh) = value_hex {
            U256::from_str_radix(vh.trim_start_matches("0x"), 16).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid value: {}", e))
            })?
        } else {
            U256::ZERO
        };
        let input = if let Some(dh) = data_hex {
            let clean = dh.trim_start_matches("0x");
            hex::decode(clean).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid data: {}", e))
            })?
        } else {
            vec![]
        };
        // Build minimal processed transaction
        let ptx = RustProcessedTransaction::new(
            B256::ZERO,
            0,
            0,
            0,
            from,
            to,
            value,
            "1".to_string(),
            nonce.unwrap_or(0),
            input,
        );
        self.prior_tx = Some(ptx);
        self.prior_gas_limit = gas_limit;
        self.prior_max_fee_per_gas_wei = max_fee_per_gas_wei;
        self.prior_max_priority_fee_per_gas_wei = max_priority_fee_per_gas_wei;
        Ok(())
    }

    #[pyo3(signature = (prior_dict))]
    fn set_prior_tx_from_dict(&mut self, prior_dict: &PyAny) -> PyResult<()> {
        let py = prior_dict.py();
        let json_mod = py.import("json")?;
        let json_str: String = json_mod
            .call_method1("dumps", (prior_dict,))
            .and_then(|obj| obj.extract())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

        // Drop fields that aren't required for replay and often contain lossy float conversions
        let mut sanitized: serde_json::Value = serde_json::from_str(&json_str).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid processed transaction dict: {e}"
            ))
        })?;

        if let Some(obj) = sanitized.as_object_mut() {
            obj.remove("state_changes");
            obj.remove("latest_states");
        }

        let processed: RustProcessedTransaction = serde_json::from_value(sanitized).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid processed transaction dict: {e}"
            ))
        })?;

        self.apply_prior_processed(processed);
        Ok(())
    }

    fn set_block_header(&mut self, header_json: &str) -> PyResult<()> {
        let sealed = parse_sealed_header_from_json(header_json).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid block header: {e}"))
        })?;
        self.block_header = Some(sealed);
        Ok(())
    }

    #[getter(token_decimals)]
    fn get_token_decimals(&self) -> Option<u8> {
        self.token_decimals
    }

    #[setter(token_decimals)]
    fn set_token_decimals(&mut self, value: u8) {
        self.token_decimals = Some(value);
    }
}

impl PyPoolBuySellParameters {
    fn apply_prior_processed(&mut self, processed: RustProcessedTransaction) {
        use std::convert::TryInto;

        let gas_used = processed.fees.gas_used;
        if gas_used > 0 {
            let scaled_limit = gas_used
                .saturating_mul(2)
                .min(DEFAULT_GAS_LIMIT_NO_PRIOR)
                .max(gas_used);
            self.prior_gas_limit = Some(scaled_limit);
        }

        self.prior_max_fee_per_gas_wei = processed
            .fees
            .max_fee_per_gas
            .and_then(|value| u128::try_from(value).ok());
        self.prior_max_priority_fee_per_gas_wei = processed
            .fees
            .max_priority_fee
            .and_then(|value| u128::try_from(value).ok());

        if self.prior_max_fee_per_gas_wei.is_none() {
            if let Ok(price) = u128::try_from(processed.fees.gas_price) {
                self.prior_max_fee_per_gas_wei = Some(price);
            }
        }

        self.prior_tx = Some(processed);
    }

    fn to_rust_config(
        &self,
        token_address: Address,
        pool_address: Address,
        pool_type: PoolType,
    ) -> PyResult<PoolBuySellParameters> {
        use alloy_primitives::U256;

        let test_amount = U256::from((self.test_amount_eth * 1e18) as u128);
        let buyer =
            Address::from_str(&self.buyer_address.trim_start_matches("0x")).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid buyer address: {}",
                    e
                ))
            })?;

        let token_decimals = self.token_decimals.ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "token_decimals must be provided before running the simulator",
            )
        })?;

        let gas_price = if self.gas_price_gwei == 0 {
            None
        } else {
            Some((self.gas_price_gwei as u128) * 1_000_000_000)
        };

        let buy_gas_limit = if self.buy_gas_limit == 0 {
            DEFAULT_BUY_GAS_LIMIT
        } else {
            self.buy_gas_limit
        };

        let approve_gas_limit = if self.approve_gas_limit == 0 {
            DEFAULT_APPROVE_GAS_LIMIT
        } else {
            self.approve_gas_limit
        };

        let sell_gas_limit = if self.sell_gas_limit == 0 {
            DEFAULT_SELL_GAS_LIMIT
        } else {
            self.sell_gas_limit
        };

        let max_fee = self
            .max_fee_per_gas_gwei
            .map(|value| ((value.max(0.0)) * 1e9).round() as u128);
        let max_priority = self
            .max_priority_fee_gwei
            .map(|value| ((value.max(0.0)) * 1e9).round() as u128);

        Ok(PoolBuySellParameters {
            token_address,
            pool_address,
            pool_type,
            test_amount,
            buyer_address: buyer,
            prior_tx: self.prior_tx.clone(),
            block_number: self.block_number,
            slippage_tolerance: self.slippage_tolerance,
            gas_price,
            max_fee_per_gas: max_fee,
            max_priority_fee_per_gas: max_priority,
            buy_gas_limit,
            approve_gas_limit,
            sell_gas_limit,
            prior_gas_limit: self.prior_gas_limit,
            prior_max_fee_per_gas: self.prior_max_fee_per_gas_wei,
            prior_max_priority_fee_per_gas: self.prior_max_priority_fee_per_gas_wei,
            weth_address: Address::from([
                0xC0, 0x2a, 0xaA, 0x39, 0xb2, 0x23, 0xFE, 0x8D, 0x0A, 0x0e, 0x5C, 0x4F, 0x27, 0xeA,
                0xD9, 0x08, 0x3C, 0x75, 0x6C, 0xc2,
            ]),
            block_delay: self.block_delay,
            token_decimals,
            block_header: self.block_header.clone(),
            uniswap_v4_config: None,
        })
    }
}

/// Python wrapper for Pool Buy Sell Simulator
#[pyclass(name = "PoolBuySellSimulator")]
pub struct PyPoolBuySellSimulator {
    simulator: Arc<TxSimulator>,
    processor: Arc<TxProcessor>,
    runtime: Arc<tokio::runtime::Runtime>,
}

impl PyPoolBuySellSimulator {
    /// Create from shared TxSimulator and TxProcessor instances (used by PyReth)
    pub fn from_shared(simulator: Arc<TxSimulator>, processor: Arc<TxProcessor>) -> PyResult<Self> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(Self {
            simulator,
            processor,
            runtime: Arc::new(runtime),
        })
    }
}

#[pymethods]
impl PyPoolBuySellSimulator {
    /// Create new PoolBuySellSimulator instance
    ///
    /// DEPRECATED: Use PyReth().pool_buy_sell_simulator() instead to avoid multiple database connections
    #[new]
    fn new() -> PyResult<Self> {
        eprintln!("WARNING: Creating standalone PoolBuySellSimulator is deprecated. Use PyReth().pool_buy_sell_simulator() instead.");

        // Create TxSimulator and TxProcessor for standalone use
        let simulator = Arc::new(
            TxSimulator::new("/home/nima/.local/share/reth/mainnet")
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?,
        );
        let processor = Arc::new(TxProcessor::new());

        Self::from_shared(simulator, processor)
    }

    /// Check if pool allows buying and selling tokens for Uniswap V2
    ///
    /// Args:
    ///     token_address: Token contract address as string
    ///     pool_address: Pool contract address as string  
    ///     config: Optional PoolBuySellParameters (default config if None)
    ///
    /// Returns:
    ///     PoolBuySellSimulationResult with trading analysis
    #[pyo3(signature = (token_address, pool_address, config=None))]
    fn check_uniswap_v2_pool(
        &self,
        _py: Python,
        token_address: &str,
        pool_address: &str,
        config: Option<&PyPoolBuySellParameters>,
    ) -> PyResult<PyPoolBuySellSimulationResult> {
        self.check_pool_internal(token_address, pool_address, PoolType::UniswapV2, config)
    }

    /// Check if pool allows buying and selling tokens for Uniswap V3
    ///
    /// Args:
    ///     token_address: Token contract address as string
    ///     pool_address: Pool contract address as string
    ///     fee_tier: Fee tier (500, 3000, or 10000 for 0.05%, 0.3%, 1%)
    ///     config: Optional PoolBuySellParameters (default config if None)
    ///
    /// Returns:
    ///     PoolBuySellSimulationResult with trading analysis
    #[pyo3(signature = (token_address, pool_address, fee_tier, config=None))]
    fn check_uniswap_v3_pool(
        &self,
        _py: Python,
        token_address: &str,
        pool_address: &str,
        fee_tier: u32,
        config: Option<&PyPoolBuySellParameters>,
    ) -> PyResult<PyPoolBuySellSimulationResult> {
        self.check_pool_internal(
            token_address,
            pool_address,
            PoolType::UniswapV3 { fee_tier },
            config,
        )
    }

    /// Check if pool allows buying and selling tokens for SushiSwap
    ///
    /// Args:
    ///     token_address: Token contract address as string
    ///     pool_address: Pool contract address as string  
    ///     config: Optional PoolBuySellParameters (default config if None)
    ///
    /// Returns:
    ///     PoolBuySellSimulationResult with trading analysis
    #[pyo3(signature = (token_address, pool_address, config=None))]
    fn check_sushiswap_pool(
        &self,
        _py: Python,
        token_address: &str,
        pool_address: &str,
        config: Option<&PyPoolBuySellParameters>,
    ) -> PyResult<PyPoolBuySellSimulationResult> {
        self.check_pool_internal(token_address, pool_address, PoolType::SushiSwap, config)
    }

    /// Check if pool allows buying and selling tokens for Uniswap V4
    ///
    /// Note: V4 uses a PoolManager + PoolId architecture and requires Router/Lock integration.
    /// This method returns a well-formed failure result indicating that v4 is not supported yet.
    #[pyo3(signature = (token_address, pool_manager_address, _pool_id_hex, config=None))]
    fn check_uniswap_v4_pool(
        &self,
        _py: Python,
        token_address: &str,
        pool_manager_address: &str,
        _pool_id_hex: &str,
        config: Option<&PyPoolBuySellParameters>,
    ) -> PyResult<PyPoolBuySellSimulationResult> {
        // For now, route through generic handler with PoolType::UniswapV4 and pool_address=pool_manager
        self.check_pool_internal(
            token_address,
            pool_manager_address,
            PoolType::UniswapV4,
            config,
        )
    }
}

impl PyPoolBuySellSimulator {
    /// Internal method to check pool viability
    fn check_pool_internal(
        &self,
        token_address: &str,
        pool_address: &str,
        pool_type: PoolType,
        config: Option<&PyPoolBuySellParameters>,
    ) -> PyResult<PyPoolBuySellSimulationResult> {
        let token_addr =
            Address::from_str(token_address.trim_start_matches("0x")).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid token address: {}",
                    e
                ))
            })?;

        let pool_addr = Address::from_str(pool_address.trim_start_matches("0x")).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid pool address: {}", e))
        })?;

        let cfg = config.ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "PoolBuySellParameters must be provided; token_decimals is required",
            )
        })?;
        let rust_config = cfg.to_rust_config(token_addr, pool_addr, pool_type)?;

        let simulator = self.simulator.clone();
        let processor = self.processor.clone();

        let result = self
            .runtime
            .block_on(
                async move { check_can_buy_sell_pool(simulator, processor, rust_config).await },
            )
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Pool viability check failed: {}",
                    e
                ))
            })?;

        Ok(PyPoolBuySellSimulationResult::from_rust_result(result))
    }

    /// Get default configuration
    fn default_config(&self) -> PyPoolBuySellParameters {
        PyPoolBuySellParameters::new()
    }

    /// Get simulator information
    fn get_info(&self, py: Python) -> PyResult<Py<pyo3::types::PyDict>> {
        let dict = pyo3::types::PyDict::new(py);
        dict.set_item("version", "0.2.0")?;
        dict.set_item("type", "PoolBuySellSimulator")?;
        dict.set_item("default_buy_amount", "0.01 ETH")?;
        dict.set_item("supports_uniswap_v2", true)?;
        dict.set_item("supports_uniswap_v3", true)?;
        dict.set_item("supports_tax_calculation", true)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        "PoolBuySellSimulator(type='viability_check', version='0.2.0')".to_string()
    }
}
