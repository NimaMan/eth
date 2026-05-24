use serde_json::{json, Value};

use super::{gas_policy, support::TraderExecutionMode};

pub(super) fn live_gas_policy_run_metadata_json(
    policy: &gas_policy::LiveRealGasPolicy,
    execution_mode: TraderExecutionMode,
) -> Value {
    json!({
        "mode": if execution_mode.uses_kartal() { "kartal-real" } else { "chain-sim-shadow" },
        "required_gas_rank_source": &policy.required_gas_rank_source,
        "gas_rank_lookback_blocks": policy.gas_rank_lookback_blocks,
        "simulated_gas_buffer_bps": policy.simulated_gas_buffer_bps,
        "max_priority_fee_gwei": policy.max_priority_fee_gwei.to_string(),
        "entry_max_estimated_gas_fee_eth": policy.entry_max_estimated_gas_fee_eth.to_string(),
        "exit_max_estimated_gas_fee_eth": policy.exit_max_estimated_gas_fee_eth.to_string(),
        "safety_buffer_eth": policy.safety_buffer_eth.to_string(),
        "v2_vault_buy_gas_limit": policy.v2_vault_buy_gas_limit,
        "v2_vault_sell_gas_limit": policy.v2_vault_sell_gas_limit,
        "entry_buy_profiles": &policy.entry_buy_gas_rank_policy,
        "tail_entry_buy_profiles": &policy.tail_entry_buy_gas_rank_policy,
        "normal_exit_profiles": &policy.normal_exit_gas_rank_policy,
        "mempool_race_exit_profiles": &policy.mempool_pre_mine_gas_rank_policy,
        "lp_approval_exit_profiles": &policy.lp_approval_exit_gas_rank_policy,
    })
}
