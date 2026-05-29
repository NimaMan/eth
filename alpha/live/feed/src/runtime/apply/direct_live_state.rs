use alloy_primitives::B256;
use eyre::Result;
use tx_processor::{
    sealed_header_from_processed_block_header, BlockStateSession, LivePoolBuySellSimulator,
    LiveStateDiffFrame, LoadedProcessedBlock as LiveBlockLoad,
};
use tx_simulator::LiveBlockState;

use super::service::{LiveTokenRuntime, LIVE_TOKEN_TRACKER_LOG_TARGET};

impl LiveTokenRuntime {
    pub(super) async fn build_direct_live_block_session(
        &self,
        loaded: &LiveBlockLoad,
        pool_simulator: &LivePoolBuySellSimulator,
        state_diffs: &[LiveStateDiffFrame],
    ) -> Result<BlockStateSession> {
        let block_number = loaded.block.header.number;
        let block_hash = loaded.block.header.hash;
        let parent_hash = loaded.block.header.parent_hash;
        let block_header = sealed_header_from_processed_block_header(&loaded.block.header);
        let simulator = pool_simulator.simulator();
        let parent_session = {
            let sessions = self.inner.direct_live_block_sessions.lock().await;
            block_number
                .checked_sub(1)
                .and_then(|parent| sessions.get(&parent).cloned())
        };

        if let Some(parent_session) = parent_session {
            let cached_parent_hash = parent_session.block_hash();
            if cached_parent_hash != parent_hash {
                self.prune_stale_direct_live_state(
                    block_number,
                    block_hash,
                    parent_hash,
                    cached_parent_hash,
                )
                .await;
            } else {
                return simulator
                    .block_state_session_from_parent_prestate_diffs(
                        &parent_session,
                        block_number,
                        block_hash,
                        parent_hash,
                        block_header,
                        state_diffs,
                    )
                    .await;
            }
        }

        simulator
            .block_state_session_from_prestate_diffs(
                block_number,
                block_hash,
                parent_hash,
                block_header,
                state_diffs,
            )
            .await
    }

    async fn prune_stale_direct_live_state(
        &self,
        block_number: u64,
        block_hash: B256,
        expected_parent_hash: B256,
        cached_parent_hash: B256,
    ) {
        let removed_sessions = {
            let mut sessions = self.inner.direct_live_block_sessions.lock().await;
            let removed_sessions = sessions.len();
            sessions.clear();
            removed_sessions
        };
        if let Err(error) = self.inner.live_tx_simulator.provider().clear() {
            tracing::error!(
                target: LIVE_TOKEN_TRACKER_LOG_TARGET,
                block_number,
                block_hash = %block_hash,
                expected_parent_hash = %expected_parent_hash,
                cached_parent_hash = %cached_parent_hash,
                removed_local_sessions = removed_sessions,
                simulator_state_consistent = false,
                error = %error,
                "failed to clear LiveTxSimulator ring buffer after parent hash mismatch; stale states may remain in simulator until evicted by newer blocks"
            );
            return;
        }
        tracing::warn!(
            target: LIVE_TOKEN_TRACKER_LOG_TARGET,
            block_number,
            block_hash = %block_hash,
            expected_parent_hash = %expected_parent_hash,
            cached_parent_hash = %cached_parent_hash,
            removed_sessions,
            "cleared stale direct live state after parent hash mismatch; rebuilding from exact Reth parent state"
        );
    }

    pub(super) async fn remember_direct_live_block_session(
        &self,
        block_number: u64,
        block_hash: B256,
        session: BlockStateSession,
    ) {
        let mut sessions = self.inner.direct_live_block_sessions.lock().await;
        sessions.insert(block_number, session.clone());
        while sessions.len() > 16 {
            let Some(oldest) = sessions.keys().next().copied() else {
                break;
            };
            sessions.remove(&oldest);
        }
        if let Err(error) = self
            .inner
            .live_tx_simulator
            .provider()
            .publish_latest(LiveBlockState::new(session).with_block_hash(block_hash))
        {
            tracing::warn!(
                target: LIVE_TOKEN_TRACKER_LOG_TARGET,
                block_number,
                error = %error,
                "failed to publish direct live block session into LiveTxSimulator"
            );
        }
    }
}
