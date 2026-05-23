//! Retention policy presets for token tracking runs.

pub mod live;
pub mod policy_types;

pub use live::{
    LivePoolDenomClass, LivePoolRetentionDecision, LiveTokenRetentionDecision,
    LiveTokenRetentionPolicy, LiveTokenRetentionReport, PoolDropReason, TokenDropReason,
    WETH_ADDRESS,
};
pub use policy_types::{
    ephemeral_terminal_scam_retention_policy, EPHEMERAL_TERMINAL_SCAM_RETENTION_MODE,
};
