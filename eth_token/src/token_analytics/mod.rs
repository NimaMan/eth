//! Token-pool analytics feature contracts.
//!
//! This module is named `token_analytics` because it belongs to the token
//! source crate, but its rows are pool-scoped: one observation stream per
//! `(token, pool)` pair.

pub mod features;
pub mod observation;

pub use features::{
    FeatureEvidenceBlocks, LpControlFeatures, PoolActivityFeatures, PoolLiquidityFeatures,
    PoolMarketFeatures, TokenAuthorityFeatures, TokenNetworkFeatures, TokenPoolAnalyticsFeatures,
    TokenPoolObservationFeatures, TokenStaticFeatures,
};
pub use observation::{
    ActiveObservationReason, ObservationBlockActivity, ObservationBlockActivitySource,
    ObservationBlockEventFlags, ObservationPoolTradingState, ObservationTransactionSummary,
    ObservationTransactionType, TokenPoolCurrentObservation, TokenPoolObservationContext,
    TokenPoolObservationKey, ACTIVE_OBSERVATION_TARGET_HORIZONS,
};
