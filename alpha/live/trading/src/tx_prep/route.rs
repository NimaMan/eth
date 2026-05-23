use serde::{Deserialize, Serialize};

use super::GasEstimateConfig;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PreparedSellRoute {
    pub protocol: String,
    pub router_address: String,
    pub calldata: String,
    pub value_wei: String,
    pub gas_limit: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_gas_used: Option<u64>,
    #[serde(default)]
    pub max_slippage_bps: Option<u32>,
}

impl PreparedSellRoute {
    pub fn apply_simulated_gas_used(
        &mut self,
        simulated_gas_used: u64,
        config: &GasEstimateConfig,
    ) -> Result<(), TxPrepRouteError> {
        self.estimated_gas_used =
            Some(self.simulation_buffered_gas_used(simulated_gas_used, config)?);
        Ok(())
    }

    pub fn require_estimated_gas_used(&self) -> Result<u64, TxPrepRouteError> {
        match self.estimated_gas_used {
            Some(estimated_gas_used) if estimated_gas_used > 0 => {
                if estimated_gas_used > self.gas_limit {
                    return Err(TxPrepRouteError::EstimatedGasExceedsLimit);
                }
                Ok(estimated_gas_used)
            }
            _ => Err(TxPrepRouteError::MissingEstimatedGasUsed),
        }
    }

    pub fn simulation_buffered_gas_used(
        &self,
        simulated_gas_used: u64,
        config: &GasEstimateConfig,
    ) -> Result<u64, TxPrepRouteError> {
        if simulated_gas_used == 0 {
            return Err(TxPrepRouteError::MissingEstimatedGasUsed);
        }
        if simulated_gas_used > self.gas_limit {
            return Err(TxPrepRouteError::EstimatedGasExceedsLimit);
        }
        Ok(
            add_bps_ceil(simulated_gas_used, config.simulated_gas_estimate_buffer_bps)
                .min(self.gas_limit),
        )
    }

    pub fn with_simulated_gas_used(
        mut self,
        simulated_gas_used: u64,
        config: &GasEstimateConfig,
    ) -> Result<Self, TxPrepRouteError> {
        let estimated_gas_used = self.simulation_buffered_gas_used(simulated_gas_used, config)?;
        self.estimated_gas_used = Some(estimated_gas_used);
        Ok(self)
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
        self.require_estimated_gas_used()?;
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

    fn route(estimated_gas_used: Option<u64>, gas_limit: u64) -> PreparedSellRoute {
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
    fn simulated_gas_used_sets_route_estimate_with_buffer() {
        let mut route = route(None, 300_000);
        let config = GasEstimateConfig::default();

        route.apply_simulated_gas_used(182_564, &config).unwrap();

        assert_eq!(route.estimated_gas_used, Some(228_205));
    }

    #[test]
    fn simulated_gas_used_replaces_existing_estimate() {
        let mut route = route(Some(180_000), 300_000);
        let config = GasEstimateConfig::default();

        route.apply_simulated_gas_used(120_000, &config).unwrap();

        assert_eq!(route.estimated_gas_used, Some(150_000));
    }
}
