use thiserror::Error;

pub type Result<T> = std::result::Result<T, EthTxExecutorError>;

#[derive(Debug, Error)]
pub enum EthTxExecutorError {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("signer error: {0}")]
    Signer(String),
    #[error("nonce error: {0}")]
    Nonce(String),
    #[error("broadcast error: {0}")]
    Broadcast(String),
    #[error("rpc error: {0}")]
    Rpc(String),
    #[error("position unavailable: {0}")]
    PositionUnavailable(String),
    #[error("repository error: {0}")]
    Repository(String),
    #[error("serialization error: {0}")]
    Serialization(String),
}

impl From<ethers_providers::ProviderError> for EthTxExecutorError {
    fn from(value: ethers_providers::ProviderError) -> Self {
        Self::Rpc(value.to_string())
    }
}

impl From<ethers_signers::WalletError> for EthTxExecutorError {
    fn from(value: ethers_signers::WalletError) -> Self {
        Self::Signer(value.to_string())
    }
}

impl From<std::io::Error> for EthTxExecutorError {
    fn from(value: std::io::Error) -> Self {
        Self::Repository(value.to_string())
    }
}

impl From<serde_json::Error> for EthTxExecutorError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serialization(value.to_string())
    }
}
