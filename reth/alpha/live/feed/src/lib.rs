//! Live confirmed-chain feed pipeline contracts.
//!
//! This crate coordinates processed confirmed blocks, block-token updates,
//! optional live-state writes, and downstream live-feed events. The concrete
//! live token runtime consumes the canonical Redis live block stream while
//! keeping token/pool state in-process for readers such as the token server and
//! mempool risk code.

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
    LiveBlockLoad, LiveTokenError, LiveTokenEvent, LiveTokenPoolSnapshot, LiveTokenProgress,
    LiveTokenReader, LiveTokenRuntime, LiveTokenRuntimeConfig, LiveTokenSnapshot, LiveTokenState,
    LiveTokenStatus, RedisBlockStreamEvent, ResolvedLiveTokenRuntimeRequest,
    StartLiveTokenRuntimeRequest,
};
pub use source::ProcessedBlockSource;
