use tx_processor::{
    LiveProcessedBlock, LiveStateDiffFrame, LoadedProcessedBlock as LiveBlockLoad,
    ProcessedBlockSource,
};

#[derive(Debug)]
pub struct LiveBlockUpdate {
    pub(super) loaded: LiveBlockLoad,
    pub(super) state_diffs: Option<Vec<LiveStateDiffFrame>>,
}

impl LiveBlockUpdate {
    pub fn from_live_processed_block(
        processed: LiveProcessedBlock,
        disk_cache_write_ms: u128,
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
                disk_cache_write_ms,
                source: ProcessedBlockSource::LiveDirect.as_str(),
            },
            state_diffs,
        }
    }

    pub fn block_number(&self) -> u64 {
        self.loaded.block.header.number
    }
}
