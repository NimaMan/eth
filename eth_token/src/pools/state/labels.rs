use std::collections::BTreeSet;

use crate::pools::state::model::PoolTrackedState;
use crate::pools::state::tracks::valuation::ValuationState;

pub fn labels_for_state(state: &PoolTrackedState) -> Vec<String> {
    let mut labels = BTreeSet::new();

    labels.insert(format!("protocol:{}", state.identity.protocol));
    labels.insert(format!("lifecycle:{}", state.lifecycle.phase.as_str()));
    labels.insert(format!("liquidity:{}", state.liquidity.class.as_str()));
    labels.insert(format!("valuation:{}", state.valuation.state.as_str()));
    labels.insert(format!("eligibility:{}", state.eligibility.category));

    if state.liquidity.has_reserves {
        labels.insert("liquidity:has_reserves".to_string());
    }
    if state.liquidity.has_effective_liquidity {
        labels.insert("liquidity:effective".to_string());
    }
    if state.liquidity.reserve_liquidity_removed {
        labels.insert("liquidity:reserve_removed".to_string());
    } else if state.liquidity.legacy_liquidity_removal_flag {
        labels.insert("liquidity:legacy_terminal_flag".to_string());
    }

    if state.routeability.raw_can_buy {
        labels.insert("route:raw_can_buy".to_string());
    }
    if state.routeability.raw_can_sell {
        labels.insert("route:raw_can_sell".to_string());
    }
    if state.routeability.effective_can_buy {
        labels.insert("route:effective_can_buy".to_string());
    }
    if state.routeability.effective_can_sell {
        labels.insert("route:effective_can_sell".to_string());
    }
    if state.routeability.raw_can_buy && !state.routeability.raw_can_sell {
        labels.insert("route:cannot_sell".to_string());
    }
    if state.routeability.raw_can_sell && !state.routeability.raw_can_buy {
        labels.insert("route:cannot_buy".to_string());
    }
    match state.routeability.economic_sellable {
        Some(true) => {
            labels.insert("route:economic_sellable".to_string());
        }
        Some(false) => {
            labels.insert("route:not_economic_sellable".to_string());
        }
        None => {}
    }

    match state.valuation.state {
        ValuationState::Priced => {
            labels.insert("valuation:priced_from_reserves".to_string());
        }
        ValuationState::TerminalZero => {
            labels.insert("valuation:terminal_zero".to_string());
        }
        ValuationState::NoMark => {
            labels.insert("valuation:no_usable_price".to_string());
        }
    }

    if state.custody.latent {
        labels.insert("custody:latent".to_string());
    }
    if state.custody.realized {
        labels.insert("custody:realized".to_string());
    }
    for capability in &state.custody.capabilities {
        labels.insert(format!(
            "custody:{}:{}",
            capability.state.as_str(),
            capability.capability.as_str()
        ));
    }

    for label in state.transfer_policy.active_labels() {
        labels.insert(label.to_string());
    }
    for label in state.sell_restrictions.active_labels() {
        labels.insert(label.to_string());
    }
    for label in state.tax_policy.active_labels() {
        labels.insert(label.to_string());
    }
    for label in state.contract_posture.active_labels() {
        labels.insert(label.to_string());
    }
    for label in state.supply_control.active_labels() {
        labels.insert(label.to_string());
    }
    for label in state.behavioral_outcomes.active_labels() {
        labels.insert(label.to_string());
    }

    if state.risk.terminal_position_risk {
        labels.insert("risk:terminal_position".to_string());
    }
    if let Some(mechanism) = state.risk.scam_mechanism.as_deref() {
        labels.insert(format!("risk:{mechanism}"));
    }

    for role in &state.roles.roles {
        labels.insert(format!("role:{}", role.role.as_str()));
    }

    if state.quality.event_backed {
        labels.insert("evidence:event_backed".to_string());
    }
    if state.quality.simulation_backed {
        labels.insert("evidence:simulation_backed".to_string());
    }
    if state.quality.trace_backed {
        labels.insert("evidence:trace_backed".to_string());
    }
    if state.quality.custody_backed {
        labels.insert("evidence:custody_backed".to_string());
    }

    labels.into_iter().collect()
}
