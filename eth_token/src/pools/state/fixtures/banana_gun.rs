use crate::pools::base::{BasePool, BasePoolConfig, PoolIdentity};
use crate::pools::state::evidence::{EvidenceConfidence, EvidenceRef, EvidenceSourceKind};
use crate::pools::state::labels::labels_for_state;
use crate::pools::state::model::PoolTrackedState;
use crate::pools::state::tracks::{CounterpartyRole, CounterpartyRoleEntry, EvidenceQualityTrack};

const WETH: &str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";

pub fn banana_gun_pass_through_state() -> PoolTrackedState {
    let mut base = BasePool::new(
        PoolIdentity::new(
            "0x6053000000000000000000000000000000000000",
            "0x3832000000000000000000000000000000000000",
            WETH,
            "uniswap_v2",
        ),
        BasePoolConfig {
            denom_decimals: Some(18),
            token1_is_denom: Some(true),
            ..BasePoolConfig::new(18)
        },
    );
    base.creator_address = Some("0xcreator000000000000000000000000000000000".to_string());
    base.update_reserves(1_000_000.0, 11.65, 25_227_090, 1_800_000_000, "0xsync");
    base.set_simulated_buy_status(true, 25_227_091, "0xbuy", 1_800_000_012);
    base.set_simulated_sell_status(true, Some(0.0), Some(0.0), 25_227_092, "0xsell");

    let mut state = PoolTrackedState::from_base_pool(&base, &[]);
    state.roles.roles = vec![
        CounterpartyRoleEntry::new(
            Some("0xuser000000000000000000000000000000000000".to_string()),
            CounterpartyRole::UserTrader,
            vec![fixture_evidence("user trader")],
        ),
        CounterpartyRoleEntry::new(
            Some("0xbananagun00000000000000000000000000000000".to_string()),
            CounterpartyRole::RouterPassThrough,
            vec![fixture_evidence("Banana Gun router pass-through")],
        ),
        CounterpartyRoleEntry::new(
            Some(state.identity.pool_address.clone()),
            CounterpartyRole::Pool,
            vec![fixture_evidence("AMM pool")],
        ),
        CounterpartyRoleEntry::new(
            Some(WETH.to_string()),
            CounterpartyRole::WethBridge,
            vec![fixture_evidence("WETH deposit/withdraw bridge")],
        ),
    ];
    refresh(&mut state);
    state
}

fn fixture_evidence(note: &str) -> EvidenceRef {
    EvidenceRef::new(EvidenceSourceKind::Fixture, EvidenceConfidence::High).with_note(note)
}

fn refresh(state: &mut PoolTrackedState) {
    state.quality = EvidenceQualityTrack::from_evidence_refs(state.track_evidence());
    state.labels = labels_for_state(state);
}
