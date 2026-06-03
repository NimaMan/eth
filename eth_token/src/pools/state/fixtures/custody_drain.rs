use serde_json::json;

use crate::custody::{CustodyCapability, CustodyFinding, CustodyState};
use crate::pools::base::{BasePool, BasePoolConfig, PoolIdentity};
use crate::pools::scam_mechanism::SCAM_HOLDER_BALANCE_BACKDOOR_DRAIN;
use crate::pools::state::model::PoolTrackedState;

const WETH: &str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";

pub fn custody_drain_state() -> PoolTrackedState {
    let mut base = BasePool::new(
        PoolIdentity::new(
            "0xdrainpool00000000000000000000000000000000",
            "0xdraintoken0000000000000000000000000000000",
            WETH,
            "uniswap_v2",
        ),
        BasePoolConfig {
            denom_decimals: Some(18),
            token1_is_denom: Some(true),
            ..BasePoolConfig::new(18)
        },
    );
    base.update_reserves(2_500_000.0, 3.0, 25_202_410, 1_800_000_000, "0xsync");
    base.set_simulated_buy_status(true, 25_202_411, "0xbuy", 1_800_000_012);
    base.set_simulated_sell_status(true, Some(0.0), Some(0.0), 25_202_412, "0xsell");
    base.mark_scam_mechanism(
        SCAM_HOLDER_BALANCE_BACKDOOR_DRAIN,
        Some(25_202_413),
        Some("0xholderdrain".to_string()),
        json!({"fixture": "holder balance drain"}),
    );
    base.reserve_tracker.is_scam = true;

    let finding = CustodyFinding {
        capability: CustodyCapability::BurnDrain,
        state: CustodyState::Realized,
        block_number: Some(25_202_413),
        evidence: json!({"fixture": "event-less holder balance drain"}),
    };

    PoolTrackedState::from_base_pool(&base, &[finding])
}
