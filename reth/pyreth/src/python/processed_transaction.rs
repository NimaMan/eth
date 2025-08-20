/// Python wrapper for ProcessedTransaction
/// 
/// Converts Rust ProcessedTransaction to Python-compatible format

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PySet};
use ethtx::data_models::transaction::ProcessedTransaction;
use ethtx::utils::to_checksum_address;
use alloy_primitives::U256;
use serde_json::Value as JsonValue;

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
        },
        JsonValue::String(s) => {
            // Check if it's a number string (for large integers)
            if s.chars().all(|c| c.is_ascii_digit() || c == '-') {
                if let Ok(n) = s.parse::<f64>() {
                    Ok(n.to_object(py))
                } else {
                    Ok(s.to_object(py))
                }
            } else {
                Ok(s.to_object(py))
            }
        },
        JsonValue::Array(arr) => {
            let list = PyList::empty(py);
            for item in arr {
                list.append(json_to_python(py, item)?)?;
            }
            Ok(list.into())
        },
        JsonValue::Object(map) => {
            let dict = PyDict::new(py);
            for (key, val) in map {
                dict.set_item(key, json_to_python(py, val)?)?;
            }
            Ok(dict.into())
        }
    }
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
    pub txn_index: u64,
    #[pyo3(get)]
    pub from_address: String,
    #[pyo3(get)]
    pub to_address: Option<String>,
    #[pyo3(get)]
    pub contract_address: Option<String>,
    #[pyo3(get)]
    pub value: String,
    #[pyo3(get)]
    pub status: String,
    #[pyo3(get)]
    pub nonce: u64,
    #[pyo3(get)]
    pub txn_type: String,
    #[pyo3(get)]
    pub actions: Vec<String>,
    #[pyo3(get)]
    pub input: String,
    #[pyo3(get)]
    pub bribe_amount: f64,
    
    // Store the original for internal use
    pub(crate) inner: ProcessedTransaction,
}

impl PyProcessedTransaction {
    /// Create from Rust ProcessedTransaction
    pub fn from_processed_transaction(ptx: ProcessedTransaction) -> Self {
        Self {
            hash: format!("0x{}", hex::encode(ptx.hash)),
            block_number: ptx.block_number,
            block_timestamp: ptx.block_timestamp,
            txn_index: ptx.txn_index,
            from_address: to_checksum_address(&ptx.from_address),
            to_address: ptx.to_address.map(|a| to_checksum_address(&a)),
            contract_address: ptx.contract_address.map(|a| to_checksum_address(&a)),
            value: ptx.value.to_string(),
            status: ptx.status.clone(),
            nonce: ptx.nonce,
            txn_type: ptx.txn_type.clone(),
            actions: ptx.actions.clone(),
            input: format!("0x{}", hex::encode(&ptx.input)),
            bribe_amount: ptx.bribe_amount,
            inner: ptx,
        }
    }
}

#[pymethods]
impl PyProcessedTransaction {
    /// Get internal transactions as Python list of dicts
    #[getter]
    fn internal_transactions(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        
        for internal_tx in &self.inner.internal_transactions {
            let dict = PyDict::new(py);
            dict.set_item("from_address", to_checksum_address(&internal_tx.from_address))?;
            dict.set_item("to_address", to_checksum_address(&internal_tx.to_address))?;
            dict.set_item("value", internal_tx.value.to_string())?;
            dict.set_item("gas_used", internal_tx.gas_used)?;
            dict.set_item("trace_type", &internal_tx.trace_type)?;
            dict.set_item("call_type", &internal_tx.call_type)?;
            dict.set_item("depth", internal_tx.depth)?;
            list.append(dict)?;
        }
        
        Ok(list.into())
    }
    
    /// Get ERC20 transfers as Python list of dicts
    #[getter]
    fn erc20_transfers(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        
        for transfer in &self.inner.erc20_transfers {
            let dict = PyDict::new(py);
            dict.set_item("from_address", to_checksum_address(&transfer.from_address))?;
            dict.set_item("to_address", to_checksum_address(&transfer.to_address))?;
            dict.set_item("token_address", to_checksum_address(&transfer.token_address))?;
            dict.set_item("amount", transfer.amount.to_string())?;
            dict.set_item("log_index", transfer.log_index)?;
            list.append(dict)?;
        }
        
        Ok(list.into())
    }
    
