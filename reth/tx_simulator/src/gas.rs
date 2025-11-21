//! Gas parameter resolution
//!
//! This module centralises every decision the simulator makes when callers omit
//! fee fields. The flow is:
//! 1. Collect explicit inputs (`GasInputs`) and optional overrides of the
//!    configured transaction gas parameters (`TxGasParameters`).
//! 2. Decide whether the transaction should be treated as legacy or EIP-1559 by
//!    looking at explicit inputs and resolved defaults.
//! 3. Clamp the resulting values so max fee ≥ base fee + priority fee and fall
//!    back to the configured minimum gas limit when needed.
//! 4. Produce a `SimulationGasParameters` carrying the resolved tx type, gas limit, max
//!    fee, and priority fee for the `TxEnv` builders.
//!
//! Having everything here lets the rest of the simulator call `prepare_tx_env_gas`
//! rather than duplicating logic across single-tx, chain, and bundle code paths.

use crate::types::FeeDefaults;
use eyre::Result;

/// Transaction gas parameters that callers can override when building requests.
#[derive(Debug, Clone, Copy, Default)]
pub struct TxGasParameters {
    pub gas_limit: Option<u64>,
    pub gas_price: Option<u128>,
    pub max_fee_per_gas: Option<u128>,
    pub max_priority_fee_per_gas: Option<u128>,
}

/// Context for resolving gas parameters.
#[derive(Debug, Clone, Copy)]
pub struct GasResolutionContext<'a> {
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
}

/// Resolve gas settings for the given lane, returning concrete parameters for simulation.
pub fn prepare_tx_env_gas(
    override_params: Option<&TxGasParameters>,
    default_params: &TxGasParameters,
    inputs: GasInputs,
    context: GasResolutionContext<'_>,
) -> Result<SimulationGasParameters> {
    let override_params = override_params.copied();
    let default_params = *default_params;
    let effective_base_fee = context
        .base_fee
        .unwrap_or(context.fee_defaults.pre_london_base_fee);

    let mut gas_limit = inputs
        .gas
        .or(override_params.and_then(|p| p.gas_limit))
        .or(default_params.gas_limit)
        .unwrap_or_else(|| {
            let capped = context.block_gas_limit.min(u128::from(u64::MAX));
            capped as u64
        });

    if gas_limit == 0 {
        gas_limit = crate::config::gas::MIN_GAS_LIMIT;
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
        let resolved_priority = priority_fee.unwrap_or(context.fee_defaults.min_priority_fee);
        let min_required = effective_base_fee.saturating_add(resolved_priority);

        let mut effective_max_fee = explicit_max_fee.unwrap_or(min_required);
        if effective_max_fee < min_required {
            effective_max_fee = min_required;
        }

        let gas_price = effective_max_fee;

        Ok(SimulationGasParameters {
            tx_type,
            gas_limit,
            gas_price,
            max_fee_per_gas: Some(effective_max_fee),
            max_priority_fee_per_gas: Some(resolved_priority),
        })
    } else {
        // Legacy transaction path.
        let fallback_base = context
            .base_fee
            .unwrap_or(context.fee_defaults.legacy_pre_london_base_fee);
        let resolved_price = legacy_gas_price.unwrap_or_else(|| {
            fallback_base.saturating_mul(context.fee_defaults.legacy_gas_price_multiplier)
        });

        Ok(SimulationGasParameters {
            tx_type,
            gas_limit,
            gas_price: resolved_price,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
        })
    }
}
