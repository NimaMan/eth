#[derive(Debug, thiserror::Error)]
pub enum LivePrioritySellPlannerError {
    #[error("unsupported order intent: {0}")]
    UnsupportedIntent(String),
    #[error("invalid planner input: {0}")]
    InvalidInput(String),
    #[error("route build failed: {0}")]
    Route(String),
    #[error("allowance check failed: {0}")]
    Allowance(String),
    #[error("pre-submit simulation failed: {0}")]
    Simulation(String),
    #[error("gas rank failed: {0}")]
    GasRank(String),
    #[error("tx prep rejected priority sell: {0}")]
    TxPrep(String),
}
