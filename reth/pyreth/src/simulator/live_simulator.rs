use super::simulator::{dict_to_unsigned_transaction, PySimulationResult};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tx_simulator::{LiveTxSimulator, TxSimulator};

/// Python wrapper for live-first transaction simulation.
#[pyclass(name = "LiveTxSimulator")]
pub struct PyLiveTxSimulator {
    runtime: Arc<Runtime>,
    simulator: LiveTxSimulator,
}

impl PyLiveTxSimulator {
    pub fn from_shared(simulator: Arc<TxSimulator>) -> Self {
        let runtime = Runtime::new().expect("Failed to create runtime");
        Self {
            runtime: Arc::new(runtime),
            simulator: LiveTxSimulator::from_simulator(simulator),
        }
    }
}

#[pymethods]
impl PyLiveTxSimulator {
    /// Initialize a standalone live simulator.
    ///
    /// Prefer `PyReth().live_simulator()` so all PyReth components share one
    /// database connection.
    #[new]
    pub fn new() -> PyResult<Self> {
        eprintln!(
            "WARNING: Creating standalone LiveTxSimulator is deprecated. Use PyReth().live_simulator() instead."
        );

        let reth_datadir = std::env::var("PYRETH_DATADIR")
            .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());
        let simulator = LiveTxSimulator::new(&reth_datadir)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        let runtime = Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(Self {
            runtime: Arc::new(runtime),
            simulator,
        })
    }

    /// Latest block with an exact tracked state source: Redis overlay first, MDBX fallback.
    pub fn latest_state_block_number(&self) -> PyResult<u64> {
        self.runtime
            .block_on(self.simulator.latest_state_block_number())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Latest block announced by live Redis, if available.
    pub fn latest_live_block_number(&self) -> PyResult<Option<u64>> {
        self.runtime
            .block_on(self.simulator.latest_live_block_number())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Latest block persisted in MDBX.
    pub fn latest_persisted_block_number(&self) -> PyResult<u64> {
        self.simulator
            .latest_persisted_block_number()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Simulate one transaction against the latest tracked live state.
    pub fn simulate_transaction(
        &self,
        transaction: &Bound<'_, PyDict>,
    ) -> PyResult<PySimulationResult> {
        let unsigned_tx = dict_to_unsigned_transaction(transaction)?;
        let simulator = self.simulator.clone();
        let result = self
            .runtime
            .block_on(async move { simulator.simulate_transaction(unsigned_tx).await })
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to simulate live transaction: {}",
                    e
                ))
            })?;

        Ok(PySimulationResult {
            success: result.success,
            gas_used: result.gas_used,
            revert_reason: result.revert_reason,
            return_data: "".to_string(),
        })
    }

    /// Simulate one transaction at an explicit block.
    pub fn simulate_transaction_at_block(
        &self,
        transaction: &Bound<'_, PyDict>,
        block_number: u64,
    ) -> PyResult<PySimulationResult> {
        let unsigned_tx = dict_to_unsigned_transaction(transaction)?;
        let simulator = self.simulator.clone();
        let result = self
            .runtime
            .block_on(async move {
                let mut chain = simulator.start_chain_at(block_number).await?;
                chain.step(unsigned_tx).await
            })
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to simulate transaction at block {}: {}",
                    block_number, e
                ))
            })?;

        Ok(PySimulationResult {
            success: result.success,
            gas_used: result.gas_used,
            revert_reason: result.revert_reason,
            return_data: "".to_string(),
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "LiveTxSimulator(backend='tx_simulator', version='{}')",
            env!("CARGO_PKG_VERSION")
        )
    }
}
