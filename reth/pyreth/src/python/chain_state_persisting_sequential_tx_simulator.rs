/// Python bindings for Chain State Persisting Sequential Transaction Simulator
/// 
/// Provides Python access to the ChainStatePersistingSequentialTxSimulator
/// and analyze_pool_viability functionality.

use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use std::sync::Arc;
use alloy_primitives::{Address, U256};
use tx_processor::{
    ChainStatePersistingSequentialTxSimulator,
    TxProcessor,
    erc20_token_trading_viability::{
        PoolViabilityConfig,
        PoolType,
        PoolViabilityResult,
    },
};
use reth_tx_simulator::{RethTxSimulator, CallRequest};
use super::processed_transaction::PyProcessedTransaction;

/// Python wrapper for ChainStatePersistingSequentialTxSimulator
/// 
/// Simulates multiple transactions in sequence where each transaction
/// sees the blockchain state changes made by all previous transactions.
#[pyclass(name = "ChainStatePersistingSequentialTxSimulator")]
pub struct PyChainStatePersistingSequentialTxSimulator {
    simulator: ChainStatePersistingSequentialTxSimulator,
    tx_processor: Arc<TxProcessor>,
}

#[pymethods]
impl PyChainStatePersistingSequentialTxSimulator {
    /// Create a new chain state persisting sequential transaction simulator
    /// 
    /// Args:
    ///     reth_datadir (str, optional): Path to Reth data directory.
    ///         Defaults to /home/nima/.local/share/reth/mainnet
    #[new]
    fn new(reth_datadir: Option<String>) -> PyResult<Self> {
        let datadir = reth_datadir.unwrap_or_else(|| {
            "/home/nima/.local/share/reth/mainnet".to_string()
        });
        
        let simulator = Arc::new(
            RethTxSimulator::new(&datadir)
                .map_err(|e| PyValueError::new_err(format!("Failed to create simulator: {}", e)))?
        );
        let tx_processor = Arc::new(
            TxProcessor::new(&datadir)
                .map_err(|e| PyValueError::new_err(format!("Failed to create tx_processor: {}", e)))?
        );
        
        Ok(Self {
            simulator: ChainStatePersistingSequentialTxSimulator::new(simulator),
            tx_processor,
        })
    }
    
    /// Initialize the blockchain state at a specific block
    /// 
    /// Args:
    ///     block_number (int, optional): Block number to fork from, or None for latest
    fn initialize_at_block(&mut self, block_number: Option<u64>) -> PyResult<()> {
        self.simulator.initialize_blockchain_state_at_block(block_number)
            .map_err(|e| PyValueError::new_err(format!("Failed to initialize blockchain state: {}", e)))
    }
    
    /// Simulate the next transaction in the sequence
    /// 
    /// Each transaction sees all state changes from previous transactions.
    /// 
    /// Args:
    ///     from_address (str): Address sending the transaction
    ///     to_address (str, optional): Address receiving the transaction
    ///     value (str, optional): ETH value in wei
    ///     data (str, optional): Transaction data as hex string
    ///     gas_limit (int, optional): Gas limit for the transaction
    ///     nonce (int, optional): Transaction nonce (auto-incremented if not provided)
    /// 
    /// Returns:
    ///     PyProcessedTransaction: The processed transaction result
    fn simulate_next_transaction(
        &mut self,
        py: Python<'_>,
        from_address: String,
        to_address: Option<String>,
        value: Option<String>,
        data: Option<String>,
        gas_limit: Option<u64>,
        nonce: Option<u64>,
    ) -> PyResult<PyProcessedTransaction> {
        // Parse addresses
        let from = from_address.parse::<Address>()
            .map_err(|e| PyValueError::new_err(format!("Invalid from address: {}", e)))?;
        
        let to = to_address.map(|addr| {
            addr.parse::<Address>()
                .map_err(|e| PyValueError::new_err(format!("Invalid to address: {}", e)))
        }).transpose()?;
        
        // Parse value
        let value_u256 = value.map(|v| {
            U256::from_str_radix(&v, 10)
                .map_err(|e| PyValueError::new_err(format!("Invalid value: {}", e)))
        }).transpose()?.unwrap_or(U256::ZERO);
        
        // Parse data
        let data_bytes = data.map(|d| {
            let hex_str = if d.starts_with("0x") { &d[2..] } else { &d };
            hex::decode(hex_str)
                .map_err(|e| PyValueError::new_err(format!("Invalid hex data: {}", e)))
        }).transpose()?;
        
        // Create CallRequest
        let call_request = CallRequest {
            from: Some(from),
            to,
            value: Some(value_u256),
            data: data_bytes.map(Into::into),
            gas: gas_limit.or(Some(3_000_000)),
            gas_price: Some(30_000_000_000), // 30 gwei default
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            nonce,
        };
        
        // Simulate transaction preserving state
        let tx_processor = self.tx_processor.clone();
        let result = py.allow_threads(|| {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(async {
                    self.simulator.simulate_next_transaction_preserving_blockchain_state(
                        call_request,
                        tx_processor,
                    ).await
                })
        }).map_err(|e| PyValueError::new_err(format!("Simulation failed: {}", e)))?;
        
        // Convert to Python ProcessedTransaction
        Ok(PyProcessedTransaction::from_processed_transaction(result.clone()))
    }
    
