/// Python bindings for tx_processor
/// 
/// Provides high-performance transaction processing for Python applications
/// through PyO3 bindings. This module exposes the ProcessedTransaction struct,
/// transaction processing capabilities, simulation, and transaction building.

use pyo3::prelude::*;

pub mod processed_transaction;
pub mod rs_tx_processor;
pub mod simulator;
pub mod chain_query;

use processed_transaction::PyProcessedTransaction;
use rs_tx_processor::PyTxProcessor;
use simulator::{PySimulator, PySequentialResult, PyTransactionResult};
use chain_query::PyChainQuery;

// Import TxBuilder from tx_builder crate
#[cfg(feature = "python")]
use tx_builder::python_bindings::PyTxBuilder;

/// Initialize the Python module
#[pymodule]
#[pyo3(name = "ethtx")]
fn ethtx_module(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    // Transaction processing classes
    m.add_class::<PyProcessedTransaction>()?;
    m.add_class::<PyTxProcessor>()?;
    
    // Transaction simulation classes
    m.add_class::<PySimulator>()?;
    
    // Chain query class for direct DB access
    m.add_class::<PyChainQuery>()?;
    
    // Transaction builder class (new!)
    #[cfg(feature = "python")]
    m.add_class::<PyTxBuilder>()?;
    
    // Add module version
    m.add("__version__", "0.2.0")?;
    m.add("__doc__", "High-performance Ethereum transaction processing, simulation, and building")?;
    
    Ok(())
}