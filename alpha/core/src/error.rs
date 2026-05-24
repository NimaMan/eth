use thiserror::Error;

use crate::ids::BlockNumber;

pub type Result<T> = std::result::Result<T, AlphaCoreError>;

#[derive(Debug, Error)]
pub enum AlphaCoreError {
    #[error("invalid position transition: {0}")]
    InvalidPositionTransition(String),

    #[error("portfolio limit rejected order: {0}")]
    PortfolioLimit(String),

    #[error("strategy error: {0}")]
    Strategy(String),

    #[error("execution adapter error: {0}")]
    Execution(String),

    #[error("execution cancelled: {reason}")]
    ExecutionCancelled {
        reason: String,
        block_number: Option<BlockNumber>,
    },

    #[error("execution deferred: {reason}")]
    ExecutionDeferred {
        reason: String,
        block_number: Option<BlockNumber>,
    },

    #[error("invalid pool address: {0}")]
    InvalidPoolAddress(String),

    #[error("store error: {0}")]
    Store(String),
}
