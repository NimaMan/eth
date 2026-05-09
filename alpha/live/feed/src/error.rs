use eth_live_state::LiveStateError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, LiveFeedError>;

#[derive(Debug, Error)]
pub enum LiveFeedError {
    #[error(transparent)]
    LiveState(#[from] LiveStateError),
    #[error("live-feed source error: {0}")]
    Source(String),
    #[error("block token processor error: {0}")]
    BlockToken(String),
    #[error("live-feed event sink error: {0}")]
    Sink(String),
}
