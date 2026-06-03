use serde::{Deserialize, Serialize};

use crate::pools::state::model::PoolTrackedState;
use crate::pools::state::tracks::valuation::ValuationState;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PoolPnlStateView {
    pub pool_address: String,
    pub token_address: String,
    pub denom_address: String,
    pub lifecycle: String,
    pub valuation_state: ValuationState,
    pub price_denom_per_token: Option<f64>,
    pub terminal_position_risk: bool,
    pub risk_mechanism: Option<String>,
    pub custody_realized: bool,
    pub labels: Vec<String>,
}

impl From<&PoolTrackedState> for PoolPnlStateView {
    fn from(state: &PoolTrackedState) -> Self {
        Self {
            pool_address: state.identity.pool_address.clone(),
            token_address: state.identity.token_address.clone(),
            denom_address: state.identity.denom_address.clone(),
            lifecycle: state.lifecycle.phase.as_str().to_string(),
            valuation_state: state.valuation.state,
            price_denom_per_token: state.valuation.price_denom_per_token,
            terminal_position_risk: state.risk.terminal_position_risk,
            risk_mechanism: state.risk.scam_mechanism.clone(),
            custody_realized: state.custody.realized,
            labels: state.labels.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RiskAtlasPoolView {
    pub pool_address: String,
    pub token_address: String,
    pub protocol: String,
    pub risk_mechanism: Option<String>,
    pub risk_label: Option<String>,
    pub terminal_position_risk: bool,
    pub custody_latent: bool,
    pub custody_realized: bool,
    pub custody_capabilities: Vec<String>,
    #[serde(default)]
    pub transfer_policy_labels: Vec<String>,
    #[serde(default)]
    pub sell_restriction_labels: Vec<String>,
    #[serde(default)]
    pub tax_policy_labels: Vec<String>,
    #[serde(default)]
    pub contract_posture_labels: Vec<String>,
    #[serde(default)]
    pub supply_control_labels: Vec<String>,
    #[serde(default)]
    pub behavioral_outcome_labels: Vec<String>,
    pub evidence_confidence: String,
    pub labels: Vec<String>,
}

impl From<&PoolTrackedState> for RiskAtlasPoolView {
    fn from(state: &PoolTrackedState) -> Self {
        Self {
            pool_address: state.identity.pool_address.clone(),
            token_address: state.identity.token_address.clone(),
            protocol: state.identity.protocol.clone(),
            risk_mechanism: state.risk.scam_mechanism.clone(),
            risk_label: state.risk.scam_label.clone(),
            terminal_position_risk: state.risk.terminal_position_risk,
            custody_latent: state.custody.latent,
            custody_realized: state.custody.realized,
            custody_capabilities: state
                .custody
                .capabilities
                .iter()
                .map(|capability| capability.capability.as_str().to_string())
                .collect(),
            transfer_policy_labels: labels_to_strings(state.transfer_policy.active_labels()),
            sell_restriction_labels: labels_to_strings(state.sell_restrictions.active_labels()),
            tax_policy_labels: labels_to_strings(state.tax_policy.active_labels()),
            contract_posture_labels: labels_to_strings(state.contract_posture.active_labels()),
            supply_control_labels: labels_to_strings(state.supply_control.active_labels()),
            behavioral_outcome_labels: labels_to_strings(state.behavioral_outcomes.active_labels()),
            evidence_confidence: format!("{:?}", state.quality.confidence).to_ascii_lowercase(),
            labels: state.labels.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LiveTradingPoolView {
    pub pool_address: String,
    pub token_address: String,
    pub denom_address: String,
    pub protocol: String,
    pub raw_can_buy: bool,
    pub raw_can_sell: bool,
    pub effective_can_buy: bool,
    pub effective_can_sell: bool,
    pub has_effective_liquidity: bool,
    pub liquidity_class: String,
    pub price_denom_per_token: Option<f64>,
    pub tradable_now: bool,
    pub block_reason: Option<String>,
    #[serde(default)]
    pub behavior_risk_blocked: bool,
    #[serde(default)]
    pub behavior_risk_blockers: Vec<String>,
    pub labels: Vec<String>,
}

impl From<&PoolTrackedState> for LiveTradingPoolView {
    fn from(state: &PoolTrackedState) -> Self {
        let behavior_risk_blockers = live_behavior_risk_blockers(state);
        let tradable_now = state.routeability.effective_can_buy
            && state.routeability.effective_can_sell
            && state.liquidity.has_effective_liquidity
            && state.valuation.state == ValuationState::Priced
            && !state.risk.terminal_position_risk
            && behavior_risk_blockers.is_empty();
        Self {
            pool_address: state.identity.pool_address.clone(),
            token_address: state.identity.token_address.clone(),
            denom_address: state.identity.denom_address.clone(),
            protocol: state.identity.protocol.clone(),
            raw_can_buy: state.routeability.raw_can_buy,
            raw_can_sell: state.routeability.raw_can_sell,
            effective_can_buy: state.routeability.effective_can_buy,
            effective_can_sell: state.routeability.effective_can_sell,
            has_effective_liquidity: state.liquidity.has_effective_liquidity,
            liquidity_class: state.liquidity.class.as_str().to_string(),
            price_denom_per_token: state.valuation.price_denom_per_token,
            tradable_now,
            block_reason: block_reason(state, &behavior_risk_blockers),
            behavior_risk_blocked: !behavior_risk_blockers.is_empty(),
            behavior_risk_blockers,
            labels: state.labels.clone(),
        }
    }
}

fn block_reason(state: &PoolTrackedState, behavior_risk_blockers: &[String]) -> Option<String> {
    if state.risk.terminal_position_risk {
        return Some("terminal_position_risk".to_string());
    }
    if let Some(blocker) = behavior_risk_blockers.first() {
        return Some(blocker.clone());
    }
    if !state.liquidity.has_effective_liquidity {
        return Some("insufficient_liquidity".to_string());
    }
    if !state.routeability.effective_can_buy {
        return Some("cannot_buy".to_string());
    }
    if !state.routeability.effective_can_sell {
        return Some("cannot_sell".to_string());
    }
    if state.valuation.state != ValuationState::Priced {
        return Some("no_usable_price".to_string());
    }
    None
}

fn live_behavior_risk_blockers(state: &PoolTrackedState) -> Vec<String> {
    let mut labels = Vec::new();
    labels.extend(labels_to_strings(
        state.transfer_policy.live_blocker_labels(),
    ));
    labels.extend(labels_to_strings(
        state.sell_restrictions.live_blocker_labels(),
    ));
    labels.extend(labels_to_strings(state.tax_policy.live_blocker_labels()));
    labels
}

fn labels_to_strings(labels: Vec<&'static str>) -> Vec<String> {
    labels.into_iter().map(str::to_string).collect()
}
