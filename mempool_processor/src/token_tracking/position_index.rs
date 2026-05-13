use alloy_primitives::U256;
use std::collections::{HashMap, HashSet};

use super::types::{Address, ConcentratedPositionApprovalContext, Pool};

pub(super) fn normalize_pool_positions(pool: &mut Pool) {
    for position in &mut pool.liquidity_positions {
        position.owner = normalize_address(&position.owner);
        position.position_manager_address = position
            .position_manager_address
            .take()
            .or_else(|| pool.position_manager_address.clone())
            .map(|address| normalize_address(&address));
        position.token_id = position
            .token_id
            .as_deref()
            .and_then(parse_position_token_id)
            .map(|token_id| format!("{token_id:#x}"));
        if position.position_share_pct.is_none() {
            position.position_share_pct =
                position_share_pct(&position.liquidity, pool.lp_total_supply);
        }
    }
}

pub(super) fn add_position_indexes_for_pool(
    pool: &Pool,
    position_manager_to_pools: &mut HashMap<Address, HashSet<Address>>,
    position_token_to_context: &mut HashMap<String, ConcentratedPositionApprovalContext>,
    position_owner_manager_to_context: &mut HashMap<
        String,
        Vec<ConcentratedPositionApprovalContext>,
    >,
) {
    for position in &pool.liquidity_positions {
        let Some(position_manager) = position
            .position_manager_address
            .as_ref()
            .or(pool.position_manager_address.as_ref())
        else {
            continue;
        };
        if position.owner.is_empty() || position.liquidity.trim().is_empty() {
            continue;
        }

        position_manager_to_pools
            .entry(position_manager.clone())
            .or_default()
            .insert(pool.address.clone());

        let context = ConcentratedPositionApprovalContext {
            token_address: pool.token_address.clone(),
            pool_address: pool.address.clone(),
            pool_type: pool.pool_type.clone(),
            denom_address: pool.denom_address.clone(),
            denom_currency: pool.denom_currency.clone(),
            position_manager_address: position_manager.clone(),
            position_id: position.position_id.clone(),
            token_id: position.token_id.clone(),
            owner: position.owner.clone(),
            position_liquidity: position.liquidity.clone(),
            pool_liquidity: pool.lp_total_supply.map(|value| value.to_string()),
            position_share_pct: position
                .position_share_pct
                .or_else(|| position_share_pct(&position.liquidity, pool.lp_total_supply)),
        };

        if let Some(token_id) = position
            .token_id
            .as_deref()
            .and_then(parse_position_token_id)
            .or_else(|| parse_position_token_id(&position.position_id))
        {
            position_token_to_context.insert(
                position_token_key(position_manager, token_id),
                context.clone(),
            );
        }

        position_owner_manager_to_context
            .entry(position_owner_manager_key(
                position_manager,
                &position.owner,
            ))
            .or_default()
            .push(context);
    }
}

pub(super) fn remove_position_indexes_for_pool(
    pool_address: &str,
    position_manager_to_pools: &mut HashMap<Address, HashSet<Address>>,
    position_token_to_context: &mut HashMap<String, ConcentratedPositionApprovalContext>,
    position_owner_manager_to_context: &mut HashMap<
        String,
        Vec<ConcentratedPositionApprovalContext>,
    >,
) {
    let pool_address = normalize_address(pool_address);
    position_token_to_context.retain(|_, context| context.pool_address != pool_address);
    position_owner_manager_to_context.retain(|_, contexts| {
        contexts.retain(|context| context.pool_address != pool_address);
        !contexts.is_empty()
    });
    position_manager_to_pools.retain(|_, pools| {
        pools.remove(&pool_address);
        !pools.is_empty()
    });
}

pub(super) fn position_token_key(position_manager: &str, token_id: U256) -> String {
    format!("{}:{token_id:#x}", normalize_address(position_manager))
}

pub(super) fn position_owner_manager_key(position_manager: &str, owner: &str) -> String {
    format!(
        "{}:{}",
        normalize_address(position_manager),
        normalize_address(owner)
    )
}

fn position_share_pct(position_liquidity: &str, pool_liquidity: Option<f64>) -> Option<f64> {
    let position_liquidity = parse_numeric_string(position_liquidity)?;
    let pool_liquidity = pool_liquidity?;
    if position_liquidity <= 0.0 || pool_liquidity <= 0.0 {
        return None;
    }
    Some(((position_liquidity / pool_liquidity) * 100.0).min(100.0))
}

fn parse_numeric_string(value: &str) -> Option<f64> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    if let Some(hex) = value.strip_prefix("0x") {
        return U256::from_str_radix(hex, 16)
            .ok()
            .map(|value| value.to_string())
            .and_then(|value| value.parse::<f64>().ok());
    }
    value.parse::<f64>().ok()
}

fn parse_position_token_id(value: &str) -> Option<U256> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    if let Some(hex) = value.strip_prefix("0x") {
        return U256::from_str_radix(hex, 16).ok();
    }
    value.parse::<U256>().ok()
}

fn normalize_address(address: &str) -> String {
    address.trim().to_ascii_lowercase()
}
