use serde_json::json;

use crate::pools::base::{BasePool, BasePoolConfig, PoolIdentity};
use crate::pools::scam_mechanism::SCAM_DIRECT_LP_LIQUIDITY_REMOVAL;
use crate::pools::state::model::PoolTrackedState;

const WETH: &str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";

pub fn lp_pull_state() -> PoolTrackedState {
    let mut base = BasePool::new(
        PoolIdentity::new(
            "0xlppool0000000000000000000000000000000000",
            "0xlptoken00000000000000000000000000000000",
            WETH,
            "uniswap_v2",
        ),
        BasePoolConfig {
            denom_decimals: Some(18),
            token1_is_denom: Some(true),
            ..BasePoolConfig::new(18)
        },
    );
    base.creator_address = Some("0xlpc0ntroller000000000000000000000000000".to_string());
    base.update_reserves(900_000.0, 4.2, 25_181_120, 1_800_000_000, "0xsync");
    base.mark_scam_mechanism(
        SCAM_DIRECT_LP_LIQUIDITY_REMOVAL,
        Some(25_181_124),
        Some("0xlppull".to_string()),
        json!({"fixture": "direct LP liquidity removal"}),
    );
    base.reserve_tracker.is_scam = true;

    PoolTrackedState::from_base_pool(&base, &[])
}
