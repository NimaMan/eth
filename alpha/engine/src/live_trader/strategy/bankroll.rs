use alloy_primitives::U256;
use eth_alpha_core::{
    amount::Amount,
    position::{Position, PositionState},
};
use eth_strategies::{shared_rules::live::LiveStrategySpec, RestoredEntryBankroll};
use eyre::Result;
use serde_json::{json, Value};

use super::support::parse_eth_decimal_to_wei;

pub(super) fn resolve_entry_bankroll_wei(spec: &LiveStrategySpec) -> Result<Option<U256>> {
    let Some(value) = spec.entry_bankroll_eth.as_deref() else {
        return Ok(None);
    };
    let label = format!("strategy {} entry_bankroll_eth", spec.strategy_name);
    parse_eth_decimal_to_wei(value, &label).map(Some)
}

pub(super) fn entry_bankroll_summary_json(
    specs: &[LiveStrategySpec],
    bankrolls_wei: &[Option<U256>],
) -> Vec<Value> {
    specs
        .iter()
        .zip(bankrolls_wei.iter())
        .map(|(spec, bankroll_wei)| {
            let source = if spec.entry_bankroll_eth.is_some() {
                "strategy_spec"
            } else {
                "none"
            };
            json!({
                "strategy_name": &spec.strategy_name,
                "entry_bankroll_eth": spec.entry_bankroll_eth.as_deref(),
                "entry_bankroll_wei": bankroll_wei.as_ref().map(|value| value.to_string()),
                "source": source,
            })
        })
        .collect()
}

pub(super) fn validate_live_real_entry_bankrolls(
    runner_name: &str,
    specs: &[LiveStrategySpec],
    bankrolls_wei: &[Option<U256>],
    validation_limit_eth: &str,
) -> Result<()> {
    let validation_limit =
        parse_eth_decimal_to_wei(validation_limit_eth, "live-real validation entry bankroll")?;
    for (spec, bankroll) in specs.iter().zip(bankrolls_wei.iter()) {
        let bankroll = bankroll.ok_or_else(|| {
            eyre::eyre!(
                "{} requires an entry bankroll <= {} for strategy {} while live-real entries are in validation mode",
                runner_name,
                validation_limit_eth,
                spec.strategy_name
            )
        })?;
        if bankroll.is_zero() || bankroll > validation_limit {
            return Err(eyre::eyre!(
                "{} requires entry bankroll in the range (0, {}] for strategy {}; got spec {:?}",
                runner_name,
                validation_limit_eth,
                spec.strategy_name,
                &spec.entry_bankroll_eth
            ));
        }
    }
    Ok(())
}

fn position_entry_spend_wei(position: &Position, fallback_buy_wei: U256) -> U256 {
    position
        .entry_cost_basis
        .map(|cost| Amount::from_decimal(cost, 18).raw)
        .unwrap_or(fallback_buy_wei)
}

fn position_exit_proceeds_wei(position: &Position) -> U256 {
    position
        .exit_proceeds
        .map(|proceeds| Amount::from_decimal(proceeds, 18).raw)
        .unwrap_or(U256::ZERO)
}

pub(super) fn restored_entry_bankroll_from_terminal_positions(
    positions: &[Position],
    fallback_buy_wei: U256,
) -> RestoredEntryBankroll {
    let mut bankroll = RestoredEntryBankroll::default();
    for position in positions {
        match position.state {
            PositionState::BuyFailed | PositionState::Cancelled => {
                bankroll.record_accounted_pool(position.key.pool_address.clone());
            }
            PositionState::BuyCancelled => {}
            PositionState::SellConfirmed => {
                bankroll.record_position_result(
                    position.key.pool_address.clone(),
                    position_entry_spend_wei(position, fallback_buy_wei),
                    position_exit_proceeds_wei(position),
                );
            }
            PositionState::Scammed => {
                bankroll.record_position_result(
                    position.key.pool_address.clone(),
                    position_entry_spend_wei(position, fallback_buy_wei),
                    U256::ZERO,
                );
            }
            PositionState::Init
            | PositionState::BuyIntentCreated
            | PositionState::BuySubmitted
            | PositionState::BuyDeferred
            | PositionState::BuyConfirmed
            | PositionState::SellIntentCreated
            | PositionState::SellSubmitted
            | PositionState::SellFailed
            | PositionState::SellCancelled => {}
        }
    }
    bankroll
}

pub(super) fn single_strategy_value<T>(
    specs: &[LiveStrategySpec],
    value: impl FnOnce(&LiveStrategySpec) -> T,
) -> Option<T> {
    if specs.len() == 1 {
        Some(value(&specs[0]))
    } else {
        None
    }
}