    /// Get ETH transfers
    #[getter]
    fn eth_transfers(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        
        for transfer in &self.inner.eth_transfers {
            let dict = PyDict::new(py);
            dict.set_item("from_address", to_checksum_address(&transfer.from_address))?;
            dict.set_item("to_address", to_checksum_address(&transfer.to_address))?;
            dict.set_item("amount", transfer.amount.to_string())?;
            list.append(dict)?;
        }
        
        Ok(list.into())
    }
    
    /// Get ERC721 transfers
    #[getter]
    fn erc721_transfers(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        
        for transfer in &self.inner.erc721_transfers {
            let dict = PyDict::new(py);
            dict.set_item("from_address", to_checksum_address(&transfer.from_address))?;
            dict.set_item("to_address", to_checksum_address(&transfer.to_address))?;
            dict.set_item("token_address", to_checksum_address(&transfer.token_address))?;
            dict.set_item("token_id", transfer.token_id.to_string())?;
            dict.set_item("log_index", transfer.log_index)?;
            list.append(dict)?;
        }
        
        Ok(list.into())
    }
    
    /// Get ERC1155 transfers
    #[getter]
    fn erc1155_transfers(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        
        for transfer in &self.inner.erc1155_transfers {
            let dict = PyDict::new(py);
            dict.set_item("from_address", to_checksum_address(&transfer.from_address))?;
            dict.set_item("to_address", to_checksum_address(&transfer.to_address))?;
            dict.set_item("token_address", to_checksum_address(&transfer.token_address))?;
            // ERC1155 has token_ids and amounts as arrays
            let token_ids: Vec<String> = transfer.token_ids.iter().map(|id| id.to_string()).collect();
            let amounts: Vec<String> = transfer.amounts.iter().map(|amt| amt.to_string()).collect();
            dict.set_item("token_ids", token_ids)?;
            dict.set_item("amounts", amounts)?;
            dict.set_item("log_index", transfer.log_index)?;
            list.append(dict)?;
        }
        
        Ok(list.into())
    }
    
    /// Get Uniswap V2 syncs
    #[getter]
    fn uniswap_v2_syncs(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        
        for sync in &self.inner.uniswap_v2_syncs {
            let dict = PyDict::new(py);
            dict.set_item("pair_address", to_checksum_address(&sync.pair_address))?;
            dict.set_item("reserve0", sync.reserve0.to_string())?;
            dict.set_item("reserve1", sync.reserve1.to_string())?;
            dict.set_item("log_index", sync.log_index)?;
            list.append(dict)?;
        }
        
        Ok(list.into())
    }
    
    /// Get Uniswap V2 swaps
    #[getter]
    fn uniswap_v2_swaps(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        
        for swap in &self.inner.uniswap_v2_swaps {
            let dict = PyDict::new(py);
            dict.set_item("pair_address", to_checksum_address(&swap.pair_address))?;
            dict.set_item("sender", to_checksum_address(&swap.sender))?;
            dict.set_item("to", to_checksum_address(&swap.to))?;
            dict.set_item("amount0_in", swap.amount0_in.to_string())?;
            dict.set_item("amount1_in", swap.amount1_in.to_string())?;
            dict.set_item("amount0_out", swap.amount0_out.to_string())?;
            dict.set_item("amount1_out", swap.amount1_out.to_string())?;
            dict.set_item("log_index", swap.log_index)?;
            list.append(dict)?;
        }
        
        Ok(list.into())
    }
    
    /// Get approvals
    #[getter]
    fn approvals(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        
        for approval in &self.inner.approvals {
            let dict = PyDict::new(py);
            dict.set_item("owner", to_checksum_address(&approval.owner))?;
            dict.set_item("spender", to_checksum_address(&approval.spender))?;
            dict.set_item("token_address", to_checksum_address(&approval.token_address))?;
            dict.set_item("amount", approval.amount.to_string())?;
            dict.set_item("log_index", approval.log_index)?;
            list.append(dict)?;
        }
        
        Ok(list.into())
    }
    
