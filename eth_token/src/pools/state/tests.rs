use serde_json::json;

use crate::custody::{CustodyCapability, CustodyFinding, CustodyState};
use crate::pools::base::{BasePool, BasePoolConfig, PoolIdentity};
use crate::pools::data_models::PoolLifecycle;
use crate::pools::state::fixtures::{
    banana_gun_pass_through_state, custody_drain_state, lp_pull_state,
    policy_restricted_routeable_state, reused_confiscation_pattern_state,
    tax_policy_without_custody_state,
};
use crate::pools::state::tracks::{
    BehavioralOutcomeSignal, CounterpartyRole, LifecyclePhase, ValuationState,
};
use crate::pools::state::views::{LiveTradingPoolView, RiskAtlasPoolView};
use crate::pools::state::PoolTrackedState;

const WETH: &str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";

fn base_pool() -> BasePool {
    let mut base = BasePool::new(
        PoolIdentity::new("0xPOOL", "0xTOKEN", WETH, "uniswap_v2"),
        BasePoolConfig {
            denom_decimals: Some(18),
            token1_is_denom: Some(true),
            ..BasePoolConfig::new(18)
        },
    );
    base.update_reserves(1_000_000.0, 1.0, 100, 1_800_000_000, "0xsync");
    base.set_simulated_buy_status(true, 101, "0xbuy", 1_800_000_012);
    base.set_simulated_sell_status(false, Some(0.0), Some(0.0), 102, "0xsellfail");
    base.state.lifecycle = PoolLifecycle::Trading;
    base
}

#[test]
fn tracks_allow_lifecycle_custody_route_and_valuation_to_coexist() {
    let base = base_pool();
    let finding = CustodyFinding {
        capability: CustodyCapability::Freeze,
        state: CustodyState::Latent,
        block_number: Some(103),
        evidence: json!({"fixture": "latent owner control"}),
    };

    let state = PoolTrackedState::from_base_pool(&base, &[finding]);

    assert_eq!(state.lifecycle.phase, LifecyclePhase::Trading);
    assert!(state.custody.latent);
    assert!(!state.custody.realized);
    assert!(state.routeability.raw_can_buy);
    assert!(!state.routeability.raw_can_sell);
    assert_eq!(state.valuation.state, ValuationState::Priced);
    assert!(state.labels.contains(&"route:cannot_sell".to_string()));
    assert!(state
        .labels
        .contains(&"custody:latent:custody_freeze".to_string()));
}

#[test]
fn banana_gun_fixture_projects_router_and_weth_roles() {
    let state = banana_gun_pass_through_state();

    assert!(state.roles.has_role(CounterpartyRole::RouterPassThrough));
    assert!(state.roles.has_role(CounterpartyRole::WethBridge));
    assert!(state
        .labels
        .contains(&"role:router_pass_through".to_string()));
    assert!(state.labels.contains(&"role:weth_bridge".to_string()));
}

#[test]
fn consumer_views_project_from_tracked_state() {
    let state = banana_gun_pass_through_state();
    let live = LiveTradingPoolView::from(&state);

    assert!(live.tradable_now);
    assert_eq!(live.block_reason, None);

    let risk_state = custody_drain_state();
    let risk_view = RiskAtlasPoolView::from(&risk_state);

    assert!(risk_view.terminal_position_risk);
    assert!(risk_view.custody_realized);
    assert_eq!(
        risk_view.risk_mechanism.as_deref(),
        Some("holder_balance_backdoor_drain")
    );
}

#[test]
fn lp_pull_fixture_keeps_lp_control_and_risk_angles_separate() {
    let state = lp_pull_state();

    assert!(state.lp_control.liquidity_removed_by_lp);
    assert!(state.risk.direct_lp_liquidity_removal);
    assert!(state.liquidity.reserve_liquidity_removed);
    assert_eq!(state.valuation.state, ValuationState::TerminalZero);
}

#[test]
fn routeable_priced_pool_can_be_blocked_by_policy_and_sell_restriction() {
    let state = policy_restricted_routeable_state();

    assert!(state.routeability.effective_can_buy);
    assert!(state.routeability.effective_can_sell);
    assert_eq!(state.valuation.state, ValuationState::Priced);
    assert!(state
        .labels
        .contains(&"policy:blacklist_present".to_string()));
    assert!(state
        .labels
        .contains(&"restriction:low_sell_limit".to_string()));

    let live = LiveTradingPoolView::from(&state);

    assert!(!live.tradable_now);
    assert!(live.behavior_risk_blocked);
    assert_eq!(
        live.block_reason.as_deref(),
        Some("policy:blacklist_present")
    );
    assert!(live
        .behavior_risk_blockers
        .contains(&"policy:blacklist_present".to_string()));
    assert!(live
        .behavior_risk_blockers
        .contains(&"restriction:low_sell_limit".to_string()));
}

#[test]
fn tax_policy_flags_do_not_imply_realized_custody() {
    let state = tax_policy_without_custody_state();

    assert!(state.labels.contains(&"tax:modifiable".to_string()));
    assert!(state.labels.contains(&"tax:extreme_sell_tax".to_string()));
    assert!(!state.custody.latent);
    assert!(!state.custody.realized);
    assert_eq!(state.valuation.state, ValuationState::Priced);

    let risk_view = RiskAtlasPoolView::from(&state);
    assert!(risk_view
        .tax_policy_labels
        .contains(&"tax:modifiable".to_string()));
    assert!(risk_view
        .tax_policy_labels
        .contains(&"tax:extreme_sell_tax".to_string()));
    assert!(!risk_view.custody_realized);
}

#[test]
fn repeated_signature_maps_to_reused_confiscation_pattern_behavior_label() {
    let state = reused_confiscation_pattern_state();

    assert!(state.behavioral_outcomes.reused_confiscation_pattern);
    assert!(state
        .labels
        .contains(&"behavior:reused_confiscation_pattern".to_string()));

    let outcome = state
        .behavioral_outcomes
        .outcomes
        .iter()
        .find(|outcome| outcome.signal == BehavioralOutcomeSignal::ReusedConfiscationPattern)
        .expect("reused confiscation outcome");
    assert_eq!(
        outcome.pattern_id.as_deref(),
        Some("confiscation_signature_cluster_v1")
    );
    assert_eq!(
        outcome.signature.as_deref(),
        Some("0x70a08231:balanceOf|0xa9059cbb:transfer")
    );

    let risk_view = RiskAtlasPoolView::from(&state);
    assert!(risk_view
        .behavioral_outcome_labels
        .contains(&"behavior:reused_confiscation_pattern".to_string()));
}
