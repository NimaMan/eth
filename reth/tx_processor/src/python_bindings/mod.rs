/// Python bindings for tx_processor
/// 
/// Provides high-performance transaction processing for Python applications
/// through PyO3 bindings. This module exposes the ProcessedTransaction struct
/// and transaction processing capabilities to Python.

use pyo3::prelude::*;

pub mod processed_transaction;
pub mod rs_tx_processor;

use processed_transaction::PyProcessedTransaction;
use rs_tx_processor::PyTxProcessor;

/// Initialize the Python module
#[pymodule]
#[pyo3(name = "rs_tx_processor")]
fn rs_tx_processor_module(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyProcessedTransaction>()?;
    m.add_class::<PyTxProcessor>()?;
    Ok(())
}