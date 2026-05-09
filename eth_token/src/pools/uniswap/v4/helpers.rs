use std::collections::{BTreeMap, BTreeSet};

use alloy_primitives::{Address, B256, U256};
use eyre::{eyre, Result};
use serde::Serialize;
use serde_json::{json, Value};
use tx_processor::tx_processor::data_models::{
    ERC721TransferEvent, UniswapV4ModifyLiquidityEvent as ProcessedV4ModifyLiquidityEvent,
};
use tx_processor::ProcessedTransaction;

use super::super::v2::{LPApprovalSnapshot, UniswapV2TxContext};
use super::UniswapV4Pool;

pub(super) fn token0_decimals(pool: &UniswapV4Pool) -> u8 {
    if pool.base.config.token1_is_denom.unwrap_or(false) {
        pool.base.config.token_decimals
    } else {
        pool.denom_decimals()
    }
}

pub(super) fn token1_decimals(pool: &UniswapV4Pool) -> u8 {
    if pool.base.config.token1_is_denom.unwrap_or(false) {
        pool.denom_decimals()
    } else {
        pool.base.config.token_decimals
    }
}

pub(super) fn event_json<T: Serialize>(event: &T, tx: &UniswapV2TxContext) -> Value {
    let mut value = serde_json::to_value(event).unwrap_or_else(|_| json!({}));
    if let Some(object) = value.as_object_mut() {
        object.insert("block_number".to_string(), json!(tx.block_number));
        object.insert("block_timestamp".to_string(), json!(tx.block_timestamp));
        object.insert("tx_hash".to_string(), json!(tx.tx_hash));
    }
    value
}

pub(super) fn add_lp_approval_amount(
    approvals: &mut BTreeMap<String, BTreeMap<String, LPApprovalSnapshot>>,
    known_routers: &BTreeSet<String>,
    owner: &str,
    spender: &str,
    amount: f64,
    block_number: u64,
    tx_hash: &str,
) {
    let owner = normalize_address(owner);
    let spender = normalize_address(spender);
    let holder_approvals = approvals.entry(owner).or_default();
    let entry = holder_approvals
        .entry(spender.clone())
        .or_insert_with(|| LPApprovalSnapshot {
            amount: 0.0,
            tx_hash: tx_hash.to_string(),
            block_number,
            is_router: known_routers.contains(&spender),
        });
    entry.amount += amount;
    if block_number >= entry.block_number {
        entry.block_number = block_number;
        entry.tx_hash = tx_hash.to_string();
    }
    entry.is_router = known_routers.contains(&spender);
}

pub(super) fn set_lp_approval_amount_at_least(
    approvals: &mut BTreeMap<String, BTreeMap<String, LPApprovalSnapshot>>,
    known_routers: &BTreeSet<String>,
    owner: &str,
    spender: &str,
    amount: f64,
    block_number: u64,
    tx_hash: &str,
) {
    let owner = normalize_address(owner);
    let spender = normalize_address(spender);
    let holder_approvals = approvals.entry(owner).or_default();
    let entry = holder_approvals
        .entry(spender.clone())
        .or_insert_with(|| LPApprovalSnapshot {
            amount,
            tx_hash: tx_hash.to_string(),
            block_number,
            is_router: known_routers.contains(&spender),
        });
    entry.amount = entry.amount.max(amount);
    if block_number >= entry.block_number {
        entry.block_number = block_number;
        entry.tx_hash = tx_hash.to_string();
    }
    entry.is_router = known_routers.contains(&spender);
}

pub(super) fn position_transfer_for_modify_event<'a>(
    event: &ProcessedV4ModifyLiquidityEvent,
    transaction: &'a ProcessedTransaction,
) -> Option<&'a ERC721TransferEvent> {
    let token_id = position_token_id(event.salt);
    transaction
        .erc721_transfers
        .iter()
        .filter(|transfer| same_address(&transfer.token_address, &address_string(&event.sender)))
        .filter(|transfer| transfer.token_id == token_id)
        .min_by_key(|transfer| transfer.log_index.abs_diff(event.log_index))
}

pub(super) fn owner_from_position_transfer(transfer: &ERC721TransferEvent) -> Option<String> {
    if !transfer.to_address.is_zero() {
        Some(address_string(&transfer.to_address))
    } else if !transfer.from_address.is_zero() {
        Some(address_string(&transfer.from_address))
    } else {
        None
    }
}

pub(super) fn position_token_id(salt: B256) -> U256 {
    U256::from_be_slice(salt.as_slice())
}

pub(super) fn apply_liquidity_delta(current: u128, delta: i128) -> u128 {
    if delta >= 0 {
        current.saturating_add(delta as u128)
    } else {
        current.saturating_sub(delta.unsigned_abs())
    }
}

pub(super) fn parse_address(value: &str) -> Result<Address> {
    value
        .parse()
        .map_err(|err| eyre!("invalid address {value}: {err}"))
}

pub(super) fn parse_hash(value: &str) -> Result<B256> {
    value
        .parse()
        .map_err(|err| eyre!("invalid pool id {value}: {err}"))
}

pub(super) fn same_address(address: &Address, value: &str) -> bool {
    address_string(address) == normalize_address(value)
}

pub(super) fn address_string(address: &Address) -> String {
    format!("{address:#x}")
}

pub(super) fn hash_string(hash: &B256) -> String {
    format!("{hash:#x}")
}

pub(super) fn normalize_address(value: impl AsRef<str>) -> String {
    value.as_ref().trim().to_ascii_lowercase()
}

pub(super) fn normalize_address_string(value: impl Into<String>) -> String {
    normalize_address(value.into())
}

pub(super) fn normalize_hash_string(value: impl AsRef<str>) -> String {
    let value = value.as_ref().trim().to_ascii_lowercase();
    if value.starts_with("0x") {
        value
    } else {
        format!("0x{value}")
    }
}
