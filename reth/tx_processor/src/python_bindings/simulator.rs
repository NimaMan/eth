/// Python wrapper for RethTxSimulator integrated into rs_tx_processor
/// 
/// Provides transaction simulation capabilities including:
/// - Single transaction simulation
/// - Sequential transaction simulation  
/// - State change analysis
/// - Transaction builder functionality

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Runtime;
use alloy_primitives::{Address, U256, Bytes};
use std::str::FromStr;

use crate::{RethTxSimulator, CallRequest};
use reth_tx_simulator::SequentialSimulationOptions;

/// Python wrapper for RethTxSimulator
/// 
/// Usage:
///   import rs_tx_processor
///   sim = rs_tx_processor.Simulator()
///   result = sim.simulate_transaction({...})
#[pyclass(name = "Simulator")]
pub struct PySimulator {
    simulator: Arc<RethTxSimulator>,
    runtime: Arc<Runtime>,
}

/// Result for single transaction simulation
#[pyclass]
#[derive(Clone)]
pub struct PySimulationResult {
    #[pyo3(get)]
    pub success: bool,
    
    #[pyo3(get)]
    pub gas_used: u64,
    
    #[pyo3(get)]
    pub revert_reason: Option<String>,
    
    #[pyo3(get)]
    pub state_changes: HashMap<String, PyAddressStateChange>,
}

/// State changes for a specific address
#[pyclass]
#[derive(Clone)]
pub struct PyAddressStateChange {
    #[pyo3(get)]
    pub eth_net: f64,
    
    #[pyo3(get)]
    pub token_net: HashMap<String, f64>,
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
    /// Uses the same hardcoded path as TxProcessor for consistency
    #[new]
    pub fn new() -> PyResult<Self> {
        let reth_datadir = "/home/nima/.local/share/reth/mainnet";
        
        let simulator = RethTxSimulator::new(reth_datadir)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        
        let runtime = Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            
        Ok(Self {
            simulator: Arc::new(simulator),
            runtime: Arc::new(runtime),
        })
    }
    
    /// Get latest block number
    pub fn get_latest_block(&self) -> PyResult<u64> {
        self.simulator.get_latest_block()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
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
    ///     PySimulationResult: Simulation results with state changes
    pub fn simulate_transaction(&self, transaction: &PyDict, block_number: Option<u64>) -> PyResult<PySimulationResult> {
        let call_request = dict_to_call_request(transaction)?;
        
        let result = if let Some(block) = block_number {
            self.runtime.block_on(async {
                self.simulator.simulate_unsigned_transaction_with_call_trace_at_block(call_request, block).await
            })
        } else {
            self.runtime.block_on(async {
                self.simulator.simulate_unsigned_transaction_with_call_trace(call_request).await
            })
        };
        
        match result {
            Ok(state_changes) => {
                let mut py_state_changes = HashMap::new();
                
                for (addr, changes) in state_changes {
                    let addr_str = format!("{:?}", addr);
                    py_state_changes.insert(addr_str, PyAddressStateChange {
                        eth_net: i256_to_f64(changes.eth_net),
                        token_net: changes.token_net.into_iter()
                            .map(|(k, v)| (k, i256_to_f64(v)))
                            .collect(),
                    });
                }
                
                Ok(PySimulationResult {
                    success: true,
                    gas_used: 0, // TODO: Add gas tracking
                    revert_reason: None,
                    state_changes: py_state_changes,
                })
            },
            Err(e) => Ok(PySimulationResult {
                success: false,
                gas_used: 0,
                revert_reason: Some(e.to_string()),
                state_changes: HashMap::new(),
            }),
        }
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
        
        let result = self.runtime.block_on(async {
            self.simulator.simulate_transaction_sequence(call_requests, sim_options).await
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

/// Convert I256 to f64 for Python compatibility
fn i256_to_f64(value: alloy_primitives::I256) -> f64 {
    let value_str = value.to_string();
    value_str.parse().unwrap_or(0.0)
}