    /// Get all processed transactions from the sequence
    /// 
    /// Returns:
    ///     List[PyProcessedTransaction]: All transactions simulated so far
    fn get_all_transactions(&self) -> Vec<PyProcessedTransaction> {
        self.simulator.get_all_processed_transactions()
            .iter()
            .map(|tx| PyProcessedTransaction::from_processed_transaction(tx.clone()))
            .collect()
    }
    
    /// Extract token amount received by an address in a specific transaction
    /// 
    /// Args:
    ///     tx_index (int): Index of the transaction in the sequence
    ///     token_address (str): Address of the token contract
    ///     holder_address (str): Address that received the tokens
    /// 
    /// Returns:
    ///     str: Amount of tokens received as a string
    fn extract_token_amount_received(
        &self,
        tx_index: usize,
        token_address: String,
        holder_address: String,
    ) -> PyResult<String> {
        let token = token_address.parse::<Address>()
            .map_err(|e| PyValueError::new_err(format!("Invalid token address: {}", e)))?;
        let holder = holder_address.parse::<Address>()
            .map_err(|e| PyValueError::new_err(format!("Invalid holder address: {}", e)))?;
        
        let amount = self.simulator.extract_token_amount_received_by_address(
            tx_index,
            token,
            holder,
        ).map_err(|e| PyValueError::new_err(format!("Failed to extract token amount: {}", e)))?;
        
        Ok(amount.to_string())
    }
    
    /// Get cumulative gas used across all transactions
    fn get_cumulative_gas_used(&self) -> u64 {
        self.simulator.get_cumulative_gas_used()
    }
    
    /// Get the block number the sequence was forked from
    fn get_fork_block_number(&self) -> Option<u64> {
        self.simulator.get_fork_block_number()
    }
    
    /// Check if blockchain state has been initialized
    fn is_initialized(&self) -> bool {
        self.simulator.is_blockchain_state_initialized()
    }
}

/// Python wrapper for pool viability analysis result
#[pyclass(name = "PoolViabilityResult")]
#[derive(Clone)]
pub struct PyPoolViabilityResult {
    #[pyo3(get)]
    pub is_tradeable: bool,
    #[pyo3(get)]
    pub buy_tax_percent: f64,
    #[pyo3(get)]
    pub sell_tax_percent: f64,
    #[pyo3(get)]
    pub tokens_received: String,
    #[pyo3(get)]
    pub eth_spent: String,
    #[pyo3(get)]
    pub eth_received: String,
    #[pyo3(get)]
    pub block_number: u64,
    #[pyo3(get)]
    pub failure_reason: Option<String>,
}

impl PyPoolViabilityResult {
    fn from_result(result: PoolViabilityResult) -> Self {
        Self {
            is_tradeable: result.is_tradeable,
            buy_tax_percent: result.buy_tax_percent,
            sell_tax_percent: result.sell_tax_percent,
            tokens_received: result.tokens_received.to_string(),
            eth_spent: result.eth_spent.to_string(),
            eth_received: result.eth_received.to_string(),
            block_number: result.block_number,
            failure_reason: result.failure_reason,
        }
    }
}

