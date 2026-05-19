use eth_alpha_core::{
    amount::DecimalAmount,
    decision_rationale::{source, DecisionReason},
    ids::{BlockNumber, PoolAddress, TokenAddress, TradeId},
};
use serde::{Deserialize, Serialize};

/// Live LP approval response policy.
///
/// The strategy treats meaningful LP approval as an entry blocker and, when we
/// already hold the pool, as a priority sell trigger. "Priority" means the real
/// execution adapter should use protected/private routing when available and a
/// fee-capped public fallback only when explicitly enabled.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BribeExitConfig {
    /// Strict lower bound: approval must be greater than this percent of LP
    /// supply to trigger the rule.
    pub min_lp_approved_pct: DecimalAmount,
    /// Maximum priority fee the real adapter may use for the urgent sell.
    pub max_priority_fee_per_gas_gwei: DecimalAmount,
    /// Maximum total ETH cost allowed for the priority exit attempt.
    pub max_total_fee_eth: DecimalAmount,
    /// Prefer protected/private builder relay submission for race exits.
    pub prefer_private_relay: bool,
    /// Allow public mempool fallback when private submission is unavailable.
    pub allow_public_mempool_fallback: bool,
}

impl Default for BribeExitConfig {
    fn default() -> Self {
        Self {
            min_lp_approved_pct: DecimalAmount::from(30),
            max_priority_fee_per_gas_gwei: DecimalAmount::from(100),
            max_total_fee_eth: DecimalAmount::new(2, 2),
            prefer_private_relay: true,
            allow_public_mempool_fallback: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum LpSignalSource {
    /// LP approval was seen before mining by the mempool processor.
    MempoolLpApproval,
    /// LP approval was observed in confirmed chain state.
    MinedLpApproval,
    /// Direct liquidity removal is already observed in confirmed chain state.
    MinedLiquidityRemoval,
}

impl Default for LpSignalSource {
    fn default() -> Self {
        Self::MinedLpApproval
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LpApprovalSignal {
    pub token_address: TokenAddress,
    pub pool_address: PoolAddress,
    pub observed_block: BlockNumber,
    pub source: LpSignalSource,
    pub approved_pct: Option<DecimalAmount>,
    /// Set when direct removal is already known for this pool.
    pub liquidity_removal_block: Option<BlockNumber>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeldPositionContext {
    pub trade_id: TradeId,
    pub token_address: TokenAddress,
    pub pool_address: PoolAddress,
    pub entry_block: BlockNumber,
    pub current_block: BlockNumber,
    pub sell_order_pending: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PriorityRoute {
    PrivateRelay,
    PublicMempool,
    PrivateRelayWithPublicFallback,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SellUrgency {
    /// Pre-mine signal. This is the cleanest live edge.
    MempoolPreMine,
    /// Approval is mined and removal has not yet been observed; race the next block.
    MinedApprovalRace,
    /// Approval was mined in the same block that confirmed our buy.
    BuyConfirmBlockApproval,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrioritySellPlan {
    pub trade_id: TradeId,
    pub token_address: TokenAddress,
    pub pool_address: PoolAddress,
    pub observed_block: BlockNumber,
    #[serde(default)]
    pub signal_source: LpSignalSource,
    pub urgency: SellUrgency,
    pub route: PriorityRoute,
    pub max_priority_fee_per_gas_gwei: DecimalAmount,
    pub max_total_fee_eth: DecimalAmount,
    pub reason: String,
}

impl PrioritySellPlan {
    pub fn event_source(&self) -> &'static str {
        match self.signal_source {
            LpSignalSource::MempoolLpApproval => source::EVENT_SOURCE_MEMPOOL_SIGNAL,
            LpSignalSource::MinedLpApproval | LpSignalSource::MinedLiquidityRemoval => {
                source::EVENT_SOURCE_RISK_ATLAS_MINED_CHAIN
            }
        }
    }

    pub fn decision_reason(&self) -> Option<DecisionReason> {
        DecisionReason::from_parts(
            Some(&self.reason),
            Some(self.event_source()),
            Some("submit_order"),
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TradeAction {
    /// No buy should be submitted for this pool.
    BlockEntry { reason: String },
    /// Submit an urgent sell through the real execution adapter.
    SubmitPrioritySell(PrioritySellPlan),
    /// No action should be taken now.
    Hold { reason: String },
    /// The direct removal is already confirmed. We can still do recovery
    /// bookkeeping, but the priority-exit edge is gone.
    TooLate { reason: String },
}

pub fn plan_lp_approval_response(
    config: &BribeExitConfig,
    position: Option<&HeldPositionContext>,
    signal: &LpApprovalSignal,
) -> TradeAction {
    if !approval_exceeds_threshold(signal, config.min_lp_approved_pct) {
        return TradeAction::Hold {
            reason: "lp_approval_below_threshold".to_string(),
        };
    }

    let Some(position) = position else {
        return TradeAction::BlockEntry {
            reason: "lp_approval_visible_before_entry".to_string(),
        };
    };

    if position.token_address != signal.token_address
        || position.pool_address != signal.pool_address
    {
        return TradeAction::Hold {
            reason: "lp_approval_not_for_position".to_string(),
        };
    }

    if position.sell_order_pending {
        return TradeAction::Hold {
            reason: "sell_order_already_pending".to_string(),
        };
    }

    if signal.source == LpSignalSource::MinedLiquidityRemoval {
        return TradeAction::TooLate {
            reason: "liquidity_removal_already_mined".to_string(),
        };
    }

    if let Some(removal_block) = signal.liquidity_removal_block {
        if removal_block <= signal.observed_block {
            return TradeAction::TooLate {
                reason: "liquidity_removal_already_mined".to_string(),
            };
        }
    }

    let urgency = match signal.source {
        LpSignalSource::MempoolLpApproval => SellUrgency::MempoolPreMine,
        LpSignalSource::MinedLpApproval if signal.observed_block == position.entry_block => {
            SellUrgency::BuyConfirmBlockApproval
        }
        LpSignalSource::MinedLpApproval => SellUrgency::MinedApprovalRace,
        LpSignalSource::MinedLiquidityRemoval => unreachable!("handled above"),
    };
    let reason = priority_sell_reason(&urgency).to_string();

    TradeAction::SubmitPrioritySell(PrioritySellPlan {
        trade_id: position.trade_id.clone(),
        token_address: position.token_address,
        pool_address: position.pool_address.clone(),
        observed_block: signal.observed_block,
        signal_source: signal.source.clone(),
        urgency,
        route: priority_route(config),
        max_priority_fee_per_gas_gwei: config.max_priority_fee_per_gas_gwei,
        max_total_fee_eth: config.max_total_fee_eth,
        reason,
    })
}

fn approval_exceeds_threshold(signal: &LpApprovalSignal, min_pct: DecimalAmount) -> bool {
    signal
        .approved_pct
        .map(|approved_pct| approved_pct > min_pct)
        .unwrap_or(false)
}

fn priority_route(config: &BribeExitConfig) -> PriorityRoute {
    match (
        config.prefer_private_relay,
        config.allow_public_mempool_fallback,
    ) {
        (true, true) => PriorityRoute::PrivateRelayWithPublicFallback,
        (true, false) => PriorityRoute::PrivateRelay,
        (false, _) => PriorityRoute::PublicMempool,
    }
}

fn priority_sell_reason(urgency: &SellUrgency) -> &'static str {
    match urgency {
        SellUrgency::MempoolPreMine => "exit.mempool_liquidity_removal_signal",
        SellUrgency::MinedApprovalRace => "exit.lp_approval_mined_race",
        SellUrgency::BuyConfirmBlockApproval => "exit.lp_approval_buy_confirm_block",
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::Address;

    use super::*;

    fn pool() -> PoolAddress {
        PoolAddress::from("0x0000000000000000000000000000000000000001:0xpool")
    }

    fn token() -> TokenAddress {
        Address::with_last_byte(0x11)
    }

    fn signal(source: LpSignalSource, block: BlockNumber) -> LpApprovalSignal {
        LpApprovalSignal {
            token_address: token(),
            pool_address: pool(),
            observed_block: block,
            source,
            approved_pct: Some(DecimalAmount::from(100)),
            liquidity_removal_block: None,
        }
    }

    fn position(entry_block: BlockNumber) -> HeldPositionContext {
        HeldPositionContext {
            trade_id: TradeId::from("trd_test"),
            token_address: token(),
            pool_address: pool(),
            entry_block,
            current_block: entry_block,
            sell_order_pending: false,
        }
    }

    #[test]
    fn visible_lp_approval_without_position_blocks_entry() {
        let action = plan_lp_approval_response(
            &BribeExitConfig::default(),
            None,
            &signal(LpSignalSource::MinedLpApproval, 10),
        );

        assert!(matches!(action, TradeAction::BlockEntry { .. }));
    }

    #[test]
    fn buy_confirm_block_lp_approval_submits_priority_sell() {
        let action = plan_lp_approval_response(
            &BribeExitConfig::default(),
            Some(&position(10)),
            &signal(LpSignalSource::MinedLpApproval, 10),
        );

        match action {
            TradeAction::SubmitPrioritySell(plan) => {
                assert_eq!(plan.signal_source, LpSignalSource::MinedLpApproval);
                assert_eq!(plan.urgency, SellUrgency::BuyConfirmBlockApproval);
                assert_eq!(plan.route, PriorityRoute::PrivateRelay);
                assert_eq!(plan.reason, "exit.lp_approval_buy_confirm_block");
            }
            other => panic!("expected priority sell, got {other:?}"),
        }
    }

    #[test]
    fn mempool_lp_approval_uses_mempool_exit_reason() {
        let action = plan_lp_approval_response(
            &BribeExitConfig::default(),
            Some(&position(10)),
            &signal(LpSignalSource::MempoolLpApproval, 12),
        );

        match action {
            TradeAction::SubmitPrioritySell(plan) => {
                assert_eq!(plan.signal_source, LpSignalSource::MempoolLpApproval);
                assert_eq!(plan.urgency, SellUrgency::MempoolPreMine);
                assert_eq!(plan.event_source(), source::EVENT_SOURCE_MEMPOOL_SIGNAL);
                assert_eq!(plan.reason, "exit.mempool_liquidity_removal_signal");
                assert_eq!(
                    plan.decision_reason().unwrap().code,
                    "exit.mempool_liquidity_removal_signal"
                );
            }
            other => panic!("expected priority sell, got {other:?}"),
        }
    }

    #[test]
    fn mempool_lp_approval_submits_pre_mine_priority_sell() {
        let action = plan_lp_approval_response(
            &BribeExitConfig::default(),
            Some(&position(10)),
            &signal(LpSignalSource::MempoolLpApproval, 12),
        );

        match action {
            TradeAction::SubmitPrioritySell(plan) => {
                assert_eq!(plan.urgency, SellUrgency::MempoolPreMine);
                assert_eq!(plan.reason, "exit.mempool_liquidity_removal_signal");
            }
            other => panic!("expected priority sell, got {other:?}"),
        }
    }

    #[test]
    fn mined_lp_approval_after_entry_submits_race_sell() {
        let action = plan_lp_approval_response(
            &BribeExitConfig::default(),
            Some(&position(10)),
            &signal(LpSignalSource::MinedLpApproval, 12),
        );

        match action {
            TradeAction::SubmitPrioritySell(plan) => {
                assert_eq!(plan.urgency, SellUrgency::MinedApprovalRace);
                assert_eq!(plan.reason, "exit.lp_approval_mined_race");
            }
            other => panic!("expected priority sell, got {other:?}"),
        }
    }

    #[test]
    fn mined_liquidity_removal_is_too_late_for_priority_edge() {
        let action = plan_lp_approval_response(
            &BribeExitConfig::default(),
            Some(&position(10)),
            &signal(LpSignalSource::MinedLiquidityRemoval, 12),
        );

        assert!(matches!(action, TradeAction::TooLate { .. }));
    }

    #[test]
    fn approval_at_threshold_does_not_trigger() {
        let mut signal = signal(LpSignalSource::MinedLpApproval, 12);
        signal.approved_pct = Some(DecimalAmount::from(30));

        let action =
            plan_lp_approval_response(&BribeExitConfig::default(), Some(&position(10)), &signal);

        assert!(matches!(action, TradeAction::Hold { .. }));
    }
}
