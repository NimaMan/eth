use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FeatureEvidenceBlocks {
    pub token_static_latest_block: Option<u64>,
    pub authority_latest_block: Option<u64>,
    pub market_latest_block: Option<u64>,
    pub liquidity_latest_block: Option<u64>,
    pub lp_control_latest_block: Option<u64>,
    pub token_control_latest_block: Option<u64>,
    pub activity_latest_block: Option<u64>,
    pub network_latest_block: Option<u64>,
}

impl FeatureEvidenceBlocks {
    pub fn latest(&self) -> Option<u64> {
        [
            self.token_static_latest_block,
            self.authority_latest_block,
            self.market_latest_block,
            self.liquidity_latest_block,
            self.lp_control_latest_block,
            self.token_control_latest_block,
            self.activity_latest_block,
            self.network_latest_block,
        ]
        .into_iter()
        .flatten()
        .max()
    }
}
