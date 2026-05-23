pub mod live_block_processor;

pub use live_block_processor::{LiveBlockProcessor, LiveBlockProcessorConfig, LiveProcessedBlock};

pub type LiveStateDiffFrame = alloy_rpc_types_trace::geth::PreStateFrame;
