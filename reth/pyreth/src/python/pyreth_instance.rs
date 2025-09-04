/// Singleton PyReth instance that manages shared database connection
/// 
/// This ensures we only open the Reth database once and share it across
/// all components (TxProcessor, Simulator, ChainQuery), preventing the
/// "too many file watches" error.

use pyo3::prelude::*;
use std::sync::Arc;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use tx_simulator::TxSimulator;

use super::simulator::PySimulator;

/// Global singleton database connection
static DB_INSTANCE: Lazy<Arc<Mutex<Option<Arc<TxSimulator>>>>> = 
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
            let reth_datadir = "/home/nima/.local/share/reth/mainnet";
            
            let simulator = TxSimulator::new(reth_datadir)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    format!("Failed to open Reth database: {}", e)
                ))?;
            
            *instance = Some(Arc::new(simulator));
        }
        
        Ok(Self {
            simulator: instance.as_ref().unwrap().clone(),
        })
    }
    
    /// Get a simulator that uses the shared database
    pub fn simulator(&self) -> PySimulator {
        PySimulator::from_shared(self.simulator.clone())
    }
    
    /// Check if database is open
    pub fn is_connected(&self) -> bool {
        true // If we have self.simulator, we're connected
    }
    
    /// Get information about the database connection
    pub fn connection_info(&self) -> PyResult<String> {
        Ok(format!(
            "Connected to Reth database at /home/nima/.local/share/reth/mainnet\n\
             Shared instance: Yes\n\
             Available components: simulator()\n\
             Components can be created without additional file watchers"
        ))
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
        let simulator = Arc::new(
            TxSimulator::new(reth_datadir)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Failed to initialize TxSimulator: {}", e)))?
        );
        *instance = Some(simulator.clone());
        Ok(simulator)
    } else {
        Ok(instance.as_ref().unwrap().clone())
    }
}