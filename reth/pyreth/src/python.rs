/// Python bindings for PyReth
///
/// Unified Python interface for Reth-based Ethereum tools including:
/// - Direct blockchain queries (ChainQuery)
/// - Transaction processing (TxProcessor)
/// - Transaction simulation (Simulator)
/// - Transaction building (TxBuilder)
use pyo3::prelude::*;

use crate::agents::envs::eth15m_env::{
    PyEth15mFeeds, PyEth15mObservation, PyPolymarketTarget, PyWindowRecord,
};
use crate::chain_query::common_addresses::register as register_common_addresses;
use crate::chain_query::function_signatures::register as register_function_signatures;
use crate::chain_query::{
    PyAccount, PyAddressTransactionRef, PyAddressTxIndexer, PyBalanceChange, PyBalanceChanges,
    PyChainQuery, PyCompleteBalances, PyPoolLiquidityInfo, PyPortfolio, PyTokenMetadata,
    PyTransactionData,
};
use crate::dex::register as register_dex;
use crate::price_reader::{PyEthPriceClient, PyPriceData};
use crate::provider::{
    PyAddressProcessedTxProvider, PyProcessedBlock, PyProcessedTxProvider,
    PyTokenProcessedTxProvider,
};
use crate::pyreth_instance::{clear_singleton, is_singleton_initialized, PyRethInstance};
use crate::simulator::{
    PyPoolBuySellParameters, PyPoolBuySellSimulationResult, PyPoolBuySellSimulator,
    PySimulationResult, PySimulator,
};
use crate::tx_processor::py_processed_transaction::PyProcessedTransaction;
use crate::tx_processor::py_tx_processor::PyTxProcessor;

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

    // Chain query classes
    m.add_class::<PyChainQuery>()?;
    m.add_class::<PyAccount>()?;
    m.add_class::<PyPortfolio>()?;
    m.add_class::<PyBalanceChange>()?;
    m.add_class::<PyBalanceChanges>()?;
    m.add_class::<PyCompleteBalances>()?;
    m.add_class::<PyTokenMetadata>()?;
    m.add_class::<PyPoolLiquidityInfo>()?;
    m.add_class::<PyAddressTxIndexer>()?;
    m.add_class::<PyAddressTransactionRef>()?;
    m.add_class::<PyTransactionData>()?;

    // Transaction processor classes
    m.add_class::<PyTxProcessor>()?;
    m.add_class::<PyProcessedTransaction>()?;

    // Pool buy sell simulator classes
    m.add_class::<PyPoolBuySellSimulator>()?;
    m.add_class::<PyPoolBuySellSimulationResult>()?;
    m.add_class::<PyPoolBuySellParameters>()?;

    // Price reader classes (core only - dex_reader, cex_reader, arbitrage disabled)
    m.add_class::<PyEthPriceClient>()?;
    m.add_class::<PyPriceData>()?;

    // Processed transaction provider classes
    m.add_class::<PyProcessedTxProvider>()?;
    m.add_class::<PyAddressProcessedTxProvider>()?;
    m.add_class::<PyTokenProcessedTxProvider>()?;
    m.add_class::<PyProcessedBlock>()?;

    // ETH15m data bindings
    m.add_class::<PyEth15mFeeds>()?;
    m.add_class::<PyEth15mObservation>()?;
    m.add_class::<PyWindowRecord>()?;
    m.add_class::<PyPolymarketTarget>()?;

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
