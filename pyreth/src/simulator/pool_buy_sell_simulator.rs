use crate::tx_processor::processed_tx_bridge::{
    processed_transaction_from_py_dict, processed_transaction_from_py_object,
    processed_transactions_from_py_iterable,
};
use crate::tx_processor::py_processed_transaction::PyProcessedTransaction;
use alloy_primitives::{Address, B256};
/// Python bindings for Pool Buy Sell Simulator
///
/// Provides Python interface for pool trading viability analysis and simulation
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyList, PyTuple};
use std::{str::FromStr, sync::Arc};
use tx_processor::simulator::types::{
    UniswapV4PoolConfig, DEFAULT_APPROVE_GAS_LIMIT, DEFAULT_BUY_GAS_LIMIT, DEFAULT_SELL_GAS_LIMIT,
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
    pub denom_spent_raw: String,
    #[pyo3(get)]
    pub denom_received_raw: String,
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
    #[pyo3(get)]
    pub prior_transactions: Vec<PyProcessedTransaction>,
}

impl PyPoolBuySellSimulationResult {
    fn from_rust_result(result: PoolBuySellSimulationResult) -> Self {
        let pool_type_str = match result.pool_type {
            PoolType::UniswapV2 => "UNISWAP-V2".to_string(),
            PoolType::SushiSwap => "SUSHI-SWAP".to_string(),
            PoolType::UniswapV3 { fee_tier } => format!("UNISWAP-V3({})", fee_tier),
            PoolType::SushiSwapV3 { fee_tier } => format!("SUSHISWAP-V3({})", fee_tier),
            PoolType::UniswapV4 => "UNISWAP-V4".to_string(),
            _ => "UNKNOWN".to_string(),
        };

        let prior_transactions: Vec<PyProcessedTransaction> = result
            .prior_transactions
            .iter()
            .cloned()
            .map(PyProcessedTransaction::from_processed_transaction)
            .collect();
        let legacy_prior = prior_transactions.first().cloned();

        Self {
            can_buy: result.can_buy,
            can_approve: result.can_approve,
            can_sell: result.can_sell,
            buy_tax_percentage: result.buy_tax_percent,
            sell_tax_percentage: result.sell_tax_percent,
            pool_type: pool_type_str,
            block_number: result.block_number,
            tokens_received_raw: result.tokens_received.to_string(),
            denom_spent_raw: result.denom_spent.to_string(),
            denom_received_raw: result.denom_received.to_string(),
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
            prior_transaction: legacy_prior,
            prior_transactions,
        }
    }
}

/// Python wrapper for PoolBuySellParameters
#[pyclass(name = "PoolBuySellParameters")]
#[derive(Clone)]
pub struct PyPoolBuySellParameters {
    #[pyo3(get, set)]
    pub denom_amount: f64,
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
    #[pyo3(get, set)]
    pub denom_address: String,
    #[pyo3(get, set)]
    pub denom_decimals: u8,
    pub token_decimals: u8,
    pub(crate) prior_txs: Vec<RustProcessedTransaction>,
    pub(crate) uniswap_v4_config: Option<UniswapV4PoolConfig>,
}

#[pymethods]
impl PyPoolBuySellParameters {
    #[new]
    fn new(token_decimals: u8, denom_decimals: u8) -> Self {
        Self {
            denom_amount: 0.0,
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
            denom_address: "0x0000000000000000000000000000000000000000".to_string(),
            denom_decimals: denom_decimals,
            token_decimals,
            prior_txs: Vec::new(),
            uniswap_v4_config: None,
        }
    }

    /// Create config with custom buy amount in denomination token units
    #[staticmethod]
    #[pyo3(signature = (amount, token_decimals, denom_decimals))]
    fn with_denom_amount(amount: f64, token_decimals: u8, denom_decimals: u8) -> Self {
        let mut config = Self::new(token_decimals, denom_decimals);
        config.denom_amount = amount;
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
        self.set_prior_sequence(vec![processed]);
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
        let mut ptx = RustProcessedTransaction::new(
            B256::ZERO,
            0,
            0,
            0,
            from,
            to,
            value,
            true,
            nonce.unwrap_or(0),
            0,
            input,
        );
        if let Some(limit) = gas_limit {
            ptx.fees.gas_limit = limit;
        }
        if let Some(max_fee) = max_fee_per_gas_wei {
            ptx.fees.max_fee_per_gas = Some(U256::from(max_fee));
        }
        if let Some(max_priority) = max_priority_fee_per_gas_wei {
            ptx.fees.max_priority_fee = Some(U256::from(max_priority));
        }
        self.set_prior_sequence(vec![ptx]);
        Ok(())
    }

