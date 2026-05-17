use serde::{Deserialize, Serialize};

use super::utils::signed_block_delta;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LpControlFeatures {
    pub lp_total_supply: Option<f64>,
    pub lp_holder_count: Option<u32>,
    pub lp_top_holder_share_pct: Option<f64>,
    pub lp_top_holder_is_creator: Option<bool>,
    pub lp_top_holder_is_owner: Option<bool>,
    pub lp_approval_count_as_of: Option<u32>,
    pub lp_first_approval_block_as_of: Option<u64>,
    pub lp_holders_with_approvals_count: Option<u32>,
    pub lp_approved_spender_count_as_of: Option<u32>,
    pub lp_approved_pct_as_of: Option<f64>,
    pub lp_approved_to_router: Option<f64>,
    pub lp_approved_to_router_pct: Option<f64>,
    pub lp_router_approved_pct_as_of: Option<f64>,
    pub lp_router_approval_seen_as_of: Option<bool>,
    pub lp_max_approval_amount_as_of: Option<f64>,
    pub last_lp_approval_block: Option<u64>,
    pub last_lp_approval_timestamp: Option<u64>,
    pub last_lp_approval_owner: Option<String>,
    pub last_lp_approval_spender: Option<String>,
    pub last_lp_approval_is_router: Option<bool>,
    pub last_lp_approval_owner_is_creator: Option<bool>,
    pub last_lp_approval_owner_is_current_owner: Option<bool>,
    pub blocks_from_first_lp_approval_to_as_of: Option<u64>,
    pub blocks_from_last_lp_approval_to_as_of: Option<u64>,
    pub blocks_from_pool_creation_to_last_lp_approval: Option<i64>,
    pub blocks_from_trading_enabled_to_last_lp_approval: Option<i64>,
    pub feature_scope: Option<String>,
}

impl LpControlFeatures {
    pub fn with_as_of_offsets(mut self, as_of_block: u64) -> Self {
        self.blocks_from_first_lp_approval_to_as_of = self
            .lp_first_approval_block_as_of
            .map(|block| as_of_block.saturating_sub(block));
        self.blocks_from_last_lp_approval_to_as_of = self
            .last_lp_approval_block
            .map(|block| as_of_block.saturating_sub(block));
        self
    }

    pub fn with_last_approval_offsets(
        mut self,
        pool_creation_block: Option<u64>,
        trading_enabled_block: Option<u64>,
    ) -> Self {
        self.blocks_from_pool_creation_to_last_lp_approval =
            signed_block_delta(pool_creation_block, self.last_lp_approval_block);
        self.blocks_from_trading_enabled_to_last_lp_approval =
            signed_block_delta(trading_enabled_block, self.last_lp_approval_block);
        self
    }
}
