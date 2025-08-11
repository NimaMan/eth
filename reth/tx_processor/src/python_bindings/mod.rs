/// Python bindings for tx_processor
/// 
/// Provides high-performance transaction processing for Python applications
/// through PyO3 bindings. This module exposes the ProcessedTransaction struct
/// and transaction processing capabilities to Python.

use pyo3::prelude::*;

pub mod processed_transaction;
#[path = "tx_processor_py.rs"]
pub mod tx_proc_impl;  // Rename to avoid conflict

use processed_transaction::PyProcessedTransaction;
use tx_proc_impl::PyTxProcessor;

/// Initialize the Python module
#[pymodule]
fn tx_processor_py(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyProcessedTransaction>()?;
    m.add_class::<PyTxProcessor>()?;
    Ok(())
}