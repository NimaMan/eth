use alloy_primitives::{I256, U256};
/// Python wrapper for ProcessedTransaction (PyO3)
///
/// Converts Rust ProcessedTransaction to Python-compatible format
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyFloat, PyList, PyLong, PySet};
use reth_chain_query::common_addresses::denom_tokens::ERC20_TOKEN_DECIMALS;
use reth_chain_query::to_checksum_address;
use serde_json::Value as JsonValue;
use tx_processor::{ProcessedBlockTransactions, ProcessedTransaction};

/// Convert serde_json::Value to Python object
fn json_to_python(py: Python, value: &JsonValue) -> PyResult<PyObject> {
    match value {
        JsonValue::Null => Ok(py.None()),
        JsonValue::Bool(b) => Ok(b.to_object(py)),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.to_object(py))
            } else if let Some(u) = n.as_u64() {
                Ok(u.to_object(py))
            } else if let Some(f) = n.as_f64() {
                Ok(f.to_object(py))
            } else {
                Ok(n.to_string().to_object(py))
            }
        }
        JsonValue::String(s) => {
            // Preserve numeric-looking strings as strings to avoid precision loss
            Ok(s.to_object(py))
        }
        JsonValue::Array(arr) => {
            let list = PyList::empty_bound(py);
            for item in arr {
                list.append(json_to_python(py, item)?)?;
            }
            Ok(list.into())
        }
        JsonValue::Object(map) => {
            let dict = PyDict::new_bound(py);
            for (key, val) in map {
                dict.set_item(key, json_to_python(py, val)?)?;
            }
            Ok(dict.into())
        }
    }
}

fn u256_to_py(py: Python, value: &U256) -> PyResult<PyObject> {
    let decimal = value.to_string();
    let int_type = py.get_type_bound::<PyLong>();
    Ok(int_type.call1((decimal,))?.into())
}

/// Python-compatible ProcessedTransaction
#[pyclass(name = "ProcessedTransaction")]
#[derive(Clone)]
pub struct PyProcessedTransaction {
    #[pyo3(get)]
    pub hash: String,
    #[pyo3(get)]
    pub block_number: u64,
    #[pyo3(get)]
    pub block_timestamp: u64,
    #[pyo3(get)]
    pub tx_index: u64,
    #[pyo3(get)]
    pub from_address: String,
    #[pyo3(get)]
    pub to_address: Option<String>,
    #[pyo3(get)]
    pub contract_address: Option<String>,
    #[pyo3(get)]
    pub value: String,
    #[pyo3(get)]
    pub status: bool,
    #[pyo3(get)]
    pub nonce: u64,
    #[pyo3(get)]
    pub raw_tx_type: u8,
    #[pyo3(get)]
    pub tx_type: String,
    #[pyo3(get)]
    pub actions: Vec<String>,
    #[pyo3(get)]
    pub input: String,
    #[pyo3(get)]
    pub tx_number: Option<u64>,

    // Store the original for internal use
    pub(crate) inner: ProcessedTransaction,
}

impl PyProcessedTransaction {
    /// Create from Rust ProcessedTransaction
    pub fn from_processed_transaction(ptx: ProcessedTransaction) -> Self {
        let checksum_from = to_checksum_address(&ptx.from_address);

        Self {
            hash: format!("0x{}", hex::encode(ptx.hash)),
            block_number: ptx.block_number,
            block_timestamp: ptx.block_timestamp,
            tx_index: ptx.tx_index,
            from_address: checksum_from,
            to_address: ptx.to_address.map(|a| to_checksum_address(&a)),
            contract_address: ptx.contract_address.map(|a| to_checksum_address(&a)),
            value: ptx.value.to_string(),
            status: ptx.status,
            nonce: ptx.nonce,
            raw_tx_type: ptx.raw_tx_type,
            tx_type: ptx.tx_type.clone(),
            actions: ptx.actions.clone(),
            input: format!("0x{}", hex::encode(&ptx.input)),
            tx_number: None,
            inner: ptx,
        }
    }

    pub fn from_block_transaction(tx: ProcessedBlockTransactions) -> Self {
        let processed = tx.processed;
        let mut this = Self::from_processed_transaction(processed);
        this.tx_number = Some(tx.metadata.tx_number);
        this
    }

    /// Convenience: wrap a slice of Rust ProcessedTransaction into Python wrappers
    pub fn from_list(ptxs: Vec<ProcessedTransaction>) -> Vec<Self> {
        ptxs.into_iter().map(Self::from).collect()
    }

    /// Get the inner ProcessedTransaction (for internal use)
    pub(crate) fn to_processed_transaction(&self) -> ProcessedTransaction {
        self.inner.clone()
    }
}

