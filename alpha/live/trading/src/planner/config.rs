use eth_alpha_core::amount::DecimalAmount;
use serde::{Deserialize, Serialize};

use crate::{GasEstimateConfig, PriorityRoute, StrategyGasRankPolicy, TxPrepConfig};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LivePrioritySellPlannerConfig {
    pub tx_prep: TxPrepConfig,
    pub priority_route: PriorityRoute,
    pub max_priority_fee_per_gas_gwei: DecimalAmount,
    pub max_total_fee_eth: DecimalAmount,
    pub expected_late_recovery_eth: DecimalAmount,
    pub require_existing_allowance: bool,
    pub gas_estimate: GasEstimateConfig,
    pub normal_exit_gas_rank_policy: StrategyGasRankPolicy,
    pub mempool_pre_mine_gas_rank_policy: StrategyGasRankPolicy,
    pub lp_approval_exit_gas_rank_policy: StrategyGasRankPolicy,
}

impl Default for LivePrioritySellPlannerConfig {
    fn default() -> Self {
        Self {
            tx_prep: TxPrepConfig {
                max_total_fee_eth: DecimalAmount::new(2, 2),
                max_priority_fee_gwei: DecimalAmount::from(100),
                safety_buffer_eth: DecimalAmount::new(1, 3),
                gas_rank_policy: StrategyGasRankPolicy::p95_first(),
                required_gas_rank_source: None,
            },
            priority_route: PriorityRoute::PublicMempool,
            max_priority_fee_per_gas_gwei: DecimalAmount::from(100),
            max_total_fee_eth: DecimalAmount::new(2, 2),
            expected_late_recovery_eth: DecimalAmount::ZERO,
            require_existing_allowance: true,
            gas_estimate: GasEstimateConfig::default(),
            normal_exit_gas_rank_policy: StrategyGasRankPolicy::p85_first(),
            mempool_pre_mine_gas_rank_policy: StrategyGasRankPolicy::mempool_race_only(),
            lp_approval_exit_gas_rank_policy: StrategyGasRankPolicy::p90_first(),
        }
    }
}
