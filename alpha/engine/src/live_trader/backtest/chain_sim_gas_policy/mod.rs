use async_trait::async_trait;
use eth_alpha_core::{
    amount::DecimalAmount,
    error::{AlphaCoreError, Result},
    execution::{ExecutionReport, ExecutionStatus},
    market::PoolSnapshot,
    order::{OrderIntent, OrderSide},
    position::Position,
};
use eth_live_trading::{
    tx_prep::GasPlanDecision, ChainServerGasRankProvider, GasEstimateConfig, GasRankProvider,
    MempoolRaceGasRankProvider, PriorityFeeBudget, PriorityFeeBudgetInput, RankedFeeCandidate,
    StrategyGasRankPolicy,
};

use crate::{EngineExecutionAdapter, PositionValueSimulation};

use super::super::gas_policy::LiveRealGasPolicy;

mod metadata;
mod policy_context;
mod route;
mod shadow_outcome;

use policy_context::{policy_allows_mempool_race_candidate, sell_policy_context};
use route::{shadow_input, shadow_route};
use shadow_outcome::{
    apply_shadow_outcome, outcome_from_selection, ShadowGasOutcome, ShadowGasSelection,
};

#[derive(Clone)]
pub(in crate::live_trader) struct ChainSimGasPolicyBacktestAdapter<E> {
    inner: E,
    gas_rank: MempoolRaceGasRankProvider<ChainServerGasRankProvider>,
    gas_policy: LiveRealGasPolicy,
    gas_estimate: GasEstimateConfig,
}

impl<E> ChainSimGasPolicyBacktestAdapter<E> {
    pub(in crate::live_trader) fn new(
        inner: E,
        chain_server_url: String,
        rpc_url: String,
        gas_policy: LiveRealGasPolicy,
    ) -> Self {
        let mut gas_estimate = GasEstimateConfig::default();
        gas_estimate.simulated_gas_estimate_buffer_bps = gas_policy.simulated_gas_buffer_bps;
        Self {
            inner,
            gas_rank: MempoolRaceGasRankProvider::new(
                ChainServerGasRankProvider::new(chain_server_url)
                    .with_lookback_blocks(gas_policy.gas_rank_lookback_blocks)
                    .with_priority_tie_breaker_gwei(gas_policy.gas_rank_priority_tie_breaker_gwei),
                rpc_url,
            )
            .with_priority_buffer_range_gwei(
                gas_policy.mempool_race_priority_buffer_min_gwei,
                gas_policy.mempool_race_priority_buffer_max_gwei,
            ),
            gas_policy,
            gas_estimate,
        }
    }
}

#[async_trait]
impl<E> EngineExecutionAdapter for ChainSimGasPolicyBacktestAdapter<E>
where
    E: EngineExecutionAdapter,
{
    async fn execute(&self, intent: OrderIntent) -> Result<ExecutionReport> {
        let mut report = self.inner.execute(intent.clone()).await?;
        if should_attach_shadow(&report) {
            match self.shadow_outcome(&intent, &report).await {
                Ok(outcome) => apply_shadow_outcome(&mut report, outcome),
                Err(error) => {
                    let outcome =
                        self.shadow_rejection_outcome(&intent, &report, error.to_string());
                    apply_shadow_outcome(&mut report, outcome);
                }
            }
        }
        Ok(report)
    }

    async fn simulate_position_value(
        &self,
        position: &Position,
        pool: &PoolSnapshot,
    ) -> Result<Option<PositionValueSimulation>> {
        self.inner.simulate_position_value(position, pool).await
    }
}

