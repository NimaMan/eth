use thiserror::Error;

pub type Result<T> = std::result::Result<T, LiveStateError>;

#[derive(Debug, Error)]
pub enum LiveStateError {
    #[error("live-state encode error: {0}")]
    Encode(String),
    #[error("live-state decode error: {0}")]
    Decode(String),
    #[error("live-state store error: {0}")]
    Store(String),
}