/// Analyze pool viability for token trading
/// 
/// This function tests if a token can be traded (buy and sell) through a DEX pool.
/// 
/// Args:
///     token_address (str): Address of the token to test
///     pool_address (str): Address of the DEX pool
///     pool_type (str): Type of pool ("UniswapV2", "UniswapV3", "SushiSwap", "Curve", or "Balancer")
///     test_amount (str, optional): Amount of ETH to test with in wei (default: 0.001 ETH)
///     processed_tx (PyProcessedTransaction, optional): Prior transaction to simulate first (e.g., enable trading)
///     block_number (int, optional): Block number to simulate at (None = latest)
///     reth_datadir (str, optional): Path to Reth data directory
/// 
/// Returns:
///     PyPoolViabilityResult: Analysis results including tax percentages and trade amounts
#[pyfunction]
#[pyo3(signature = (token_address, pool_address, pool_type="UniswapV2", test_amount=None, processed_tx=None, block_number=None, reth_datadir=None))]
pub fn analyze_pool_viability(
    py: Python<'_>,
    token_address: String,
    pool_address: String,
    pool_type: &str,
    test_amount: Option<String>,
    processed_tx: Option<&PyProcessedTransaction>,
    block_number: Option<u64>,
    reth_datadir: Option<String>,
) -> PyResult<PyPoolViabilityResult> {
    // Parse addresses
    let token = token_address.parse::<Address>()
        .map_err(|e| PyValueError::new_err(format!("Invalid token address: {}", e)))?;
    let pool = pool_address.parse::<Address>()
        .map_err(|e| PyValueError::new_err(format!("Invalid pool address: {}", e)))?;
    
    // Parse pool type
    let pool_type_enum = match pool_type {
        "UniswapV2" => PoolType::UniswapV2,
        "UniswapV3" => PoolType::UniswapV3 { fee_tier: 3000 }, // Default to 0.3% fee tier
        "SushiSwap" => PoolType::SushiSwap,
        "Curve" => PoolType::Curve,
        "Balancer" => PoolType::Balancer,
        _ => return Err(PyValueError::new_err(format!("Invalid pool type: {}. Must be UniswapV2, UniswapV3, SushiSwap, Curve, or Balancer", pool_type))),
    };
    
    // Parse test amount
    let test_amount_u256 = test_amount.map(|v| {
        U256::from_str_radix(&v, 10)
            .map_err(|e| PyValueError::new_err(format!("Invalid test amount: {}", e)))
    }).transpose()?.unwrap_or(U256::from(10_000_000_000_000_000u64)); // Default 0.01 ETH
    
    // Create configuration
    let mut config = PoolViabilityConfig::new(token, pool, pool_type_enum)
        .with_test_amount(test_amount_u256);
    
    // Set block number if provided
    config.block_number = block_number;
    
    // Add prior transaction if provided
    if let Some(_prior_tx) = processed_tx {
        // TODO: Convert PyProcessedTransaction back to ProcessedTransaction
        // This would require implementing a conversion method
        // For now, prior_tx functionality is not available through Python
    }
    
    // Get datadir
    let datadir = reth_datadir.unwrap_or_else(|| {
        "/home/nima/.local/share/reth/mainnet".to_string()
    });
    
    // Create simulator and processor
    let simulator = Arc::new(
        RethTxSimulator::new(&datadir)
            .map_err(|e| PyValueError::new_err(format!("Failed to create simulator: {}", e)))?
    );
    let tx_processor = Arc::new(
        TxProcessor::new(&datadir)
            .map_err(|e| PyValueError::new_err(format!("Failed to create tx_processor: {}", e)))?
    );
    
    // Run analysis
    let result = py.allow_threads(|| {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async {
                tx_processor::erc20_token_trading_viability::analyzer::analyze_pool_viability(
                    simulator, 
                    tx_processor, 
                    config
                ).await
            })
    }).map_err(|e| PyValueError::new_err(format!("Pool viability analysis failed: {}", e)))?;
    
    Ok(PyPoolViabilityResult::from_result(result))
}