    /// Configure Uniswap V4-specific pool parameters required for simulation.
    #[pyo3(signature = (
        pool_manager,
        pool_id_hex,
        currency0,
        currency1,
        fee,
        tick_spacing,
        hooks,
        hook_data_hex=None
    ))]
    fn set_uniswap_v4_config(
        &mut self,
        pool_manager: &str,
        pool_id_hex: &str,
        currency0: &str,
        currency1: &str,
        fee: u32,
        tick_spacing: i32,
        hooks: &str,
        hook_data_hex: Option<&str>,
    ) -> PyResult<()> {
        let pool_manager_addr =
            Address::from_str(pool_manager.trim_start_matches("0x")).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid pool manager address: {}",
                    e
                ))
            })?;
        let currency0_addr =
            Address::from_str(currency0.trim_start_matches("0x")).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid currency0 address: {}",
                    e
                ))
            })?;
        let currency1_addr =
            Address::from_str(currency1.trim_start_matches("0x")).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid currency1 address: {}",
                    e
                ))
            })?;
        let hooks_addr = Address::from_str(hooks.trim_start_matches("0x")).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid hooks address: {}", e))
        })?;

        let pool_id = B256::from_str(pool_id_hex.trim_start_matches("0x")).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid pool id (expected 32-byte hex): {}",
                e
            ))
        })?;

        let hook_data = if let Some(data) = hook_data_hex {
            let clean = data.trim_start_matches("0x");
            hex::decode(clean).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid hook data hex: {}",
                    e
                ))
            })?
        } else {
            Vec::new()
        };

        self.uniswap_v4_config = Some(UniswapV4PoolConfig {
            pool_manager: pool_manager_addr,
            pool_id,
            currency0: currency0_addr,
            currency1: currency1_addr,
            fee,
            tick_spacing,
            hooks: hooks_addr,
            hook_data,
        });

        Ok(())
    }

    #[pyo3(signature = (prior_dict))]
    fn set_prior_tx_from_dict(&mut self, prior_dict: &Bound<'_, PyAny>) -> PyResult<()> {
        let processed = processed_transaction_from_py_dict(prior_dict)?;
        self.set_prior_sequence(vec![processed]);
        Ok(())
    }

    /// Accept a prior transaction represented as either the Python ProcessedTransaction
    /// dataclass, a PyProcessedTransaction, or a plain dictionary matching the schema.
    #[pyo3(signature = (prior_tx))]
    fn set_prior_processed_transaction(&mut self, prior_tx: &Bound<'_, PyAny>) -> PyResult<()> {
        if prior_tx.is_instance_of::<PyList>() || prior_tx.is_instance_of::<PyTuple>() {
            let transactions = processed_transactions_from_py_iterable(prior_tx)?;
            self.set_prior_sequence(transactions);
        } else {
            let processed = processed_transaction_from_py_object(prior_tx)?;
            self.set_prior_sequence(vec![processed]);
        }
        Ok(())
    }

    #[pyo3(signature = (prior_iterable))]
    fn set_prior_transactions(&mut self, prior_iterable: &Bound<'_, PyAny>) -> PyResult<()> {
        let transactions = processed_transactions_from_py_iterable(prior_iterable)?;
        self.set_prior_sequence(transactions);
        Ok(())
    }

    #[getter(token_decimals)]
    fn get_token_decimals(&self) -> u8 {
        self.token_decimals
    }

    #[setter(token_decimals)]
    fn set_token_decimals(&mut self, value: u8) {
        self.token_decimals = value;
    }
}

impl PyPoolBuySellParameters {
    fn set_prior_sequence(&mut self, transactions: Vec<RustProcessedTransaction>) {
        self.prior_txs = transactions;
    }

