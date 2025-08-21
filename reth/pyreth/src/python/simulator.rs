/// Python wrapper for RethTxSimulator integrated into pyreth
/// 
/// Provides transaction simulation capabilities including:
/// - Single transaction simulation
/// - Sequential transaction simulation  
/// - State change analysis
/// - Transaction builder functionality

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::sync::Arc;
use tokio::runtime::Runtime;
use alloy_primitives::{Address, U256, Bytes, B256};
use std::str::FromStr;

use reth_tx_simulator::CallRequest;
use ethtx::TxProcessor;
use super::processed_transaction::PyProcessedTransaction;
use reth_tx_simulator::SequentialSimulationOptions;

/// Python wrapper for RethTxSimulator
/// 
/// Usage:
///   import pyreth
///   reth = pyreth.PyReth()
///   sim = reth.simulator()
///   result = sim.simulate_transaction({...})
#[pyclass(name = "Simulator")]
pub struct PySimulator {
    runtime: Arc<Runtime>,
    tx_processor: Arc<TxProcessor>,
}

impl PySimulator {
    /// Create from shared processor instance (used by PyReth)
    pub fn from_shared(processor: Arc<TxProcessor>) -> Self {
        let runtime = Runtime::new()
            .expect("Failed to create runtime");
        
        Self {
            runtime: Arc::new(runtime),
            tx_processor: processor,
        }
    }
}

/// Result for sequential simulation
#[pyclass]
#[derive(Clone)]
pub struct PySequentialResult {
    #[pyo3(get)]
    pub total_transactions: usize,
    
    #[pyo3(get)]
    pub successful_transactions: usize,
    
    #[pyo3(get)]
    pub failed_transactions: usize,
    
    #[pyo3(get)]
    pub total_gas_used: u64,
    
    #[pyo3(get)]
    pub sequence_success: bool,
    
    #[pyo3(get)]
    pub results: Vec<PyTransactionResult>,
}

/// Individual transaction result in a sequence
#[pyclass]
#[derive(Clone)]
pub struct PyTransactionResult {
    #[pyo3(get)]
    pub transaction_index: usize,
    
    #[pyo3(get)]
    pub success: bool,
    
    #[pyo3(get)]
    pub gas_used: u64,
    
    #[pyo3(get)]
    pub revert_reason: Option<String>,
}

#[pymethods]
impl PySimulator {
    /// Initialize simulator with Reth database
    /// 
    /// DEPRECATED: Use PyReth().simulator() instead to avoid multiple database connections
    #[new]
    pub fn new() -> PyResult<Self> {
        eprintln!("WARNING: Creating standalone Simulator is deprecated. Use PyReth().simulator() instead.");
        
        let reth_datadir = "/home/nima/.local/share/reth/mainnet";
        
        let runtime = Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        
        // Create TxProcessor which contains its own simulator
        let tx_processor = TxProcessor::new(reth_datadir)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            
        Ok(Self {
            runtime: Arc::new(runtime),
            tx_processor: Arc::new(tx_processor),
        })
    }
    
