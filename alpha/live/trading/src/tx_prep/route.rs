use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PreparedSellRoute {
    pub protocol: String,
    pub router_address: String,
    pub calldata: String,
    pub value_wei: String,
    pub gas_limit: u64,
    pub estimated_gas_used: u64,
    #[serde(default)]
    pub max_slippage_bps: Option<u32>,
}

impl PreparedSellRoute {
    pub fn validate(&self) -> Result<(), TxPrepRouteError> {
        if self.router_address.trim().is_empty() {
            return Err(TxPrepRouteError::MissingRouterAddress);
        }
        if !self.calldata.trim().starts_with("0x") || self.calldata.trim().len() <= 2 {
            return Err(TxPrepRouteError::MissingCalldata);
        }
        if self.gas_limit == 0 {
            return Err(TxPrepRouteError::MissingGasLimit);
        }
        if self.estimated_gas_used == 0 {
            return Err(TxPrepRouteError::MissingEstimatedGasUsed);
        }
        if self.estimated_gas_used > self.gas_limit {
            return Err(TxPrepRouteError::EstimatedGasExceedsLimit);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, thiserror::Error, Eq, PartialEq, Serialize, Deserialize)]
pub enum TxPrepRouteError {
    #[error("prepared route is missing router address")]
    MissingRouterAddress,
    #[error("prepared route is missing calldata")]
    MissingCalldata,
    #[error("prepared route gas_limit must be greater than zero")]
    MissingGasLimit,
    #[error("prepared route estimated_gas_used must be greater than zero")]
    MissingEstimatedGasUsed,
    #[error("prepared route estimated_gas_used exceeds gas_limit")]
    EstimatedGasExceedsLimit,
}
