use std::str::FromStr;
use std::sync::Arc;

use alloy_primitives::{Address, B256};
use pyo3::prelude::*;
use tokio::runtime::Runtime;

use tx_processor::{
    AddressProcessedTxProvider as CoreAddressProvider, ProcessedBlock,
    ProcessedTxProvider as CoreProcessedTxProvider, TokenProcessedTxProvider as CoreTokenProvider,
};
use tx_simulator::TxSimulator;

use crate::tx_processor::py_processed_transaction::PyProcessedTransaction;

const DEFAULT_DATADIR: &str = "/home/nima/.local/share/reth/mainnet";

#[pyclass(name = "ProcessedBlock")]
#[derive(Clone)]
pub struct PyProcessedBlock {
    #[pyo3(get)]
    pub number: u64,
    #[pyo3(get)]
    pub hash: String,
    #[pyo3(get)]
    pub parent_hash: String,
    #[pyo3(get)]
    pub timestamp: u64,
    #[pyo3(get)]
    pub gas_used: u64,
    #[pyo3(get)]
    pub gas_limit: u64,
    #[pyo3(get)]
    pub base_fee_per_gas: Option<String>,
    #[pyo3(get)]
    pub transactions: Vec<PyProcessedTransaction>,
}

impl From<ProcessedBlock> for PyProcessedBlock {
    fn from(block: ProcessedBlock) -> Self {
        let transactions = block
            .transactions
            .into_iter()
            .map(|tx| PyProcessedTransaction::from(tx.processed))
            .collect();

        Self {
            number: block.header.number,
            hash: format!("0x{}", hex::encode(block.header.hash)),
            parent_hash: format!("0x{}", hex::encode(block.header.parent_hash)),
            timestamp: block.header.timestamp,
            gas_used: block.header.gas_used,
            gas_limit: block.header.gas_limit,
            base_fee_per_gas: block.header.base_fee_per_gas.map(|fee| fee.to_string()),
            transactions,
        }
    }
}

#[derive(Clone)]
#[pyclass(name = "ProcessedTxProvider")]
pub struct PyProcessedTxProvider {
    core: Arc<CoreProcessedTxProvider>,
    runtime: Arc<Runtime>,
}

impl PyProcessedTxProvider {
    pub fn from_simulator(simulator: Arc<TxSimulator>) -> PyResult<Self> {
        let runtime = Runtime::new().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to create tokio runtime: {}",
                e
            ))
        })?;

        let provider_factory = simulator.provider_factory().clone();
        let core =
            CoreProcessedTxProvider::with_provider_factory(provider_factory).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to create ProcessedTxProvider: {}",
                    e
                ))
            })?;

        Ok(Self {
            core: Arc::new(core),
            runtime: Arc::new(runtime),
        })
    }

    fn new_internal(core: CoreProcessedTxProvider, runtime: Runtime) -> Self {
        Self {
            core: Arc::new(core),
            runtime: Arc::new(runtime),
        }
    }
}

#[pymethods]
impl PyProcessedTxProvider {
    #[new]
    pub fn new() -> PyResult<Self> {
        let datadir =
            std::env::var("PYRETH_DATADIR").unwrap_or_else(|_| DEFAULT_DATADIR.to_string());

        let runtime = Runtime::new().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to create tokio runtime: {}",
                e
            ))
        })?;

        let core = CoreProcessedTxProvider::new(&datadir).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to create ProcessedTxProvider: {}",
                e
            ))
        })?;

        Ok(Self::new_internal(core, runtime))
    }

    pub fn process_block(&self, block_number: u64) -> PyResult<PyProcessedBlock> {
        let core = self.core.clone();
        let block = self
            .runtime
            .block_on(async move { core.process_block(block_number).await })
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to process block {}: {}",
                    block_number, e
                ))
            })?;

        Ok(block.into())
    }

    pub fn process_block_batch(&self, block_numbers: Vec<u64>) -> PyResult<Vec<PyProcessedBlock>> {
        let core = self.core.clone();
        let blocks = self
            .runtime
            .block_on(async move { core.process_block_batch(block_numbers, None).await })
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to process block batch: {}",
                    e
                ))
            })?;

        Ok(blocks.into_iter().map(Into::into).collect())
    }

    pub fn address_provider(&self) -> PyResult<PyAddressProcessedTxProvider> {
        PyAddressProcessedTxProvider::from_core(self.core.clone(), self.runtime.clone())
    }

    pub fn token_provider(&self) -> PyResult<PyTokenProcessedTxProvider> {
        PyTokenProcessedTxProvider::from_core(self.core.clone(), self.runtime.clone())
    }

    pub fn processed_transaction_by_hash(
        &self,
        tx_hash: String,
    ) -> PyResult<PyProcessedTransaction> {
        let hash = parse_hash(&tx_hash)?;
        let core = self.core.clone();
        let tx = self
            .runtime
            .block_on(async move { core.process_transaction_by_hash(hash).await })
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to process transaction {}: {}",
                    tx_hash, e
                ))
            })?;

        Ok(PyProcessedTransaction::from(tx))
    }

    pub fn load_transaction_from_hash(&self, tx_hash: String) -> PyResult<PyProcessedTransaction> {
        let hash = parse_hash(&tx_hash)?;
        let core = self.core.clone();
        let tx = self
            .runtime
            .block_on(async move { core.load_transaction_from_hash_db_only(hash).await })
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to load transaction {}: {}",
                    tx_hash, e
                ))
            })?;

        Ok(PyProcessedTransaction::from(tx))
    }

    pub fn get_latest_block(&self) -> PyResult<u64> {
        self.runtime
            .block_on(async { self.core.get_latest_block().await })
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to fetch latest block: {}",
                    e
                ))
            })
    }
}

