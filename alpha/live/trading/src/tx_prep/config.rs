use serde::{Deserialize, Serialize};

pub const DEFAULT_SIMULATED_GAS_ESTIMATE_BUFFER_BPS: u64 = 2_500;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GasEstimateConfig {
    pub simulated_gas_estimate_buffer_bps: u64,
}

impl Default for GasEstimateConfig {
    fn default() -> Self {
        Self {
            simulated_gas_estimate_buffer_bps: DEFAULT_SIMULATED_GAS_ESTIMATE_BUFFER_BPS,
        }
    }
}
