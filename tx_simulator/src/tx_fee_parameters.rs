//! Transaction fee parameter validation and packaging.
//!
//! Callers must supply the gas limit they want to simulate. Fee fields are kept
//! when supplied, but unsigned simulation requests are normalized to the block
//! base-fee floor so replay helpers do not fail before reaching EVM execution.

use crate::types::FeeDefaults;
use eyre::{eyre, Result};

/// Transaction fee parameters that callers can override when building requests.
#[derive(Debug, Clone, Copy, Default)]
pub struct TxGasParameters {
    pub gas_limit: Option<u64>,
    pub gas_price: Option<u128>,
    pub max_fee_per_gas: Option<u128>,
    pub max_priority_fee_per_gas: Option<u128>,
    pub max_fee_per_blob_gas: Option<u128>,
}

/// Context required to validate fee parameters.
#[derive(Debug, Clone, Copy)]
pub struct TxFeeContext<'a> {
    pub fee_defaults: &'a FeeDefaults,
    pub block_gas_limit: u128,
    pub base_fee: Option<u128>,
}

/// Finalized gas parameters applied to the transaction environment.
#[derive(Debug, Clone)]
pub struct SimulationGasParameters {
    pub tx_type: GasTxType,
    pub gas_limit: u64,
    pub gas_price: u128,
    pub effective_gas_price: u128,
    pub max_fee_per_gas: Option<u128>,
    pub max_priority_fee_per_gas: Option<u128>,
    pub max_fee_per_blob_gas: Option<u128>,
}

impl SimulationGasParameters {
    pub fn legacy_gas_price(&self) -> Option<u128> {
        if matches!(self.tx_type, GasTxType::Legacy) {
            Some(self.gas_price)
        } else {
            None
        }
    }
}

/// Transaction type after gas resolution.
#[derive(Debug, Clone, Copy)]
pub enum GasTxType {
    Legacy,
    Eip1559,
}

impl GasTxType {
    pub fn as_reth_tx_type(self) -> u8 {
        match self {
            GasTxType::Legacy => 0,
            GasTxType::Eip1559 => 2,
        }
    }
}

pub(crate) fn effective_paid_gas_price_from_tx_env(
    tx_type: u8,
    gas_price: u128,
    gas_priority_fee: Option<u128>,
    base_fee: Option<u128>,
) -> Option<u128> {
    if gas_priority_fee.is_some() || matches!(tx_type, 2 | 3 | 4) {
        let base_fee = base_fee?;
        return Some(gas_price.min(base_fee.saturating_add(gas_priority_fee.unwrap_or(0))));
    }

    Some(match base_fee {
        Some(base_fee) => gas_price.max(base_fee),
        None => gas_price,
    })
}

/// Input parameters provided by the caller / unsigned transaction.
#[derive(Debug, Clone, Copy)]
pub struct GasInputs {
    pub gas: Option<u64>,
    pub gas_price: Option<u128>,
    pub max_fee_per_gas: Option<u128>,
    pub max_priority_fee_per_gas: Option<u128>,
    pub max_fee_per_blob_gas: Option<u128>,
    pub has_blob: bool,
}

