use thiserror::Error;

#[derive(Debug, Error)]
pub enum EnvError {
    #[error("data feed unavailable: {0}")]
    Data(String),
    #[error("simulation error: {0}")]
    Simulation(String),
    #[error(transparent)]
    Other(#[from] eyre::Report),
}

impl EnvError {
    pub fn data(msg: impl Into<String>) -> Self {
        EnvError::Data(msg.into())
    }
}
