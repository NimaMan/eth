use alloy_primitives::{Address, Bytes, U256};
/// Python wrapper for TxSimulator - Minimal simulation functionality
///
/// Provides transaction simulation capabilities using tx_simulator module
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::str::FromStr;
use std::sync::Arc;
use tokio::runtime::Runtime;

use tx_simulator::{TxSimulator, UnsignedTransaction};

/// Python wrapper for TxSimulator
#[pyclass(name = "Simulator")]
pub struct PySimulator {
    runtime: Arc<Runtime>,
    simulator: Arc<TxSimulator>,
}

impl PySimulator {
    /// Create from shared simulator instance (used by PyReth)
    pub fn from_shared(simulator: Arc<TxSimulator>) -> Self {
        let runtime = Runtime::new().expect("Failed to create runtime");

        Self {
            runtime: Arc::new(runtime),
            simulator,
        }
    }
}

/// Python simulation result
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
    pub return_data: String,
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

        let simulator = TxSimulator::new(reth_datadir)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(Self {
            runtime: Arc::new(runtime),
            simulator: Arc::new(simulator),
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
    ///     PySimulationResult: Basic simulation result with success/gas/error info
    pub fn simulate_transaction(
        &self,
        transaction: &PyDict,
        block_number: Option<u64>,
    ) -> PyResult<PySimulationResult> {
        // Convert Python dict to UnsignedTransaction
        let unsigned_tx = dict_to_unsigned_transaction(transaction)?;

        let simulator = self.simulator.clone();
        let result = self
            .runtime
            .block_on(async move {
                let mut chain = simulator.start_simulation_chain(block_number).await?;
                chain.step(unsigned_tx).await
            })
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to simulate transaction: {}",
                    e
                ))
            })?;

        Ok(PySimulationResult {
            success: result.success,
            gas_used: result.gas_used,
            revert_reason: result.revert_reason,
            return_data: "".to_string(), // Basic SimulationResult doesn't have output data
        })
    }

    /// Build a transaction from simple parameters
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

    /// Get simulator version information
    pub fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    fn __repr__(&self) -> String {
        format!(
            "Simulator(backend='tx_simulator', version='{}')",
            env!("CARGO_PKG_VERSION")
        )
    }
}

/// Convert Python dict to UnsignedTransaction
fn dict_to_unsigned_transaction(tx_dict: &PyDict) -> PyResult<UnsignedTransaction> {
    let from = tx_dict
        .get_item("from")?
        .map(|v| v.extract::<String>())
        .transpose()?
        .map(|s| Address::from_str(&s.trim_start_matches("0x")))
        .transpose()
        .map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid from address: {}", e))
        })?;

    let to = tx_dict
        .get_item("to")?
        .map(|v| v.extract::<String>())
        .transpose()?
        .map(|s| Address::from_str(&s.trim_start_matches("0x")))
        .transpose()
        .map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid to address: {}", e))
        })?;

    let value = tx_dict
        .get_item("value")?
        .map(|v| {
            if let Ok(s) = v.extract::<String>() {
                U256::from_str(&s.trim_start_matches("0x")).map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid value: {}", e))
                })
            } else if let Ok(n) = v.extract::<u64>() {
                Ok(U256::from(n))
            } else {
                Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "Value must be string or int",
                ))
            }
        })
        .transpose()?;

    let data = tx_dict
        .get_item("data")?
        .map(|v| v.extract::<String>())
        .transpose()?
        .map(|s| {
            if s.is_empty() || s == "0x" {
                Ok(Bytes::new())
            } else {
                hex::decode(&s.trim_start_matches("0x"))
                    .map(Bytes::from)
                    .map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                            "Invalid data: {}",
                            e
                        ))
                    })
            }
        })
        .transpose()?;

    let gas = tx_dict
        .get_item("gas")?
        .map(|v| {
            if let Ok(s) = v.extract::<String>() {
                s.parse::<u64>().map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid gas: {}", e))
                })
            } else if let Ok(n) = v.extract::<u64>() {
                Ok(n)
            } else {
                Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "Gas must be string or int",
                ))
            }
        })
        .transpose()?;

    let gas_price = tx_dict
        .get_item("gas_price")?
        .map(|v| {
            if let Ok(s) = v.extract::<String>() {
                s.parse::<u128>().map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Invalid gas_price: {}",
                        e
                    ))
                })
            } else if let Ok(n) = v.extract::<u128>() {
                Ok(n)
            } else {
                Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "Gas price must be string or int",
                ))
            }
        })
        .transpose()?;

    let nonce = tx_dict
        .get_item("nonce")?
        .map(|v| v.extract::<u64>())
        .transpose()?;

    Ok(UnsignedTransaction {
        from,
        to,
        value,
        data,
        gas,
        gas_price,
        nonce,
        ..Default::default()
    })
}