    /// Get latest block number
    pub fn get_latest_block(&self) -> PyResult<u64> {
        // Use the TxProcessor's method to get latest block
        let tx_processor = self.tx_processor.clone();
        self.runtime.block_on(async move {
            tx_processor.get_latest_block().await
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }
    
    /// Get the base fee for the latest block
    /// 
    /// Returns the base fee in wei (EIP-1559 base fee)
    pub fn get_latest_base_fee(&self) -> PyResult<u128> {
        let tx_processor = self.tx_processor.clone();
        self.runtime.block_on(async move {
            tx_processor.get_latest_base_fee().await
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }
    
    /// Get the base fee for a specific block
    /// 
    /// Args:
    ///     block_number (int): The block number to get base fee for
    ///     
    /// Returns:
    ///     int: Base fee in wei
    pub fn get_base_fee_at_block(&self, block_number: u64) -> PyResult<u128> {
        let tx_processor = self.tx_processor.clone();
        self.runtime.block_on(async move {
            tx_processor.get_base_fee_at_block(block_number).await
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }
    
    /// Get ETH balance for an address
    /// 
    /// Args:
    ///     address (str): Ethereum address
    ///     block_number (int, optional): Block number to query at (default: latest)
    /// 
    /// Returns:
    ///     str: Balance in wei as string
    pub fn get_balance(&self, address: &str, block_number: Option<u64>) -> PyResult<String> {
        let addr = Address::from_str(address.trim_start_matches("0x"))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Invalid address: {}", e)
            ))?;
        
        let tx_processor = self.tx_processor.clone();
        let balance = self.runtime.block_on(async move {
            tx_processor.get_balance(addr, block_number).await
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
    pub fn get_nonce(&self, address: &str, block_number: Option<u64>) -> PyResult<u64> {
        let addr = Address::from_str(address.trim_start_matches("0x"))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Invalid address: {}", e)
            ))?;
        
        let tx_processor = self.tx_processor.clone();
        self.runtime.block_on(async move {
            tx_processor.get_nonce(addr, block_number).await
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Failed to get nonce: {}", e)
        ))
    }
    
    /// Get ERC20 token balance for an address
    /// 
    /// Args:
    ///     token_address (str): ERC20 token contract address
    ///     holder_address (str): Address to check balance for
    ///     block_number (int, optional): Block number to query at (default: latest)
    /// 
    /// Returns:
    ///     str: Token balance as string (in token's smallest unit)
    pub fn get_token_balance(&self, token_address: &str, holder_address: &str, block_number: Option<u64>) -> PyResult<String> {
        let token = Address::from_str(token_address.trim_start_matches("0x"))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Invalid token address: {}", e)
            ))?;
        
        let holder = Address::from_str(holder_address.trim_start_matches("0x"))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Invalid holder address: {}", e)
            ))?;
        
        let tx_processor = self.tx_processor.clone();
        let balance = self.runtime.block_on(async move {
            tx_processor.get_token_balance(token, holder, block_number).await
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Failed to get token balance: {}", e)
        ))?;
        
        Ok(balance.to_string())
    }
    
    /// Build a transaction from simple parameters
    /// 
    /// This is a convenience method for creating transactions without
    /// manually specifying all the low-level details.
    /// 
    /// Args:
    ///     from_address (str): Sender address
    ///     to_address (str): Recipient address (optional for contract creation)
    ///     value (str or int): Value in wei (optional, default 0)
    ///     data (str): Transaction data (optional, default empty)
    ///     gas_limit (int): Gas limit (optional, default 21000)
    ///     gas_price (int): Gas price in wei (optional, default 20 gwei)
    ///     nonce (int): Transaction nonce (optional, will be auto-detected)
    ///     
    /// Returns:
    ///     dict: Transaction dictionary ready for simulation
    #[pyo3(signature = (from_address, to_address=None, value=None, data=None, gas_limit=None, gas_price=None, nonce=None))]
    pub fn build_transaction(
        &self,
        from_address: &str,
        to_address: Option<&str>,
        value: Option<&str>,
        data: Option<&str>,
        gas_limit: Option<u64>,
        gas_price: Option<u64>,
        nonce: Option<u64>,
    ) -> PyResult<Py<PyDict>> {
        Python::with_gil(|py| {
            let tx_dict = PyDict::new(py);
            
            // Required fields
            tx_dict.set_item("from", from_address)?;
            
            // Optional fields
            if let Some(to) = to_address {
                tx_dict.set_item("to", to)?;
            }
            
            if let Some(val) = value {
                tx_dict.set_item("value", val)?;
            } else {
                tx_dict.set_item("value", "0")?;
            }
            
            if let Some(d) = data {
                tx_dict.set_item("data", d)?;
            } else {
                tx_dict.set_item("data", "0x")?;
            }
            
            tx_dict.set_item("gas", gas_limit.unwrap_or(21000))?;
            tx_dict.set_item("gas_price", gas_price.unwrap_or(20_000_000_000))?; // 20 gwei
            
            if let Some(n) = nonce {
                tx_dict.set_item("nonce", n)?;
            }
            
            Ok(tx_dict.into())
        })
    }
    
    /// Simulate a single transaction
    /// 
    /// Args:
    ///     transaction (dict): Transaction parameters
    ///         - from: sender address (str)
    ///         - to: recipient address (str, optional for contract creation)
    ///         - value: value in wei (str or int, optional)
    ///         - data: transaction data (str, optional)
    ///         - gas: gas limit (int, optional)
    ///         - gas_price: gas price in wei (int, optional)
    ///         - nonce: transaction nonce (int, optional)
    ///     block_number (int, optional): Block number to simulate at (default: latest)
    ///     
    /// Returns:
    ///     ProcessedTransaction: Full transaction details with events and internal transactions
    pub fn simulate_transaction(&self, transaction: &PyDict, block_number: Option<u64>) -> PyResult<PyProcessedTransaction> {
        // Just call simulate_and_process - they should be the same
        self.simulate_and_process(transaction, block_number)
    }
    
    /// Simulate a transaction and return ProcessedTransaction
    /// 
    /// This method simulates a transaction and processes the result into a full
    /// ProcessedTransaction object with decoded events, internal transactions, and state changes.
    /// 
    /// Args:
    ///     transaction (dict): Transaction parameters
    ///         - from: sender address (str)
    ///         - to: recipient address (str, optional for contract creation)
    ///         - value: value in wei (str or int, optional, default 0)
    ///         - data: transaction data (str, optional, default empty)
    ///         - gas: gas limit (int, optional, default 3000000)
    ///         - gas_price: gas price in wei (int, optional, default 20 gwei)
    ///         - nonce: transaction nonce (int, optional, will auto-detect)
    ///     block_number (int, optional): Block number to simulate at (default: latest)
    ///     
    /// Returns:
    ///     ProcessedTransaction: Full transaction details with events and internal transactions
    pub fn simulate_and_process(&self, transaction: &PyDict, block_number: Option<u64>) -> PyResult<PyProcessedTransaction> {
        // Convert Python dict to CallRequest
        let call_request = dict_to_call_request(transaction)?;
        
        // Call the Rust core method that handles all the logic
        let tx_processor = self.tx_processor.clone();
        let processed_tx = self.runtime.block_on(async move {
            tx_processor.simulate_and_process_transaction(call_request, block_number).await
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Failed to process transaction: {}", e)
        ))?;
        
        // Convert to Python type
        Ok(PyProcessedTransaction::from_processed_transaction(processed_tx))
    }
    
    /// Simulate a sequence of transactions
    /// 
    /// Args:
    ///     transactions (list): List of transaction dictionaries
    ///     options (dict, optional): Sequential simulation options
    ///         - stop_on_failure: Stop if any transaction fails (bool, default: True)
    ///         - auto_increment_nonces: Auto-increment nonces (bool, default: True)
    ///         - at_block: Block number to simulate at (int, optional)
    ///         
    /// Returns:
    ///     PySequentialResult: Results for the entire sequence
    pub fn simulate_sequence(&self, transactions: Vec<&PyDict>, options: Option<&PyDict>) -> PyResult<PySequentialResult> {
        // Convert transactions
        let mut call_requests = Vec::new();
        for tx_dict in transactions {
            call_requests.push(dict_to_call_request(tx_dict)?);
        }
        
        // Parse options
        let sim_options = if let Some(opts) = options {
            SequentialSimulationOptions {
                stop_on_failure: opts.get_item("stop_on_failure")?
                    .map(|v| v.extract::<bool>()).transpose()?
                    .unwrap_or(true),
                auto_increment_nonces: opts.get_item("auto_increment_nonces")?
                    .map(|v| v.extract::<bool>()).transpose()?
                    .unwrap_or(true),
                at_block: opts.get_item("at_block")?
                    .map(|v| v.extract::<u64>()).transpose()?,
                gas_limit_per_tx: None,
            }
        } else {
            SequentialSimulationOptions::default()
        };
        
        let tx_processor = self.tx_processor.clone();
        let result = self.runtime.block_on(async move {
            tx_processor.simulate_transaction_sequence(call_requests, sim_options).await
        });
        
        match result {
            Ok(seq_result) => {
                let results = seq_result.results.into_iter().enumerate().map(|(idx, res)| {
                    PyTransactionResult {
                        transaction_index: idx,
                        success: res.success,
                        gas_used: res.gas_used,
                        revert_reason: res.revert_reason,
                    }
                }).collect();
                
                Ok(PySequentialResult {
                    total_transactions: seq_result.total_transactions,
                    successful_transactions: seq_result.successful_transactions,
                    failed_transactions: seq_result.failed_transactions,
                    total_gas_used: seq_result.total_gas_used,
                    sequence_success: seq_result.sequence_success,
                    results,
                })
            },
            Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string())),
        }
    }
    
    /// Simulate a sequence of transactions and return ProcessedTransaction for each
    /// 
    /// This method runs a sequence simulation and returns a ProcessedTransaction object
    /// for each successful transaction, or an error message for failed transactions.
    /// 
    /// Args:
    ///     transactions (list): List of transaction dictionaries
    ///     options (dict, optional): Sequential simulation options
    ///         - stop_on_failure: Stop if any transaction fails (bool, default: True)
    ///         - auto_increment_nonces: Auto-increment nonces (bool, default: True)
    ///         - at_block: Block number to simulate at (int, optional)
    ///         
    /// Returns:
    ///     list: List where each element is either a ProcessedTransaction (success) 
    ///           or an error dict with 'error' key (failure)
    pub fn simulate_sequence_with_details(&self, transactions: Vec<&PyDict>, options: Option<&PyDict>) -> PyResult<Vec<PyObject>> {
        // Convert transactions
        let mut call_requests = Vec::new();
        for tx_dict in transactions {
            call_requests.push(dict_to_call_request(tx_dict)?);
        }
        
        // Parse options
        let sim_options = if let Some(opts) = options {
            SequentialSimulationOptions {
                stop_on_failure: opts.get_item("stop_on_failure")?
                    .map(|v| v.extract::<bool>()).transpose()?
                    .unwrap_or(true),
                auto_increment_nonces: opts.get_item("auto_increment_nonces")?
                    .map(|v| v.extract::<bool>()).transpose()?
                    .unwrap_or(true),
                at_block: opts.get_item("at_block")?
                    .map(|v| v.extract::<u64>()).transpose()?,
                gas_limit_per_tx: None,
            }
        } else {
            SequentialSimulationOptions::default()
        };
        
        let tx_processor = self.tx_processor.clone();
        let results = self.runtime.block_on(async move {
            tx_processor.simulate_sequence_with_details(call_requests, sim_options).await
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Failed to simulate sequence: {}", e)
        ))?;
        
        // Convert results to Python objects
        Python::with_gil(|py| {
            let mut py_results = Vec::new();
            
            for result in results {
                match result {
                    Ok(processed_tx) => {
                        // Convert ProcessedTransaction to Python
                        let py_tx = PyProcessedTransaction::from_processed_transaction(processed_tx);
                        py_results.push(py_tx.into_py(py));
                    },
                    Err(error) => {
                        // Create error dict
                        let error_dict = PyDict::new(py);
                        error_dict.set_item("error", error.to_string())?;
                        py_results.push(error_dict.into_py(py));
                    }
                }
            }
            
            Ok(py_results)
        })
    }
    
    /// Build an ERC20 approve transaction
    /// 
    /// Args:
    ///     from_address (str): Sender address
    ///     token_address (str): ERC20 token contract address
    ///     spender (str): Address to approve
    ///     amount (str or int): Amount to approve (in token units, not wei)
    ///     
    /// Returns:
    ///     dict: Approve transaction ready for simulation
    pub fn build_approve(
        &self,
        from_address: &str,
        token_address: &str,
        spender: &str,
        amount: &str,
    ) -> PyResult<Py<PyDict>> {
        // Build approve(address,uint256) calldata
        let spender_clean = spender.trim_start_matches("0x").to_lowercase();
        let amount_hex = if amount.starts_with("0x") {
            amount.trim_start_matches("0x").to_string()
        } else {
            // Convert decimal string to hex
            let amount_int = amount.parse::<u128>()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid amount"))?;
            format!("{:064x}", amount_int)
        };
        
        let calldata = format!(
            "0x095ea7b3{:0>24}{}{}", 
            "0".repeat(24),
            spender_clean,
            amount_hex
        );
        
        self.build_transaction(
            from_address,
            Some(token_address),
            None,  // No ETH value for approve
            Some(&calldata),
            Some(50000),  // Standard gas for approve
            None,
            None,
        )
    }
    
    /// Build a Uniswap V2 swap transaction
    /// 
    /// Args:
    ///     from_address (str): Sender address
    ///     router_address (str): Uniswap V2 Router address (default: 0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D)
    ///     token_in (str): Input token address (use "ETH" for ETH)
    ///     token_out (str): Output token address (use "ETH" for ETH)
    ///     amount_in (str): Amount to swap (in wei for ETH, token units for tokens)
    ///     amount_out_min (str): Minimum output amount (default: 0)
    ///     
    /// Returns:
    ///     dict: Swap transaction ready for simulation
    #[pyo3(signature = (from_address, token_in, token_out, amount_in, router_address=None, amount_out_min=None))]
    pub fn build_swap(
        &self,
        from_address: &str,
        token_in: &str,
        token_out: &str,
        amount_in: &str,
        router_address: Option<&str>,
        amount_out_min: Option<&str>,
    ) -> PyResult<Py<PyDict>> {
        let router = router_address.unwrap_or("0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D");
        let weth_address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
        
        // Determine swap type and build calldata
        let is_eth_in = token_in.to_uppercase() == "ETH";
        let is_eth_out = token_out.to_uppercase() == "ETH";
        
        // Convert amounts to hex
        let amount_in_hex = if amount_in.starts_with("0x") {
            amount_in.trim_start_matches("0x").to_string()
        } else {
            let amount = amount_in.parse::<u128>()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid amount_in"))?;
            format!("{:064x}", amount)
        };
        
        let amount_out_min_hex = if let Some(min) = amount_out_min {
            if min.starts_with("0x") {
                min.trim_start_matches("0x").to_string()
            } else {
                let amount = min.parse::<u128>()
                    .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid amount_out_min"))?;
                format!("{:064x}", amount)
            }
        } else {
            "0".repeat(64)  // Default to 0 minimum
        };
        
        // Generate deadline (20 minutes from now)
        let deadline = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() + 1200;
        let deadline_hex = format!("{:064x}", deadline);
        
        let (method_id, value, calldata) = if is_eth_in && !is_eth_out {
            // ETH -> Token: swapExactETHForTokens
            let path = vec![weth_address, token_out];
            let calldata = format!(
                "0x7ff36ab5{}{:0>64}{:0>64}{}{:064x}{}{}",
                amount_out_min_hex,
                "80",  // path offset
                &from_address.trim_start_matches("0x").to_lowercase(),
                deadline_hex,
                path.len(),
                "0".repeat(24) + &weth_address.trim_start_matches("0x").to_lowercase(),
                "0".repeat(24) + &token_out.trim_start_matches("0x").to_lowercase(),
            );
            ("swapExactETHForTokens", Some(amount_in), calldata)
        } else if !is_eth_in && is_eth_out {
            // Token -> ETH: swapExactTokensForETH
            let path = vec![token_in, weth_address];
            let calldata = format!(
                "0x18cbafe5{}{}{:0>64}{:0>64}{}{:064x}{}{}",
                amount_in_hex,
                amount_out_min_hex,
                "a0",  // path offset
                &from_address.trim_start_matches("0x").to_lowercase(),
                deadline_hex,
                path.len(),
                "0".repeat(24) + &token_in.trim_start_matches("0x").to_lowercase(),
                "0".repeat(24) + &weth_address.trim_start_matches("0x").to_lowercase(),
            );
            ("swapExactTokensForETH", None, calldata)
        } else if !is_eth_in && !is_eth_out {
            // Token -> Token: swapExactTokensForTokens
            let path = vec![token_in, weth_address, token_out];  // Usually goes through WETH
            let calldata = format!(
                "0x38ed1739{}{}{:0>64}{:0>64}{}{:064x}{}{}{}",
                amount_in_hex,
                amount_out_min_hex,
                "a0",  // path offset
                &from_address.trim_start_matches("0x").to_lowercase(),
                deadline_hex,
                path.len(),
                "0".repeat(24) + &token_in.trim_start_matches("0x").to_lowercase(),
                "0".repeat(24) + &weth_address.trim_start_matches("0x").to_lowercase(),
                "0".repeat(24) + &token_out.trim_start_matches("0x").to_lowercase(),
            );
            ("swapExactTokensForTokens", None, calldata)
        } else {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>("Cannot swap ETH to ETH"));
        };
        
        self.build_transaction(
            from_address,
            Some(router),
            value,
            Some(&calldata),
            Some(200000),  // Standard gas for swaps
            None,
            None,
        )
    }
    
    /// Build a transaction from a hash for simulation
    /// 
    /// Fetches an existing transaction from the database and returns it
    /// in a format ready for simulation. Useful for replaying transactions
    /// or building sequential simulations from real transactions.
    /// 
    /// Args:
    ///     tx_hash (str): Transaction hash to fetch (0x prefixed hex string)
    ///     
    /// Returns:
    ///     dict: Transaction dictionary ready for simulation with all parameters
    ///     
    /// Example:
    ///     ```python
    ///     # Fetch an existing swap transaction
    ///     tx = sim.build_transaction_from_hash("0xabc123...")
    ///     
    ///     # Use it in a sequence with other transactions
    ///     approve_tx = sim.build_approve(...)
    ///     sell_tx = sim.build_swap(...)
    ///     
    ///     results = sim.simulate_sequence_with_details([tx, approve_tx, sell_tx])
    ///     ```
    pub fn build_transaction_from_hash(&self, tx_hash: &str) -> PyResult<Py<PyDict>> {
        use alloy_primitives::B256;
        use std::str::FromStr;
        
        // Parse transaction hash
        let hash = B256::from_str(tx_hash)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Invalid transaction hash: {}", e)
            ))?;
        
        // Fetch transaction data from DB
        let tx_processor = self.tx_processor.clone();
        let call_request = self.runtime.block_on(async move {
            tx_processor.get_transaction_for_simulation(hash).await
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Failed to fetch transaction: {}", e)
        ))?;
        
        // Convert CallRequest to Python dict
        Python::with_gil(|py| {
            let tx_dict = PyDict::new(py);
            
            // Add from address (required)
            if let Some(from) = call_request.from {
                tx_dict.set_item("from", crate::utils::checksum::alloy_address_to_checksum(from))?;
            }
            
            // Add to address (optional)
            if let Some(to) = call_request.to {
                tx_dict.set_item("to", crate::utils::checksum::alloy_address_to_checksum(to))?;
            }
            
            // Add value
            if let Some(value) = call_request.value {
                tx_dict.set_item("value", value.to_string())?;
            } else {
                tx_dict.set_item("value", "0")?;
            }
            
            // Add data
            if let Some(data) = call_request.data {
                tx_dict.set_item("data", format!("0x{}", hex::encode(data)))?;
            } else {
                tx_dict.set_item("data", "0x")?;
            }
            
            // Add gas parameters
            tx_dict.set_item("gas", call_request.gas.unwrap_or(3_000_000))?;
            tx_dict.set_item("gas_price", call_request.gas_price.unwrap_or(20_000_000_000))?;
            
            // Add nonce if present
            if let Some(nonce) = call_request.nonce {
                tx_dict.set_item("nonce", nonce)?;
            }
            
            Ok(tx_dict.into())
        })
    }
    
    /// Get simulator version information
    pub fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }
    
    fn __repr__(&self) -> String {
        format!("Simulator(backend='reth_tx_simulator', version='{}')", 
                env!("CARGO_PKG_VERSION"))
    }
}