    fn to_rust_config(
        &self,
        token_address: Address,
        pool_address: Address,
        pool_type: PoolType,
    ) -> PyResult<PoolBuySellParameters> {
        use alloy_primitives::U256;

        if self.denom_decimals == 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "denom_decimals must be greater than zero",
            ));
        }
        if self.denom_amount <= 0.0 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "denom_amount must be greater than zero",
            ));
        }
        let denom_scale = 10_f64.powi(self.denom_decimals as i32);
        let denom_amount = U256::from((self.denom_amount * denom_scale).round() as u128);
        let buyer =
            Address::from_str(&self.buyer_address.trim_start_matches("0x")).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid buyer address: {}",
                    e
                ))
            })?;

        let denom_address = Address::from_str(self.denom_address.trim().trim_start_matches("0x"))
            .map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid denom address: {}", e))
        })?;
        if denom_address.is_zero() {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "denom_address must be provided",
            ));
        }

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

        let v4_config = if matches!(pool_type, PoolType::UniswapV4) {
            Some(self.uniswap_v4_config.clone().ok_or_else(|| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "set_uniswap_v4_config(...) must be called before checking a Uniswap V4 pool",
                )
            })?)
        } else {
            self.uniswap_v4_config.clone()
        };

        Ok(PoolBuySellParameters {
            token_address,
            pool_address,
            pool_type,
            test_amount: denom_amount,
            buyer_address: buyer,
            prior_txs: self.prior_txs.clone(),
            block_number: self.block_number,
            block_header: None,
            slippage_tolerance: self.slippage_tolerance,
            gas_price,
            max_fee_per_gas: max_fee,
            max_priority_fee_per_gas: max_priority,
            buy_gas_limit,
            approve_gas_limit,
            sell_gas_limit,
            weth_address: Address::from([
                0xC0, 0x2a, 0xaA, 0x39, 0xb2, 0x23, 0xFE, 0x8D, 0x0A, 0x0e, 0x5C, 0x4F, 0x27, 0xeA,
                0xD9, 0x08, 0x3C, 0x75, 0x6C, 0xc2,
            ]),
            denom_address,
            denom_decimals: self.denom_decimals,
            block_delay: self.block_delay,
            token_decimals: self.token_decimals,
            uniswap_v4_config: v4_config,
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
    /// DEPRECATED: Use the module-level pool_buy_sell_simulator() accessor instead to avoid multiple database connections
    #[new]
    fn new() -> PyResult<Self> {
        eprintln!("WARNING: Creating standalone PoolBuySellSimulator is deprecated. Use pyreth.pool_buy_sell_simulator() instead.");

        // Create TxSimulator and TxProcessor for standalone use
        let simulator = Arc::new(
            TxSimulator::new("/home/nima/.local/share/reth/mainnet")
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?,
        );
        let processor = Arc::new(TxProcessor::new());

        Self::from_shared(simulator, processor)
    }

    /// Return a fresh default configuration object.
    #[pyo3(signature = (token_decimals, denom_decimals))]
    fn default_config(&self, token_decimals: u8, denom_decimals: u8) -> PyPoolBuySellParameters {
        PyPoolBuySellParameters::new(token_decimals, denom_decimals)
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
    #[pyo3(signature = (token_address, pool_address, config=None, prior_transactions=None))]
    fn check_uniswap_v2_pool(
        &self,
        _py: Python,
        token_address: &str,
        pool_address: &str,
        config: Option<&PyPoolBuySellParameters>,
        prior_transactions: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyPoolBuySellSimulationResult> {
        let extra_priors = prior_transactions
            .map(processed_transactions_from_py_iterable)
            .transpose()?;
        self.check_pool_internal(
            token_address,
            pool_address,
            PoolType::UniswapV2,
            config,
            extra_priors,
        )
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
    #[pyo3(signature = (token_address, pool_address, fee_tier, config=None, prior_transactions=None))]
    fn check_uniswap_v3_pool(
        &self,
        _py: Python,
        token_address: &str,
        pool_address: &str,
        fee_tier: u32,
        config: Option<&PyPoolBuySellParameters>,
        prior_transactions: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyPoolBuySellSimulationResult> {
        let extra_priors = prior_transactions
            .map(processed_transactions_from_py_iterable)
            .transpose()?;
        self.check_pool_internal(
            token_address,
            pool_address,
            PoolType::UniswapV3 { fee_tier },
            config,
            extra_priors,
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
    #[pyo3(signature = (token_address, pool_address, config=None, prior_transactions=None))]
    fn check_sushiswap_pool(
        &self,
        _py: Python,
        token_address: &str,
        pool_address: &str,
        config: Option<&PyPoolBuySellParameters>,
        prior_transactions: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyPoolBuySellSimulationResult> {
        let extra_priors = prior_transactions
            .map(processed_transactions_from_py_iterable)
            .transpose()?;
        self.check_pool_internal(
            token_address,
            pool_address,
            PoolType::SushiSwap,
            config,
            extra_priors,
        )
    }

    /// Check if pool allows buying and selling tokens for Uniswap V4
    ///
    /// Note: V4 uses a PoolManager + PoolId architecture and requires Router/Lock integration.
    /// This method returns a well-formed failure result indicating that v4 is not supported yet.
    #[pyo3(signature = (token_address, pool_manager_address, _pool_id_hex, config=None, prior_transactions=None))]
    fn check_uniswap_v4_pool(
        &self,
        _py: Python,
        token_address: &str,
        pool_manager_address: &str,
        _pool_id_hex: &str,
        config: Option<&PyPoolBuySellParameters>,
        prior_transactions: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyPoolBuySellSimulationResult> {
        let extra_priors = prior_transactions
            .map(processed_transactions_from_py_iterable)
            .transpose()?;
        // For now, route through generic handler with PoolType::UniswapV4 and pool_address=pool_manager
        self.check_pool_internal(
            token_address,
            pool_manager_address,
            PoolType::UniswapV4,
            config,
            extra_priors,
        )
    }

    /// Get simulator information
    fn get_info(&self, py: Python) -> PyResult<Py<pyo3::types::PyDict>> {
        let dict = pyo3::types::PyDict::new_bound(py);
        dict.set_item("version", "0.2.0")?;
        dict.set_item("type", "PoolBuySellSimulator")?;
        dict.set_item("default_buy_amount", "0.01 ETH")?;
        dict.set_item("supports_uniswap_v2", true)?;
        dict.set_item("supports_uniswap_v3", true)?;
        dict.set_item("supports_tax_calculation", true)?;
        Ok(dict.unbind())
    }

    fn __repr__(&self) -> String {
        "PoolBuySellSimulator(type='viability_check', version='0.2.0')".to_string()
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
        extra_prior_transactions: Option<Vec<RustProcessedTransaction>>,
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
        let mut rust_config = cfg.to_rust_config(token_addr, pool_addr, pool_type)?;
        if let Some(mut extra) = extra_prior_transactions {
            rust_config.prior_txs.append(&mut extra);
        }

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
}
