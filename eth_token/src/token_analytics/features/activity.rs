use serde::{Deserialize, Serialize};

use super::sell_flow::ObservedSellTransferFlow;
use super::utils::{finite_non_negative, ratio_if_positive, signed_volume_imbalance};
use crate::token_analytics::observation::ObservationBlockActivity;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PoolActivityFeatures {
    pub block_tx_count: u32,
    pub block_token_transfer_count: u32,
    pub block_token_transfer_volume: f64,
    pub block_token_transfer_to_total_supply_ratio: Option<f64>,
    pub block_token_transfer_to_pool_token_reserve_ratio: Option<f64>,
    pub block_observed_sell_tx_count: u32,
    pub block_sell_seller_token_to_pool_ratio: Option<f64>,
    pub block_sell_seller_token_to_token_contract_ratio: Option<f64>,
    pub block_sell_seller_token_to_other_ratio: Option<f64>,
    pub block_sell_token_contract_to_pool_reserve_ratio: Option<f64>,
    pub block_sell_token_contract_to_pool_seller_out_ratio: Option<f64>,
    pub block_denom_transfer_count: u32,
    pub block_buy_volume_denom: f64,
    pub block_sell_volume_denom: f64,
    pub block_buy_sell_volume_imbalance: Option<f64>,
    pub block_total_bribe_eth: f64,
    pub cumulative_active_observation_count: u64,
    pub cumulative_tx_count: u64,
    pub cumulative_token_transfer_count: u64,
    pub cumulative_denom_transfer_count: u64,
    pub cumulative_buy_volume_denom: f64,
    pub cumulative_sell_volume_denom: f64,
    pub cumulative_total_bribe_eth: f64,
    pub first_activity_block: Option<u64>,
    pub last_activity_block: Option<u64>,
    pub blocks_since_last_activity: Option<u64>,
    pub activity_density: Option<f64>,
    pub tx_per_active_observation: Option<f64>,
    pub net_buy_volume_denom: f64,
    pub buy_sell_volume_ratio: Option<f64>,
    pub denom_token_transfer_ratio: Option<f64>,
    pub active_observations_last_10: Option<u32>,
    pub tx_count_last_10: Option<u64>,
    pub active_observations_last_50: Option<u32>,
    pub tx_count_last_50: Option<u64>,
    pub active_observations_last_100: Option<u32>,
    pub tx_count_last_100: Option<u64>,
    pub active_density_last_10: Option<f64>,
    pub active_density_last_50: Option<f64>,
    pub active_density_last_100: Option<f64>,
    pub tx_share_last_10_to_total: Option<f64>,
    pub tx_share_last_50_to_total: Option<f64>,
    pub tx_share_last_100_to_total: Option<f64>,
    pub feature_scope: Option<String>,
}

impl PoolActivityFeatures {
    pub fn set_block_volume(&mut self, buy_volume_denom: f64, sell_volume_denom: f64) {
        self.block_buy_volume_denom = finite_non_negative(buy_volume_denom);
        self.block_sell_volume_denom = finite_non_negative(sell_volume_denom);
        self.block_buy_sell_volume_imbalance =
            signed_volume_imbalance(self.block_buy_volume_denom, self.block_sell_volume_denom);
    }

    pub fn set_token_transfer_volume_context(
        &mut self,
        token_transfer_volume: f64,
        total_supply_scaled: Option<f64>,
        pool_token_reserve: Option<f64>,
    ) {
        self.block_token_transfer_volume = finite_non_negative(token_transfer_volume);
        self.block_token_transfer_to_total_supply_ratio =
            ratio_if_positive(self.block_token_transfer_volume, total_supply_scaled);
        self.block_token_transfer_to_pool_token_reserve_ratio =
            ratio_if_positive(self.block_token_transfer_volume, pool_token_reserve);
    }

    pub fn set_observed_sell_transfer_flow(&mut self, flow: ObservedSellTransferFlow) {
        let seller_token_out = finite_non_negative(flow.seller_token_out);
        let seller_token_to_pool = finite_non_negative(flow.seller_token_to_pool);
        let seller_token_to_token_contract =
            finite_non_negative(flow.seller_token_to_token_contract);
        let seller_token_to_other = finite_non_negative(flow.seller_token_to_other);
        let token_contract_to_pool = finite_non_negative(flow.token_contract_to_pool);

        self.block_observed_sell_tx_count = flow.observed_sell_tx_count;
        self.block_sell_seller_token_to_pool_ratio =
            ratio_if_positive(seller_token_to_pool, Some(seller_token_out));
        self.block_sell_seller_token_to_token_contract_ratio =
            ratio_if_positive(seller_token_to_token_contract, Some(seller_token_out));
        self.block_sell_seller_token_to_other_ratio =
            ratio_if_positive(seller_token_to_other, Some(seller_token_out));
        self.block_sell_token_contract_to_pool_reserve_ratio =
            ratio_if_positive(token_contract_to_pool, flow.pool_token_reserve);
        self.block_sell_token_contract_to_pool_seller_out_ratio =
            ratio_if_positive(token_contract_to_pool, Some(seller_token_out));
    }
}

impl From<&ObservationBlockActivity> for PoolActivityFeatures {
    fn from(activity: &ObservationBlockActivity) -> Self {
        Self {
            block_tx_count: activity.tx_count,
            block_token_transfer_count: activity.token_transfer_count,
            block_token_transfer_volume: finite_non_negative(activity.token_transfer_volume),
            block_denom_transfer_count: activity.denom_transfer_count,
            block_total_bribe_eth: activity.total_bribe_eth,
            ..Default::default()
        }
    }
}