#[pyclass(name = "AddressProcessedTxProvider")]
pub struct PyAddressProcessedTxProvider {
    provider: Arc<CoreAddressProvider>,
    runtime: Arc<Runtime>,
}

impl PyAddressProcessedTxProvider {
    fn from_core(core: Arc<CoreProcessedTxProvider>, runtime: Arc<Runtime>) -> PyResult<Self> {
        let provider = CoreAddressProvider::new(core).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to create AddressProcessedTxProvider: {}",
                e
            ))
        })?;

        Ok(Self {
            provider: Arc::new(provider),
            runtime,
        })
    }
}

#[pymethods]
impl PyAddressProcessedTxProvider {
    pub fn load_blocks_for_address(
        &self,
        address: String,
        start_block: u64,
        end_block: u64,
    ) -> PyResult<()> {
        let address = parse_address(&address)?;
        let provider = self.provider.clone();

        self.runtime
            .block_on(async move {
                provider
                    .load_blocks_for_address(address, start_block, end_block)
                    .await
            })
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to load blocks for address {}: {}",
                    address, e
                ))
            })
    }

    pub fn transactions_for(&self, address: String) -> PyResult<Vec<PyProcessedTransaction>> {
        let address = parse_address(&address)?;
        let provider = self.provider.clone();

        let txs = self
            .runtime
            .block_on(async move { provider.transactions_for(address).await });

        Ok(txs.into_iter().map(Into::into).collect())
    }

    pub fn transaction_by_hash(&self, tx_hash: String) -> PyResult<PyProcessedTransaction> {
        let hash = parse_hash(&tx_hash)?;
        let provider = self.provider.clone();

        let tx = self
            .runtime
            .block_on(async move { provider.transaction_by_hash(hash).await })
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to fetch transaction {}: {}",
                    tx_hash, e
                ))
            })?;

        Ok(PyProcessedTransaction::from(tx))
    }
}

#[pyclass(name = "TokenProcessedTxProvider")]
pub struct PyTokenProcessedTxProvider {
    provider: Arc<CoreTokenProvider>,
    runtime: Arc<Runtime>,
}

impl PyTokenProcessedTxProvider {
    fn from_core(core: Arc<CoreProcessedTxProvider>, runtime: Arc<Runtime>) -> PyResult<Self> {
        let provider = CoreTokenProvider::new(core).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to create TokenProcessedTxProvider: {}",
                e
            ))
        })?;

        Ok(Self {
            provider: Arc::new(provider),
            runtime,
        })
    }
}

#[pymethods]
impl PyTokenProcessedTxProvider {
    pub fn load_blocks_for_token(
        &self,
        token: String,
        start_block: u64,
        end_block: u64,
    ) -> PyResult<()> {
        let token = parse_address(&token)?;
        let provider = self.provider.clone();

        self.runtime
            .block_on(async move {
                provider
                    .load_blocks_for_token(token, start_block, end_block)
                    .await
            })
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to load blocks for token {}: {}",
                    token, e
                ))
            })
    }

    pub fn transactions_for(&self, token: String) -> PyResult<Vec<PyProcessedTransaction>> {
        let token = parse_address(&token)?;
        let provider = self.provider.clone();

        let txs = self
            .runtime
            .block_on(async move { provider.transactions_for(token).await });

        Ok(txs.into_iter().map(Into::into).collect())
    }

    pub fn transaction_by_hash(&self, tx_hash: String) -> PyResult<PyProcessedTransaction> {
        let hash = parse_hash(&tx_hash)?;
        let provider = self.provider.clone();

        let tx = self
            .runtime
            .block_on(async move { provider.transaction_by_hash(hash).await })
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to fetch transaction {}: {}",
                    tx_hash, e
                ))
            })?;

        Ok(PyProcessedTransaction::from(tx))
    }
}

fn parse_address(value: &str) -> PyResult<Address> {
    Address::from_str(value.trim_start_matches("0x")).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid address {}: {}", value, e))
    })
}

fn parse_hash(value: &str) -> PyResult<B256> {
    B256::from_str(value.trim_start_matches("0x")).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Invalid transaction hash {}: {}",
            value, e
        ))
    })
}
