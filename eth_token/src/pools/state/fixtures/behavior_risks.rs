use crate::pools::base::{BasePool, BasePoolConfig, PoolIdentity};
use crate::pools::data_models::PoolLifecycle;
use crate::pools::state::model::PoolTrackedState;
use crate::pools::state::observations::{
    BehavioralOutcomeObservation, SellRestrictionObservation, TaxPolicyObservation,
    TransferPolicyObservation,
};
use crate::pools::state::tracks::{
    BehavioralOutcomeSignal, BehavioralOutcomesTrack, SellRestrictionSignal, SellRestrictionsTrack,
    TaxPolicySignal, TaxPolicyTrack, TransferPolicySignal, TransferPolicyTrack,
};

const WETH: &str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";

pub fn policy_restricted_routeable_state() -> PoolTrackedState {
    let mut state = routeable_priced_state(
        "0xpolicyblockedpool000000000000000000000000",
        "0xpolicyblockedtoken0000000000000000000000",
    );

    let mut blacklist = TransferPolicyObservation::synthetic(
        TransferPolicySignal::Blacklist,
        "scanner matched owner-controlled blacklist check",
    );
    blacklist.function = Some("isBlacklisted(address)".to_string());
    blacklist.selector = Some("0x01234567".to_string());

    let mut low_sell_limit = SellRestrictionObservation::synthetic(
        SellRestrictionSignal::LowSellLimit,
        "sell simulation found a dust-sized maximum sell limit",
    );
    low_sell_limit.limit_tokens = Some(25.0);
    low_sell_limit.limit_bps_of_supply = Some(0.5);

    state.transfer_policy = TransferPolicyTrack::from_observations(&[blacklist]);
    state.sell_restrictions = SellRestrictionsTrack::from_observations(&[low_sell_limit]);
    state.refresh_derived();
    state
}

pub fn tax_policy_without_custody_state() -> PoolTrackedState {
    let mut state = routeable_priced_state(
        "0xtaxpolicypool000000000000000000000000000",
        "0xtaxpolicytoken00000000000000000000000000",
    );

    let mut modifiable = TaxPolicyObservation::synthetic(
        TaxPolicySignal::ModifiableTax,
        "scanner matched owner-controlled tax setter",
    );
    modifiable.selector = Some("0x4d2d3f2a".to_string());

    let mut extreme = TaxPolicyObservation::synthetic(
        TaxPolicySignal::ExtremeSellTax,
        "sell simulation observed an extreme sell tax",
    );
    extreme.buy_tax_percent = Some(1.0);
    extreme.sell_tax_percent = Some(92.0);

    state.tax_policy = TaxPolicyTrack::from_observations(&[modifiable, extreme]);
    state.refresh_derived();
    state
}

pub fn reused_confiscation_pattern_state() -> PoolTrackedState {
    let mut state = routeable_priced_state(
        "0xreusedpatternpool00000000000000000000000",
        "0xreusedpatterntoken000000000000000000000",
    );

    let mut outcome = BehavioralOutcomeObservation::synthetic(
        BehavioralOutcomeSignal::ReusedConfiscationPattern,
        "bytecode/function signature matched repeated confiscation pattern",
    );
    outcome.pattern_id = Some("confiscation_signature_cluster_v1".to_string());
    outcome.signature = Some("0x70a08231:balanceOf|0xa9059cbb:transfer".to_string());
    outcome.sample_size = Some(4);

    state.behavioral_outcomes = BehavioralOutcomesTrack::from_observations(&[outcome]);
    state.refresh_derived();
    state
}

fn routeable_priced_state(pool_address: &str, token_address: &str) -> PoolTrackedState {
    let mut base = BasePool::new(
        PoolIdentity::new(pool_address, token_address, WETH, "uniswap_v2"),
        BasePoolConfig {
            denom_decimals: Some(18),
            token1_is_denom: Some(true),
            ..BasePoolConfig::new(18)
        },
    );
    base.update_reserves(2_000_000.0, 8.0, 25_300_000, 1_800_000_000, "0xsync");
    base.set_simulated_buy_status(true, 25_300_001, "0xbuy", 1_800_000_012);
    base.set_simulated_sell_status(true, Some(0.0), Some(0.0), 25_300_002, "0xsell");
    base.state.lifecycle = PoolLifecycle::Trading;

    PoolTrackedState::from_base_pool(&base, &[])
}
