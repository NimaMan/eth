use super::live::LiveTokenRetentionPolicy;

pub const TERMINAL_SCAM_IMMEDIATE_RETENTION_MODE: &str = "terminal_scam_immediate";
pub const TERMINAL_OR_IDLE_50K_RETENTION_MODE: &str = "terminal_or_idle_50k";
pub const TERMINAL_OR_IDLE_50K_RETENTION_BLOCKS: u64 = 50_000;

/// Run-only retention for terminal scam outcomes.
///
/// This keeps active/non-terminal pools available for observation building, but
/// drops a token as soon as scam evidence is observed and the current block's
/// observations have been collected.
pub fn terminal_scam_immediate_retention_policy() -> LiveTokenRetentionPolicy {
    LiveTokenRetentionPolicy {
        drop_scam_tokens_immediately: true,
        retain_liquidity_removal_pools_for_blocks: Some(0),
        ..LiveTokenRetentionPolicy::default()
    }
}

/// Historical PnL retention that keeps enough post-terminal and idle behavior
/// for aggregate snapshots before dropping state from the token cache.
pub fn terminal_or_idle_50k_retention_policy() -> LiveTokenRetentionPolicy {
    LiveTokenRetentionPolicy {
        retain_terminal_scam_tokens_for_blocks: Some(TERMINAL_OR_IDLE_50K_RETENTION_BLOCKS),
        drop_tokens_after_inactivity_blocks: Some(TERMINAL_OR_IDLE_50K_RETENTION_BLOCKS),
        drop_tokens_without_pools_after_blocks: Some(TERMINAL_OR_IDLE_50K_RETENTION_BLOCKS),
        drop_tokens_without_retained_pools_after_blocks: Some(
            TERMINAL_OR_IDLE_50K_RETENTION_BLOCKS,
        ),
        retain_liquidity_removal_pools_for_blocks: Some(TERMINAL_OR_IDLE_50K_RETENTION_BLOCKS),
        ..LiveTokenRetentionPolicy::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_or_idle_50k_sets_all_delayed_drop_windows() {
        let policy = terminal_or_idle_50k_retention_policy();

        assert_eq!(
            policy.retain_terminal_scam_tokens_for_blocks,
            Some(TERMINAL_OR_IDLE_50K_RETENTION_BLOCKS)
        );
        assert_eq!(
            policy.drop_tokens_after_inactivity_blocks,
            Some(TERMINAL_OR_IDLE_50K_RETENTION_BLOCKS)
        );
        assert_eq!(
            policy.retain_liquidity_removal_pools_for_blocks,
            Some(TERMINAL_OR_IDLE_50K_RETENTION_BLOCKS)
        );
        assert!(!policy.drop_scam_tokens_immediately);
    }
}
