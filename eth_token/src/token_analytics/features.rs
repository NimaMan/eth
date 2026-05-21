use serde::{Deserialize, Serialize};

use super::observation::{TokenPoolObservationContext, TokenPoolObservationKey};

pub mod activity;
pub mod authority;
pub mod evidence;
pub mod liquidity;
pub mod lp_control;
pub mod market;
pub mod network;
pub mod sell_flow;
pub mod token;
pub mod token_control;
pub mod utils;

pub use activity::PoolActivityFeatures;
pub use authority::TokenAuthorityFeatures;
pub use evidence::FeatureEvidenceBlocks;
pub use liquidity::PoolLiquidityFeatures;
pub use lp_control::LpControlFeatures;
pub use market::PoolMarketFeatures;
pub use network::TokenNetworkFeatures;
pub use sell_flow::ObservedSellTransferFlow;
pub use token::TokenStaticFeatures;
pub use token_control::TokenControlFeatures;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TokenPoolAnalyticsFeatures {
    pub key: TokenPoolObservationKey,
    pub observation: TokenPoolObservationContext,
    pub features: TokenPoolObservationFeatures,
}

impl TokenPoolAnalyticsFeatures {
    pub fn as_of_block(&self) -> u64 {
        self.observation.block_number
    }

    pub fn latest_evidence_block(&self) -> Option<u64> {
        self.features.latest_evidence_block()
    }

    pub fn has_future_evidence_leakage(&self) -> bool {
        self.features
            .has_future_evidence_leakage(self.as_of_block())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TokenPoolObservationFeatures {
    pub token: TokenStaticFeatures,
    pub authority: TokenAuthorityFeatures,
    pub market: PoolMarketFeatures,
    pub liquidity: PoolLiquidityFeatures,
    pub lp_control: LpControlFeatures,
    pub token_control: TokenControlFeatures,
    pub activity: PoolActivityFeatures,
    pub network: TokenNetworkFeatures,
    pub evidence_blocks: FeatureEvidenceBlocks,
}

impl TokenPoolObservationFeatures {
    pub fn latest_evidence_block(&self) -> Option<u64> {
        self.evidence_blocks.latest()
    }

    pub fn has_future_evidence_leakage(&self, as_of_block: u64) -> bool {
        self.latest_evidence_block()
            .is_some_and(|latest| latest > as_of_block)
    }
}

#[cfg(test)]
mod tests {
    use super::utils::ZERO_ADDRESS;
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
            features.pool_creation_to_last_lp_approval_chain_block_delta,
            Some(5)
        );
        assert_eq!(
            features.blocks_from_trading_enabled_to_last_lp_approval,
            Some(-5)
        );
        assert_eq!(
            features.trading_enabled_to_last_lp_approval_chain_block_delta,
            Some(-5)
        );
    }

    #[test]
    fn detects_future_evidence_leakage() {
        let features = TokenPoolAnalyticsFeatures {
            observation: TokenPoolObservationContext::new(7, 100, None),
            features: TokenPoolObservationFeatures {
                evidence_blocks: FeatureEvidenceBlocks {
                    liquidity_latest_block: Some(99),
                    network_latest_block: Some(101),
                    ..Default::default()
                },
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

    #[test]
    fn token_transfer_volume_ratios_use_supply_and_pool_reserve() {
        let mut features = PoolActivityFeatures::default();
        features.set_token_transfer_volume_context(10.0, Some(1_000.0), Some(200.0));

        assert_eq!(features.block_token_transfer_volume, 10.0);
        assert_eq!(
            features.block_token_transfer_to_total_supply_ratio,
            Some(0.01)
        );
        assert_eq!(
            features.block_token_transfer_to_pool_token_reserve_ratio,
            Some(0.05)
        );
    }

    #[test]
    fn observed_sell_transfer_flow_uses_ratios() {
        let mut features = PoolActivityFeatures::default();
        features.set_observed_sell_transfer_flow(ObservedSellTransferFlow {
            observed_sell_tx_count: 4,
            seller_token_out: 1_000.0,
            seller_token_to_pool: 10.0,
            seller_token_to_token_contract: 990.0,
            seller_token_to_other: 0.0,
            token_contract_to_pool: 2_000.0,
            pool_token_reserve: Some(10_000.0),
        });

        assert_eq!(features.block_observed_sell_tx_count, 4);
        assert_eq!(features.block_sell_seller_token_to_pool_ratio, Some(0.01));
        assert_eq!(
            features.block_sell_seller_token_to_token_contract_ratio,
            Some(0.99)
        );
        assert_eq!(
            features.block_sell_token_contract_to_pool_reserve_ratio,
            Some(0.2)
        );
        assert_eq!(
            features.block_sell_token_contract_to_pool_seller_out_ratio,
            Some(2.0)
        );
    }

    #[test]
    fn authority_features_detect_zero_owner() {
        let features = TokenAuthorityFeatures::with_owner_context(
            Some("0xabc".to_string()),
            Some(ZERO_ADDRESS.to_string()),
            true,
        );

        assert_eq!(features.current_owner_is_zero_address, Some(true));
        assert_eq!(features.current_owner_is_creator, Some(false));
    }

    #[test]
    fn lp_control_features_compute_as_of_offsets() {
        let features = LpControlFeatures {
            lp_first_approval_block_as_of: Some(90),
            last_lp_approval_block: Some(95),
            ..Default::default()
        }
        .with_as_of_offsets(100);

        assert_eq!(features.blocks_from_first_lp_approval_to_as_of, Some(10));
        assert_eq!(features.blocks_from_last_lp_approval_to_as_of, Some(5));
        assert_eq!(
            features.first_lp_approval_to_as_of_chain_block_delta,
            Some(10)
        );
        assert_eq!(
            features.last_lp_approval_to_as_of_chain_block_delta,
            Some(5)
        );
    }
}
