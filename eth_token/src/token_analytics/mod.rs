//! Token-pool analytics feature contracts.
//!
//! This module is named `token_analytics` because it belongs to the token
//! source crate, but its rows are pool-scoped: one observation stream per
//! `(token, pool)` pair.

pub mod features;
pub mod observation;

pub use features::{
    FeatureEvidenceBlocks, LpControlFeatures, ObservedSellTransferFlow, PoolActivityFeatures,
    PoolLiquidityFeatures, PoolMarketFeatures, TokenAuthorityFeatures, TokenNetworkFeatures,
    TokenPoolAnalyticsFeatures, TokenPoolObservationFeatures, TokenStaticFeatures,
};
pub use observation::{
    build_current_observation, build_historical_observations_for_pool,
    build_historical_observations_for_token, collect_current_observations, observation_pool_key,
    ActiveObservationReason, ObservationBlockAction, ObservationBlockActivity,
    ObservationBlockActivitySource, ObservationBlockEventFlags, ObservationPoolTradingState,
    ObservationSellFlow, ObservationTokenPoolMovement, ObservationTransactionClassification,
    ObservationTransactionSummary, ObservationTransactionType, ObservationTransferSummary,
    TokenPoolCurrentObservation, TokenPoolObservationContext, TokenPoolObservationKey,
    ACTIVE_OBSERVATION_TARGET_HORIZONS,
};
