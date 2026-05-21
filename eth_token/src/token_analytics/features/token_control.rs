use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TokenControlFeatures {
    pub control_transfer_from_count_as_of: Option<u32>,
    pub control_transfer_from_count_in_block: Option<u32>,
    pub control_transfer_from_seen_as_of: Option<bool>,
    pub control_transfer_from_in_block: Option<bool>,
    pub control_transfer_from_after_renounce_seen_as_of: Option<bool>,
    pub control_transfer_from_after_renounce_in_block: Option<bool>,
    pub control_transfer_from_holder_to_burn_seen_as_of: Option<bool>,
    pub control_transfer_from_holder_to_burn_in_block: Option<bool>,
    pub control_transfer_from_pair_seen_as_of: Option<bool>,
    pub control_transfer_from_pair_in_block: Option<bool>,
    pub control_transfer_from_without_transfer_log_seen_as_of: Option<bool>,
    pub control_transfer_from_without_transfer_log_in_block: Option<bool>,
    pub pair_token_to_control_seen_as_of: Option<bool>,
    pub pair_token_to_control_in_block: Option<bool>,
    pub pair_token_to_control_amount_in_block: Option<f64>,
    pub pair_token_to_control_to_total_supply_ratio: Option<f64>,
    pub pair_token_to_control_to_pool_reserve_ratio: Option<f64>,
    pub pair_balance_backdoor_signal_seen_as_of: Option<bool>,
    pub pair_balance_backdoor_signal_in_block: Option<bool>,
    pub last_control_transfer_from_block: Option<u64>,
    pub last_pair_token_to_control_block: Option<u64>,
    pub last_pair_balance_backdoor_signal_block: Option<u64>,
    pub last_pair_balance_backdoor_signal_to_as_of_chain_block_delta: Option<u64>,
    pub feature_scope: Option<String>,
}

impl TokenControlFeatures {
    pub fn with_as_of_offsets(mut self, as_of_block: u64) -> Self {
        self.last_pair_balance_backdoor_signal_to_as_of_chain_block_delta = self
            .last_pair_balance_backdoor_signal_block
            .map(|block| as_of_block.saturating_sub(block));
        self
    }
}
