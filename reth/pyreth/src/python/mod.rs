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
pub mod pyreth_instance;
pub mod trading_simulator;
pub mod chain_state_persisting_sequential_tx_simulator;
pub mod price_reader;

use processed_transaction::PyProcessedTransaction;
use tx_processor::PyTxProcessor;
use simulator::{PySimulator, PySequentialResult, PyTransactionResult};
use chain_query::PyChainQuery;
use pyreth_instance::{PyRethInstance, clear_singleton, is_singleton_initialized};
use trading_simulator::{PyTradingSimulator, PyTradingSequenceResult, PyBuySellConfig};
use chain_state_persisting_sequential_tx_simulator::{
    PyChainStatePersistingSequentialTxSimulator,
    PyPoolViabilityResult,
    analyze_pool_viability
};
use price_reader::{PyEthPriceClient, PyPriceData};

// Import TxBuilder from tx_builder crate
#[cfg(feature = "python")]
use tx_builder::python_bindings::PyTxBuilder;

/// Initialize the Python module
#[pymodule]
#[pyo3(name = "pyreth")]
fn pyreth_module(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    // Main singleton instance for shared database connection
    m.add_class::<PyRethInstance>()?;
    
    // Utility functions for singleton management
    m.add_function(wrap_pyfunction!(clear_singleton, m)?)?;
    m.add_function(wrap_pyfunction!(is_singleton_initialized, m)?)?;
    
    // Chain query class for direct DB access
    m.add_class::<PyChainQuery>()?;
    
    // Transaction processing classes
    m.add_class::<PyTxProcessor>()?;
    m.add_class::<PyProcessedTransaction>()?;
    
    // Transaction simulation classes
    m.add_class::<PySimulator>()?;
    m.add_class::<PySequentialResult>()?;
    m.add_class::<PyTransactionResult>()?;
    
    // Trading simulation classes
    m.add_class::<PyTradingSimulator>()?;
    m.add_class::<PyTradingSequenceResult>()?;
    m.add_class::<PyBuySellConfig>()?;
    
    // Chain state persisting sequential simulation classes
    m.add_class::<PyChainStatePersistingSequentialTxSimulator>()?;
    m.add_class::<PyPoolViabilityResult>()?;
    m.add_function(wrap_pyfunction!(analyze_pool_viability, m)?)?;
    
    // Transaction builder class
    #[cfg(feature = "python")]
    m.add_class::<PyTxBuilder>()?;
    
    // Price reader classes
    m.add_class::<PyEthPriceClient>()?;
    m.add_class::<PyPriceData>()?;
    
    // Add module metadata
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__doc__", "PyReth - Python bindings for Reth-based Ethereum tools")?;
    m.add("__author__", "PyReth Team")?;
    
    Ok(())
}