#[derive(Clone, Debug)]
pub struct CalibrationRunConfig {
    pub eth_tx_executor_base_url: String,
    pub bearer_token: String,
    pub require_dry_run: bool,
    pub fetch_policy_journal: bool,
    pub refresh_simulation_block: bool,
}

impl CalibrationRunConfig {
    pub fn new(
        eth_tx_executor_base_url: impl Into<String>,
        bearer_token: impl Into<String>,
    ) -> Self {
        Self {
            eth_tx_executor_base_url: eth_tx_executor_base_url.into(),
            bearer_token: bearer_token.into(),
            require_dry_run: true,
            fetch_policy_journal: true,
            refresh_simulation_block: false,
        }
    }
}