impl<E> ChainSimGasPolicyBacktestAdapter<E>
where
    E: EngineExecutionAdapter,
{
    async fn shadow_outcome(
        &self,
        intent: &OrderIntent,
        report: &ExecutionReport,
    ) -> Result<ShadowGasOutcome> {
        let gas_limit = match intent.side {
            OrderSide::Buy => self.gas_policy.v2_vault_buy_gas_limit,
            OrderSide::Sell => self.gas_policy.v2_vault_sell_gas_limit,
        };
        let simulated_gas_used = report.gas_used.ok_or_else(|| {
            AlphaCoreError::Execution(
                "gas-policy shadow requires chain-sim gas_used evidence".to_string(),
            )
        })?;
        let route = shadow_route(gas_limit, simulated_gas_used, &self.gas_estimate)?;
        let estimated_gas_used = route.require_estimated_gas_used().map_err(|error| {
            AlphaCoreError::Execution(format!("gas-policy shadow invalid route: {error}"))
        })?;
        let gas_rank = self
            .gas_rank
            .ranked_fee_candidates(&shadow_input(intent), &route)
            .await
            .map_err(|error| AlphaCoreError::Execution(error.to_string()))?;
        let selection = match intent.side {
            OrderSide::Buy => {
                let candidates = self.allowed_gas_candidates(
                    &gas_rank.candidates,
                    &self.gas_policy.entry_buy_gas_rank_policy,
                );
                self.select_buy_gas(intent, &candidates, estimated_gas_used)
            }
            OrderSide::Sell => self.select_sell_gas(
                intent,
                report,
                gas_rank.predicted_base_fee_gwei,
                &gas_rank.candidates,
                estimated_gas_used,
            ),
        };
        Ok(outcome_from_selection(
            intent,
            report,
            gas_limit,
            estimated_gas_used,
            selection,
        ))
    }

    fn select_buy_gas(
        &self,
        intent: &OrderIntent,
        candidates: &[RankedFeeCandidate],
        estimated_gas_used: u64,
    ) -> ShadowGasSelection {
        let policy_context = self.gas_policy.buy_policy_context(
            intent
                .decision_reason
                .as_ref()
                .map(|reason| reason.code.as_str()),
        );
        let policy = policy_context.policy;
        let capped = candidates
            .iter()
            .filter(|candidate| {
                candidate.priority_fee_gwei <= self.gas_policy.max_priority_fee_gwei
                    && candidate.estimated_max_cost_eth(estimated_gas_used)
                        <= self.gas_policy.entry_max_estimated_gas_fee_eth
            })
            .cloned()
            .collect::<Vec<_>>();

        match policy.choose_candidate(&capped) {
            Some(candidate) => ShadowGasSelection::selected(
                policy,
                candidate,
                estimated_gas_used,
                policy_context.action,
                policy_context.signal,
                policy_context.guard,
            ),
            None => ShadowGasSelection::rejected(
                policy,
                policy_context.action,
                policy_context.signal,
                policy_context.guard,
                "gas_rank_exceeds_entry_policy",
            ),
        }
    }

    fn select_sell_gas(
        &self,
        intent: &OrderIntent,
        report: &ExecutionReport,
        predicted_base_fee_gwei: DecimalAmount,
        candidates: &[RankedFeeCandidate],
        estimated_gas_used: u64,
    ) -> ShadowGasSelection {
        let policy_context = sell_policy_context(intent, &self.gas_policy);
        let candidates = self.allowed_gas_candidates(candidates, policy_context.policy);
        let protected_exit_value_eth = report
            .filled_amount
            .as_ref()
            .map(|amount| amount.to_decimal())
            .unwrap_or_default();
        let budget = PriorityFeeBudget::from_input(&PriorityFeeBudgetInput {
            protected_exit_value_eth,
            expected_late_recovery_eth: DecimalAmount::ZERO,
            safety_buffer_eth: self.gas_policy.safety_buffer_eth,
            predicted_base_fee_gwei,
            estimated_gas_used,
            max_total_fee_eth: self.gas_policy.exit_max_estimated_gas_fee_eth,
            configured_max_priority_fee_gwei: self.gas_policy.max_priority_fee_gwei,
        });

        match policy_context
            .policy
            .choose_ranked_fee(&budget, &candidates)
        {
            GasPlanDecision::UseRanked(plan) => ShadowGasSelection::selected_plan(
                policy_context.policy,
                plan.label,
                plan.priority_fee_gwei,
                plan.max_fee_per_gas_gwei,
                plan.estimated_priority_spend_eth,
                plan.estimated_max_cost_eth,
                plan.source,
                policy_context.action,
                policy_context.signal,
                "exit_value_capped_gas_rank",
            ),
            GasPlanDecision::Reject { reason, .. } => ShadowGasSelection::rejected(
                policy_context.policy,
                policy_context.action,
                policy_context.signal,
                "exit_value_capped_gas_rank",
                reason,
            ),
        }
    }

    fn allowed_gas_candidates(
        &self,
        candidates: &[RankedFeeCandidate],
        policy: &StrategyGasRankPolicy,
    ) -> Vec<RankedFeeCandidate> {
        candidates
            .iter()
            .filter(|candidate| {
                candidate.source.as_deref()
                    == Some(self.gas_policy.required_gas_rank_source.as_str())
                    || policy_allows_mempool_race_candidate(policy, candidate)
            })
            .cloned()
            .collect()
    }

    fn shadow_rejection_outcome(
        &self,
        intent: &OrderIntent,
        report: &ExecutionReport,
        reason: String,
    ) -> ShadowGasOutcome {
        let gas_limit = match intent.side {
            OrderSide::Buy => self.gas_policy.v2_vault_buy_gas_limit,
            OrderSide::Sell => self.gas_policy.v2_vault_sell_gas_limit,
        };
        let selection = match intent.side {
            OrderSide::Buy => {
                let policy_context = self.gas_policy.buy_policy_context(
                    intent
                        .decision_reason
                        .as_ref()
                        .map(|reason| reason.code.as_str()),
                );
                ShadowGasSelection::rejected(
                    policy_context.policy,
                    policy_context.action,
                    policy_context.signal,
                    "live_backtest_chain_sim_gas_policy",
                    reason,
                )
            }
            OrderSide::Sell => {
                let policy_context = sell_policy_context(intent, &self.gas_policy);
                ShadowGasSelection::rejected(
                    policy_context.policy,
                    policy_context.action,
                    policy_context.signal,
                    "live_backtest_chain_sim_gas_policy",
                    reason,
                )
            }
        };
        outcome_from_selection(
            intent,
            report,
            gas_limit,
            report.gas_used.unwrap_or_default(),
            selection,
        )
    }
}

fn should_attach_shadow(report: &ExecutionReport) -> bool {
    matches!(
        report.status,
        ExecutionStatus::Confirmed | ExecutionStatus::Failed
    ) && report.gas_used.is_some()
}
