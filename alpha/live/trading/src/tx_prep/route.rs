use serde::{Deserialize, Serialize};

pub const DEFAULT_SIMULATED_GAS_ESTIMATE_BUFFER_BPS: u64 = 500;

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
    pub fn apply_simulated_gas_used(
        &mut self,
        simulated_gas_used: u64,
    ) -> Result<(), TxPrepRouteError> {
        if simulated_gas_used == 0 {
            return Err(TxPrepRouteError::MissingEstimatedGasUsed);
        }
        if simulated_gas_used > self.gas_limit {
            return Err(TxPrepRouteError::EstimatedGasExceedsLimit);
        }

        let buffered = add_bps_ceil(
            simulated_gas_used,
            DEFAULT_SIMULATED_GAS_ESTIMATE_BUFFER_BPS,
        )
        .min(self.gas_limit);
        self.estimated_gas_used = self.estimated_gas_used.max(buffered);
        Ok(())
    }

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

fn add_bps_ceil(value: u64, bps: u64) -> u64 {
    let increment = (u128::from(value) * u128::from(bps)).div_ceil(10_000);
    let total = u128::from(value).saturating_add(increment);
    total.min(u128::from(u64::MAX)) as u64
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

#[cfg(test)]
mod tests {
    use super::*;

    fn route(estimated_gas_used: u64, gas_limit: u64) -> PreparedSellRoute {
        PreparedSellRoute {
            protocol: "uniswap_v2_trading_vault".to_string(),
            router_address: "0x0000000000000000000000000000000000000001".to_string(),
            calldata: "0x1234".to_string(),
            value_wei: "0".to_string(),
            gas_limit,
            estimated_gas_used,
            max_slippage_bps: Some(500),
        }
    }

    #[test]
    fn simulated_gas_used_raises_route_estimate_with_buffer() {
        let mut route = route(130_000, 300_000);

        route.apply_simulated_gas_used(182_564).unwrap();

        assert_eq!(route.estimated_gas_used, 191_693);
    }

    #[test]
    fn simulated_gas_used_does_not_lower_existing_estimate() {
        let mut route = route(180_000, 300_000);

        route.apply_simulated_gas_used(120_000).unwrap();

        assert_eq!(route.estimated_gas_used, 180_000);
    }
}
