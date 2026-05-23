use super::live::LiveTokenRetentionPolicy;

pub const EPHEMERAL_TERMINAL_SCAM_RETENTION_MODE: &str = "ephemeral_terminal_scam";

/// Run-only retention for terminal scam outcomes.
///
/// This keeps active/non-terminal pools available for observation building, but
/// drops a token as soon as scam evidence is observed and the current block's
/// observations have been collected.
pub fn ephemeral_terminal_scam_retention_policy() -> LiveTokenRetentionPolicy {
    LiveTokenRetentionPolicy {
        drop_scam_tokens_immediately: true,
        retain_liquidity_removal_pools_for_blocks: Some(0),
        ..LiveTokenRetentionPolicy::default()
    }
}
