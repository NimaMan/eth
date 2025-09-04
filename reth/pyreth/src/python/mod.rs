/// Python bindings for PyReth
/// 
/// Unified Python interface for Reth-based Ethereum tools including:
/// - Direct blockchain queries (ChainQuery)
/// - Transaction processing (TxProcessor)
/// - Transaction simulation (Simulator)
/// - Transaction building (TxBuilder)

use pyo3::prelude::*;

pub mod simulator;
pub mod pyreth_instance;

use simulator::{PySimulator, PySimulationResult};
use pyreth_instance::{PyRethInstance, clear_singleton, is_singleton_initialized};

/// Initialize the Python module
#[pymodule]
#[pyo3(name = "pyreth")]
pub fn pyreth_module(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    // Main singleton instance for shared database connection
    m.add_class::<PyRethInstance>()?;
    
    // Utility functions for singleton management
    m.add_function(wrap_pyfunction!(clear_singleton, m)?)?;
    m.add_function(wrap_pyfunction!(is_singleton_initialized, m)?)?;
    
    // Transaction simulation classes
    m.add_class::<PySimulator>()?;
    m.add_class::<PySimulationResult>()?;
    
    // Add module metadata
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__doc__", "PyReth - Python bindings for Ethereum transaction simulation")?;
    m.add("__author__", "PyReth Team")?;
    
    Ok(())
}