impl From<ProcessedTransaction> for PyProcessedTransaction {
    fn from(ptx: ProcessedTransaction) -> Self {
        Self::from_processed_transaction(ptx)
    }
}

impl From<&ProcessedTransaction> for PyProcessedTransaction {
    fn from(ptx: &ProcessedTransaction) -> Self {
        Self::from_processed_transaction(ptx.clone())
    }
}

#[pymethods]
impl PyProcessedTransaction {
    /// Convert to a Python dict matching eth_data.tx_processor ProcessedTransaction schema
    fn to_dict(&self, py: Python) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new_bound(py);

        // Core fields
        dict.set_item("hash", &self.hash)?;
        dict.set_item("block_number", self.block_number)?;
        dict.set_item("block_timestamp", self.block_timestamp)?;
        dict.set_item("tx_index", self.tx_index)?;
        dict.set_item("from_address", &self.from_address)?;
        dict.set_item("to_address", &self.to_address)?;
        dict.set_item("contract_address", &self.contract_address)?;
        dict.set_item("value", u256_to_py(py, &self.inner.value)?)?;
        dict.set_item("status", &self.status)?;
        dict.set_item("nonce", self.nonce)?;
        dict.set_item("raw_tx_type", self.raw_tx_type)?;
        dict.set_item("input", &self.input)?;
        dict.set_item("tx_type", &self.tx_type)?;
        dict.set_item("actions", &self.actions)?;
        dict.set_item("tx_number", self.tx_number)?;

        // Fees
        let fees = PyDict::new_bound(py);
        fees.set_item("gas_price", u256_to_py(py, &self.inner.fees.gas_price)?)?;
        fees.set_item("gas_used", self.inner.fees.gas_used)?;
        fees.set_item("gas_limit", self.inner.fees.gas_limit)?;
        fees.set_item("tx_fee", u256_to_py(py, &self.inner.fees.tx_fee)?)?;
        fees.set_item("protocol_type", &self.inner.fees.protocol_type)?;
        if let Some(v) = &self.inner.fees.max_fee_per_gas {
            fees.set_item("max_fee_per_gas", u256_to_py(py, v)?)?;
        } else {
            fees.set_item("max_fee_per_gas", py.None())?;
        }
        if let Some(v) = &self.inner.fees.max_priority_fee {
            fees.set_item("max_priority_fee", u256_to_py(py, v)?)?;
        } else {
            fees.set_item("max_priority_fee", py.None())?;
        }
        if let Some(v) = &self.inner.fees.max_fee_per_blob_gas {
            fees.set_item("max_fee_per_blob_gas", u256_to_py(py, v)?)?;
        } else {
            fees.set_item("max_fee_per_blob_gas", py.None())?;
        }
        if let Some(v) = self.inner.fees.blob_gas_used {
            fees.set_item("blob_gas_used", v)?;
        } else {
            fees.set_item("blob_gas_used", py.None())?;
        }
        dict.set_item("fees", fees)?;
        dict.set_item("bribe_amount", u256_to_py(py, &self.inner.bribe_amount)?)?;

        // Sets to lists
        let uniq = PyList::empty_bound(py);
        for a in &self.inner.unique_addresses {
            uniq.append(to_checksum_address(a))?;
        }
        dict.set_item("unique_addresses", uniq)?;

        let erc20c = PyList::empty_bound(py);
        for a in &self.inner.erc20_contracts {
            erc20c.append(to_checksum_address(a))?;
        }
        dict.set_item("erc20_contracts", erc20c)?;

        let erc721c = PyList::empty_bound(py);
        for a in &self.inner.erc721_contracts {
            erc721c.append(to_checksum_address(a))?;
        }
        dict.set_item("erc721_contracts", erc721c)?;

        let erc1155c = PyList::empty_bound(py);
        for a in &self.inner.erc1155_contracts {
            erc1155c.append(to_checksum_address(a))?;
        }
        dict.set_item("erc1155_contracts", erc1155c)?;

        // Helper: list of serde-serializable structs -> Python list of dicts
        fn vec_to_pylist<T: serde::Serialize>(py: Python, v: &Vec<T>) -> PyResult<Py<PyList>> {
            let list = PyList::empty_bound(py);
            for item in v {
                let j = serde_json::to_value(item).unwrap_or(JsonValue::Null);
                list.append(json_to_python(py, &j)?)?;
            }
            Ok(list.unbind())
        }

        // Transfers and events
        dict.set_item(
            "eth_transfers",
            vec_to_pylist(py, &self.inner.eth_transfers)?,
        )?;
        dict.set_item(
            "erc20_transfers",
            vec_to_pylist(py, &self.inner.erc20_transfers)?,
        )?;
        dict.set_item(
            "erc721_transfers",
            vec_to_pylist(py, &self.inner.erc721_transfers)?,
        )?;
        dict.set_item(
            "erc1155_transfers",
            vec_to_pylist(py, &self.inner.erc1155_transfers)?,
        )?;
        dict.set_item(
            "internal_transactions",
            vec_to_pylist(py, &self.inner.internal_transactions)?,
        )?;