    /// Get mints
    #[getter]
    fn mints(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        
        for mint in &self.inner.mints {
            let dict = PyDict::new(py);
            dict.set_item("pair_address", to_checksum_address(&mint.pair_address))?;
            dict.set_item("sender", to_checksum_address(&mint.sender))?;
            dict.set_item("amount0", mint.amount0.to_string())?;
            dict.set_item("amount1", mint.amount1.to_string())?;
            dict.set_item("log_index", mint.log_index)?;
            list.append(dict)?;
        }
        
        Ok(list.into())
    }
    
    /// Get burns
    #[getter]
    fn burns(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        
        for burn in &self.inner.burns {
            let dict = PyDict::new(py);
            dict.set_item("pair_address", to_checksum_address(&burn.pair_address))?;
            dict.set_item("sender", to_checksum_address(&burn.sender))?;
            dict.set_item("amount", burn.amount.to_string())?;
            dict.set_item("log_index", burn.log_index)?;
            list.append(dict)?;
        }
        
        Ok(list.into())
    }
    
    /// Get deposits
    #[getter]
    fn deposits(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        
        for deposit in &self.inner.deposits {
            let dict = PyDict::new(py);
            // DepositAction has complex optional fields
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
        
        Ok(list.into())
    }
    
    /// Get withdraws
    #[getter]
    fn withdraws(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        
        for withdraw in &self.inner.withdraws {
            let dict = PyDict::new(py);
            dict.set_item("sender", to_checksum_address(&withdraw.sender))?;
            dict.set_item("amount", withdraw.amount.to_string())?;
            dict.set_item("log_index", withdraw.log_index)?;
            list.append(dict)?;
        }
        
        Ok(list.into())
    }
    
    /// Get unique addresses involved in transaction
    #[getter]
    fn unique_addresses(&self, py: Python) -> PyResult<Py<PySet>> {
        let set = PySet::empty(py)?;
        
        for addr in &self.inner.unique_addresses {
            set.add(to_checksum_address(&addr))?;
        }
        
        Ok(set.into())
    }
    
    /// Get ERC20 contract addresses
    #[getter]
    fn erc20_contracts(&self, py: Python) -> PyResult<Py<PySet>> {
        let set = PySet::empty(py)?;
        
        for addr in &self.inner.erc20_contracts {
            set.add(to_checksum_address(&addr))?;
        }
        
        Ok(set.into())
    }
    
    /// Get pair events
    #[getter]
    fn pair_events(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        
        for event in &self.inner.pair_events {
            let dict = PyDict::new(py);
            dict.set_item("pair_address", to_checksum_address(&event.pair_address))?;
            dict.set_item("token0", to_checksum_address(&event.token0))?;
            dict.set_item("token1", to_checksum_address(&event.token1))?;
            dict.set_item("log_index", event.log_index)?;
            list.append(dict)?;
        }
        
        Ok(list.into())
    }
    
    /// Get owner events
    #[getter]
    fn owner_events(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        
        for event in &self.inner.owner_events {
            let dict = PyDict::new(py);
            dict.set_item("contract_address", to_checksum_address(&event.contract_address))?;
            dict.set_item("previous_owner", to_checksum_address(&event.previous_owner))?;
            dict.set_item("new_owner", to_checksum_address(&event.new_owner))?;
            dict.set_item("log_index", event.log_index)?;
            list.append(dict)?;
        }
        
        Ok(list.into())
    }
    
    /// Get contract creation events
    #[getter]
    fn contract_creation_events(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        
        for event in &self.inner.contract_creation_events {
            let dict = PyDict::new(py);
            dict.set_item("contract_address", to_checksum_address(&event.contract_address))?;
            dict.set_item("contract_type", &event.contract_type)?;
            dict.set_item("symbol", &event.symbol)?;
            dict.set_item("decimals", event.decimals)?;
            dict.set_item("name", &event.name)?;
            if let Some(supply) = &event.total_supply {
                dict.set_item("total_supply", supply.to_string())?;
            }
            list.append(dict)?;
        }
        
        Ok(list.into())
    }
    
    /// Get trading enabled events
    #[getter]
    fn trading_enabled_events(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        
        for event in &self.inner.trading_enabled_events {
            let dict = PyDict::new(py);
            dict.set_item("token_address", to_checksum_address(&event.token_address))?;
            dict.set_item("log_index", event.log_index)?;
            list.append(dict)?;
        }
        
        Ok(list.into())
    }
    
    /// Get trading disabled events
    #[getter]
    fn trading_disabled_events(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        
        for event in &self.inner.trading_disabled_events {
            let dict = PyDict::new(py);
            dict.set_item("token_address", to_checksum_address(&event.token_address))?;
            dict.set_item("log_index", event.log_index)?;
            list.append(dict)?;
        }
        
        Ok(list.into())
    }
    
    /// Get Uniswap V3 pools
    #[getter]
    fn uniswap_v3_pools(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        
        for pool in &self.inner.uniswap_v3_pools {
            let dict = PyDict::new(py);
            dict.set_item("pool_address", to_checksum_address(&pool.pool))?;
            dict.set_item("token0", to_checksum_address(&pool.token0))?;
            dict.set_item("token1", to_checksum_address(&pool.token1))?;
            dict.set_item("fee", pool.fee)?;
            dict.set_item("tick_spacing", pool.tick_spacing)?;
            dict.set_item("log_index", pool.log_index)?;
            list.append(dict)?;
        }
        
        Ok(list.into())
    }
    
    /// Get Uniswap V3 initializations
    #[getter]
    fn uniswap_v3_initializations(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        // Implementation for V3 initializations if needed
        Ok(list.into())
    }
    
    /// Get Uniswap V3 burns
    #[getter]
    fn uniswap_v3_burns(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        // Implementation for V3 burns if needed
        Ok(list.into())
    }
    
    /// Get Uniswap V3 mints
    #[getter]
    fn uniswap_v3_mints(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        // Implementation for V3 mints if needed
        Ok(list.into())
    }
    
    /// Get Uniswap V3 positions
    #[getter]
    fn uniswap_v3_positions(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        // Implementation for V3 positions if needed
        Ok(list.into())
    }
    
    /// Get Uniswap V3 increases
    #[getter]
    fn uniswap_v3_increases(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        // Implementation for V3 increases if needed
        Ok(list.into())
    }
    
    /// Get Uniswap V3 decreases
    #[getter]
    fn uniswap_v3_decreases(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        // Implementation for V3 decreases if needed
        Ok(list.into())
    }
    
    /// Get Uniswap V4 initializes
    #[getter]
    fn uniswap_v4_initializes(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        // Implementation for V4 initializes if needed
        Ok(list.into())
    }
    
    /// Get Uniswap V4 modifies
    #[getter]
    fn uniswap_v4_modifies(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        // Implementation for V4 modifies if needed
        Ok(list.into())
    }
    
    /// Get Uniswap V4 swaps
    #[getter]
    fn uniswap_v4_swaps(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        // Implementation for V4 swaps if needed
        Ok(list.into())
    }
    
    /// Get Permit2 events
    #[getter]
    fn permit2_events(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        // Implementation for Permit2 events if needed
        Ok(list.into())
    }
    
    /// Get Uniswap V3 swaps
    #[getter]
    fn uniswap_v3_swaps(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        
        for swap in &self.inner.uniswap_v3_swaps {
            let dict = PyDict::new(py);
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
        
        Ok(list.into())
    }
    
    /// Get other events
    #[getter]
    fn other_events(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        
        for event in &self.inner.other_events {
            let dict = PyDict::new(py);
            for (key, value) in event {
                // Convert serde_json::Value to nested Python object
                let py_value = json_to_python(py, &value)?;
                dict.set_item(key, py_value)?;
            }
            list.append(dict)?;
        }
        
        Ok(list.into())
    }
    
    /// Get latest states
    #[getter]
    fn latest_states(&self, py: Python) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);
        
        for (addr, value) in &self.inner.latest_states {
            let addr_str = to_checksum_address(&addr);
            // Convert serde_json::Value to nested Python dict
            let py_value = json_to_python(py, &value)?;
            dict.set_item(addr_str, py_value)?;
        }
        
        Ok(dict.into())
    }
    
