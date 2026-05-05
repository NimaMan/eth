use once_cell::sync::Lazy;
use parking_lot::Mutex;
/// Singleton PyReth instance that manages shared database connection
///
/// This ensures we only open the Reth database once and share it across
/// all components (TxProcessor, Simulator, ChainQuery), preventing the
/// "too many file watches" error.
use pyo3::prelude::*;
use std::sync::Arc;
use tx_processor::tx_processor::TxProcessor;
use tx_simulator::TxSimulator;

use super::chain_query::PyChainQuery;
use super::provider::PyProcessedTxProvider;
use super::simulator::PyLiveTxSimulator;
use super::simulator::PyPoolBuySellSimulator;
use super::simulator::PySimulator;
use super::tx_processor::py_tx_processor::PyTxProcessor;

/// Global singleton database connection
static DB_INSTANCE: Lazy<Arc<Mutex<Option<Arc<TxSimulator>>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

static PROVIDER_INSTANCE: Lazy<Arc<Mutex<Option<Arc<PyProcessedTxProvider>>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

/// Main PyReth instance that owns the shared database connection
///
/// Usage:
///   import pyreth
///   
///   # Create main instance (opens database once)
///   reth = pyreth.PyReth()
///   
///   # Get components that share the database
///   processor = reth.tx_processor()
///   simulator = reth.simulator()
///   query = reth.chain_query()
#[pyclass(name = "PyReth")]
pub struct PyRethInstance {
    simulator: Arc<TxSimulator>,
    processed_tx_provider: Arc<PyProcessedTxProvider>,
}

#[pymethods]
impl PyRethInstance {
    /// Initialize PyReth with shared database connection
    ///
    /// This will create the database connection on first call and reuse it
    /// for all subsequent instances.
    #[new]
    pub fn new() -> PyResult<Self> {
        let mut instance = DB_INSTANCE.lock();

        if instance.is_none() {
            // Create only once - this is where the database is opened
            // Allow overriding datadir via PYRETH_DATADIR env var
            let reth_datadir = std::env::var("PYRETH_DATADIR")
                .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());

            let simulator = TxSimulator::new(&reth_datadir).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to open Reth database: {}",
                    e
                ))
            })?;

            *instance = Some(Arc::new(simulator));
        }

        let simulator = instance.as_ref().unwrap().clone();

        let mut provider_instance = PROVIDER_INSTANCE.lock();
        if provider_instance.is_none() {
            let provider = PyProcessedTxProvider::from_simulator(simulator.clone())?;
            *provider_instance = Some(Arc::new(provider));
        }

        Ok(Self {
            simulator,
            processed_tx_provider: provider_instance.as_ref().unwrap().clone(),
        })
    }

    /// Get a simulator that uses the shared database
    pub fn simulator(&self) -> PySimulator {
        PySimulator::from_shared(self.simulator.clone())
    }

    /// Get a live-first simulator that targets the latest tracked Redis state.
    pub fn live_simulator(&self) -> PyLiveTxSimulator {
        PyLiveTxSimulator::from_shared(self.simulator.clone())
    }

    /// Get a chain query interface that uses the shared database
    pub fn chain_query(&self) -> PyResult<PyChainQuery> {
        PyChainQuery::from_simulator(self.simulator.clone())
    }

    /// Get a tx processor that uses the shared database
    pub fn tx_processor(&self) -> PyResult<PyTxProcessor> {
        PyTxProcessor::from_simulator(self.simulator.clone())
    }

    /// Get a processed transaction provider that uses the shared database
    pub fn processed_tx_provider(&self) -> PyResult<PyProcessedTxProvider> {
        Ok((*self.processed_tx_provider).clone())
    }

    /// Get a pool buy sell simulator that uses the shared database
    pub fn pool_buy_sell_simulator(&self) -> PyResult<PyPoolBuySellSimulator> {
        // Create TxProcessor - it doesn't need provider factory for simple operations
        let processor = Arc::new(TxProcessor::new());

        PyPoolBuySellSimulator::from_shared(self.simulator.clone(), processor)
    }

    /// Check if database is open
    pub fn is_connected(&self) -> bool {
        true // If we have self.simulator, we're connected
    }

    /// Get information about the database connection
    pub fn connection_info(&self) -> PyResult<String> {
        let datadir = std::env::var("PYRETH_DATADIR")
            .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());
        Ok(format!(
            "Connected to Reth database at {}\n\
             Shared instance: Yes\n\
             Available components: simulator(), live_simulator(), chain_query(), tx_processor(), pool_buy_sell_simulator(), price_client()\n\
             Components can be created without additional file watchers"
        , datadir))
    }
}

/// Clear the singleton instance (mainly for testing)
///
/// This forces the next PyReth() call to create a new database connection.
/// Use with caution as it may leave existing components with stale references.
#[pyfunction]
pub fn clear_singleton() -> PyResult<()> {
    let mut instance = DB_INSTANCE.lock();
    *instance = None;
    let mut provider = PROVIDER_INSTANCE.lock();
    *provider = None;
    Ok(())
}

/// Check if singleton is initialized
#[pyfunction]
pub fn is_singleton_initialized() -> bool {
    DB_INSTANCE.lock().is_some()
}

/// Get or create singleton instance (internal use)
pub fn get_or_create_singleton(reth_datadir: &str) -> PyResult<Arc<TxSimulator>> {
    let mut instance = DB_INSTANCE.lock();

    if instance.is_none() {
        let simulator = Arc::new(TxSimulator::new(reth_datadir).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!(
                "Failed to initialize TxSimulator: {}",
                e
            ))
        })?);
        *instance = Some(simulator.clone());
        Ok(simulator)
    } else {
        Ok(instance.as_ref().unwrap().clone())
    }
}
