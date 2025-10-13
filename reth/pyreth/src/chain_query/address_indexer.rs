use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use alloy_primitives::Address;
use pyo3::prelude::*;
use tokio::runtime::Runtime;

use reth_chain_query::reth_index::{AddressParticipation, AddressTxWriter, RethIndexDB};
use reth_chain_query::RethQueryProvider;

use crate::chain_query::PyAddressTransactionRef;
use crate::pyreth_instance::get_or_create_singleton;

const DEFAULT_DATADIR: &str = "/home/nima/.local/share/reth/mainnet";
fn resolve_datadir(datadir: Option<String>) -> String {
    datadir
        .or_else(|| std::env::var("PYRETH_DATADIR").ok())
        .unwrap_or_else(|| DEFAULT_DATADIR.to_string())
}

fn resolve_index_dir(datadir: &str, index_path: Option<String>) -> PathBuf {
    if let Some(path) = index_path.or_else(|| std::env::var("PYRETH_ADDRESS_INDEX_DIR").ok()) {
        return PathBuf::from(path);
    }
    // Default alongside the main Reth database to keep datasets co-located.
    Path::new(datadir).join("reth_index")
}

#[pyclass(name = "AddressTxIndexer")]
pub struct PyAddressTxIndexer {
    db: Arc<RethIndexDB>,
    writer: Option<AddressTxWriter>,
    provider: Arc<RethQueryProvider>,
    runtime: Arc<Runtime>,
    #[allow(dead_code)]
    datadir: String,
    read_only: bool,
}

impl PyAddressTxIndexer {
    fn new_internal(
        index_db: Arc<RethIndexDB>,
        writer: Option<AddressTxWriter>,
        provider: Arc<RethQueryProvider>,
        runtime: Arc<Runtime>,
        datadir: String,
        read_only: bool,
    ) -> Self {
        Self {
            db: index_db,
            writer,
            provider,
            runtime,
            datadir,
            read_only,
        }
    }
}

#[pymethods]
impl PyAddressTxIndexer {
    #[new]
    #[pyo3(signature = (datadir=None, index_path=None, read_only=None))]
    pub fn new(
        datadir: Option<String>,
        index_path: Option<String>,
        read_only: Option<bool>,
    ) -> PyResult<Self> {
        let datadir = resolve_datadir(datadir);
        let index_dir = resolve_index_dir(&datadir, index_path);
        let read_only = read_only.unwrap_or(false);

        if !read_only {
            std::fs::create_dir_all(&index_dir).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to create index directory {}: {}",
                    index_dir.display(),
                    e
                ))
            })?;
        } else if !index_dir.exists() {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "RethIndex directory {} does not exist",
                index_dir.display()
            )));
        }

        let index_db = if read_only {
            Arc::new(RethIndexDB::open_read_only(&index_dir).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to open RethIndex DB at {}: {}",
                    index_dir.display(),
                    e
                ))
            })?)
        } else {
            Arc::new(RethIndexDB::open(&index_dir).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to open RethIndex DB at {}: {}",
                    index_dir.display(),
                    e
                ))
            })?)
        };

        let simulator = get_or_create_singleton(&datadir).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to open Reth datadir {}: {}",
                datadir, e
            ))
        })?;

        let provider = RethQueryProvider::with_simulator(simulator).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to create RethQueryProvider: {}",
                e
            ))
        })?;

        let provider = Arc::new(provider.with_reth_index_db(index_db.clone()));
        let writer = if read_only {
            None
        } else {
            Some(AddressTxWriter::new(index_db.clone(), provider.clone()))
        };

        let runtime = Arc::new(Runtime::new().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to create tokio runtime: {}",
                e
            ))
        })?);

        Ok(Self::new_internal(
            index_db, writer, provider, runtime, datadir, read_only,
        ))
    }

    #[pyo3(signature = (block_number, transactions))]
    pub fn write_transactions(
        &self,
        block_number: u64,
        transactions: Vec<(u64, Vec<String>)>,
    ) -> PyResult<u64> {
        if self.read_only {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "AddressTxIndexer is read-only; writing is disabled",
            ));
        }

        if transactions.is_empty() {
            return Ok(0);
        }

        let mut participations: Vec<AddressParticipation> = Vec::with_capacity(transactions.len());

        for (tx_index, addresses) in transactions {
            if addresses.is_empty() {
                continue;
            }

            let mut parsed: Vec<Address> = Vec::with_capacity(addresses.len());
            for address_str in addresses {
                let normalized = address_str.trim();
                let without_prefix = normalized.strip_prefix("0x").unwrap_or(normalized);
                let address = Address::from_str(without_prefix).map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Invalid address {address_str}: {e}",
                    ))
                })?;
                parsed.push(address);
            }

            if parsed.is_empty() {
                continue;
            }

            participations.push(AddressParticipation {
                tx_index,
                addresses: parsed,
            });
        }

        if participations.is_empty() {
            return Ok(0);
        }

        let writer = self.writer.as_ref().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "AddressTxIndexer writer unavailable (read-only mode)",
            )
        })?;

        let inserted = writer
            .ingest_block_participation(block_number, participations)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(inserted as u64)
    }

    pub fn address_transactions(&self, address: &str) -> PyResult<Vec<PyAddressTransactionRef>> {
        let normalized = address.trim();
        let parsed = Address::from_str(normalized)
            .or_else(|_| {
                if normalized.starts_with("0x") {
                    Address::from_str(normalized.trim_start_matches("0x"))
                } else {
                    Address::from_str(&format!("0x{normalized}"))
                }
            })
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid address {address}: {e}"
                ))
            })?;

        let provider = self.provider.clone();
        let refs = self
            .runtime
            .block_on(async move { provider.transactions_for_address(parsed).await })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(refs
            .into_iter()
            .map(PyAddressTransactionRef::from)
            .collect())
    }

    pub fn block_has_indices(&self, block_number: u64) -> PyResult<bool> {
        Ok(self.provider.get_block_tx_indices(block_number).is_ok())
    }
}
