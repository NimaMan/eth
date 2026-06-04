use std::collections::HashMap;
use std::str::FromStr;

use alloy_primitives::U256;
use eth_alpha_core::{
    amount::{Amount, DecimalAmount},
    ids::{BlockNumber, PoolAddress, PositionId, StrategyName},
    risk::RiskPolicy,
    store::TradingStore,
    Strategy,
};
use eth_alpha_store::ActiveHoldCounterRecord;
use eth_strategies::{
    shared_rules::{entry::init_policy::EntryInitPolicyConfig, live::LiveStrategySpec},
    Alpha11Config, LiveAlpha11Config, LiveAlpha11Strategy, LiveSnipeAllConfig,
    LiveSnipeAllStrategy, RestoredEntryBankroll, SnipeAllConfig, ALPHA11_STRATEGY_IMPL,
};
use eyre::{eyre, Result, WrapErr};
use rust_decimal::Decimal;

use super::support::parse_u256_decimal;
use eth_alpha_engine::{AlphaEngine, EngineExecutionAdapter};

pub(super) struct LiveStrategyRestore {
    pub(super) seen_pools: Vec<PoolAddress>,
    pub(super) active_hold_counters: Vec<(PositionId, u64, Option<BlockNumber>)>,
    pub(super) entry_bankroll: RestoredEntryBankroll,
}

pub(super) fn install_live_strategies<E, R, S>(
    engine: &mut AlphaEngine<E, R, S>,
    runner_name: &str,
    strategy_specs: &[LiveStrategySpec],
    entry_bankrolls_wei: &[Option<U256>],
    entry_enabled: bool,
    seen_pools_by_strategy: &HashMap<String, Vec<PoolAddress>>,
    active_hold_counters_by_strategy: &HashMap<String, Vec<ActiveHoldCounterRecord>>,
    entry_bankrolls_by_strategy: &HashMap<String, RestoredEntryBankroll>,
) -> Result<()>
where
    E: EngineExecutionAdapter,
    R: RiskPolicy,
    S: TradingStore,
{
    for (spec, entry_bankroll_wei) in strategy_specs
        .iter()
        .zip(entry_bankrolls_wei.iter().copied())
    {
        let seen_pools = seen_pools_by_strategy
            .get(&spec.strategy_name)
            .cloned()
            .unwrap_or_default();
        let active_hold_counters = active_hold_counters_by_strategy
            .get(&spec.strategy_name)
            .into_iter()
            .flat_map(|counters| counters.iter())
            .map(|counter| {
                (
                    counter.position_id.clone(),
                    counter.count,
                    counter.last_block,
                )
            })
            .collect::<Vec<_>>();
        let restored_entry_bankroll = entry_bankrolls_by_strategy
            .get(&spec.strategy_name)
            .cloned()
            .unwrap_or_default();
        let strategy = build_live_strategy(
            runner_name,
            spec,
            entry_bankroll_wei,
            entry_enabled,
            LiveStrategyRestore {
                seen_pools,
                active_hold_counters,
                entry_bankroll: restored_entry_bankroll,
            },
        )?;
        engine.add_strategy(strategy);
    }
    Ok(())
}

