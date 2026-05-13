//! Live confirmed-chain feed pipeline contracts.
//!
//! This crate coordinates processed confirmed blocks, block-token updates, and
//! downstream live-feed events. The concrete live token runtime warms from
//! historical processed blocks and then consumes direct in-process
//! `LiveBlockUpdate`s from the chain server while keeping token/pool state
//! in-process for HTTP readers, mempool risk code, and alpha consumers.

pub mod block_token;
pub mod error;
pub mod event;
pub mod pipeline;
pub mod runtime;
pub mod source;

pub use block_token::{BlockTokenProcessor, BlockTokenUpdate};
pub use error::{LiveFeedError, Result};
pub use event::{
    BlockProcessedEvent, LiveFeedEvent, LiveFeedEventSink, NoopLiveFeedEventSink,
    RecordingLiveFeedEventSink,
};
pub use pipeline::{LiveFeedBlockInput, LiveFeedPipeline};
pub use runtime::{
    LiveBlockLoad, LiveBlockUpdate, LiveTokenError, LiveTokenEvent, LiveTokenPoolSnapshot,
    LiveTokenProgress, LiveTokenReader, LiveTokenRuntime, LiveTokenRuntimeConfig,
    LiveTokenSnapshot, LiveTokenState, LiveTokenStatus, ResolvedLiveTokenRuntimeRequest,
    StartLiveTokenRuntimeRequest,
};
pub use source::ProcessedBlockSource;
