use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PreSubmitSimulation {
    pub block_number: u64,
    pub block_hash: Option<String>,
    pub state_root: Option<String>,
    pub expected_output_token: Option<String>,
    pub expected_output_amount: Option<String>,
    pub min_output_amount: Option<String>,
    pub expected_recovery_eth: eth_alpha_core::amount::DecimalAmount,
    #[serde(default)]
    pub gas_used: Option<u64>,
    pub would_revert: bool,
    #[serde(default)]
    pub metadata: Value,
}

impl PreSubmitSimulation {
    pub fn validate(&self) -> Result<(), TxPrepSimulationError> {
        if self.would_revert {
            return Err(TxPrepSimulationError::WouldRevert);
        }
        if self.expected_recovery_eth <= eth_alpha_core::amount::DecimalAmount::ZERO {
            return Err(TxPrepSimulationError::NoExpectedRecovery);
        }
        if self.gas_used.is_none() {
            return Err(TxPrepSimulationError::MissingGasUsed);
        }
        Ok(())
    }

    pub fn metadata(&self) -> Value {
        json!({
            "expected_recovery_eth": self.expected_recovery_eth,
            "would_revert": self.would_revert,
            "gas_used": self.gas_used,
            "extra": self.metadata,
        })
    }
}

#[derive(Clone, Debug, thiserror::Error, Eq, PartialEq, Serialize, Deserialize)]
pub enum TxPrepSimulationError {
    #[error("pre-submit simulation indicates the sell would revert")]
    WouldRevert,
    #[error("pre-submit simulation has no expected recovery")]
    NoExpectedRecovery,
    #[error("pre-submit simulation has no gas_used; fallback gas estimates are not allowed")]
    MissingGasUsed,
}
