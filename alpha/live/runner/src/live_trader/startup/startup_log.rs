use alloy_primitives::U256;
use serde_json::Value;
use tracing::info;

use super::support::TraderExecutionMode;

pub(super) struct StartupLogInput<'a> {
    pub(super) token_server_url: &'a str,
    pub(super) reth_datadir: &'a str,
    pub(super) run_id: &'a str,
    pub(super) execution_mode: TraderExecutionMode,
    pub(super) strategy_set: &'a Option<String>,
    pub(super) strategy_count: usize,
    pub(super) observation_strategy_name: &'a str,
    pub(super) stale_runs: u64,
    pub(super) replay_current: bool,
    pub(super) single_max_entry_pools: Option<usize>,
    pub(super) single_entry_bankroll_eth: &'a Option<String>,
    pub(super) single_entry_bankroll_wei: Option<U256>,
    pub(super) seen_pool_blocks: usize,
    pub(super) seen_signal_ids: usize,
    pub(super) seen_mined_pool_risk_keys: usize,
    pub(super) restored_seen_pools: usize,
    pub(super) restored_active_hold_counters: usize,
    pub(super) restored_entry_bankroll_position_count: usize,
    pub(super) restored_entry_bankroll_accounted_pool_count: usize,
    pub(super) restored_entry_bankroll_spent_wei: &'a U256,
    pub(super) restored_entry_bankroll_recovered_wei: &'a U256,
    pub(super) restored_stale_submitted_positions: usize,
    pub(super) restored_position_count: usize,
    pub(super) next_order_sequence: u64,
    pub(super) _entry_bankroll_summary: &'a [Value],
}

pub(super) fn log_alpha_trader_start(input: StartupLogInput<'_>) {
    info!(
        token_server_url = %input.token_server_url,
        reth_datadir = %input.reth_datadir,
        run_id = %input.run_id,
        mode = %input.execution_mode.label(),
        strategy_set = ?input.strategy_set,
        strategy_count = input.strategy_count,
        observation_strategy_name = %input.observation_strategy_name,
        stale_runs = input.stale_runs,
        replay_current = input.replay_current,
        max_entry_pools = ?input.single_max_entry_pools,
        entry_bankroll_eth = ?input.single_entry_bankroll_eth,
        entry_bankroll_wei = ?input.single_entry_bankroll_wei.map(|value| value.to_string()),
        restored_pool_watermarks = input.seen_pool_blocks,
        restored_signal_watermarks = input.seen_signal_ids,
        restored_mined_pool_risk_watermarks = input.seen_mined_pool_risk_keys,
        restored_seen_pools = input.restored_seen_pools,
        restored_active_hold_counters = input.restored_active_hold_counters,
        restored_entry_bankroll_positions = input.restored_entry_bankroll_position_count,
        restored_entry_bankroll_accounted_pools = input.restored_entry_bankroll_accounted_pool_count,
        restored_entry_bankroll_spent_wei = %input.restored_entry_bankroll_spent_wei,
        restored_entry_bankroll_recovered_wei = %input.restored_entry_bankroll_recovered_wei,
        restored_stale_submitted_positions = input.restored_stale_submitted_positions,
        restored_positions = input.restored_position_count,
        next_order_sequence = input.next_order_sequence,
        "starting alpha trader"
    );
}
