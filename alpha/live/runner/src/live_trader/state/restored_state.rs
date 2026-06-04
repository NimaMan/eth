use std::collections::HashMap;

use alloy_primitives::U256;
use eth_alpha_core::{
    ids::PoolAddress, portfolio::PortfolioState, position::PositionState, store::TradingStore,
};
use eth_alpha_store::{ActiveHoldCounterRecord, PostgresTradingStore};
use eth_strategies::{shared_rules::live::LiveStrategySpec, RestoredEntryBankroll};
use eyre::{Result, WrapErr};

use super::{
    bankroll::restored_entry_bankroll_from_terminal_positions,
    position_state::release_stale_submitted_position, support::parse_u256_decimal,
};

pub(super) struct RestoredRuntimeState {
    pub(super) portfolio: PortfolioState,
    pub(super) seen_pools_by_strategy: HashMap<String, Vec<PoolAddress>>,
    pub(super) active_hold_counters_by_strategy: HashMap<String, Vec<ActiveHoldCounterRecord>>,
    pub(super) entry_bankrolls_by_strategy: HashMap<String, RestoredEntryBankroll>,
    pub(super) entry_bankroll_position_count: usize,
    pub(super) entry_bankroll_accounted_pool_count: usize,
    pub(super) entry_bankroll_spent_wei: U256,
    pub(super) entry_bankroll_recovered_wei: U256,
    pub(super) stale_submitted_positions: usize,
    pub(super) active_position_count: usize,
}

pub(super) async fn restore_runtime_state(
    store: &PostgresTradingStore,
    strategy_specs: &[LiveStrategySpec],
) -> Result<RestoredRuntimeState> {
    let mut portfolio = PortfolioState::default();
    let mut seen_pools_by_strategy = HashMap::new();
    let mut active_hold_counters_by_strategy = HashMap::new();
    let mut entry_bankrolls_by_strategy = HashMap::new();
    let mut entry_bankroll_position_count = 0usize;
    let mut stale_submitted_positions = 0usize;

    for spec in strategy_specs {
        let buy_wei = parse_u256_decimal(&spec.buy_wei).wrap_err_with(|| {
            format!(
                "invalid buy_wei for live strategy {}: {}",
                spec.strategy_name, spec.buy_wei
            )
        })?;
        let seen_pools = store
            .load_seen_pools(&spec.strategy_name)
            .await
            .wrap_err_with(|| {
                format!(
                    "failed to restore seen alpha pools for {}",
                    spec.strategy_name
                )
            })?;
        seen_pools_by_strategy.insert(spec.strategy_name.clone(), seen_pools);

        let active_hold_counters = store
            .load_active_hold_counters(&spec.strategy_name)
            .await
            .wrap_err_with(|| {
                format!(
                    "failed to restore active hold counters for {}",
                    spec.strategy_name
                )
            })?;
        active_hold_counters_by_strategy.insert(spec.strategy_name.clone(), active_hold_counters);

        let terminal_positions = store
            .load_terminal_positions(&spec.strategy_name)
            .await
            .wrap_err_with(|| {
                format!(
                    "failed to restore terminal alpha positions for {}",
                    spec.strategy_name
                )
            })?;
        entry_bankroll_position_count += terminal_positions.len();
        let restored_entry_bankroll =
            restored_entry_bankroll_from_terminal_positions(&terminal_positions, buy_wei);
        entry_bankrolls_by_strategy.insert(spec.strategy_name.clone(), restored_entry_bankroll);
        for position in terminal_positions
            .into_iter()
            .filter(|position| position.state == PositionState::BuyDeferred)
        {
            portfolio.positions.insert(position.id.clone(), position);
        }

        let restored_positions = store
            .load_active_positions(&spec.strategy_name)
            .await
            .wrap_err_with(|| {
                format!(
                    "failed to restore active alpha positions for {}",
                    spec.strategy_name
                )
            })?;
        for mut position in restored_positions {
            if release_stale_submitted_position(&mut position) {
                stale_submitted_positions += 1;
                store.upsert_position(&position).await.wrap_err_with(|| {
                    format!(
                        "failed to persist stale submitted position recovery for {}",
                        position.id.0
                    )
                })?;
            }
            portfolio.positions.insert(position.id.clone(), position);
        }
    }

    let active_position_count = portfolio.active_position_count();
    let entry_bankroll_accounted_pool_count = entry_bankrolls_by_strategy
        .values()
        .map(RestoredEntryBankroll::accounted_pool_count)
        .sum::<usize>();
    let entry_bankroll_spent_wei = entry_bankrolls_by_strategy
        .values()
        .fold(U256::ZERO, |acc, bankroll| {
            acc.saturating_add(bankroll.spent_wei())
        });
    let entry_bankroll_recovered_wei = entry_bankrolls_by_strategy
        .values()
        .fold(U256::ZERO, |acc, bankroll| {
            acc.saturating_add(bankroll.recovered_wei())
        });

    Ok(RestoredRuntimeState {
        portfolio,
        seen_pools_by_strategy,
        active_hold_counters_by_strategy,
        entry_bankrolls_by_strategy,
        entry_bankroll_position_count,
        entry_bankroll_accounted_pool_count,
        entry_bankroll_spent_wei,
        entry_bankroll_recovered_wei,
        stale_submitted_positions,
        active_position_count,
    })
}
