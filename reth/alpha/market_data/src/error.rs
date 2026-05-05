use eth_live_state::LiveStateError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, MarketDataError>;

#[derive(Debug, Error)]
pub enum MarketDataError {
    #[error(transparent)]
    LiveState(#[from] LiveStateError),
    #[error("market-data source error: {0}")]
    Source(String),
    #[error("block token processor error: {0}")]
    BlockToken(String),
    #[error("market-data event sink error: {0}")]
    Sink(String),
}
