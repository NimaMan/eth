use tx_processor::{
    LiveProcessedBlock, LiveStateDiffFrame, LoadedProcessedBlock as LiveBlockLoad,
    ProcessedBlockSource,
};

#[derive(Debug, Clone, Default)]
pub struct LiveBlockReplayWriteMetrics {
    pub disk_cache_write_ms: u128,
    pub address_index_failures: u64,
    pub last_address_index_error: Option<String>,
}

#[derive(Debug)]
pub struct LiveBlockUpdate {
    pub(super) loaded: LiveBlockLoad,
    pub(super) state_diffs: Option<Vec<LiveStateDiffFrame>>,
}

impl LiveBlockUpdate {
    pub fn from_live_processed_block(
        processed: LiveProcessedBlock,
        replay_write_metrics: LiveBlockReplayWriteMetrics,
    ) -> Self {
        let upstream_ms = processed
            .processed_at
            .signed_duration_since(processed.head_arrival)
            .num_milliseconds()
            .max(0) as u128;
        let state_diffs = processed.state_diffs;
        Self {
            loaded: LiveBlockLoad {
                block: processed.processed_block,
                upstream_ms,
                disk_cache_hit: false,
                disk_cache_read_ms: 0,
                disk_cache_write_ms: replay_write_metrics.disk_cache_write_ms,
                address_index_failures: replay_write_metrics.address_index_failures,
                last_address_index_error: replay_write_metrics.last_address_index_error,
                source: ProcessedBlockSource::LiveDirect.as_str(),
            },
            state_diffs,
        }
    }

    pub fn block_number(&self) -> u64 {
        self.loaded.block.header.number
    }
}