    /// Get state changes as Python dict
    #[getter]
    fn state_changes(&self, py: Python) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);
        
        for (addr, value) in &self.inner.state_changes {
            let addr_str = to_checksum_address(&addr);
            // Convert serde_json::Value to nested Python dict
            let py_value = json_to_python(py, &value)?;
            dict.set_item(addr_str, py_value)?;
        }
        
        Ok(dict.into())
    }
    
    /// Get transaction fees as dict
    #[getter]
    fn fees(&self, py: Python) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);
        dict.set_item("gas_price", self.inner.fees.gas_price.to_string())?;
        dict.set_item("gas_used", self.inner.fees.gas_used)?;
        dict.set_item("txn_fee", self.inner.fees.txn_fee.to_string())?;
        dict.set_item("protocol_type", &self.inner.fees.protocol_type)?;
        if let Some(max_fee) = self.inner.fees.max_fee_per_gas {
            dict.set_item("max_fee_per_gas", max_fee.to_string())?;
        }
        if let Some(max_priority) = self.inner.fees.max_priority_fee {
            dict.set_item("max_priority_fee", max_priority.to_string())?;
        }
        Ok(dict.into())
    }
    
    /// Convert to Python dict (for compatibility with existing code)
    fn to_dict(&self, py: Python) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);
        
        // Basic fields
        dict.set_item("hash", &self.hash)?;
        dict.set_item("block_number", self.block_number)?;
        dict.set_item("block_timestamp", self.block_timestamp)?;
        dict.set_item("txn_index", self.txn_index)?;
        dict.set_item("from_address", &self.from_address)?;
        dict.set_item("to_address", &self.to_address)?;
        dict.set_item("contract_address", &self.contract_address)?;
        dict.set_item("value", &self.value)?;
        dict.set_item("status", &self.status)?;
        dict.set_item("nonce", self.nonce)?;
        dict.set_item("txn_type", &self.txn_type)?;
        dict.set_item("actions", &self.actions)?;
        dict.set_item("input", &self.input)?;
        dict.set_item("bribe_amount", self.bribe_amount)?;
        
        // Address sets
        dict.set_item("unique_addresses", self.unique_addresses(py)?)?;
        dict.set_item("erc20_contracts", self.erc20_contracts(py)?)?;
        
        // Transfer events
        dict.set_item("eth_transfers", self.eth_transfers(py)?)?;
        dict.set_item("erc20_transfers", self.erc20_transfers(py)?)?;
        dict.set_item("erc721_transfers", self.erc721_transfers(py)?)?;
        dict.set_item("erc1155_transfers", self.erc1155_transfers(py)?)?;
        dict.set_item("internal_transactions", self.internal_transactions(py)?)?;
        
        // DEX events
        dict.set_item("uniswap_v2_syncs", self.uniswap_v2_syncs(py)?)?;
        dict.set_item("uniswap_v2_swaps", self.uniswap_v2_swaps(py)?)?;
        dict.set_item("uniswap_v3_pools", self.uniswap_v3_pools(py)?)?;
        dict.set_item("uniswap_v3_initializations", self.uniswap_v3_initializations(py)?)?;
        dict.set_item("uniswap_v3_burns", self.uniswap_v3_burns(py)?)?;
        dict.set_item("uniswap_v3_mints", self.uniswap_v3_mints(py)?)?;
        dict.set_item("uniswap_v3_swaps", self.uniswap_v3_swaps(py)?)?;
        dict.set_item("uniswap_v3_positions", self.uniswap_v3_positions(py)?)?;
        dict.set_item("uniswap_v3_increases", self.uniswap_v3_increases(py)?)?;
        dict.set_item("uniswap_v3_decreases", self.uniswap_v3_decreases(py)?)?;
        dict.set_item("uniswap_v4_initializes", self.uniswap_v4_initializes(py)?)?;
        dict.set_item("uniswap_v4_modifies", self.uniswap_v4_modifies(py)?)?;
        dict.set_item("uniswap_v4_swaps", self.uniswap_v4_swaps(py)?)?;
        dict.set_item("permit2_events", self.permit2_events(py)?)?;
        
        // Other events
        dict.set_item("approvals", self.approvals(py)?)?;
        dict.set_item("mints", self.mints(py)?)?;
        dict.set_item("burns", self.burns(py)?)?;
        dict.set_item("deposits", self.deposits(py)?)?;
        dict.set_item("withdraws", self.withdraws(py)?)?;
        dict.set_item("pair_events", self.pair_events(py)?)?;
        dict.set_item("owner_events", self.owner_events(py)?)?;
        dict.set_item("contract_creation_events", self.contract_creation_events(py)?)?;
        dict.set_item("trading_enabled_events", self.trading_enabled_events(py)?)?;
        dict.set_item("trading_disabled_events", self.trading_disabled_events(py)?)?;
        dict.set_item("other_events", self.other_events(py)?)?;
        
        // State and fees
        dict.set_item("state_changes", self.state_changes(py)?)?;
        dict.set_item("latest_states", self.latest_states(py)?)?;
        dict.set_item("fees", self.fees(py)?)?;
        
        Ok(dict.into())
    }
    
    fn __repr__(&self) -> String {
        // Format addresses set - show all
        let unique_addrs: Vec<String> = self.inner.unique_addresses.iter()
            .map(|a| format!("'0x{}'", hex::encode(a).to_uppercase()))
            .collect();
        let unique_addrs_str = format!("{{{}}}", unique_addrs.join(", "));
        
        // Format ERC20 contracts - show all
        let erc20_contracts: Vec<String> = self.inner.erc20_contracts.iter()
            .map(|a| format!("'0x{}'", hex::encode(a).to_uppercase()))
            .collect();
        let erc20_contracts_str = format!("{{{}}}", erc20_contracts.join(", "));
        
        // Format ERC20 transfers - show all
        let erc20_transfers: Vec<String> = self.inner.erc20_transfers.iter()
            .map(|t| format!("ERC20Transfer(token_address='0x{}', from_address='0x{}', to_address='0x{}', amount='{}', log_index={})",
                hex::encode(t.token_address).to_uppercase(),
                hex::encode(t.from_address).to_uppercase(),
                hex::encode(t.to_address).to_uppercase(),
                t.amount,
                t.log_index
            ))
            .collect();
        let erc20_transfers_repr = format!("[{}]", erc20_transfers.join(", "));
        
        // Format internal transactions - show all
        let internal_txs: Vec<String> = self.inner.internal_transactions.iter()
            .map(|tx| format!("InternalTransaction(from_address='0x{}', to_address='0x{}', value={}, depth={}, type='{}', gas_used={}, error=None)",
                hex::encode(tx.from_address).to_uppercase(),
                hex::encode(tx.to_address).to_uppercase(),
                if tx.value == U256::ZERO { "0.0".to_string() } else { format!("{:.18}", tx.value.to_string().parse::<f64>().unwrap_or(0.0) / 1e18) },
                tx.depth,
                tx.trace_type,
                tx.gas_used
            ))
            .collect();
        let internal_repr = format!("[{}]", internal_txs.join(", "));
        
        // Simple list formatter for empty lists
        let format_list = |_name: &str, len: usize| -> String {
            if len == 0 {
                "[]".to_string()
            } else {
                // For non-empty lists that aren't fully formatted, just show count
                // This is only used for lists we haven't fully implemented yet
                format!("[{} items]", len)
            }
        };
        
        // Format fees
        let fees_repr = format!("TransactionFees(gas_price={}, gas_used={}, txn_fee={}, protocol_type='{}', max_fee_per_gas={}, max_priority_fee={})",
            self.inner.fees.gas_price,
            self.inner.fees.gas_used,
            self.inner.fees.txn_fee.to_string().parse::<f64>().unwrap_or(0.0) / 1e18,
            self.inner.fees.protocol_type,
            self.inner.fees.max_fee_per_gas.map_or("None".to_string(), |v| v.to_string()),
            self.inner.fees.max_priority_fee.map_or("None".to_string(), |v| v.to_string())
        );
        
        // Format state changes - show all
        let state_changes: Vec<String> = self.inner.state_changes.iter()
            .map(|(addr, val)| format!("'0x{}': {}", 
                hex::encode(addr).to_uppercase(),
                serde_json::to_string(&val).unwrap_or_else(|_| "{}".to_string())
            ))
            .collect();
        let state_changes_repr = format!("{{{}}}", state_changes.join(", "));
        
        format!(
            "ProcessedTransaction(hash='{}', block_number={}, block_timestamp={}, txn_index={}, from_address='0x{}', to_address={}, contract_address={}, value={}, status={}, nonce={}, txn_type='{}', actions={:?}, fees={}, bribe_amount={}, unique_addresses={}, erc20_contracts={}, eth_transfers={}, erc20_transfers={}, erc721_transfers={}, erc1155_transfers={}, internal_transactions={}, uniswap_v2_syncs={}, uniswap_v2_swaps={}, approvals={}, mints={}, burns={}, deposits={}, withdraws={}, pair_events={}, owner_events={}, contract_creation_events={}, trading_enabled_events={}, trading_disabled_events={}, uniswap_v3_pools={}, uniswap_v3_initializations={}, uniswap_v3_burns={}, uniswap_v3_mints={}, uniswap_v3_swaps={}, uniswap_v3_positions={}, uniswap_v3_increases={}, uniswap_v3_decreases={}, uniswap_v4_initializes={}, uniswap_v4_modifies={}, uniswap_v4_swaps={}, permit2_events={}, other_events={}, state_changes={}, latest_states={}, input='{}')",
            self.hash,
            self.block_number,
            self.block_timestamp,
            self.txn_index,
            hex::encode(self.inner.from_address).to_uppercase(),
            self.to_address.as_ref().map_or("None".to_string(), |a| format!("'0x{}'", a.to_uppercase())),
            self.contract_address.as_ref().map_or("None".to_string(), |a| format!("'0x{}'", a.to_uppercase())),
            if self.value == "0" { "0.0".to_string() } else { format!("{:.18}", self.value.parse::<f64>().unwrap_or(0.0) / 1e18) },
            if self.status == "1" { "True" } else { "False" },
            self.nonce,
            self.txn_type,
            self.actions,
            fees_repr,
            self.bribe_amount,
            unique_addrs_str,
            erc20_contracts_str,
            format_list("eth_transfers", self.inner.eth_transfers.len()),
            erc20_transfers_repr,
            format_list("erc721_transfers", self.inner.erc721_transfers.len()),
            format_list("erc1155_transfers", self.inner.erc1155_transfers.len()),
            internal_repr,
            format_list("uniswap_v2_syncs", self.inner.uniswap_v2_syncs.len()),
            format_list("uniswap_v2_swaps", self.inner.uniswap_v2_swaps.len()),
            format_list("approvals", self.inner.approvals.len()),
            format_list("mints", self.inner.mints.len()),
            format_list("burns", self.inner.burns.len()),
            format_list("deposits", self.inner.deposits.len()),
            format_list("withdraws", self.inner.withdraws.len()),
            format_list("pair_events", self.inner.pair_events.len()),
            format_list("owner_events", self.inner.owner_events.len()),
            format_list("contract_creation_events", self.inner.contract_creation_events.len()),
            format_list("trading_enabled_events", self.inner.trading_enabled_events.len()),
            format_list("trading_disabled_events", self.inner.trading_disabled_events.len()),
            format_list("uniswap_v3_pools", self.inner.uniswap_v3_pools.len()),
            format_list("uniswap_v3_initializations", self.inner.uniswap_v3_initializations.len()),
            format_list("uniswap_v3_burns", self.inner.uniswap_v3_burns.len()),
            format_list("uniswap_v3_mints", self.inner.uniswap_v3_mints.len()),
            format_list("uniswap_v3_swaps", self.inner.uniswap_v3_swaps.len()),
            format_list("uniswap_v3_positions", self.inner.uniswap_v3_positions.len()),
            format_list("uniswap_v3_increases", self.inner.uniswap_v3_increases.len()),
            format_list("uniswap_v3_decreases", self.inner.uniswap_v3_decreases.len()),
            format_list("uniswap_v4_initializes", self.inner.uniswap_v4_initializes.len()),
            format_list("uniswap_v4_modifies", self.inner.uniswap_v4_modifies.len()),
            format_list("uniswap_v4_swaps", self.inner.uniswap_v4_swaps.len()),
            format_list("permit2_events", self.inner.permit2_events.len()),
            format_list("other_events", self.inner.other_events.len()),
            state_changes_repr,
            format_list("latest_states", self.inner.latest_states.len()),
            self.input
        )
    }
}