/// Singleton PyReth instance that manages shared database connection
/// 
/// This ensures we only open the Reth database once and share it across
/// all components (TxProcessor, Simulator, ChainQuery), preventing the
/// "too many file watches" error.

use pyo3::prelude::*;
use std::sync::Arc;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use ethtx::TxProcessor;

use super::tx_processor::PyTxProcessor;
use super::simulator::PySimulator;
use super::chain_query::PyChainQuery;

/// Global singleton database connection
static DB_INSTANCE: Lazy<Arc<Mutex<Option<Arc<TxProcessor>>>>> = 
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
    processor: Arc<TxProcessor>,
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
            
            let processor = TxProcessor::new(reth_datadir)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    format!("Failed to open Reth database: {}", e)
                ))?;
            
            *instance = Some(Arc::new(processor));
        }
        
        Ok(Self {
            processor: instance.as_ref().unwrap().clone(),
        })
    }
    
    /// Get a transaction processor that uses the shared database
    pub fn tx_processor(&self) -> PyTxProcessor {
        PyTxProcessor::from_shared(self.processor.clone())
    }
    
    /// Get a simulator that uses the shared database
    pub fn simulator(&self) -> PySimulator {
        PySimulator::from_shared(self.processor.clone())
    }
    
    /// Get a chain query interface that uses the shared database
    pub fn chain_query(&self) -> PyChainQuery {
        PyChainQuery::from_shared(self.processor.clone())
    }
    
    /// Check if database is open
    pub fn is_connected(&self) -> bool {
        true // If we have self.processor, we're connected
    }
    
    /// Get information about the database connection
    pub fn connection_info(&self) -> PyResult<String> {
        Ok(format!(
            "Connected to Reth database at /home/nima/.local/share/reth/mainnet\n\
             Shared instance: Yes\n\
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