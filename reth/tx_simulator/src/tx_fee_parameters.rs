//! Transaction fee parameter validation and packaging.
//!
//! Callers must supply the exact gas limit and fee caps they want to simulate.
//! This module simply checks those fields are present, ensures legacy vs
//! EIP-1559 mode is respected, and packages the values into a
//! `SimulationGasParameters` struct that the TxEnv builders can consume. No
//! defaults or heuristics are applied here—missing inputs result in errors.

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
    let _ = context;

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

    let tx_type = if explicit_legacy {
        GasTxType::Legacy
    } else {
        GasTxType::Eip1559
    };

    if matches!(tx_type, GasTxType::Eip1559) {
        let resolved_priority = priority_fee
            .ok_or_else(|| eyre!("missing max_priority_fee_per_gas for EIP-1559 transaction"))?;
        let effective_max_fee = explicit_max_fee
            .ok_or_else(|| eyre!("missing max_fee_per_gas for EIP-1559 transaction"))?;
        let gas_price = effective_max_fee;

        Ok(SimulationGasParameters {
            tx_type,
            gas_limit,
            gas_price,
            max_fee_per_gas: Some(effective_max_fee),
            max_priority_fee_per_gas: Some(resolved_priority),
            max_fee_per_blob_gas: blob_fee,
        })
    } else {
        let resolved_price =
            legacy_gas_price.ok_or_else(|| eyre!("missing gas_price for legacy transaction"))?;

        Ok(SimulationGasParameters {
            tx_type,
            gas_limit,
            gas_price: resolved_price,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            max_fee_per_blob_gas: blob_fee,
        })
    }
}
