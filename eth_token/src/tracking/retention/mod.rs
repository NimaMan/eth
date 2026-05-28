//! Retention policy presets for token tracking runs.

pub mod live;
pub mod policy_types;

pub use live::{
    LivePoolDenomClass, LivePoolRetentionDecision, LiveTokenRetentionDecision,
    LiveTokenRetentionPolicy, LiveTokenRetentionReport, PoolDropReason, TokenDropReason,
    WETH_ADDRESS,
};
pub use policy_types::{
    terminal_or_idle_50k_retention_policy, terminal_scam_immediate_retention_policy,
    TERMINAL_OR_IDLE_50K_RETENTION_BLOCKS, TERMINAL_OR_IDLE_50K_RETENTION_MODE,
    TERMINAL_SCAM_IMMEDIATE_RETENTION_MODE,
};
