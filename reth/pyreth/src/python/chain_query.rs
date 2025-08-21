/// Python bindings for ChainQuery
/// 
/// Provides Python interface for direct blockchain database queries

use pyo3::prelude::*;
use pyo3::types::PyDict;
use alloy_primitives::{Address, B256, U256};
use std::str::FromStr;
use std::sync::Arc;

use ethtx::TxProcessor;

/// Python wrapper for ChainQuery
/// 
/// Usage:
///   import pyreth
///   query = pyreth.ChainQuery()  # DEPRECATED
///   # OR (preferred):
///   reth = pyreth.PyReth()
///   query = reth.chain_query()
#[pyclass(name = "ChainQuery")]
pub struct PyChainQuery {
    // Use the ChainQuery from TxProcessor when shared, standalone when created directly
    inner: Arc<ethtx::chain_query::ChainQuery>,
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
        
        // Hardcoded reth_datadir
        let reth_datadir = "/home/nima/.local/share/reth/mainnet";
        
        // Create tokio runtime for async operations
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to create runtime: {}", e)
            ))?;
        
        // Create ChainQuery
        let chain_query = ethtx::chain_query::ChainQuery::new(reth_datadir)
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
    
    // Account queries
    
    /// Get ETH balance for an address
    /// 
    /// Args:
    ///     address (str): Ethereum address
    ///     block_number (int, optional): Block number to query at (default: latest)
    /// 
    /// Returns:
    ///     str: Balance in wei as string
    fn get_balance(&self, address: &str, block_number: Option<u64>) -> PyResult<String> {
        let addr = Address::from_str(address.trim_start_matches("0x"))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Invalid address: {}", e)
            ))?;
        
        let chain_query = self.inner.clone();
        let balance = self.runtime.block_on(async move {
            chain_query.get_balance(addr, block_number).await
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Failed to get balance: {}", e)
        ))?;
        
        Ok(balance.to_string())
    }
    
    /// Get nonce for an address
    /// 
    /// Args:
    ///     address (str): Ethereum address
    ///     block_number (int, optional): Block number to query at (default: latest)
    /// 
    /// Returns:
    ///     int: Account nonce
    fn get_nonce(&self, address: &str, block_number: Option<u64>) -> PyResult<u64> {
        let addr = Address::from_str(address.trim_start_matches("0x"))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Invalid address: {}", e)
            ))?;
        
        let chain_query = self.inner.clone();
        self.runtime.block_on(async move {
            chain_query.get_nonce(addr, block_number).await
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Failed to get nonce: {}", e)
        ))
    }
    
    /// Check if an address has code (is a contract)
    /// 
    /// Args:
    ///     address (str): Ethereum address
    ///     block_number (int, optional): Block number to query at (default: latest)
    /// 
    /// Returns:
    ///     bool: True if address is a contract, False if EOA
    fn has_code(&self, address: &str, block_number: Option<u64>) -> PyResult<bool> {
        let addr = Address::from_str(address.trim_start_matches("0x"))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Invalid address: {}", e)
            ))?;
        
        let chain_query = self.inner.clone();
        self.runtime.block_on(async move {
            chain_query.has_code(addr, block_number).await
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Failed to check code: {}", e)
        ))
    }
    
    /// Get complete account information
    /// 
    /// Args:
    ///     address (str): Ethereum address
    ///     block_number (int, optional): Block number to query at (default: latest)
    /// 
    /// Returns:
    ///     dict: Account info with balance, nonce, and has_code
    fn get_account_info(&self, py: Python, address: &str, block_number: Option<u64>) -> PyResult<Py<PyDict>> {
        let addr = Address::from_str(address.trim_start_matches("0x"))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Invalid address: {}", e)
            ))?;
        
        let chain_query = self.inner.clone();
        let info = self.runtime.block_on(async move {
            chain_query.account.get_account_info(addr, block_number).await
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Failed to get account info: {}", e)
        ))?;
        
        let dict = PyDict::new(py);
        dict.set_item("address", crate::utils::to_checksum_address(&info.address))?;
        dict.set_item("balance", info.balance.to_string())?;
        dict.set_item("nonce", info.nonce)?;
        dict.set_item("has_code", info.has_code)?;
        
        Ok(dict.into())
    }
    
    // Token queries
    
    /// Get ERC20 token balance for an address
    /// 
    /// Args:
    ///     token_address (str): ERC20 token contract address
    ///     holder_address (str): Address to check balance for
    ///     block_number (int, optional): Block number to query at (default: latest)
    /// 
    /// Returns:
    ///     str: Token balance as string (in token's smallest unit)
    fn get_token_balance(&self, token_address: &str, holder_address: &str, block_number: Option<u64>) -> PyResult<String> {
        let token = Address::from_str(token_address.trim_start_matches("0x"))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Invalid token address: {}", e)
            ))?;
        
        let holder = Address::from_str(holder_address.trim_start_matches("0x"))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Invalid holder address: {}", e)
            ))?;
        
        let chain_query = self.inner.clone();
        let balance = self.runtime.block_on(async move {
            chain_query.get_token_balance(token, holder, block_number).await
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Failed to get token balance: {}", e)
        ))?;
        
        Ok(balance.to_string())
    }
    
    /// Get ERC20 token total supply
    /// 
    /// Args:
    ///     token_address (str): ERC20 token contract address
    ///     block_number (int, optional): Block number to query at (default: latest)
    /// 
    /// Returns:
    ///     str: Total supply as string
    fn get_token_total_supply(&self, token_address: &str, block_number: Option<u64>) -> PyResult<String> {
        let token = Address::from_str(token_address.trim_start_matches("0x"))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Invalid token address: {}", e)
            ))?;
        
        let chain_query = self.inner.clone();
        let supply = self.runtime.block_on(async move {
            chain_query.get_token_total_supply(token, block_number).await
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Failed to get total supply: {}", e)
        ))?;
        
        Ok(supply.to_string())
    }
    
    /// Get ERC20 token decimals
    /// 
    /// Args:
    ///     token_address (str): ERC20 token contract address
    ///     block_number (int, optional): Block number to query at (default: latest)
    /// 
    /// Returns:
    ///     int: Token decimals (defaults to 18 if not found)
    fn get_token_decimals(&self, token_address: &str, block_number: Option<u64>) -> PyResult<u8> {
        let token = Address::from_str(token_address.trim_start_matches("0x"))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Invalid token address: {}", e)
            ))?;
        
        let chain_query = self.inner.clone();
        self.runtime.block_on(async move {
            chain_query.token.get_erc20_decimals(token, block_number).await
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Failed to get decimals: {}", e)
        ))
    }
    
    /// Get ERC20 allowance
    /// 
    /// Args:
    ///     token_address (str): ERC20 token contract address
    ///     owner_address (str): Token owner address
    ///     spender_address (str): Approved spender address
    ///     block_number (int, optional): Block number to query at (default: latest)
    /// 
    /// Returns:
    ///     str: Allowance amount as string
    fn get_allowance(&self, token_address: &str, owner_address: &str, spender_address: &str, block_number: Option<u64>) -> PyResult<String> {
        let token = Address::from_str(token_address.trim_start_matches("0x"))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Invalid token address: {}", e)
            ))?;
        
        let owner = Address::from_str(owner_address.trim_start_matches("0x"))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Invalid owner address: {}", e)
            ))?;
        
        let spender = Address::from_str(spender_address.trim_start_matches("0x"))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Invalid spender address: {}", e)
            ))?;
        
        let chain_query = self.inner.clone();
        let allowance = self.runtime.block_on(async move {
            chain_query.token.get_allowance(token, owner, spender, block_number).await
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Failed to get allowance: {}", e)
        ))?;
        
        Ok(allowance.to_string())
    }
    
    // Storage queries
    
    /// Read storage at a specific slot
    /// 
    /// Args:
    ///     address (str): Contract address
    ///     slot (str): Storage slot as hex string
    ///     block_number (int, optional): Block number to query at (default: latest)
    /// 
    /// Returns:
    ///     str: Storage value as string
    fn get_storage_at(&self, address: &str, slot: &str, block_number: Option<u64>) -> PyResult<String> {
        let addr = Address::from_str(address.trim_start_matches("0x"))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Invalid address: {}", e)
            ))?;
        
        let slot_key = if slot.starts_with("0x") {
            B256::from_str(slot.trim_start_matches("0x"))
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    format!("Invalid slot: {}", e)
                ))?
        } else {
            // Try to parse as number
            let slot_num = slot.parse::<u64>()
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    format!("Invalid slot number: {}", e)
                ))?;
            B256::from(U256::from(slot_num))
        };
        
        let chain_query = self.inner.clone();
        let value = self.runtime.block_on(async move {
            chain_query.get_storage_at(addr, slot_key, block_number).await
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Failed to read storage: {}", e)
        ))?;
        
        Ok(value.to_string())
    }
    
    // Block queries
    
    /// Get block information
    /// 
    /// Args:
    ///     block_number (int, optional): Block number (default: latest)
    /// 
    /// Returns:
    ///     dict: Block info with number, timestamp, base_fee, gas_limit, gas_used
    fn get_block_info(&self, py: Python, block_number: Option<u64>) -> PyResult<Py<PyDict>> {
        let chain_query = self.inner.clone();
        let info = self.runtime.block_on(async move {
            chain_query.block.get_block_info(block_number).await
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Failed to get block info: {}", e)
        ))?;
        
        let dict = PyDict::new(py);
        dict.set_item("number", info.number)?;
        dict.set_item("timestamp", info.timestamp)?;
        dict.set_item("base_fee_per_gas", info.base_fee_per_gas)?;
        dict.set_item("gas_limit", info.gas_limit)?;
        dict.set_item("gas_used", info.gas_used)?;
        
        Ok(dict.into())
    }
    
    /// Get block timestamp
    /// 
    /// Args:
    ///     block_number (int, optional): Block number (default: latest)
    /// 
    /// Returns:
    ///     int: Unix timestamp
    fn get_block_timestamp(&self, block_number: Option<u64>) -> PyResult<u64> {
        let chain_query = self.inner.clone();
        self.runtime.block_on(async move {
            chain_query.block.get_block_timestamp(block_number).await
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Failed to get block timestamp: {}", e)
        ))
    }
    
    fn __repr__(&self) -> String {
        format!("ChainQuery(backend='Reth DB', path='/home/nima/.local/share/reth/mainnet')")
    }
}