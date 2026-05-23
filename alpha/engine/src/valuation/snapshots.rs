use eth_alpha_core::{
    amount::DecimalAmount,
    market::PoolSnapshot,
    position::{Position, PositionSnapshot, PositionState},
};

use crate::PositionValueSimulation;

const DISPLAY_ZERO_VALUE_SCALE: u32 = 6;

pub(crate) fn should_snapshot_position_for_pool(position: &Position) -> bool {
    if position.drained {
        return position.has_exposure();
    }
    matches!(
        position.state,
        PositionState::BuyConfirmed | PositionState::SellFailed | PositionState::SellCancelled
    )
}

pub(crate) fn zero_value_snapshot(
    position: &Position,
    block_number: u64,
    pool: Option<&PoolSnapshot>,
) -> PositionSnapshot {
    let pool = valuation_safe_pool(pool, block_number);
    let cost = position.entry_cost_basis.unwrap_or_default();
    let realized = position.realized_pnl();
    let unrealized = if position.is_closed() {
        DecimalAmount::ZERO
    } else {
        -cost
    };
    let roi = if cost.is_zero() {
        DecimalAmount::ZERO
    } else if position.is_closed() {
        realized / cost
    } else {
        DecimalAmount::from(-1)
    };
    let snapshot = PositionSnapshot {
        position_id: position.id.clone(),
        trade_id: position.trade_id.clone(),
        state: position.state.clone(),
        block_number,
        observed_block_number: pool.map(|pool| pool.latest_block).or(Some(block_number)),
        valuation_block_number: Some(block_number),
        current_value_eth: DecimalAmount::ZERO,
        realized_profit_eth: realized,
        unrealized_profit_eth: unrealized,
        roi,
        pool_price_to_initial_price_ratio: None,
        pool_initial_price_denom_per_token: None,
        pool_price_denom_per_token: None,
        pool_liquidity_denom: None,
        pool_token_reserve: None,
        pool_denom_symbol: None,
    };
    snapshot_with_pool_metrics(snapshot, pool)
}

pub(crate) fn simulated_value_snapshot(
    position: &Position,
    simulation: PositionValueSimulation,
    pool: Option<&PoolSnapshot>,
) -> PositionSnapshot {
    let pool = valuation_safe_pool(pool, simulation.block_number);
    let current_value = simulation.current_value.to_decimal();
    let cost = position.entry_cost_basis.unwrap_or_default();
    let realized = position.realized_pnl();
    let snapshot = PositionSnapshot {
        position_id: position.id.clone(),
        trade_id: position.trade_id.clone(),
        state: position.state.clone(),
        block_number: simulation.block_number,
        observed_block_number: pool.map(|pool| pool.latest_block),
        valuation_block_number: Some(simulation.block_number),
        current_value_eth: current_value,
        realized_profit_eth: realized,
        unrealized_profit_eth: if cost.is_zero() {
            DecimalAmount::ZERO
        } else {
            current_value - cost
        },
        roi: if cost.is_zero() {
            DecimalAmount::ZERO
        } else {
            ((current_value + realized) / cost) - DecimalAmount::from(1)
        },
        pool_price_to_initial_price_ratio: None,
        pool_initial_price_denom_per_token: None,
        pool_price_denom_per_token: None,
        pool_liquidity_denom: None,
        pool_token_reserve: None,
        pool_denom_symbol: None,
    };
    snapshot_with_pool_metrics(snapshot, pool)
}

pub(crate) fn snapshot_with_pool_metrics(
    mut snapshot: PositionSnapshot,
    pool: Option<&PoolSnapshot>,
) -> PositionSnapshot {
    let Some(pool) = pool else {
        return snapshot;
    };
    let valuation_block = snapshot
        .valuation_block_number
        .unwrap_or(snapshot.block_number);
    if pool.latest_block > valuation_block {
        return snapshot;
    }
    if is_display_zero_value(snapshot.current_value_eth) {
        return snapshot;
    }

    snapshot.pool_price_to_initial_price_ratio = pool.price_ratio_to_initial;
    snapshot.pool_initial_price_denom_per_token = pool.initial_price_denom_per_token;
    snapshot.pool_price_denom_per_token = pool.price_denom_per_token;
    snapshot.pool_liquidity_denom = Some(pool.denom_reserve);
    snapshot.pool_token_reserve = Some(pool.token_reserve);
    snapshot.pool_denom_symbol = pool.denom_symbol.clone();

    snapshot
}

pub(crate) fn valuation_safe_pool(
    pool: Option<&PoolSnapshot>,
    valuation_block: u64,
) -> Option<&PoolSnapshot> {
    pool.filter(|pool| pool.latest_block <= valuation_block)
}

fn is_display_zero_value(value: DecimalAmount) -> bool {
    value.abs() <= DecimalAmount::new(1, DISPLAY_ZERO_VALUE_SCALE)
}
