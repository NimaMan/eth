//! Confirmed-chain market-data pipeline contracts.
//!
//! This crate does not own Redis or mempool simulation. It coordinates
//! processed confirmed blocks, block-token updates, optional live-state writes,
//! and downstream market-data events.

pub mod block_token;
pub mod error;
pub mod event;
pub mod pipeline;
pub mod source;

pub use block_token::{BlockTokenProcessor, BlockTokenUpdate};
pub use error::{MarketDataError, Result};
pub use event::{
    BlockProcessedEvent, MarketDataEvent, MarketDataEventSink, NoopMarketDataEventSink,
    RecordingMarketDataEventSink,
};
pub use pipeline::{MarketBlockInput, MarketDataPipeline};
pub use source::ProcessedBlockSource;
