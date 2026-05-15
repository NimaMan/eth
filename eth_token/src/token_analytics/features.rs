use serde::{Deserialize, Serialize};

use crate::pools::PoolLifecycle;

use super::observation::{
    ObservationBlockActivity, TokenPoolObservationContext, TokenPoolObservationKey,
};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TokenPoolAnalyticsFeatures {
    pub key: TokenPoolObservationKey,
    pub observation: TokenPoolObservationContext,
    pub token: TokenStaticFeatures,
    pub authority: TokenAuthorityFeatures,
    pub market: PoolMarketFeatures,
    pub liquidity: PoolLiquidityFeatures,
    pub lp_control: LpControlFeatures,
    pub activity: PoolActivityFeatures,
    pub network: TokenNetworkFeatures,
    pub evidence_blocks: FeatureEvidenceBlocks,
}

impl TokenPoolAnalyticsFeatures {
    pub fn as_of_block(&self) -> u64 {
        self.observation.block_number
    }

    pub fn latest_evidence_block(&self) -> Option<u64> {
        self.evidence_blocks.latest()
    }

    pub fn has_future_evidence_leakage(&self) -> bool {
        self.latest_evidence_block()
            .is_some_and(|latest| latest > self.as_of_block())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TokenStaticFeatures {
    pub token_creation_block: Option<u64>,
    pub token_age_blocks: Option<u64>,
    pub pool_creation_after_token_blocks: Option<u64>,
    pub decimals: Option<u8>,
    pub total_supply_scaled: Option<f64>,
    pub total_supply_from_transfers: Option<f64>,
}

impl TokenStaticFeatures {
    pub fn from_blocks(
        token_creation_block: Option<u64>,
        pool_creation_block: Option<u64>,
        as_of_block: u64,
    ) -> Self {
        Self {
            token_creation_block,
            token_age_blocks: token_creation_block.map(|block| as_of_block.saturating_sub(block)),
            pool_creation_after_token_blocks: token_creation_block
                .zip(pool_creation_block)
                .map(|(token_block, pool_block)| pool_block.saturating_sub(token_block)),
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TokenAuthorityFeatures {
    pub creator_address: Option<String>,
    pub current_owner: Option<String>,
    pub current_owner_is_creator: Option<bool>,
    pub ownership_renounced: bool,
    pub renouncement_block: Option<u64>,
    pub control_address_count: Option<u32>,
    pub control_address_tx_count_in_block: Option<u32>,
}

impl TokenAuthorityFeatures {
    pub fn with_owner_context(
        creator_address: Option<String>,
        current_owner: Option<String>,
        ownership_renounced: bool,
    ) -> Self {
        let current_owner_is_creator = creator_address
            .as_deref()
            .zip(current_owner.as_deref())
            .map(|(creator, owner)| creator.eq_ignore_ascii_case(owner));

        Self {
            creator_address,
            current_owner,
            current_owner_is_creator,
            ownership_renounced,
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PoolMarketFeatures {
    pub pool_creation_block: Option<u64>,
    pub trading_enabled_block: Option<u64>,
    pub pool_age_blocks: Option<u64>,
    pub trading_age_blocks: Option<u64>,
    pub lifecycle: Option<PoolLifecycle>,
    pub can_buy: bool,
    pub can_sell: bool,
    pub effective_can_buy: bool,
    pub effective_can_sell: bool,
    pub buy_tax: Option<f64>,
    pub sell_tax: Option<f64>,
    pub first_observed_buy_block: Option<u64>,
    pub first_observed_sell_block: Option<u64>,
    pub first_failed_sell_block: Option<u64>,
    pub last_trading_failure_class: Option<String>,
}

impl PoolMarketFeatures {
    pub fn with_ages(
        pool_creation_block: Option<u64>,
        trading_enabled_block: Option<u64>,
        as_of_block: u64,
    ) -> Self {
        Self {
            pool_creation_block,
            trading_enabled_block,
            pool_age_blocks: pool_creation_block.map(|block| as_of_block.saturating_sub(block)),
            trading_age_blocks: trading_enabled_block
                .map(|block| as_of_block.saturating_sub(block)),
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PoolLiquidityFeatures {
    pub denom_reserve: f64,
    pub token_reserve: f64,
    pub total_liquidity_denom: f64,
    pub price_denom_per_token: f64,
    pub initial_price_denom_per_token: Option<f64>,
    pub price_to_initial_ratio: Option<f64>,
    pub initial_denom_reserve: Option<f64>,
    pub denom_reserve_to_initial_ratio: Option<f64>,
    pub initial_token_reserve: Option<f64>,
    pub token_reserve_to_initial_ratio: Option<f64>,
    pub max_denom_reserve_so_far: Option<f64>,
    pub denom_reserve_drawdown_from_max: Option<f64>,
    pub reserve_observation_count: u32,
}

impl PoolLiquidityFeatures {
    pub fn new(
        denom_reserve: f64,
        token_reserve: f64,
        total_liquidity_denom: f64,
        price_denom_per_token: f64,
    ) -> Self {
        Self {
            denom_reserve: finite_non_negative(denom_reserve),
            token_reserve: finite_non_negative(token_reserve),
            total_liquidity_denom: finite_non_negative(total_liquidity_denom),
            price_denom_per_token: finite_non_negative(price_denom_per_token),
            ..Default::default()
        }
    }

    pub fn with_initial_reserves(
        mut self,
        initial_denom_reserve: Option<f64>,
        initial_token_reserve: Option<f64>,
        initial_price_denom_per_token: Option<f64>,
    ) -> Self {
        self.initial_denom_reserve = valid_positive(initial_denom_reserve);
        self.initial_token_reserve = valid_positive(initial_token_reserve);
        self.initial_price_denom_per_token = valid_positive(initial_price_denom_per_token);
        self.denom_reserve_to_initial_ratio =
            ratio_to_initial(self.denom_reserve, self.initial_denom_reserve);
        self.token_reserve_to_initial_ratio =
            ratio_to_initial(self.token_reserve, self.initial_token_reserve);
        self.price_to_initial_ratio = ratio_to_initial(
            self.price_denom_per_token,
            self.initial_price_denom_per_token,
        );
        self
    }

    pub fn with_max_denom_reserve(mut self, max_denom_reserve_so_far: Option<f64>) -> Self {
        self.max_denom_reserve_so_far = valid_positive(max_denom_reserve_so_far);
        self.denom_reserve_drawdown_from_max = self.max_denom_reserve_so_far.and_then(|max| {
            if max > 0.0 {
                Some((max - self.denom_reserve).max(0.0) / max)
            } else {
                None
            }
        });
        self
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LpControlFeatures {
    pub lp_total_supply: Option<f64>,
    pub lp_holder_count: Option<u32>,
    pub lp_top_holder_share_pct: Option<f64>,
    pub lp_top_holder_is_creator: Option<bool>,
    pub lp_top_holder_is_owner: Option<bool>,
    pub lp_holders_with_approvals_count: Option<u32>,
    pub lp_approved_to_router: Option<f64>,
    pub lp_approved_to_router_pct: Option<f64>,
    pub last_lp_approval_block: Option<u64>,
    pub last_lp_approval_timestamp: Option<u64>,
    pub last_lp_approval_owner: Option<String>,
    pub last_lp_approval_spender: Option<String>,
    pub last_lp_approval_is_router: Option<bool>,
    pub blocks_from_pool_creation_to_last_lp_approval: Option<i64>,
    pub blocks_from_trading_enabled_to_last_lp_approval: Option<i64>,
}

impl LpControlFeatures {
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

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PoolActivityFeatures {
    pub block_tx_count: u32,
    pub block_token_transfer_count: u32,
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
}

impl PoolActivityFeatures {
    pub fn set_block_volume(&mut self, buy_volume_denom: f64, sell_volume_denom: f64) {
        self.block_buy_volume_denom = finite_non_negative(buy_volume_denom);
        self.block_sell_volume_denom = finite_non_negative(sell_volume_denom);
        self.block_buy_sell_volume_imbalance =
            signed_volume_imbalance(self.block_buy_volume_denom, self.block_sell_volume_denom);
    }
}

impl From<&ObservationBlockActivity> for PoolActivityFeatures {
    fn from(activity: &ObservationBlockActivity) -> Self {
        Self {
            block_tx_count: activity.tx_count,
            block_token_transfer_count: activity.token_transfer_count,
            block_denom_transfer_count: activity.denom_transfer_count,
            block_total_bribe_eth: activity.total_bribe_eth,
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TokenNetworkFeatures {
    pub unique_address_count: Option<u32>,
    pub new_address_count_in_block: Option<u32>,
    pub pool_recycling_transfer_count: Option<u32>,
    pub one_to_many_transfer_count: Option<u32>,
    pub many_to_one_transfer_count: Option<u32>,
    pub creator_centrality: Option<f64>,
    pub owner_centrality: Option<f64>,
    pub largest_non_protocol_cluster_size: Option<u32>,
    pub shared_non_protocol_funder_count: Option<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FeatureEvidenceBlocks {
    pub token_static_latest_block: Option<u64>,
    pub authority_latest_block: Option<u64>,
    pub market_latest_block: Option<u64>,
    pub liquidity_latest_block: Option<u64>,
    pub lp_control_latest_block: Option<u64>,
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
            self.activity_latest_block,
            self.network_latest_block,
        ]
        .into_iter()
        .flatten()
        .max()
    }
}

fn valid_positive(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite() && *value > 0.0)
}

fn finite_non_negative(value: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        0.0
    }
}

fn ratio_to_initial(current: f64, initial: Option<f64>) -> Option<f64> {
    let initial = initial?;
    if current.is_finite() && current >= 0.0 && initial > 0.0 {
        Some(current / initial)
    } else {
        None
    }
}

fn signed_block_delta(start: Option<u64>, end: Option<u64>) -> Option<i64> {
    Some(end? as i64 - start? as i64)
}

fn signed_volume_imbalance(buy: f64, sell: f64) -> Option<f64> {
    let total = buy + sell;
    if total > 0.0 && total.is_finite() {
        Some((buy - sell) / total)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn liquidity_features_compute_initial_ratios() {
        let features = PoolLiquidityFeatures::new(4.0, 200.0, 4.0, 0.02)
            .with_initial_reserves(Some(2.0), Some(100.0), Some(0.01))
            .with_max_denom_reserve(Some(8.0));

        assert_eq!(features.denom_reserve_to_initial_ratio, Some(2.0));
        assert_eq!(features.token_reserve_to_initial_ratio, Some(2.0));
        assert_eq!(features.price_to_initial_ratio, Some(2.0));
        assert_eq!(features.denom_reserve_drawdown_from_max, Some(0.5));
    }

    #[test]
    fn lp_approval_offsets_are_signed() {
        let features = LpControlFeatures {
            last_lp_approval_block: Some(95),
            ..Default::default()
        }
        .with_last_approval_offsets(Some(90), Some(100));

        assert_eq!(
            features.blocks_from_pool_creation_to_last_lp_approval,
            Some(5)
        );
        assert_eq!(
            features.blocks_from_trading_enabled_to_last_lp_approval,
            Some(-5)
        );
    }

    #[test]
    fn detects_future_evidence_leakage() {
        let features = TokenPoolAnalyticsFeatures {
            observation: TokenPoolObservationContext::new(7, 100, None),
            evidence_blocks: FeatureEvidenceBlocks {
                liquidity_latest_block: Some(99),
                network_latest_block: Some(101),
                ..Default::default()
            },
            ..Default::default()
        };

        assert_eq!(features.latest_evidence_block(), Some(101));
        assert!(features.has_future_evidence_leakage());
    }

    #[test]
    fn block_volume_imbalance_is_signed() {
        let mut features = PoolActivityFeatures::default();
        features.set_block_volume(3.0, 1.0);

        assert_eq!(features.block_buy_sell_volume_imbalance, Some(0.5));
    }
}
