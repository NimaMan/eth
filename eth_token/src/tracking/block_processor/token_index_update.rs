use crate::tracking::{
    normalize_address, LiveTokenRetentionPolicy, LiveTokenRetentionReport, TrackedTokenIndexUpdate,
    TrackedTokenStatus,
};

use super::processor::BlockTokenProcessor;

impl BlockTokenProcessor {
    pub(in crate::tracking::block_processor) fn refresh_token_index(
        &mut self,
        token_address: &str,
        current_block: u64,
    ) {
        let Some(status) = self.registry.token(token_address).map(|token| {
            if token.is_scam() {
                TrackedTokenStatus::InactiveHiddenMint
            } else if token.trading_enabled() {
                TrackedTokenStatus::Active
            } else {
                TrackedTokenStatus::Creation
            }
        }) else {
            self.network_graphs
                .remove(&normalize_address(token_address));
            return;
        };
        self.index_registry_token(token_address, status, current_block);
    }

    pub(in crate::tracking::block_processor) fn index_registry_token(
        &mut self,
        token_address: &str,
        status: TrackedTokenStatus,
        current_block: u64,
    ) -> TrackedTokenIndexUpdate {
        let update = self.token_index.index_registry_token(
            &mut self.registry,
            token_address,
            status,
            current_block,
        );
        self.cleanup_network_graphs_for_index_update(&update);
        update
    }
    pub fn apply_retention_policy(
        &mut self,
        policy: &LiveTokenRetentionPolicy,
        current_block: u64,
    ) -> LiveTokenRetentionReport {
        let report =
            self.token_index
                .apply_retention_policy(&mut self.registry, policy, current_block);
        self.cleanup_network_graphs_to_registry();
        report
    }

    pub fn apply_index_retention_policy(
        &mut self,
        current_block: u64,
    ) -> Option<LiveTokenRetentionReport> {
        let report = self
            .token_index
            .apply_live_retention_policy(&mut self.registry, current_block)?;
        self.cleanup_network_graphs_to_registry();
        Some(report)
    }
}
