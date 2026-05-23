use alloy_primitives::U256;
use async_trait::async_trait;
use eth_alpha_core::{
    amount::{Amount, DecimalAmount},
    error::{AlphaCoreError, Result},
    execution::{ExecutionReport, ExecutionStatus, MinedExecutionEvidence},
    market::PoolSnapshot,
    order::{OrderIntent, OrderSide},
    position::Position,
};
use eth_live_trading::{
    tx_prep::GasPlanDecision, ChainServerGasRankProvider, GasEstimateConfig, GasRankProvider,
    PreparedSellRoute, PriorityFeeBudget, PriorityFeeBudgetInput, RankedFeeCandidate,
    StrategyGasRankPolicy,
};

use crate::{EngineExecutionAdapter, PositionValueSimulation};

use super::super::gas_policy::LiveRealGasPolicy;

#[derive(Clone)]
pub(in crate::live_trader) struct ChainSimGasPolicyBacktestAdapter<E> {
    inner: E,
    gas_rank: ChainServerGasRankProvider,
    gas_policy: LiveRealGasPolicy,
    gas_estimate: GasEstimateConfig,
}

impl<E> ChainSimGasPolicyBacktestAdapter<E> {
    pub(in crate::live_trader) fn new(
        inner: E,
        chain_server_url: String,
        gas_policy: LiveRealGasPolicy,
    ) -> Self {
        let mut gas_estimate = GasEstimateConfig::default();
        gas_estimate.simulated_gas_estimate_buffer_bps = gas_policy.simulated_gas_buffer_bps;
        Self {
            inner,
            gas_rank: ChainServerGasRankProvider::new(chain_server_url)
                .with_lookback_blocks(gas_policy.gas_rank_lookback_blocks),
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
            .ranked_fee_candidates(&shadow_input(), &route)
            .await
            .map_err(|error| AlphaCoreError::Execution(error.to_string()))?;
        let candidates = gas_rank
            .candidates
            .into_iter()
            .filter(|candidate| {
                candidate.source.as_deref()
                    == Some(self.gas_policy.required_gas_rank_source.as_str())
            })
            .collect::<Vec<_>>();
        let selection = match intent.side {
            OrderSide::Buy => self.select_buy_gas(&candidates, estimated_gas_used),
            OrderSide::Sell => self.select_sell_gas(
                intent,
                report,
                gas_rank.predicted_base_fee_gwei,
                &candidates,
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
        candidates: &[RankedFeeCandidate],
        estimated_gas_used: u64,
    ) -> ShadowGasSelection {
        let policy = &self.gas_policy.entry_buy_gas_rank_policy;
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
                "entry_buy",
                "entry.buy_eligible_pool_once",
                "entry_estimated_gas_fee_cap",
            ),
            None => ShadowGasSelection::rejected(
                policy,
                "entry_buy",
                "entry.buy_eligible_pool_once",
                "entry_estimated_gas_fee_cap",
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

        match policy_context.policy.choose_ranked_fee(&budget, candidates) {
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
            OrderSide::Buy => ShadowGasSelection::rejected(
                &self.gas_policy.entry_buy_gas_rank_policy,
                "entry_buy",
                "entry.buy_eligible_pool_once",
                "live_backtest_chain_sim_gas_policy",
                reason,
            ),
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

struct SellPolicyContext<'a> {
    policy: &'a StrategyGasRankPolicy,
    action: &'static str,
    signal: String,
}

fn sell_policy_context<'a>(
    intent: &OrderIntent,
    gas_policy: &'a LiveRealGasPolicy,
) -> SellPolicyContext<'a> {
    let reason = intent
        .decision_reason
        .as_ref()
        .map(|reason| reason.code.as_str())
        .unwrap_or("exit.unknown");
    let source = intent
        .decision_reason
        .as_ref()
        .and_then(|reason| reason.source.as_deref())
        .unwrap_or_default();

    if reason.contains("buy_confirm") {
        return SellPolicyContext {
            policy: &gas_policy.lp_approval_exit_gas_rank_policy,
            action: "lp_approval_exit",
            signal: reason.to_string(),
        };
    }
    if source.contains("mempool") || reason.contains("mempool") {
        return SellPolicyContext {
            policy: &gas_policy.mempool_pre_mine_gas_rank_policy,
            action: "mempool_race_exit",
            signal: reason.to_string(),
        };
    }
    if reason.contains("lp_approval") || reason.contains("liquidity_removal") {
        return SellPolicyContext {
            policy: &gas_policy.lp_approval_exit_gas_rank_policy,
            action: "lp_approval_exit",
            signal: reason.to_string(),
        };
    }
    SellPolicyContext {
        policy: &gas_policy.normal_exit_gas_rank_policy,
        action: "normal_exit",
        signal: reason.to_string(),
    }
}

fn should_attach_shadow(report: &ExecutionReport) -> bool {
    matches!(
        report.status,
        ExecutionStatus::Confirmed | ExecutionStatus::Failed
    ) && report.gas_used.is_some()
}

fn shadow_route(
    gas_limit: u64,
    simulated_gas_used: u64,
    gas_estimate: &GasEstimateConfig,
) -> Result<PreparedSellRoute> {
    PreparedSellRoute {
        protocol: "live_backtest_chain_sim_gas_policy".to_string(),
        router_address: "0x0000000000000000000000000000000000000000".to_string(),
        calldata: "0x00".to_string(),
        value_wei: "0".to_string(),
        gas_limit,
        estimated_gas_used: None,
        max_slippage_bps: None,
    }
    .with_simulated_gas_used(simulated_gas_used, gas_estimate)
    .map_err(|error| AlphaCoreError::Execution(format!("gas-policy shadow route failed: {error}")))
}

fn shadow_input() -> eth_live_trading::LivePrioritySellPlannerInput {
    use alloy_primitives::Address;
    use eth_alpha_core::{
        amount::Amount,
        ids::{PoolAddress, PortfolioId, PositionId, StrategyName, TradeId, WalletId},
        market::{PoolProtocol, PoolSnapshot},
        order::{OrderIntent, OrderSide},
        position::{Position, PositionKey},
    };
    use eth_live_trading::{PlannerTxContext, TxPrepRequestContext};
    use serde_json::json;

    let pool_address = PoolAddress::from(
        "0x0000000000000000000000000000000000000000:0x0000000000000000000000000000000000000000",
    );
    let strategy_name = StrategyName("gas-policy-shadow".to_string());
    let trade_id = TradeId("gas-policy-shadow".to_string());
    let token_address = Address::ZERO;
    eth_live_trading::LivePrioritySellPlannerInput {
        context: PlannerTxContext {
            tx: TxPrepRequestContext {
                chain_id: 1,
                from: Address::ZERO.to_string(),
                strategy_name: strategy_name.0.clone(),
                strategy_run_id: None,
                observed_block: None,
                source_metadata: json!({"source": "live_backtest_chain_sim_gas_policy"}),
            },
            current_block: 0,
            deadline_unix_secs: 0,
        },
        intent: OrderIntent {
            trade_id: Some(trade_id.clone()),
            portfolio_id: PortfolioId("gas-policy-shadow".to_string()),
            wallet_id: WalletId("gas-policy-shadow".to_string()),
            strategy_name: strategy_name.clone(),
            side: OrderSide::Sell,
            token_address,
            pool_address: pool_address.clone(),
            protocol: PoolProtocol::UniswapV2,
            amount: Amount::zero(18),
            route: None,
            max_slippage_bps: 0,
            deadline_secs: 0,
            decision_reason: None,
        },
        position: Position::with_trade_id(
            PositionId(trade_id.0.clone()),
            trade_id,
            PositionKey {
                portfolio_id: PortfolioId("gas-policy-shadow".to_string()),
                wallet_id: WalletId("gas-policy-shadow".to_string()),
                strategy_name,
                token_address,
                pool_address: pool_address.clone(),
                protocol: PoolProtocol::UniswapV2,
            },
        ),
        pool: PoolSnapshot {
            address: pool_address,
            token_address,
            protocol: PoolProtocol::UniswapV2,
            denom_address: None,
            denom_symbol: None,
            denom_reserve: DecimalAmount::ZERO,
            token_reserve: DecimalAmount::ZERO,
            price_denom_per_token: None,
            initial_price_denom_per_token: None,
            price_ratio_to_initial: None,
            creation_block: Some(0),
            token_decimals: Some(18),
            fee_tier: None,
            uniswap_v4: None,
            latest_block: 0,
            can_buy: false,
            can_sell: false,
            is_scam: false,
        },
        min_output_amount: None,
        source_metadata: json!({"source": "live_backtest_chain_sim_gas_policy"}),
    }
}

struct ShadowGasSelection {
    status: String,
    action: String,
    signal: String,
    profiles: Vec<String>,
    selected_profile: Option<String>,
    max_fee_per_gas_gwei: Option<DecimalAmount>,
    priority_fee_gwei: Option<DecimalAmount>,
    estimated_priority_spend_eth: Option<DecimalAmount>,
    estimated_max_cost_eth: Option<DecimalAmount>,
    source: Option<String>,
    guard: String,
}

impl ShadowGasSelection {
    fn is_selected(&self) -> bool {
        self.status == "selected"
    }
}

impl ShadowGasSelection {
    fn selected(
        policy: &StrategyGasRankPolicy,
        candidate: RankedFeeCandidate,
        estimated_gas_used: u64,
        action: impl Into<String>,
        signal: impl Into<String>,
        guard: impl Into<String>,
    ) -> Self {
        let estimated_priority_spend_eth =
            candidate.estimated_priority_spend_eth(estimated_gas_used);
        let estimated_max_cost_eth = candidate.estimated_max_cost_eth(estimated_gas_used);
        Self::selected_plan(
            policy,
            candidate.label,
            candidate.priority_fee_gwei,
            candidate.max_fee_per_gas_gwei,
            estimated_priority_spend_eth,
            estimated_max_cost_eth,
            candidate.source,
            action,
            signal,
            guard,
        )
    }

    fn selected_plan(
        policy: &StrategyGasRankPolicy,
        label: String,
        priority_fee_gwei: DecimalAmount,
        max_fee_per_gas_gwei: DecimalAmount,
        estimated_priority_spend_eth: DecimalAmount,
        estimated_max_cost_eth: DecimalAmount,
        source: Option<String>,
        action: impl Into<String>,
        signal: impl Into<String>,
        guard: impl Into<String>,
    ) -> Self {
        Self {
            status: "selected".to_string(),
            action: action.into(),
            signal: signal.into(),
            profiles: gas_policy_profile_labels(policy),
            selected_profile: Some(label),
            max_fee_per_gas_gwei: Some(max_fee_per_gas_gwei),
            priority_fee_gwei: Some(priority_fee_gwei),
            estimated_priority_spend_eth: Some(estimated_priority_spend_eth),
            estimated_max_cost_eth: Some(estimated_max_cost_eth),
            source,
            guard: guard.into(),
        }
    }

    fn rejected(
        policy: &StrategyGasRankPolicy,
        action: impl Into<String>,
        signal: impl Into<String>,
        guard: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            status: format!("rejected:{}", reason.into()),
            action: action.into(),
            signal: signal.into(),
            profiles: gas_policy_profile_labels(policy),
            selected_profile: None,
            max_fee_per_gas_gwei: None,
            priority_fee_gwei: None,
            estimated_priority_spend_eth: None,
            estimated_max_cost_eth: None,
            source: None,
            guard: guard.into(),
        }
    }
}

struct ShadowGasOutcome {
    evidence: MinedExecutionEvidence,
    gas_cost: Option<Amount>,
    cancel_reason: Option<String>,
}

fn apply_shadow_outcome(report: &mut ExecutionReport, outcome: ShadowGasOutcome) {
    report.mined_evidence = Some(outcome.evidence);
    if let Some(reason) = outcome.cancel_reason {
        report.status = ExecutionStatus::Cancelled;
        report.filled_amount = None;
        report.token_amount = None;
        report.gas_cost = None;
        report.error = Some(reason);
        return;
    }
    if let Some(gas_cost) = outcome.gas_cost {
        report.gas_cost = Some(gas_cost);
    }
}

fn outcome_from_selection(
    intent: &OrderIntent,
    report: &ExecutionReport,
    gas_limit: u64,
    estimated_gas_used: u64,
    selection: ShadowGasSelection,
) -> ShadowGasOutcome {
    let (gas_cost, effective_gas_price_wei, paid_gas_cost_wei) =
        policy_paid_gas_cost(report, &selection);
    let cancel_reason = (!selection.is_selected()).then(|| {
        format!(
            "gas policy shadow rejected {} {}: {}",
            match intent.side {
                OrderSide::Buy => "buy",
                OrderSide::Sell => "sell",
            },
            selection.action,
            selection.status
        )
    });
    let evidence = MinedExecutionEvidence {
        receipt_block_number: report.block_number,
        submitted_block_number: report.block_number.map(|block| block.saturating_sub(1)),
        expected_confirmation_block: report.block_number,
        confirmation_lag_blocks: Some(1),
        effective_gas_price_wei,
        paid_gas_cost_wei,
        selected_gas_limit: Some(gas_limit.to_string()),
        selected_max_fee_per_gas_wei: selection.max_fee_per_gas_gwei.map(gwei_to_wei_string),
        selected_max_priority_fee_per_gas_wei: selection.priority_fee_gwei.map(gwei_to_wei_string),
        selected_bribe_priority_fee_per_gas_wei: selection
            .priority_fee_gwei
            .map(gwei_to_wei_string),
        selected_bribe_max_fee_per_gas_wei: selection.max_fee_per_gas_gwei.map(gwei_to_wei_string),
        gas_policy_action: Some(selection.action),
        gas_policy_signal: Some(selection.signal),
        gas_policy_status: Some(selection.status),
        gas_policy_profile: selection.selected_profile,
        gas_policy_profiles: Some(selection.profiles),
        gas_rank_source: selection.source,
        gas_estimated_max_cost_eth: selection.estimated_max_cost_eth.map(decimal_string),
        gas_estimated_priority_spend_eth: selection
            .estimated_priority_spend_eth
            .map(decimal_string),
        gas_policy_guard: Some(format!(
            "{};chain_sim_estimated_gas_used={estimated_gas_used};side={}",
            selection.guard,
            match intent.side {
                OrderSide::Buy => "buy",
                OrderSide::Sell => "sell",
            }
        )),
        ..MinedExecutionEvidence::default()
    };
    ShadowGasOutcome {
        evidence,
        gas_cost,
        cancel_reason,
    }
}

fn policy_paid_gas_cost(
    report: &ExecutionReport,
    selection: &ShadowGasSelection,
) -> (Option<Amount>, Option<String>, Option<String>) {
    if !selection.is_selected() {
        return (None, None, None);
    }
    let Some(gas_used) = report.gas_used else {
        return (None, None, None);
    };
    if gas_used == 0 {
        return (None, None, None);
    }
    let Some(priority_fee_wei) = selection.priority_fee_gwei.map(gwei_to_wei) else {
        return (None, None, None);
    };
    let Some(max_fee_wei) = selection.max_fee_per_gas_gwei.map(gwei_to_wei) else {
        return (None, None, None);
    };
    let gas_used_u256 = U256::from(gas_used);
    let simulated_paid = report
        .gas_cost
        .as_ref()
        .map(|amount| amount.raw)
        .unwrap_or(U256::ZERO);
    let public_paid = simulated_paid
        .saturating_add(priority_fee_wei.saturating_mul(gas_used_u256))
        .min(max_fee_wei.saturating_mul(gas_used_u256));
    let effective_gas_price = public_paid / gas_used_u256;
    (
        Some(Amount {
            raw: public_paid,
            decimals: 18,
        }),
        Some(effective_gas_price.to_string()),
        Some(public_paid.to_string()),
    )
}

fn gas_policy_profile_labels(policy: &StrategyGasRankPolicy) -> Vec<String> {
    policy
        .preference_order
        .iter()
        .map(|profile| profile.label().to_string())
        .collect()
}

fn gwei_to_wei_string(value: DecimalAmount) -> String {
    gwei_to_wei(value).to_string()
}

fn gwei_to_wei(value: DecimalAmount) -> U256 {
    let wei = decimal_floor_string(
        value.max(DecimalAmount::ZERO) * DecimalAmount::from(1_000_000_000u64),
    );
    U256::from_str_radix(&wei, 10).unwrap_or(U256::ZERO)
}

fn decimal_string(value: DecimalAmount) -> String {
    value.normalize().to_string()
}

fn decimal_floor_string(value: DecimalAmount) -> String {
    let text = value.normalize().to_string();
    match text.find('.') {
        Some(index) => text[..index].to_string(),
        None => text,
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{Address, U256};
    use eth_alpha_core::{
        amount::Amount,
        execution::{ExecutionReport, ExecutionStatus},
        ids::{OrderId, PoolAddress, PortfolioId, StrategyName, WalletId},
        market::PoolProtocol,
        order::{OrderIntent, OrderSide},
    };
    use eth_live_trading::StrategyGasRankPolicy;

    use super::*;

    fn report(gas_used: u64, simulated_gas_cost_wei: u128) -> ExecutionReport {
        ExecutionReport {
            order_id: OrderId("order-1".to_string()),
            status: ExecutionStatus::Confirmed,
            tx_hash: None,
            block_number: Some(100),
            filled_amount: Some(Amount {
                raw: U256::from(1_000_000_000_000_000u64),
                decimals: 18,
            }),
            token_amount: None,
            gas_used: Some(gas_used),
            gas_cost: Some(Amount {
                raw: U256::from(simulated_gas_cost_wei),
                decimals: 18,
            }),
            mined_evidence: None,
            error: None,
        }
    }

    fn selected(priority_gwei: i64, max_fee_gwei: i64) -> ShadowGasSelection {
        ShadowGasSelection::selected_plan(
            &StrategyGasRankPolicy::p50_first(),
            "p50".to_string(),
            DecimalAmount::from(priority_gwei),
            DecimalAmount::from(max_fee_gwei),
            DecimalAmount::ZERO,
            DecimalAmount::ZERO,
            Some("eth_chain_server_gas_rank".to_string()),
            "entry_buy",
            "entry.buy_eligible_pool_once",
            "entry_estimated_gas_fee_cap",
        )
    }

    fn intent(side: OrderSide) -> OrderIntent {
        OrderIntent {
            trade_id: None,
            portfolio_id: PortfolioId("portfolio".to_string()),
            wallet_id: WalletId("wallet".to_string()),
            strategy_name: StrategyName("strategy".to_string()),
            side,
            token_address: Address::with_last_byte(1),
            pool_address: PoolAddress::from("0xtoken:0xpool"),
            protocol: PoolProtocol::UniswapV2,
            amount: Amount::zero(18),
            route: None,
            max_slippage_bps: 0,
            deadline_secs: 0,
            decision_reason: None,
        }
    }

    #[test]
    fn selected_policy_gas_cost_adds_priority_to_simulated_base_fee() {
        let report = report(100_000, 10_000_000_000_000);
        let (gas_cost, effective_gas_price, paid_gas_cost) =
            policy_paid_gas_cost(&report, &selected(2, 20));

        assert_eq!(
            gas_cost.expect("gas cost").raw,
            U256::from(210_000_000_000_000u128)
        );
        assert_eq!(effective_gas_price.as_deref(), Some("2100000000"));
        assert_eq!(paid_gas_cost.as_deref(), Some("210000000000000"));
    }

    #[test]
    fn selected_policy_gas_cost_is_capped_by_max_fee() {
        let report = report(100_000, 1_400_000_000_000_000);
        let (gas_cost, effective_gas_price, paid_gas_cost) =
            policy_paid_gas_cost(&report, &selected(2, 15));

        assert_eq!(
            gas_cost.expect("gas cost").raw,
            U256::from(1_500_000_000_000_000u128)
        );
        assert_eq!(effective_gas_price.as_deref(), Some("15000000000"));
        assert_eq!(paid_gas_cost.as_deref(), Some("1500000000000000"));
    }

    #[test]
    fn rejected_policy_cancels_the_shadow_execution_without_fill_or_gas() {
        let selection = ShadowGasSelection::rejected(
            &StrategyGasRankPolicy::p50_first(),
            "normal_exit",
            "exit.max_hold_active_blocks",
            "exit_value_capped_gas_rank",
            "gas_rank_exceeds_value_cap",
        );
        let mut report = report(100_000, 10_000_000_000_000);
        let outcome = outcome_from_selection(
            &intent(OrderSide::Sell),
            &report,
            300_000,
            150_000,
            selection,
        );

        apply_shadow_outcome(&mut report, outcome);

        assert_eq!(report.status, ExecutionStatus::Cancelled);
        assert!(report.filled_amount.is_none());
        assert!(report.gas_cost.is_none());
        assert!(report.error.as_deref().unwrap().contains("rejected"));
    }
}