pub(super) fn build_live_strategy(
    runner_name: &str,
    spec: &LiveStrategySpec,
    entry_bankroll_wei: Option<U256>,
    entry_enabled: bool,
    restore: LiveStrategyRestore,
) -> Result<Box<dyn Strategy>> {
    let buy_wei = parse_u256_decimal(&spec.buy_wei).wrap_err_with(|| {
        format!(
            "invalid buy_wei for live strategy {}: {}",
            spec.strategy_name, spec.buy_wei
        )
    })?;
    let min_liquidity_eth = Decimal::from_str(&spec.min_liquidity_eth).wrap_err_with(|| {
        format!(
            "invalid min_liquidity_eth for live strategy {}: {}",
            spec.strategy_name, spec.min_liquidity_eth
        )
    })?;
    let min_liquidity_usd = Decimal::from_str(&spec.min_liquidity_usd).wrap_err_with(|| {
        format!(
            "invalid min_liquidity_usd for live strategy {}: {}",
            spec.strategy_name, spec.min_liquidity_usd
        )
    })?;
    let stop_loss_ratio = spec
        .stop_loss_ratio
        .as_deref()
        .and_then(|s| Decimal::from_str(s).ok());
    let take_profit_ratio = spec
        .take_profit_ratio
        .as_deref()
        .and_then(|s| Decimal::from_str(s).ok());
    let lp_approval_gate_min_pct = spec
        .lp_approval_gate_min_pct
        .as_deref()
        .and_then(|s| Decimal::from_str(s).ok());
    let entry_init_max_price_ratio_to_initial = spec
        .entry_init_policy
        .max_price_ratio_to_initial
        .as_deref()
        .and_then(|s| Decimal::from_str(s).ok());
    let entry_init_policy = EntryInitPolicyConfig {
        max_age_blocks: spec.entry_init_policy.max_age_blocks,
        require_pool_creation_block: spec.entry_init_policy.require_pool_creation_block,
        max_price_ratio_to_initial: entry_init_max_price_ratio_to_initial,
        allow_missing_price_ratio: spec.entry_init_policy.allow_missing_price_ratio,
    };
    let min_sell_pool_denom_reserve = spec
        .min_sell_pool_denom_reserve
        .as_deref()
        .and_then(|s| Decimal::from_str(s).ok())
        .unwrap_or_else(|| SnipeAllConfig::default().min_sell_pool_denom_reserve);
    let snipe_all_config = SnipeAllConfig {
        strategy_name: StrategyName(spec.strategy_name.clone()),
        buy_amount: Amount {
            raw: buy_wei,
            decimals: 18,
        },
        sell_fraction: DecimalAmount::from(1),
        min_denom_reserve: min_liquidity_eth,
        min_stable_denom_reserve: min_liquidity_usd,
        min_sell_pool_denom_reserve,
        entry_enabled,
        max_entry_pools: spec.max_entry_pools,
        entry_bankroll_wei,
        stop_loss_ratio,
        take_profit_ratio,
        max_hold_blocks: spec.max_hold_blocks,
        exit_on_liquidity_removal: spec.exit_liquidity_removal,
        exit_on_tax: spec.exit_tax,
        exit_on_lp_approval: spec.exit_lp_approval,
        exit_on_critical_lp_approval_only: spec.exit_lp_approval_critical_only,
        exit_on_scam: spec.exit_scam,
        allowed_protocols: spec.allowed_protocols.clone(),
        block_entry_on_lp_approval: spec.block_entry_on_lp_approval,
        lp_approval_gate_min_pct,
        entry_init_policy,
        defer_buy_confirm_block_lp_approval_to_max_hold: spec
            .defer_buy_confirm_block_lp_approval_to_max_hold,
        lp_approval_exit_defer_max_trading_enabled_age_blocks: spec
            .lp_approval_exit_defer_max_trading_enabled_age_blocks,
        ..SnipeAllConfig::default()
    };

    match spec.strategy_impl.as_str() {
        ALPHA11_STRATEGY_IMPL => Ok(Box::new(LiveAlpha11Strategy::with_restored_runtime_state(
            LiveAlpha11Config::new(Alpha11Config::new(snipe_all_config)),
            restore.seen_pools,
            restore.active_hold_counters,
            restore.entry_bankroll,
        ))),
        "snipe-all" => Ok(Box::new(LiveSnipeAllStrategy::with_restored_runtime_state(
            LiveSnipeAllConfig::new(snipe_all_config),
            restore.seen_pools,
            restore.active_hold_counters,
            restore.entry_bankroll,
        ))),
        other => Err(eyre!(
            "{} cannot instantiate unsupported live strategy_impl {} for {}",
            runner_name,
            other,
            spec.strategy_name
        )),
    }
}
