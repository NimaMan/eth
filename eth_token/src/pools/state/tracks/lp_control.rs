use serde::{Deserialize, Serialize};

use crate::pools::base::BasePool;
use crate::pools::flags::PoolStateFlags;
use crate::pools::state::evidence::EvidenceRef;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LpControlTrack {
    pub creator_address: Option<String>,
    pub lp_owned_by_creator: Option<bool>,
    pub lp_burned_or_locked: Option<bool>,
    pub lp_approval_exposure_pct: Option<f64>,
    pub lp_holder_concentration_pct: Option<f64>,
    pub liquidity_removed_by_lp: bool,
    pub evidence: Vec<EvidenceRef>,
}

impl LpControlTrack {
    pub fn from_base_and_flags(base: &BasePool, flags: &PoolStateFlags) -> Self {
        Self {
            creator_address: base.creator_address.clone(),
            lp_owned_by_creator: None,
            lp_burned_or_locked: None,
            lp_approval_exposure_pct: None,
            lp_holder_concentration_pct: None,
            liquidity_removed_by_lp: flags.risk.direct_lp_liquidity_removal,
            evidence: vec![EvidenceRef::flags_projection()
                .at_block(flags.latest_block_number)
                .with_tx(base.scam_tx_hash.clone())],
        }
    }
}
