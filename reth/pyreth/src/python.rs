/// Python bindings for PyReth
///
/// Unified Python interface for Reth-based Ethereum tools including:
/// - Direct blockchain queries (ChainQuery)
/// - Transaction processing (TxProcessor)
/// - Transaction simulation (Simulator)
/// - Transaction building (TxBuilder)
use pyo3::prelude::*;

use crate::chain_query::common_addresses::register as register_common_addresses;
use crate::chain_query::function_signatures::register as register_function_signatures;
use crate::chain_query::{
    PyAccount, PyAddressBlockParticipationIndexFetcher, PyAddressBlockParticipationIndexer,
    PyBalanceChange, PyBalanceChanges, PyChainQuery, PyCompleteBalances, PyPoolLiquidityInfo,
    PyPortfolio, PyTokenMetadata, PyTransactionData,
};
use crate::dex::register as register_dex;
use crate::provider::{
    PyAddressProcessedTxProvider, PyProcessedBlock, PyProcessedTxProvider,
    PyTokenProcessedTxProvider,
};
use crate::pyreth_instance::{
    block_processor, chain_query, clear_singleton, is_singleton_initialized, live_simulator,
    pool_buy_sell_simulator, processed_tx_provider, simulator, tx_processor, PyRethInstance,
};
use crate::simulator::{
    PyLiveTxSimulator, PyPoolBuySellParameters, PyPoolBuySellSimulationResult,
    PyPoolBuySellSimulator, PySimulationResult, PySimulator,
};
use crate::tx_processor::py_processed_transaction::PyProcessedTransaction;
use crate::tx_processor::py_tx_processor::PyTxProcessor;

/// Initialize the Python module
#[pymodule]
#[pyo3(name = "pyreth")]
pub fn pyreth_module(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Main singleton instance for shared database connection
    m.add_class::<PyRethInstance>()?;

    // Utility functions for singleton management
    m.add_function(wrap_pyfunction!(clear_singleton, m)?)?;
    m.add_function(wrap_pyfunction!(is_singleton_initialized, m)?)?;

    // Singleton-backed component accessors
    m.add_function(wrap_pyfunction!(block_processor, m)?)?;
    m.add_function(wrap_pyfunction!(processed_tx_provider, m)?)?;
    m.add_function(wrap_pyfunction!(tx_processor, m)?)?;
    m.add_function(wrap_pyfunction!(chain_query, m)?)?;
    m.add_function(wrap_pyfunction!(simulator, m)?)?;
    m.add_function(wrap_pyfunction!(live_simulator, m)?)?;
    m.add_function(wrap_pyfunction!(pool_buy_sell_simulator, m)?)?;

    // Transaction simulation classes
    m.add_class::<PySimulator>()?;
    m.add_class::<PyLiveTxSimulator>()?;
    m.add_class::<PySimulationResult>()?;

    // Chain query classes
    m.add_class::<PyChainQuery>()?;
    m.add_class::<PyAccount>()?;
    m.add_class::<PyPortfolio>()?;
    m.add_class::<PyBalanceChange>()?;
    m.add_class::<PyBalanceChanges>()?;
    m.add_class::<PyCompleteBalances>()?;
    m.add_class::<PyTokenMetadata>()?;
    m.add_class::<PyPoolLiquidityInfo>()?;
    m.add_class::<PyAddressBlockParticipationIndexer>()?;
    m.add_class::<PyAddressBlockParticipationIndexFetcher>()?;
    m.add_class::<PyTransactionData>()?;

    // Transaction processor classes
    m.add_class::<PyTxProcessor>()?;
    m.add_class::<PyProcessedTransaction>()?;

    // Pool buy sell simulator classes
    m.add_class::<PyPoolBuySellSimulator>()?;
    m.add_class::<PyPoolBuySellSimulationResult>()?;
    m.add_class::<PyPoolBuySellParameters>()?;

    // Transaction processor classes
    m.add_class::<PyProcessedTxProvider>()?;
    m.add_class::<PyAddressProcessedTxProvider>()?;
    m.add_class::<PyTokenProcessedTxProvider>()?;
    m.add_class::<PyProcessedBlock>()?;

    // Common addresses utilities
    register_common_addresses(m)?;
    register_function_signatures(m)?;
    register_dex(m)?;

    // Add module metadata
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add(
        "__doc__",
        "PyReth - Python bindings for Ethereum transaction simulation",
    )?;
    m.add("__author__", "PyReth Team")?;

    Ok(())
}
