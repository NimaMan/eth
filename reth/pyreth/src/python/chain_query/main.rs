/// Main ChainQuery class that orchestrates all query modules
/// 
/// This is the primary interface that Python users will interact with.
/// It delegates to specialized modules for different types of queries.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use alloy_primitives::Address;
use std::str::FromStr;
use std::sync::Arc;

use tx_processor::TxProcessor;

/// Python wrapper for ChainQuery
/// 
/// Usage:
///   import pyreth
///   reth = pyreth.PyReth()
///   query = reth.chain_query()
#[pyclass(name = "ChainQuery")]
pub struct PyChainQuery {
    inner: Arc<reth_chain_query::ChainQuery>,
    runtime: Arc<tokio::runtime::Runtime>,
}

impl PyChainQuery {
    /// Create from shared TxProcessor instance (used by PyReth)
    pub fn from_shared(processor: Arc<TxProcessor>) -> Self {
        let runtime = tokio::runtime::Runtime::new()
            .expect("Failed to create runtime");
        
        Self {
            inner: processor.chain_query.clone(),
            runtime: Arc::new(runtime),
        }
    }
}

#[pymethods]
impl PyChainQuery {
    /// Create new ChainQuery instance
    /// 
    /// DEPRECATED: Use PyReth().chain_query() instead to avoid multiple database connections
    #[new]
    fn new() -> PyResult<Self> {
        eprintln!("WARNING: Creating standalone ChainQuery is deprecated. Use PyReth().chain_query() instead.");
        
        let reth_datadir = "/home/nima/.local/share/reth/mainnet";
        
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to create runtime: {}", e)
            ))?;
        
        let chain_query = reth_chain_query::ChainQuery::new(reth_datadir)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to create ChainQuery: {}", e)
            ))?;
        
        Ok(Self {
            inner: Arc::new(chain_query),
            runtime: Arc::new(runtime),
        })
    }
    
    /// Get the latest block number
    fn get_latest_block(&self) -> PyResult<u64> {
        self.inner.get_latest_block()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }
    
    // Account queries - delegate to account module
    
    /// Get ETH balance for an address
    /// 
    /// Args:
    ///     address (str): Ethereum address
    ///     block_number (int, optional): Block number to query at (default: latest)
    /// 
    /// Returns:
    ///     str: Balance in wei as string
    fn get_balance(&self, address: &str, block_number: Option<u64>) -> PyResult<String> {
        super::account::get_balance(self, address, block_number)
    }
    
    /// Get nonce for an address
    fn get_nonce(&self, address: &str, block_number: Option<u64>) -> PyResult<u64> {
        super::account::get_nonce(self, address, block_number)
    }
    
    /// Check if an address has code (is a contract)
    fn has_code(&self, address: &str, block_number: Option<u64>) -> PyResult<bool> {
        super::account::has_code(self, address, block_number)
    }
    
    /// Get complete account information
    fn get_account_info(&self, py: Python, address: &str, block_number: Option<u64>) -> PyResult<Py<PyDict>> {
        super::account::get_account_info(self, py, address, block_number)
    }
    
    // Token queries - delegate to token module
    
    /// Get ERC20 token balance for an address
    fn get_token_balance(&self, token_address: &str, holder_address: &str, block_number: Option<u64>) -> PyResult<String> {
        super::token::get_token_balance(self, token_address, holder_address, block_number)
    }
    
    /// Get ERC20 token total supply
    fn get_token_total_supply(&self, token_address: &str, block_number: Option<u64>) -> PyResult<String> {
        super::token::get_token_total_supply(self, token_address, block_number)
    }
    
    /// Get ERC20 token decimals
    fn get_token_decimals(&self, token_address: &str, block_number: Option<u64>) -> PyResult<u8> {
        super::token::get_token_decimals(self, token_address, block_number)
    }
    
    /// Get ERC20 token name
    fn get_token_name(&self, token_address: &str, block_number: Option<u64>) -> PyResult<String> {
        super::token::get_token_name(self, token_address, block_number)
    }
    
    /// Get ERC20 token symbol
    fn get_token_symbol(&self, token_address: &str, block_number: Option<u64>) -> PyResult<String> {
        super::token::get_token_symbol(self, token_address, block_number)
    }
    
    /// Get ERC20 allowance
    fn get_allowance(&self, token_address: &str, owner_address: &str, spender_address: &str, block_number: Option<u64>) -> PyResult<String> {
        super::token::get_allowance(self, token_address, owner_address, spender_address, block_number)
    }
    
    /// Get comprehensive token information
    fn get_token_info(&self, py: Python, token_address: &str, block_number: Option<u64>) -> PyResult<Py<PyDict>> {
        super::token::get_token_info(self, py, token_address, block_number)
    }
    
    // Storage queries - delegate to storage module
    
    /// Read storage at a specific slot
    fn get_storage_at(&self, address: &str, slot: &str, block_number: Option<u64>) -> PyResult<String> {
        super::storage::get_storage_at(self, address, slot, block_number)
    }
    
    // Block queries - delegate to block module
    
    /// Get block information
    fn get_block_info(&self, py: Python, block_number: Option<u64>) -> PyResult<Py<PyDict>> {
        super::block::get_block_info(self, py, block_number)
    }
    
    /// Get block timestamp
    fn get_block_timestamp(&self, block_number: Option<u64>) -> PyResult<u64> {
        super::block::get_block_timestamp(self, block_number)
    }
    
    // High-level analysis methods - delegate to analysis module
    
    /// Analyze stablecoin market share
    fn analyze_stablecoin_market(&self, py: Python) -> PyResult<Py<PyDict>> {
        super::analysis::analyze_stablecoin_market(self, py)
    }
    
    /// Analyze token holdings for multiple tokens and addresses
    fn analyze_token_holdings(&self, py: Python, token_addresses: Vec<&str>, holder_addresses: Vec<&str>, block_number: Option<u64>) -> PyResult<Py<PyDict>> {
        super::analysis::analyze_token_holdings(self, py, token_addresses, holder_addresses, block_number)
    }
    
    /// Get whale portfolios across multiple tokens
    fn get_whale_portfolios(&self, py: Python, token_addresses: Vec<&str>, whale_addresses: Vec<&str>, block_number: Option<u64>) -> PyResult<Py<PyDict>> {
        super::analysis::get_whale_portfolios(self, py, token_addresses, whale_addresses, block_number)
    }
    
    // Entity analysis methods - delegate to entities module
    
    /// Identify entity type for an address
    /// 
    /// Args:
    ///     address (str): Ethereum address
    /// 
    /// Returns:
    ///     str: Entity type ('stablecoin', 'cex', 'etf', or 'unknown')
    fn identify_entity(&self, address: &str) -> PyResult<String> {
        super::entities::identify_entity(address)
    }
    
    /// Check if address is a stablecoin
    fn is_stablecoin(&self, address: &str) -> PyResult<bool> {
        super::entities::is_stablecoin_address(address)
    }
    
    /// Check if address is a CEX address
    fn is_cex(&self, address: &str) -> PyResult<bool> {
        super::entities::is_cex(address)
    }
    
    /// Check if address is an ETF address
    fn is_etf(&self, address: &str) -> PyResult<bool> {
        super::entities::is_etf(address)
    }
    
    /// Get stablecoin information
    fn get_stablecoin_info(&self, py: Python, address: &str) -> PyResult<Option<Py<PyDict>>> {
        super::entities::get_stablecoin_info(py, address)
    }
    
    /// Get CEX information
    fn get_cex_info(&self, py: Python, address: &str) -> PyResult<Option<Py<PyDict>>> {
        super::entities::get_cex_info(py, address)
    }
    
    /// Get ETF information
    fn get_etf_info(&self, py: Python, address: &str) -> PyResult<Option<Py<PyDict>>> {
        super::entities::get_etf_info(py, address)
    }
    
    /// Analyze stablecoin market share
    /// 
    /// Args:
    ///     block_number (int, optional): Block number to query at (default: latest)
    /// 
    /// Returns:
    ///     dict: Market analysis with stablecoins, concentration metrics
    fn analyze_stablecoin_market_share(&self, py: Python, block_number: Option<u64>) -> PyResult<Py<PyDict>> {
        super::entities::analyze_stablecoin_market_share(self, py, block_number)
    }
    
    /// Analyze stablecoin market by currency unit
    /// 
    /// Args:
    ///     block_number (int, optional): Block number to query at (default: latest)
    /// 
    /// Returns:
    ///     dict: Market analysis grouped by currency unit (USD, EUR, JPY, etc.)
    fn analyze_stablecoin_market_by_unit(&self, py: Python, block_number: Option<u64>) -> PyResult<Py<PyDict>> {
        super::entities::analyze_stablecoin_market_by_unit(self, py, block_number)
    }
    
    /// Get top N stablecoins by market cap
    /// 
    /// Args:
    ///     n (int): Number of top stablecoins to return
    ///     block_number (int, optional): Block number to query at (default: latest)
    /// 
    /// Returns:
    ///     list: Top stablecoins with market data
    fn get_top_stablecoins(&self, py: Python, n: usize, block_number: Option<u64>) -> PyResult<Py<pyo3::types::PyList>> {
        super::entities::get_top_stablecoins(self, py, n, block_number)
    }
    
    /// Get CEX balances
    /// 
    /// Args:
    ///     block_number (int, optional): Block number to query at (default: latest)
    /// 
    /// Returns:
    ///     dict: CEX balances by exchange
    fn get_cex_balances(&self, py: Python, block_number: Option<u64>) -> PyResult<Py<PyDict>> {
        super::entities::get_cex_balances(self, py, block_number)
    }
    
    /// Get ETF holdings
    /// 
    /// Args:
    ///     block_number (int, optional): Block number to query at (default: latest)
    /// 
    /// Returns:
    ///     dict: ETF holdings by provider
    fn get_etf_holdings(&self, py: Python, block_number: Option<u64>) -> PyResult<Py<PyDict>> {
        super::entities::get_etf_holdings(self, py, block_number)
    }
    
    /// Get entity statistics
    /// 
    /// Returns:
    ///     dict: Statistics about tracked entities
    fn get_entity_stats(&self, py: Python) -> PyResult<Py<PyDict>> {
        super::entities::get_entity_stats(py)
    }
    
    /// Calculate ETF flows between blocks
    /// 
    /// Args:
    ///     from_block (int): Starting block number
    ///     to_block (int): Ending block number
    /// 
    /// Returns:
    ///     dict: Flow data for all ETF providers
    fn calculate_etf_flows_between_blocks(&self, py: Python, from_block: u64, to_block: u64) -> PyResult<Py<PyDict>> {
        super::entities::calculate_etf_flows_between_blocks(self, py, from_block, to_block)
    }
    
    /// Calculate CEX flows between blocks
    /// 
    /// Args:
    ///     from_block (int): Starting block number
    ///     to_block (int): Ending block number
    /// 
    /// Returns:
    ///     dict: Flow data for all CEX exchanges
    fn calculate_cex_flows_between_blocks(&self, py: Python, from_block: u64, to_block: u64) -> PyResult<Py<PyDict>> {
        super::entities::calculate_cex_flows_between_blocks(self, py, from_block, to_block)
    }
    
    // Time conversion utilities
    
    /// Convert block number to timestamp
    /// 
    /// Args:
    ///     block_number (int): Block number
    /// 
    /// Returns:
    ///     str: ISO 8601 timestamp
    fn block_to_timestamp(&self, block_number: u64) -> PyResult<String> {
        super::time_utils::block_to_timestamp(self, block_number)
    }
    
    /// Convert timestamp to block number
    /// 
    /// Args:
    ///     timestamp (str): ISO 8601 timestamp
    /// 
    /// Returns:
    ///     int: Block number
    fn timestamp_to_block(&self, timestamp: &str) -> PyResult<u64> {
        super::time_utils::timestamp_to_block(self, timestamp)
    }
    
    /// Get block range for time period
    /// 
    /// Args:
    ///     start_timestamp (str): Start time (ISO 8601)
    ///     end_timestamp (str): End time (ISO 8601)
    /// 
    /// Returns:
    ///     tuple: (start_block, end_block)
    fn get_blocks_for_time_range(&self, start_timestamp: &str, end_timestamp: &str) -> PyResult<(u64, u64)> {
        super::time_utils::get_blocks_for_period(self, start_timestamp, end_timestamp)
    }
    
    /// Get blocks for last N periods
    /// 
    /// Args:
    ///     n (int): Number of periods
    ///     period_type (str): 'hour', 'day', 'week', 'month'
    /// 
    /// Returns:
    ///     list: Period boundaries with block ranges
    fn get_blocks_for_last_n_periods(&self, py: Python, n: usize, period_type: &str) -> PyResult<Py<PyList>> {
        super::time_utils::get_blocks_for_last_n_periods(self, py, n, period_type)
    }
    
    /// Calculate stablecoin supply changes between blocks
    /// 
    /// Args:
    ///     from_block (int): Starting block number
    ///     to_block (int): Ending block number
    /// 
    /// Returns:
    ///     dict: Supply change data for all stablecoins
    fn calculate_stablecoin_supply_changes_between_blocks(&self, py: Python, from_block: u64, to_block: u64) -> PyResult<Py<PyDict>> {
        super::entities::calculate_stablecoin_supply_changes_between_blocks(self, py, from_block, to_block)
    }
    
    fn __repr__(&self) -> String {
        format!("ChainQuery(backend='Reth DB', path='/home/nima/.local/share/reth/mainnet')")
    }
}

// Internal accessor for other modules
impl PyChainQuery {
    pub fn chain_query(&self) -> &Arc<reth_chain_query::ChainQuery> {
        &self.inner
    }
    
    pub fn runtime(&self) -> &Arc<tokio::runtime::Runtime> {
        &self.runtime
    }
}