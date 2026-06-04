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
    #[error("pre-submit simulation state not ready: selected_block={selected_block} required_block={required_block} current_block={current_block} latest_reth_finished_block={latest_reth_finished_block} latest_historical_context_block={latest_historical_context_block} source={state_source}")]
    SimulationStateNotReady {
        selected_block: u64,
        required_block: u64,
        current_block: u64,
        latest_reth_finished_block: u64,
        latest_historical_context_block: u64,
        state_source: String,
    },
    #[error("gas rank failed: {0}")]
    GasRank(String),
    #[error("tx prep rejected priority sell: {0}")]
    TxPrep(String),
}

impl LivePrioritySellPlannerError {
    pub const fn is_deferrable(&self) -> bool {
        matches!(self, Self::SimulationStateNotReady { .. })
    }

    pub const fn deferral_block_number(&self) -> Option<u64> {
        match self {
            Self::SimulationStateNotReady { current_block, .. } => Some(*current_block),
            _ => None,
        }
    }

    pub fn is_pre_broadcast_reject(&self) -> bool {
        match self {
            Self::Simulation(reason) => {
                let reason = reason.to_ascii_lowercase();
                reason.contains("would revert")
                    || reason.contains("no expected recovery")
                    || reason.contains("min-output is zero")
            }
            _ => false,
        }
    }
}