/// Resolve gas settings for the given lane, returning concrete parameters for simulation.
pub fn prepare_tx_env_gas(
    override_params: Option<&TxGasParameters>,
    default_params: &TxGasParameters,
    inputs: GasInputs,
    context: TxFeeContext<'_>,
) -> Result<SimulationGasParameters> {
    let override_params = override_params.copied();
    let default_params = *default_params;
    let gas_limit = inputs
        .gas
        .or(override_params.and_then(|p| p.gas_limit))
        .or(default_params.gas_limit)
        .ok_or_else(|| eyre!("missing gas limit for transaction"))?;
    if gas_limit == 0 {
        return Err(eyre!("transaction gas limit must be greater than zero"));
    }

    let explicit_max_fee = inputs
        .max_fee_per_gas
        .or(override_params.and_then(|p| p.max_fee_per_gas))
        .or(default_params.max_fee_per_gas);
    let priority_fee = inputs
        .max_priority_fee_per_gas
        .or(override_params.and_then(|p| p.max_priority_fee_per_gas))
        .or(default_params.max_priority_fee_per_gas);
    let legacy_gas_price = inputs
        .gas_price
        .or(override_params.and_then(|p| p.gas_price))
        .or(default_params.gas_price);
    let blob_fee = inputs
        .max_fee_per_blob_gas
        .or(override_params.and_then(|p| p.max_fee_per_blob_gas))
        .or(default_params.max_fee_per_blob_gas);

    if inputs.has_blob && blob_fee.is_none() {
        return Err(eyre!(
            "missing max_fee_per_blob_gas for transaction containing blob data"
        ));
    }

    let has_any_eip1559_input =
        inputs.max_fee_per_gas.is_some() || inputs.max_priority_fee_per_gas.is_some();
    let lane_signals_eip = override_params
        .and_then(|p| p.max_fee_per_gas.or(p.max_priority_fee_per_gas))
        .is_some()
        || default_params
            .max_fee_per_gas
            .or(default_params.max_priority_fee_per_gas)
            .is_some();
    let explicit_legacy = legacy_gas_price.is_some() && !has_any_eip1559_input && !lane_signals_eip;
    let has_base_fee = context.base_fee.is_some();

    let tx_type = if explicit_legacy {
        GasTxType::Legacy
    } else if has_any_eip1559_input || lane_signals_eip || has_base_fee {
        GasTxType::Eip1559
    } else {
        GasTxType::Legacy
    };

    if matches!(tx_type, GasTxType::Eip1559) {
        let base_fee = context.base_fee.ok_or_else(|| {
            eyre!("missing block base fee for EIP-1559 transaction gas accounting")
        })?;
        let resolved_priority = priority_fee.unwrap_or(0);
        let requested_max_fee = explicit_max_fee.unwrap_or(base_fee);
        let effective_max_fee = requested_max_fee.max(base_fee).max(resolved_priority);
        let effective_gas_price = effective_max_fee.min(base_fee.saturating_add(resolved_priority));

        Ok(SimulationGasParameters {
            tx_type,
            gas_limit,
            gas_price: effective_max_fee,
            effective_gas_price,
            max_fee_per_gas: Some(effective_max_fee),
            max_priority_fee_per_gas: Some(resolved_priority),
            max_fee_per_blob_gas: blob_fee,
        })
    } else {
        let resolved_price =
            legacy_gas_price.ok_or_else(|| eyre!("missing gas_price for legacy transaction"))?;
        let resolved_price = match context.base_fee {
            Some(base_fee) => resolved_price.max(base_fee),
            None => resolved_price,
        };

        Ok(SimulationGasParameters {
            tx_type,
            gas_limit,
            gas_price: resolved_price,
            effective_gas_price: resolved_price,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            max_fee_per_blob_gas: blob_fee,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FeeDefaults;

    static FEE_DEFAULTS: FeeDefaults = FeeDefaults {
        chain_id: Some(1),
        max_fee_per_blob_gas: 0,
    };

    fn context(base_fee: Option<u128>) -> TxFeeContext<'static> {
        TxFeeContext {
            fee_defaults: &FEE_DEFAULTS,
            block_gas_limit: 30_000_000,
            base_fee,
        }
    }

    fn inputs() -> GasInputs {
        GasInputs {
            gas: Some(100_000),
            gas_price: None,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            max_fee_per_blob_gas: None,
            has_blob: false,
        }
    }

    #[test]
    fn missing_fee_fields_default_to_eip1559_base_fee_when_base_fee_exists() {
        let resolved = prepare_tx_env_gas(
            None,
            &TxGasParameters::default(),
            inputs(),
            context(Some(1_500_000_000)),
        )
        .expect("gas should resolve");

        assert!(matches!(resolved.tx_type, GasTxType::Eip1559));
        assert_eq!(resolved.gas_limit, 100_000);
        assert_eq!(resolved.max_fee_per_gas, Some(1_500_000_000));
        assert_eq!(resolved.max_priority_fee_per_gas, Some(0));
        assert_eq!(resolved.effective_gas_price, 1_500_000_000);
    }

    #[test]
    fn missing_priority_fee_defaults_to_zero_for_eip1559_input() {
        let mut gas_inputs = inputs();
        gas_inputs.max_fee_per_gas = Some(2_000_000_000);

        let resolved = prepare_tx_env_gas(
            None,
            &TxGasParameters::default(),
            gas_inputs,
            context(Some(1_000_000_000)),
        )
        .expect("gas should resolve");

        assert!(matches!(resolved.tx_type, GasTxType::Eip1559));
        assert_eq!(resolved.max_fee_per_gas, Some(2_000_000_000));
        assert_eq!(resolved.max_priority_fee_per_gas, Some(0));
        assert_eq!(resolved.effective_gas_price, 1_000_000_000);
    }

    #[test]
    fn eip1559_max_fee_is_clamped_to_base_fee_floor() {
        let mut gas_inputs = inputs();
        gas_inputs.max_fee_per_gas = Some(500_000_000);

        let resolved = prepare_tx_env_gas(
            None,
            &TxGasParameters::default(),
            gas_inputs,
            context(Some(1_000_000_000)),
        )
        .expect("gas should resolve");

        assert_eq!(resolved.max_fee_per_gas, Some(1_000_000_000));
        assert_eq!(resolved.effective_gas_price, 1_000_000_000);
    }

    #[test]
    fn legacy_gas_price_is_clamped_to_base_fee_floor() {
        let mut gas_inputs = inputs();
        gas_inputs.gas_price = Some(500_000_000);

        let resolved = prepare_tx_env_gas(
            None,
            &TxGasParameters::default(),
            gas_inputs,
            context(Some(1_000_000_000)),
        )
        .expect("gas should resolve");

        assert!(matches!(resolved.tx_type, GasTxType::Legacy));
        assert_eq!(resolved.legacy_gas_price(), Some(1_000_000_000));
        assert_eq!(resolved.effective_gas_price, 1_000_000_000);
    }

    #[test]
    fn eip1559_effective_gas_price_uses_base_plus_priority_not_fee_cap() {
        let mut gas_inputs = inputs();
        gas_inputs.max_fee_per_gas = Some(2_000_000_000);
        gas_inputs.max_priority_fee_per_gas = Some(100_000_000);

        let resolved = prepare_tx_env_gas(
            None,
            &TxGasParameters::default(),
            gas_inputs,
            context(Some(1_000_000_000)),
        )
        .expect("gas should resolve");

        assert_eq!(resolved.max_fee_per_gas, Some(2_000_000_000));
        assert_eq!(resolved.effective_gas_price, 1_100_000_000);
    }

    #[test]
    fn eip1559_input_requires_block_base_fee() {
        let mut gas_inputs = inputs();
        gas_inputs.max_fee_per_gas = Some(2_000_000_000);

        let err = prepare_tx_env_gas(None, &TxGasParameters::default(), gas_inputs, context(None))
            .expect_err("EIP-1559 accounting needs a block base fee");

        assert!(err.to_string().contains("missing block base fee"));
    }
}
