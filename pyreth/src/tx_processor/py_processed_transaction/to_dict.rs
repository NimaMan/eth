use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use reth_chain_query::to_checksum_address;
use serde_json::Value as JsonValue;

use super::{
    convert_currency_amount, json_to_python, token_movements_to_python, u256_to_py,
    PyProcessedTransaction,
};

pub(super) fn build_processed_transaction_dict(
    wrapper: &PyProcessedTransaction,
    py: Python,
) -> PyResult<Py<PyDict>> {
    let dict = PyDict::new_bound(py);

    // Core fields
    dict.set_item("hash", &wrapper.hash)?;
    dict.set_item("block_number", wrapper.block_number)?;
    dict.set_item("block_timestamp", wrapper.block_timestamp)?;
    dict.set_item("tx_index", wrapper.tx_index)?;
    dict.set_item("from_address", &wrapper.from_address)?;
    dict.set_item("to_address", &wrapper.to_address)?;
    dict.set_item("contract_address", &wrapper.contract_address)?;
    dict.set_item("value", u256_to_py(py, &wrapper.inner.value)?)?;
    dict.set_item("status", &wrapper.status)?;
    dict.set_item("nonce", wrapper.nonce)?;
    dict.set_item("raw_tx_type", wrapper.raw_tx_type)?;
    dict.set_item("input", &wrapper.input)?;
    dict.set_item("tx_type", &wrapper.tx_type)?;
    dict.set_item("actions", &wrapper.actions)?;
    dict.set_item("tx_number", wrapper.tx_number)?;

    // Fees
    let fees = PyDict::new_bound(py);
    fees.set_item("gas_price", u256_to_py(py, &wrapper.inner.fees.gas_price)?)?;
    fees.set_item("gas_used", wrapper.inner.fees.gas_used)?;
    fees.set_item("gas_limit", wrapper.inner.fees.gas_limit)?;
    fees.set_item("tx_fee", u256_to_py(py, &wrapper.inner.fees.tx_fee)?)?;
    fees.set_item("protocol_type", &wrapper.inner.fees.protocol_type)?;
    if let Some(v) = &wrapper.inner.fees.max_fee_per_gas {
        fees.set_item("max_fee_per_gas", u256_to_py(py, v)?)?;
    } else {
        fees.set_item("max_fee_per_gas", py.None())?;
    }
    if let Some(v) = &wrapper.inner.fees.max_priority_fee {
        fees.set_item("max_priority_fee", u256_to_py(py, v)?)?;
    } else {
        fees.set_item("max_priority_fee", py.None())?;
    }
    if let Some(v) = &wrapper.inner.fees.max_fee_per_blob_gas {
        fees.set_item("max_fee_per_blob_gas", u256_to_py(py, v)?)?;
    } else {
        fees.set_item("max_fee_per_blob_gas", py.None())?;
    }
    if let Some(v) = wrapper.inner.fees.blob_gas_used {
        fees.set_item("blob_gas_used", v)?;
    } else {
        fees.set_item("blob_gas_used", py.None())?;
    }
    dict.set_item("fees", fees)?;
    dict.set_item("bribe_amount", u256_to_py(py, &wrapper.inner.bribe_amount)?)?;

    // Sets to lists
    let uniq = PyList::empty_bound(py);
    for a in &wrapper.inner.unique_addresses {
        uniq.append(to_checksum_address(a))?;
    }
    dict.set_item("unique_addresses", uniq)?;

    let erc20c = PyList::empty_bound(py);
    for a in &wrapper.inner.erc20_contracts {
        erc20c.append(to_checksum_address(a))?;
    }
    dict.set_item("erc20_contracts", erc20c)?;

    let erc721c = PyList::empty_bound(py);
    for a in &wrapper.inner.erc721_contracts {
        erc721c.append(to_checksum_address(a))?;
    }
    dict.set_item("erc721_contracts", erc721c)?;

    let erc1155c = PyList::empty_bound(py);
    for a in &wrapper.inner.erc1155_contracts {
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
        vec_to_pylist(py, &wrapper.inner.eth_transfers)?,
    )?;
    dict.set_item(
        "erc20_transfers",
        vec_to_pylist(py, &wrapper.inner.erc20_transfers)?,
    )?;
    dict.set_item(
        "erc721_transfers",
        vec_to_pylist(py, &wrapper.inner.erc721_transfers)?,
    )?;
    dict.set_item(
        "erc1155_transfers",
        vec_to_pylist(py, &wrapper.inner.erc1155_transfers)?,
    )?;
    dict.set_item(
        "internal_transactions",
        vec_to_pylist(py, &wrapper.inner.internal_transactions)?,
    )?;

    dict.set_item(
        "uniswap_v2_syncs",
        vec_to_pylist(py, &wrapper.inner.uniswap_v2_syncs)?,
    )?;
    dict.set_item(
        "uniswap_v2_swaps",
        vec_to_pylist(py, &wrapper.inner.uniswap_v2_swaps)?,
    )?;

    dict.set_item(
        "erc20_approval_events",
        vec_to_pylist(py, &wrapper.inner.erc20_approval_events)?,
    )?;
    dict.set_item(
        "erc721_approval_events",
        vec_to_pylist(py, &wrapper.inner.erc721_approval_events)?,
    )?;
    dict.set_item(
        "uniswap_v2_mints",
        vec_to_pylist(py, &wrapper.inner.uniswap_v2_mints)?,
    )?;
    dict.set_item(
        "uniswap_v2_burns",
        vec_to_pylist(py, &wrapper.inner.uniswap_v2_burns)?,
    )?;
    dict.set_item(
        "deposit_events",
        vec_to_pylist(py, &wrapper.inner.deposit_events)?,
    )?;
    dict.set_item(
        "withdraw_events",
        vec_to_pylist(py, &wrapper.inner.withdraw_events)?,
    )?;
    dict.set_item(
        "uniswap_v2_pair_created_events",
        vec_to_pylist(py, &wrapper.inner.uniswap_v2_pair_created_events)?,
    )?;
    dict.set_item(
        "ownership_transferred_events",
        vec_to_pylist(py, &wrapper.inner.ownership_transferred_events)?,
    )?;
    dict.set_item(
        "ownership_transfer_started_events",
        vec_to_pylist(py, &wrapper.inner.ownership_transfer_started_events)?,
    )?;
    dict.set_item(
        "access_control_role_granted_events",
        vec_to_pylist(py, &wrapper.inner.access_control_role_granted_events)?,
    )?;
    dict.set_item(
        "access_control_role_revoked_events",
        vec_to_pylist(py, &wrapper.inner.access_control_role_revoked_events)?,
    )?;
    dict.set_item(
        "proxy_admin_changed_events",
        vec_to_pylist(py, &wrapper.inner.proxy_admin_changed_events)?,
    )?;
    dict.set_item(
        "contract_creation_events",
        vec_to_pylist(py, &wrapper.inner.contract_creation_events)?,
    )?;
    dict.set_item(
        "trading_enabled_events",
        vec_to_pylist(py, &wrapper.inner.trading_enabled_events)?,
    )?;
    dict.set_item(
        "trading_disabled_events",
        vec_to_pylist(py, &wrapper.inner.trading_disabled_events)?,
    )?;

    dict.set_item(
        "uniswap_v3_pools",
        vec_to_pylist(py, &wrapper.inner.uniswap_v3_pools)?,
    )?;
    dict.set_item(
        "uniswap_v3_initializations",
        vec_to_pylist(py, &wrapper.inner.uniswap_v3_initializations)?,
    )?;
    dict.set_item(
        "uniswap_v3_burns",
        vec_to_pylist(py, &wrapper.inner.uniswap_v3_burns)?,
    )?;
    dict.set_item(
        "uniswap_v3_mints",
        vec_to_pylist(py, &wrapper.inner.uniswap_v3_mints)?,
    )?;
    dict.set_item(
        "uniswap_v3_swaps",
        vec_to_pylist(py, &wrapper.inner.uniswap_v3_swaps)?,
    )?;
    dict.set_item(
        "uniswap_v3_positions",
        vec_to_pylist(py, &wrapper.inner.uniswap_v3_positions)?,
    )?;
    dict.set_item(
        "uniswap_v3_increases",
        vec_to_pylist(py, &wrapper.inner.uniswap_v3_increases)?,
    )?;
    dict.set_item(
        "uniswap_v3_decreases",
        vec_to_pylist(py, &wrapper.inner.uniswap_v3_decreases)?,
    )?;

    dict.set_item(
        "uniswap_v4_initializes",
        vec_to_pylist(py, &wrapper.inner.uniswap_v4_initializes)?,
    )?;
    dict.set_item(
        "uniswap_v4_modifies",
        vec_to_pylist(py, &wrapper.inner.uniswap_v4_modifies)?,
    )?;
    dict.set_item(
        "uniswap_v4_swaps",
        vec_to_pylist(py, &wrapper.inner.uniswap_v4_swaps)?,
    )?;
    dict.set_item(
        "uniswap_v4_donates",
        vec_to_pylist(py, &wrapper.inner.uniswap_v4_donates)?,
    )?;
    dict.set_item(
        "uniswap_v4_protocol_fee_updates",
        vec_to_pylist(py, &wrapper.inner.uniswap_v4_protocol_fee_updates)?,
    )?;
    dict.set_item(
        "uniswap_v4_dynamic_lp_fee_updates",
        vec_to_pylist(py, &wrapper.inner.uniswap_v4_dynamic_lp_fee_updates)?,
    )?;
    dict.set_item(
        "uniswap_v4_protocol_fee_controller_updates",
        vec_to_pylist(
            py,
            &wrapper.inner.uniswap_v4_protocol_fee_controller_updates,
        )?,
    )?;
    dict.set_item(
        "uniswap_v4_balance_deltas",
        vec_to_pylist(py, &wrapper.inner.uniswap_v4_balance_deltas)?,
    )?;
    dict.set_item(
        "permit2_events",
        vec_to_pylist(py, &wrapper.inner.permit2_events)?,
    )?;
    let access_list = PyList::empty_bound(py);
    for item in &wrapper.inner.access_list {
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
    for hash in &wrapper.inner.blob_versioned_hashes {
        blob_hashes.append(format!("{:#x}", hash))?;
    }
    dict.set_item("blob_versioned_hashes", blob_hashes)?;
    let auth_list = PyList::empty_bound(py);
    for auth in &wrapper.inner.signed_authorizations {
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
    for ev in &wrapper.inner.other_events {
        let j = serde_json::to_value(ev).unwrap_or(JsonValue::Null);
        other.append(json_to_python(py, &j)?)?;
    }
    dict.set_item("other_events", other)?;

    // Address balance changes: expose map keyed by checksum addresses
    let sc = PyDict::new_bound(py);
    for (addr, changes) in &wrapper.inner.address_balance_changes {
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
        entry.set_item(
            "movements",
            token_movements_to_python(py, &changes.movements)?,
        )?;
        sc.set_item(addr_str, entry)?;
    }
    dict.set_item("address_balance_changes", sc)?;

    // latest_states map
    let ls = PyDict::new_bound(py);
    for (addr, val) in &wrapper.inner.latest_states {
        let addr_str = to_checksum_address(addr);
        let j = serde_json::to_value(val).unwrap_or(JsonValue::Null);
        ls.set_item(addr_str, json_to_python(py, &j)?)?;
    }
    dict.set_item("latest_states", ls)?;

    Ok(dict.unbind())
}
