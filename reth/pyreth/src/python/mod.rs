/// Python bindings for PyReth
/// 
/// Unified Python interface for Reth-based Ethereum tools including:
/// - Direct blockchain queries (ChainQuery)
/// - Transaction processing (TxProcessor)
/// - Transaction simulation (Simulator)
/// - Transaction building (TxBuilder)

use pyo3::prelude::*;

pub mod processed_transaction;
pub mod tx_processor;
pub mod simulator;
pub mod chain_query;

use processed_transaction::PyProcessedTransaction;
use tx_processor::PyTxProcessor;
use simulator::{PySimulator, PySequentialResult, PyTransactionResult};
use chain_query::PyChainQuery;

// Import TxBuilder from tx_builder crate
#[cfg(feature = "python")]
use tx_builder::python_bindings::PyTxBuilder;

/// Initialize the Python module
#[pymodule]
#[pyo3(name = "pyreth")]
fn pyreth_module(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    // Chain query class for direct DB access
    m.add_class::<PyChainQuery>()?;
    
    // Transaction processing classes
    m.add_class::<PyTxProcessor>()?;
    m.add_class::<PyProcessedTransaction>()?;
    
    // Transaction simulation classes
    m.add_class::<PySimulator>()?;
    m.add_class::<PySequentialResult>()?;
    m.add_class::<PyTransactionResult>()?;
    
    // Transaction builder class
    #[cfg(feature = "python")]
    m.add_class::<PyTxBuilder>()?;
    
    // Add module metadata
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__doc__", "PyReth - Python bindings for Reth-based Ethereum tools")?;
    m.add("__author__", "PyReth Team")?;
    
    Ok(())
}