        dict.set_item(
            "uniswap_v2_syncs",
            vec_to_pylist(py, &self.inner.uniswap_v2_syncs)?,
        )?;
        dict.set_item(
            "uniswap_v2_swaps",
            vec_to_pylist(py, &self.inner.uniswap_v2_swaps)?,
        )?;

        dict.set_item(
            "erc20_approval_events",
            vec_to_pylist(py, &self.inner.erc20_approval_events)?,
        )?;
        dict.set_item(
            "erc721_approval_events",
            vec_to_pylist(py, &self.inner.erc721_approval_events)?,
        )?;
        dict.set_item(
            "uniswap_v2_mints",
            vec_to_pylist(py, &self.inner.uniswap_v2_mints)?,
        )?;
        dict.set_item(
            "uniswap_v2_burns",
            vec_to_pylist(py, &self.inner.uniswap_v2_burns)?,
        )?;
        dict.set_item(
            "deposit_events",
            vec_to_pylist(py, &self.inner.deposit_events)?,
        )?;
        dict.set_item(
            "withdraw_events",
            vec_to_pylist(py, &self.inner.withdraw_events)?,
        )?;
        dict.set_item(
            "uniswap_v2_pair_created_events",
            vec_to_pylist(py, &self.inner.uniswap_v2_pair_created_events)?,
        )?;
        dict.set_item(
            "ownership_transferred_events",
            vec_to_pylist(py, &self.inner.ownership_transferred_events)?,
        )?;
        dict.set_item(
            "ownership_transfer_started_events",
            vec_to_pylist(py, &self.inner.ownership_transfer_started_events)?,
        )?;
        dict.set_item(
            "access_control_role_granted_events",
            vec_to_pylist(py, &self.inner.access_control_role_granted_events)?,
        )?;
        dict.set_item(
            "access_control_role_revoked_events",
            vec_to_pylist(py, &self.inner.access_control_role_revoked_events)?,
        )?;
        dict.set_item(
            "proxy_admin_changed_events",
            vec_to_pylist(py, &self.inner.proxy_admin_changed_events)?,
        )?;
        dict.set_item(
            "contract_creation_events",
            vec_to_pylist(py, &self.inner.contract_creation_events)?,
        )?;
        dict.set_item(
            "trading_enabled_events",
            vec_to_pylist(py, &self.inner.trading_enabled_events)?,
        )?;
        dict.set_item(
            "trading_disabled_events",
            vec_to_pylist(py, &self.inner.trading_disabled_events)?,
        )?;

        dict.set_item(
            "uniswap_v3_pools",
            vec_to_pylist(py, &self.inner.uniswap_v3_pools)?,
        )?;
        dict.set_item(
            "uniswap_v3_initializations",
            vec_to_pylist(py, &self.inner.uniswap_v3_initializations)?,
        )?;
        dict.set_item(
            "uniswap_v3_burns",
            vec_to_pylist(py, &self.inner.uniswap_v3_burns)?,
        )?;
        dict.set_item(
            "uniswap_v3_mints",
            vec_to_pylist(py, &self.inner.uniswap_v3_mints)?,
        )?;
        dict.set_item(
            "uniswap_v3_swaps",
            vec_to_pylist(py, &self.inner.uniswap_v3_swaps)?,
        )?;
        dict.set_item(
            "uniswap_v3_positions",
            vec_to_pylist(py, &self.inner.uniswap_v3_positions)?,
        )?;
        dict.set_item(
            "uniswap_v3_increases",
            vec_to_pylist(py, &self.inner.uniswap_v3_increases)?,
        )?;
        dict.set_item(
            "uniswap_v3_decreases",
            vec_to_pylist(py, &self.inner.uniswap_v3_decreases)?,
        )?;

