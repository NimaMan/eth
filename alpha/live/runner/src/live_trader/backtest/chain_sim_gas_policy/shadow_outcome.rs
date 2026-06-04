use alloy_primitives::U256;
use eth_alpha_core::{
    amount::{Amount, DecimalAmount},
    execution::{ExecutionReport, ExecutionStatus, MinedExecutionEvidence},
    order::{OrderIntent, OrderSide},
};
use eth_live_trading::{RankedFeeCandidate, StrategyGasRankPolicy};

use super::metadata::tail_entry_ordering_evidence;

pub(super) struct ShadowGasSelection {
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
    pub(super) fn action(&self) -> &str {
        &self.action
    }

    fn is_selected(&self) -> bool {
        self.status == "selected"
    }

    pub(super) fn selected(
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

    pub(super) fn selected_plan(
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

    pub(super) fn rejected(
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

pub(super) struct ShadowGasOutcome {
    evidence: MinedExecutionEvidence,
    gas_cost: Option<Amount>,
    cancel_reason: Option<String>,
}

pub(super) fn apply_shadow_outcome(report: &mut ExecutionReport, outcome: ShadowGasOutcome) {
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

pub(super) fn outcome_from_selection(
    intent: &OrderIntent,
    report: &ExecutionReport,
    gas_limit: u64,
    estimated_gas_used: u64,
    selection: ShadowGasSelection,
) -> ShadowGasOutcome {
    let (gas_cost, effective_gas_price_wei, paid_gas_cost_wei) =
        policy_paid_gas_cost(report, &selection);
    let tail_entry_ordering = tail_entry_ordering_evidence(intent, &selection);
    let gas_policy_guard = shadow_gas_policy_guard(&selection, intent.side, estimated_gas_used);
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
    let mut evidence = report.mined_evidence.clone().unwrap_or_default();
    evidence.receipt_block_number = evidence.receipt_block_number.or(report.block_number);
    evidence.simulation_block_number = evidence.simulation_block_number.or(report.block_number);
    evidence.submitted_block_number = evidence
        .submitted_block_number
        .or_else(|| report.block_number.map(|block| block.saturating_sub(1)));
    evidence.expected_confirmation_block =
        evidence.expected_confirmation_block.or(report.block_number);
    evidence.confirmation_lag_blocks = match (
        evidence.receipt_block_number,
        evidence.expected_confirmation_block,
    ) {
        (Some(actual), Some(expected)) => Some(actual as i64 - expected as i64),
        _ => evidence.confirmation_lag_blocks,
    };
    evidence.effective_gas_price_wei = effective_gas_price_wei;
    evidence.paid_gas_cost_wei = paid_gas_cost_wei;
    evidence.selected_gas_limit = Some(gas_limit.to_string());
    evidence.selected_max_fee_per_gas_wei = selection.max_fee_per_gas_gwei.map(gwei_to_wei_string);
    evidence.selected_max_priority_fee_per_gas_wei =
        selection.priority_fee_gwei.map(gwei_to_wei_string);
    evidence.selected_bribe_priority_fee_per_gas_wei =
        selection.priority_fee_gwei.map(gwei_to_wei_string);
    evidence.selected_bribe_max_fee_per_gas_wei =
        selection.max_fee_per_gas_gwei.map(gwei_to_wei_string);
    evidence.gas_policy_action = Some(selection.action);
    evidence.gas_policy_signal = Some(selection.signal);
    evidence.gas_policy_status = Some(selection.status);
    evidence.gas_policy_profile = selection.selected_profile;
    evidence.gas_policy_profiles = Some(selection.profiles);
    evidence.gas_rank_source = selection.source;
    evidence.gas_estimated_max_cost_eth = selection.estimated_max_cost_eth.map(decimal_string);
    evidence.gas_estimated_priority_spend_eth =
        selection.estimated_priority_spend_eth.map(decimal_string);
    evidence.gas_policy_guard = Some(gas_policy_guard);
    evidence.gas_policy_tail_after_tx_hash = tail_entry_ordering
        .as_ref()
        .and_then(|evidence| evidence.tail_after_tx_hash.clone());
    evidence.gas_policy_dependency_priority_fee_wei = tail_entry_ordering
        .as_ref()
        .and_then(|evidence| evidence.dependency_priority_fee_wei.clone());
    evidence.gas_policy_dependency_gas_price_wei = tail_entry_ordering
        .as_ref()
        .and_then(|evidence| evidence.dependency_gas_price_wei.clone());
    ShadowGasOutcome {
        evidence,
        gas_cost,
        cancel_reason,
    }
}

fn shadow_gas_policy_guard(
    selection: &ShadowGasSelection,
    side: OrderSide,
    estimated_gas_used: u64,
) -> String {
    let mut guard = format!(
        "{};chain_sim_estimated_gas_used={estimated_gas_used};side={}",
        selection.guard,
        match side {
            OrderSide::Buy => "buy",
            OrderSide::Sell => "sell",
        }
    );
    if selection.action == "tail_entry_buy" {
        guard.push_str(
            ";tail_entry_validation_mode=post_mine_n_plus_1;exact_overlay_simulation=false",
        );
    }
    guard
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
        amount::{Amount, DecimalAmount},
        decision_rationale::{DecisionReason, ReasonCategory},
        execution::{ExecutionReport, ExecutionStatus, MinedExecutionEvidence},
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

    fn selected_tail_entry(priority_gwei: i64, max_fee_gwei: i64) -> ShadowGasSelection {
        ShadowGasSelection::selected_plan(
            &StrategyGasRankPolicy::p85_first(),
            "p85".to_string(),
            DecimalAmount::from(priority_gwei),
            DecimalAmount::from(max_fee_gwei),
            DecimalAmount::ZERO,
            DecimalAmount::ZERO,
            Some("eth_chain_server_gas_rank".to_string()),
            "tail_entry_buy",
            "entry.tail_after_enabling_tx",
            "tail_entry_estimated_gas_fee_cap",
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

    fn tail_entry_intent() -> OrderIntent {
        let mut intent = intent(OrderSide::Buy);
        intent.decision_reason = Some(DecisionReason {
            code: "entry.tail_after_enabling_tx".to_string(),
            category: ReasonCategory::Entry,
            label: "Entry: tail after enabling tx".to_string(),
            source: Some("mempool_signal".to_string()),
            raw: Some("entry.tail_after_enabling_tx".to_string()),
            details: serde_json::json!({
                "risk_event_evidence": {
                    "mempool_entry_evidence": {
                        "dependency_fee_metadata": {
                            "tail_after_tx_hash": "0x3333333333333333333333333333333333333333333333333333333333333333",
                            "dependency_priority_fee_wei": "123",
                            "dependency_gas_price_wei": "456"
                        }
                    }
                }
            }),
        });
        intent
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

    #[test]
    fn shadow_policy_preserves_live_chain_sim_block_evidence() {
        let mut report = report(100_000, 10_000_000_000_000);
        report.mined_evidence = Some(MinedExecutionEvidence {
            receipt_block_number: Some(100),
            simulation_block_number: Some(100),
            submitted_block_number: Some(99),
            expected_confirmation_block: Some(100),
            confirmation_lag_blocks: Some(0),
            receipt_status: Some("live_backtest_chain_sim".to_string()),
            ..MinedExecutionEvidence::default()
        });
        let outcome = outcome_from_selection(
            &intent(OrderSide::Buy),
            &report,
            300_000,
            150_000,
            selected(2, 20),
        );

        assert_eq!(outcome.evidence.submitted_block_number, Some(99));
        assert_eq!(outcome.evidence.expected_confirmation_block, Some(100));
        assert_eq!(outcome.evidence.simulation_block_number, Some(100));
        assert_eq!(outcome.evidence.receipt_block_number, Some(100));
        assert_eq!(outcome.evidence.confirmation_lag_blocks, Some(0));
        assert_eq!(
            outcome.evidence.receipt_status.as_deref(),
            Some("live_backtest_chain_sim")
        );
        assert_eq!(
            outcome.evidence.gas_policy_action.as_deref(),
            Some("entry_buy")
        );
    }

    #[test]
    fn tail_entry_shadow_records_ordering_evidence() {
        let report = report(100_000, 10_000_000_000_000);
        let outcome = outcome_from_selection(
            &tail_entry_intent(),
            &report,
            300_000,
            150_000,
            selected_tail_entry(2, 20),
        );

        assert_eq!(
            outcome.evidence.gas_policy_tail_after_tx_hash.as_deref(),
            Some("0x3333333333333333333333333333333333333333333333333333333333333333")
        );
        assert_eq!(
            outcome
                .evidence
                .gas_policy_dependency_priority_fee_wei
                .as_deref(),
            Some("123")
        );
        assert_eq!(
            outcome
                .evidence
                .gas_policy_dependency_gas_price_wei
                .as_deref(),
            Some("456")
        );
        assert!(outcome
            .evidence
            .gas_policy_guard
            .as_deref()
            .unwrap_or_default()
            .contains("tail_entry_validation_mode=post_mine_n_plus_1"));
        assert!(outcome
            .evidence
            .gas_policy_guard
            .as_deref()
            .unwrap_or_default()
            .contains("exact_overlay_simulation=false"));
    }
}