// Helper functions for conversion

/// Convert Python dict to CallRequest
fn dict_to_call_request(tx_dict: &PyDict) -> PyResult<CallRequest> {
    let from = tx_dict.get_item("from")?
        .map(|v| v.extract::<String>())
        .transpose()?
        .map(|s| Address::from_str(&s.trim_start_matches("0x")))
        .transpose()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid from address: {}", e)))?;
    
    let to = tx_dict.get_item("to")?
        .map(|v| v.extract::<String>())
        .transpose()?
        .map(|s| Address::from_str(&s.trim_start_matches("0x")))
        .transpose()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid to address: {}", e)))?;
    
    let value = tx_dict.get_item("value")?
        .map(|v| {
            if let Ok(s) = v.extract::<String>() {
                U256::from_str(&s.trim_start_matches("0x"))
                    .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid value: {}", e)))
            } else if let Ok(n) = v.extract::<u64>() {
                Ok(U256::from(n))
            } else {
                Err(PyErr::new::<pyo3::exceptions::PyValueError, _>("Value must be string or int"))
            }
        })
        .transpose()?;
    
    let data = tx_dict.get_item("data")?
        .map(|v| v.extract::<String>())
        .transpose()?
        .map(|s| {
            if s.is_empty() || s == "0x" {
                Ok(Bytes::new())
            } else {
                hex::decode(&s.trim_start_matches("0x"))
                    .map(Bytes::from)
                    .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid data: {}", e)))
            }
        })
        .transpose()?;
    
    let gas = tx_dict.get_item("gas")?
        .map(|v| v.extract::<u64>())
        .transpose()?;
    
    let gas_price = tx_dict.get_item("gas_price")?
        .map(|v| v.extract::<u128>())
        .transpose()?;
    
    let nonce = tx_dict.get_item("nonce")?
        .map(|v| v.extract::<u64>())
        .transpose()?;
    
    Ok(CallRequest {
        from,
        to,
        value,
        data,
        gas,
        gas_price,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        nonce,
    })
}