        dict.set_item(
            "uniswap_v4_initializes",
            vec_to_pylist(py, &self.inner.uniswap_v4_initializes)?,
        )?;
        dict.set_item(
            "uniswap_v4_modifies",
            vec_to_pylist(py, &self.inner.uniswap_v4_modifies)?,
        )?;
        dict.set_item(
            "uniswap_v4_swaps",
            vec_to_pylist(py, &self.inner.uniswap_v4_swaps)?,
        )?;
        dict.set_item(
            "uniswap_v4_donates",
            vec_to_pylist(py, &self.inner.uniswap_v4_donates)?,
        )?;
        dict.set_item(
            "uniswap_v4_protocol_fee_updates",
            vec_to_pylist(py, &self.inner.uniswap_v4_protocol_fee_updates)?,
        )?;
        dict.set_item(
            "uniswap_v4_dynamic_lp_fee_updates",
            vec_to_pylist(py, &self.inner.uniswap_v4_dynamic_lp_fee_updates)?,
        )?;
        dict.set_item(
            "uniswap_v4_protocol_fee_controller_updates",
            vec_to_pylist(py, &self.inner.uniswap_v4_protocol_fee_controller_updates)?,
        )?;
        dict.set_item(
            "uniswap_v4_balance_deltas",
            vec_to_pylist(py, &self.inner.uniswap_v4_balance_deltas)?,
        )?;
        dict.set_item(
            "permit2_events",
            vec_to_pylist(py, &self.inner.permit2_events)?,
        )?;
        let access_list = PyList::empty_bound(py);
        for item in &self.inner.access_list {
            let entry = PyDict::new_bound(py);
            entry.set_item("address", to_checksum_address(&item.address))?;
            let keys = PyList::empty_bound(py);
            for key in &item.storage_keys {
                keys.append(format!("{:#x}", key))?;
            }
            entry.set_item("storage_keys", keys)?;
            access_list.append(entry)?;
        }
        dict.set_item("access_list", access_list)?;
        let blob_hashes = PyList::empty_bound(py);
        for hash in &self.inner.blob_versioned_hashes {
            blob_hashes.append(format!("{:#x}", hash))?;
        }
        dict.set_item("blob_versioned_hashes", blob_hashes)?;
        let auth_list = PyList::empty_bound(py);
        for auth in &self.inner.signed_authorizations {
            let json_value = serde_json::to_value(auth).map_err(|err| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Failed to serialize signed authorization: {}",
                    err
                ))
            })?;
            auth_list.append(json_to_python(py, &json_value)?)?;
        }
        dict.set_item("signed_authorizations", auth_list)?;

        // Other events
        let other = PyList::empty_bound(py);
        for ev in &self.inner.other_events {
            let j = serde_json::to_value(ev).unwrap_or(JsonValue::Null);
            other.append(json_to_python(py, &j)?)?;
        }
        dict.set_item("other_events", other)?;

        // Address balance changes: expose map keyed by checksum addresses
        let sc = PyDict::new_bound(py);
        for (addr, changes) in &self.inner.address_balance_changes {
            let addr_str = to_checksum_address(addr);
            let entry = PyDict::new_bound(py);
            // currency_net
            let cnet = PyDict::new_bound(py);
            for (sym, amount) in &changes.currency_net {
                if let Some(converted) = convert_currency_amount(py, sym, amount)? {
                    cnet.set_item(sym, converted)?;
                }
            }
            entry.set_item("currency_net", cnet)?;
            // token_net
            let tnet = PyDict::new_bound(py);
            for (tok, amount) in &changes.token_net {
                tnet.set_item(tok, amount.to_string())?;
            }
            entry.set_item("token_net", tnet)?;
            sc.set_item(addr_str, entry)?;
        }
        dict.set_item("address_balance_changes", sc)?;

        // latest_states map
        let ls = PyDict::new_bound(py);
        for (addr, val) in &self.inner.latest_states {
            let addr_str = to_checksum_address(addr);
            let j = serde_json::to_value(val).unwrap_or(JsonValue::Null);
            ls.set_item(addr_str, json_to_python(py, &j)?)?;
        }
        dict.set_item("latest_states", ls)?;

        Ok(dict.unbind())
    }

    #[getter]
    fn bribe_amount(&self, py: Python) -> PyResult<PyObject> {
        u256_to_py(py, &self.inner.bribe_amount)
    }

    /// Serialize to compact JSON using to_dict()
    fn to_json(&self, py: Python) -> PyResult<String> {
        let dict = self.to_dict(py)?;
        let obj = dict.bind(py);
        // Use Python's json to ensure consistent formatting
        let json_mod = py.import_bound("json")?;
        let s: String = json_mod.call_method1("dumps", (obj,))?.extract()?;
        Ok(s)
    }

    /// Get internal transactions as Python list of dicts
    #[getter]
    fn internal_transactions(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty_bound(py);

        for internal_tx in &self.inner.internal_transactions {
            let dict = PyDict::new_bound(py);
            dict.set_item(
                "from_address",
                to_checksum_address(&internal_tx.from_address),
            )?;
            match internal_tx.to_address {
                Some(addr) => dict.set_item("to_address", to_checksum_address(&addr))?,
                None => dict.set_item("to_address", py.None())?,
            }
            dict.set_item("value", internal_tx.value.to_string())?;
            dict.set_item("gas", internal_tx.gas)?;
            dict.set_item("gas_used", internal_tx.gas_used)?;
            dict.set_item("trace_type", &internal_tx.trace_type)?;
            dict.set_item("call_type", &internal_tx.call_type)?;
            dict.set_item("depth", internal_tx.depth)?;
            match &internal_tx.error {
                Some(err) => dict.set_item("error", err)?,
                None => dict.set_item("error", py.None())?,
            }
            list.append(dict)?;
        }

        Ok(list.unbind())
    }

    /// Get ERC20 transfers as Python list of dicts
    #[getter]
    fn erc20_transfers(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty_bound(py);

        for transfer in &self.inner.erc20_transfers {
            let dict = PyDict::new_bound(py);
            let token_checksum = to_checksum_address(&transfer.token_address);
            let from_checksum = to_checksum_address(&transfer.from_address);
            let to_checksum = to_checksum_address(&transfer.to_address);

            dict.set_item("from_address", from_checksum)?;
            dict.set_item("to_address", to_checksum)?;
            dict.set_item("token_address", token_checksum)?;
            dict.set_item("amount", transfer.amount.to_string())?;
            dict.set_item("log_index", transfer.log_index)?;
            list.append(dict)?;
        }

        Ok(list.unbind())
    }

    /// Get ETH transfers
    #[getter]
    fn eth_transfers(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty_bound(py);

        for transfer in &self.inner.eth_transfers {
            let dict = PyDict::new_bound(py);
            dict.set_item("from_address", to_checksum_address(&transfer.from_address))?;
            dict.set_item("to_address", to_checksum_address(&transfer.to_address))?;
            dict.set_item("amount", transfer.amount.to_string())?;
            list.append(dict)?;
        }

        Ok(list.unbind())
    }

    /// Get ERC721 transfers
    #[getter]
    fn erc721_transfers(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty_bound(py);

        for transfer in &self.inner.erc721_transfers {
            let dict = PyDict::new_bound(py);
            dict.set_item("from_address", to_checksum_address(&transfer.from_address))?;
            dict.set_item("to_address", to_checksum_address(&transfer.to_address))?;
            dict.set_item(
                "token_address",
                to_checksum_address(&transfer.token_address),
            )?;
            dict.set_item("token_id", transfer.token_id.to_string())?;
            dict.set_item("log_index", transfer.log_index)?;
            list.append(dict)?;
        }

        Ok(list.unbind())
    }

    /// Get ERC1155 transfers
    #[getter]
    fn erc1155_transfers(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty_bound(py);

        for transfer in &self.inner.erc1155_transfers {
            let dict = PyDict::new_bound(py);
            dict.set_item("from_address", to_checksum_address(&transfer.from_address))?;
            dict.set_item("to_address", to_checksum_address(&transfer.to_address))?;
            dict.set_item(
                "token_address",
                to_checksum_address(&transfer.token_address),
            )?;
            // ERC1155 has token_ids and amounts as arrays
            let token_ids: Vec<String> =
                transfer.token_ids.iter().map(|id| id.to_string()).collect();
            let amounts: Vec<String> = transfer.amounts.iter().map(|amt| amt.to_string()).collect();
            dict.set_item("token_ids", token_ids)?;
            dict.set_item("amounts", amounts)?;
            dict.set_item("log_index", transfer.log_index)?;
            list.append(dict)?;
        }

        Ok(list.unbind())
    }

    /// Get Uniswap V2 syncs
    #[getter]
    fn uniswap_v2_syncs(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty_bound(py);

        for sync in &self.inner.uniswap_v2_syncs {
            let dict = PyDict::new_bound(py);
            dict.set_item("pair_address", to_checksum_address(&sync.pair_address))?;
            dict.set_item("reserve0", sync.reserve0.to_string())?;
            dict.set_item("reserve1", sync.reserve1.to_string())?;
            dict.set_item("log_index", sync.log_index)?;
            list.append(dict)?;
        }

        Ok(list.unbind())
    }

    /// Get Uniswap V2 swaps
    #[getter]
    fn uniswap_v2_swaps(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty_bound(py);

        for swap in &self.inner.uniswap_v2_swaps {
            let dict = PyDict::new_bound(py);
            dict.set_item("pair_address", to_checksum_address(&swap.pair_address))?;
            dict.set_item("sender", to_checksum_address(&swap.sender))?;
            dict.set_item("to", to_checksum_address(&swap.to))?;
            dict.set_item("amount0In", swap.amount0_in.to_string())?;
            dict.set_item("amount1In", swap.amount1_in.to_string())?;
            dict.set_item("amount0Out", swap.amount0_out.to_string())?;
            dict.set_item("amount1Out", swap.amount1_out.to_string())?;
            dict.set_item("log_index", swap.log_index)?;
            list.append(dict)?;
        }

        Ok(list.unbind())
    }

    /// Get ERC20 approvals
    #[getter]
    fn erc20_approval_events(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty_bound(py);

        for approval in &self.inner.erc20_approval_events {
            let dict = PyDict::new_bound(py);
            dict.set_item("owner", to_checksum_address(&approval.owner))?;
            dict.set_item("spender", to_checksum_address(&approval.spender))?;
            dict.set_item(
                "token_address",
                to_checksum_address(&approval.token_address),
            )?;
            dict.set_item("amount", approval.amount.to_string())?;
            dict.set_item("log_index", approval.log_index)?;
            list.append(dict)?;
        }

        Ok(list.unbind())
    }

    /// Get Uniswap V2 mints
    #[getter]
    fn uniswap_v2_mints(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty_bound(py);

        for mint in &self.inner.uniswap_v2_mints {
            let dict = PyDict::new_bound(py);
            dict.set_item("pair_address", to_checksum_address(&mint.pair_address))?;
            dict.set_item("sender", to_checksum_address(&mint.sender))?;
            dict.set_item("amount0", mint.amount0.to_string())?;
            dict.set_item("amount1", mint.amount1.to_string())?;
            dict.set_item("log_index", mint.log_index)?;
            list.append(dict)?;
        }

        Ok(list.unbind())
    }

    /// Get Uniswap V2 burns
    #[getter]
    fn uniswap_v2_burns(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty_bound(py);

        for burn in &self.inner.uniswap_v2_burns {
            let dict = PyDict::new_bound(py);
            dict.set_item("pair_address", to_checksum_address(&burn.pair_address))?;
            dict.set_item("sender", to_checksum_address(&burn.sender))?;
            dict.set_item("amount0", burn.amount0.to_string())?;
            dict.set_item("amount1", burn.amount1.to_string())?;
            dict.set_item("log_index", burn.log_index)?;
            list.append(dict)?;
        }

        Ok(list.unbind())
    }

    /// Get deposit events
    #[getter]
    fn deposit_events(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty_bound(py);

        for deposit in &self.inner.deposit_events {
            let dict = PyDict::new_bound(py);
            // DepositEvent has complex optional fields
            if let Some(sender) = deposit.sender {
                dict.set_item("sender", to_checksum_address(&sender))?;
            }
            if let Some(amount) = deposit.amount {
                dict.set_item("amount", amount.to_string())?;
            }
            if let Some(pair_addr) = deposit.pair_address {
                dict.set_item("pair_address", to_checksum_address(&pair_addr))?;
            }
            if let Some(log_idx) = deposit.log_index {
                dict.set_item("log_index", log_idx)?;
            }
            list.append(dict)?;
        }

        Ok(list.unbind())
    }

    /// Get withdraw events
    #[getter]
    fn withdraw_events(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty_bound(py);

        for withdraw in &self.inner.withdraw_events {
            let dict = PyDict::new_bound(py);
            if let Some(sender) = withdraw.sender {
                dict.set_item("sender", to_checksum_address(&sender))?;
            }
            dict.set_item("amount", withdraw.amount.to_string())?;
            dict.set_item("log_index", withdraw.log_index)?;
            list.append(dict)?;
        }

        Ok(list.unbind())
    }

    /// Get unique addresses involved in transaction
    #[getter]
    fn unique_addresses(&self, py: Python) -> PyResult<Py<PySet>> {
        let set = PySet::empty_bound(py)?;

        for addr in &self.inner.unique_addresses {
            set.add(to_checksum_address(&addr))?;
        }

        Ok(set.unbind())
    }

    /// Get ERC20 contract addresses
    #[getter]
    fn erc20_contracts(&self, py: Python) -> PyResult<Py<PySet>> {
        let set = PySet::empty_bound(py)?;

        for addr in &self.inner.erc20_contracts {
            set.add(to_checksum_address(&addr))?;
        }

        Ok(set.unbind())
    }

    /// Get Uniswap V2 pair created events
    #[getter]
    fn uniswap_v2_pair_created_events(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty_bound(py);

        for event in &self.inner.uniswap_v2_pair_created_events {
            let dict = PyDict::new_bound(py);
            dict.set_item("pair_address", to_checksum_address(&event.pair_address))?;
            dict.set_item("token0", to_checksum_address(&event.token0))?;
            dict.set_item("token1", to_checksum_address(&event.token1))?;
            dict.set_item("log_index", event.log_index)?;
            list.append(dict)?;
        }

        Ok(list.unbind())
    }

    /// Get ownership transferred events
    #[getter]
    fn ownership_transferred_events(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty_bound(py);

        for event in &self.inner.ownership_transferred_events {
            let dict = PyDict::new_bound(py);
            dict.set_item(
                "contract_address",
                to_checksum_address(&event.contract_address),
            )?;
            dict.set_item("previous_owner", to_checksum_address(&event.previous_owner))?;
            dict.set_item("new_owner", to_checksum_address(&event.new_owner))?;
            dict.set_item("log_index", event.log_index)?;
            list.append(dict)?;
        }

        Ok(list.unbind())
    }

    /// Get contract creation events
    #[getter]
    fn contract_creation_events(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty_bound(py);

        for event in &self.inner.contract_creation_events {
            let dict = PyDict::new_bound(py);
            dict.set_item(
                "contract_address",
                to_checksum_address(&event.contract_address),
            )?;
            list.append(dict)?;
        }

        Ok(list.unbind())
    }

    /// Get trading enabled events
    #[getter]
    fn trading_enabled_events(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty_bound(py);

        for event in &self.inner.trading_enabled_events {
            let dict = PyDict::new_bound(py);
            dict.set_item("token_address", to_checksum_address(&event.token_address))?;
            dict.set_item("log_index", event.log_index)?;
            list.append(dict)?;
        }

        Ok(list.unbind())
    }

    /// Get trading disabled events
    #[getter]
    fn trading_disabled_events(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty_bound(py);

        for event in &self.inner.trading_disabled_events {
            let dict = PyDict::new_bound(py);
            dict.set_item("token_address", to_checksum_address(&event.token_address))?;
            dict.set_item("log_index", event.log_index)?;
            list.append(dict)?;
        }

        Ok(list.unbind())
    }

    /// Get Uniswap V3 pools
    #[getter]
    fn uniswap_v3_pools(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty_bound(py);

        for pool in &self.inner.uniswap_v3_pools {
            let dict = PyDict::new_bound(py);
            dict.set_item("pool_address", to_checksum_address(&pool.pool))?;
            dict.set_item("token0", to_checksum_address(&pool.token0))?;
            dict.set_item("token1", to_checksum_address(&pool.token1))?;
            dict.set_item("fee", pool.fee)?;
            dict.set_item("tick_spacing", pool.tick_spacing)?;
            dict.set_item("log_index", pool.log_index)?;
            list.append(dict)?;
        }

        Ok(list.unbind())
    }

    /// Get Uniswap V3 initializations
    #[getter]
    fn uniswap_v3_initializations(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty_bound(py);
        // Implementation for V3 initializations if needed
        Ok(list.unbind())
    }

    /// Get Uniswap V3 burns
    #[getter]
    fn uniswap_v3_burns(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty_bound(py);
        // Implementation for V3 burns if needed
        Ok(list.unbind())
    }

    /// Get Uniswap V3 mints
    #[getter]
    fn uniswap_v3_mints(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty_bound(py);
        // Implementation for V3 mints if needed
        Ok(list.unbind())
    }

    /// Get Uniswap V3 positions
    #[getter]
    fn uniswap_v3_positions(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty_bound(py);
        // Implementation for V3 positions if needed
        Ok(list.unbind())
    }

    /// Get Uniswap V3 increases
    #[getter]
    fn uniswap_v3_increases(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty_bound(py);
        // Implementation for V3 increases if needed
        Ok(list.unbind())
    }

    /// Get Uniswap V3 decreases
    #[getter]
    fn uniswap_v3_decreases(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty_bound(py);
        // Implementation for V3 decreases if needed
        Ok(list.unbind())
    }

    /// Get Uniswap V4 initializes
    #[getter]
    fn uniswap_v4_initializes(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty_bound(py);
        // Implementation for V4 initializes if needed
        Ok(list.unbind())
    }

    /// Get Uniswap V4 modifies
    #[getter]
    fn uniswap_v4_modifies(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty_bound(py);
        // Implementation for V4 modifies if needed
        Ok(list.unbind())
    }

    /// Get Uniswap V4 swaps
    #[getter]
    fn uniswap_v4_swaps(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty_bound(py);
        // Implementation for V4 swaps if needed
        Ok(list.unbind())
    }

    /// Get Permit2Event events
    #[getter]
    fn permit2_events(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty_bound(py);
        // Implementation for Permit2Event events if needed
        Ok(list.unbind())
    }

    /// Get Uniswap V3 swaps
    #[getter]
    fn uniswap_v3_swaps(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty_bound(py);

        for swap in &self.inner.uniswap_v3_swaps {
            let dict = PyDict::new_bound(py);
            dict.set_item("pool_address", to_checksum_address(&swap.pool_address))?;
            dict.set_item("sender", to_checksum_address(&swap.sender))?;
            dict.set_item("recipient", to_checksum_address(&swap.recipient))?;
            dict.set_item("amount0", swap.amount0.to_string())?;
            dict.set_item("amount1", swap.amount1.to_string())?;
            dict.set_item("sqrt_price_x96", swap.sqrt_price_x96.to_string())?;
            dict.set_item("liquidity", swap.liquidity.to_string())?;
            dict.set_item("tick", swap.tick)?;
            dict.set_item("log_index", swap.log_index)?;
            list.append(dict)?;
        }

        Ok(list.unbind())
    }

    /// Get other events
    #[getter]
    fn other_events(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty_bound(py);

        for event in &self.inner.other_events {
            let dict = PyDict::new_bound(py);
            for (key, value) in event {
                // Convert serde_json::Value to nested Python object
                let py_value = json_to_python(py, &value)?;
                dict.set_item(key, py_value)?;
            }
            list.append(dict)?;
        }

        Ok(list.unbind())
    }

    /// Get latest states
    #[getter]
    fn latest_states(&self, py: Python) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new_bound(py);

        for (addr, value) in &self.inner.latest_states {
            let addr_str = to_checksum_address(&addr);
            // Convert serde_json::Value to nested Python dict
            let py_value = json_to_python(py, &value)?;
            dict.set_item(addr_str, py_value)?;
        }

        Ok(dict.unbind())
    }

    /// Get address balance changes as Python dict
    #[getter]
    fn address_balance_changes(&self, py: Python) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new_bound(py);

        for (addr, balance_change) in &self.inner.address_balance_changes {
            let addr_str = to_checksum_address(&addr);

            // Convert AddressBalanceChange to Python dict
            let change_dict = PyDict::new_bound(py);

            // Add token_net
            let token_net_dict = PyDict::new_bound(py);
            for (token, amount) in &balance_change.token_net {
                token_net_dict.set_item(token, amount.to_string())?;
            }
            change_dict.set_item("token_net", token_net_dict)?;

            // Add currency_net (convert ETH from wei to ETH, others as-is)
            let currency_net_dict = PyDict::new_bound(py);
            for (currency, amount) in &balance_change.currency_net {
                if currency == "ETH" {
                    let magnitude = amount.unsigned_abs();
                    let wei_str = magnitude.to_string();
                    let wei_value: f64 = wei_str.parse().unwrap_or(0.0);
                    let eth_amount = wei_value / 1e18;
                    let signed = if amount.is_negative() {
                        -eth_amount
                    } else {
                        eth_amount
                    };
                    currency_net_dict.set_item(currency, signed)?;
                } else {
                    // Other currencies - keep as string for now
                    currency_net_dict.set_item(currency, amount.to_string())?;
                }
            }
            change_dict.set_item("currency_net", currency_net_dict)?;

            dict.set_item(addr_str, change_dict)?;
        }

        Ok(dict.unbind())
    }

    /// Get transaction fees as dict
    #[getter]
    fn fees(&self, py: Python) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new_bound(py);
        dict.set_item("gas_price", self.inner.fees.gas_price.to_string())?;
        dict.set_item("gas_used", self.inner.fees.gas_used)?;
        dict.set_item("tx_fee", self.inner.fees.tx_fee.to_string())?;
        dict.set_item("protocol_type", &self.inner.fees.protocol_type)?;
        if let Some(max_fee) = self.inner.fees.max_fee_per_gas {
            dict.set_item("max_fee_per_gas", max_fee.to_string())?;
        }
        if let Some(max_priority) = self.inner.fees.max_priority_fee {
            dict.set_item("max_priority_fee", max_priority.to_string())?;
        }
        Ok(dict.unbind())
    }

    // NOTE: to_dict() is defined earlier in this impl and returns the
    // canonical Python-facing transaction schema.

    fn __repr__(&self) -> String {
        format!(
            "ProcessedTransaction(hash='{}', block_number={}, tx_index={}, status={}, actions={}, input_len={})",
            self.hash,
            self.block_number,
            self.tx_index,
            if self.status { "True" } else { "False" },
            self.actions.len(),
            self.input.len(),
        )
    }
}
fn convert_currency_amount(py: Python, symbol: &str, amount: &I256) -> PyResult<Option<PyObject>> {
    let decimals: Option<i32> = if symbol == "ETH" {
        Some(18)
    } else {
        ERC20_TOKEN_DECIMALS.get(symbol).copied().map(i32::from)
    };

    let Some(decimals) = decimals else {
        return Ok(None);
    };

    let magnitude = amount.unsigned_abs();
    let magnitude_str = magnitude.to_string();
    let int_type = py.get_type_bound::<PyLong>();
    let py_int = int_type.call1((magnitude_str.as_str(), 10))?;
    let mut py_value = py_int.into_py(py);

    if amount.is_negative() {
        let neg = py_value.bind(py).call_method0("__neg__")?;
        py_value = neg.into_py(py);
    }

    let py_float_value = py_value.bind(py).call_method0("__float__")?;
    let rust_float: f64 = py_float_value.extract()?;
    let divisor = 10f64.powi(decimals);
    let result = PyFloat::new_bound(py, rust_float / divisor);

    Ok(Some(result.into()))
}
