//! Gas parameter resolution
//!
//! This module centralises every decision the simulator makes when callers omit
//! fee fields. The flow is:
//! 1. Collect explicit inputs (`GasInputs`) and optional overrides of the
//!    configured transaction gas parameters (`TxGasParameters`).
//! 2. Decide whether the transaction should be treated as legacy or EIP-1559 by
//!    looking at explicit inputs and resolved defaults.
//! 3. If it is EIP-1559, derive the missing pieces using a `GasHeuristic`
//!    strategy:
//!       • `DynamicTip` – single-call simulations derive a tip + headroom from
//!         the current base fee (respecting `FeeDefaults` safety floors).
//!       • `Multiplier` – bundle/chain sims reuse the existing fixed multiplier +
//!         default tip behaviour.
//! 4. Clamp the resulting values so max fee ≥ base fee + priority fee and fall
//!    back to the configured minimum gas limit when needed.
//! 5. Produce a `ResolvedGasParameters` carrying the resolved tx type, gas limit, max
//!    fee, and priority fee for the `TxEnv` builders.
//!
//! Having everything here lets the rest of the simulator call `resolve_gas`
//! rather than duplicating heuristics across single-tx, chain, and bundle code paths.

use crate::types::FeeDefaults;
use eyre::{ensure, Result};

/// Transaction gas parameters that callers can override when building requests.
#[derive(Debug, Clone, Copy, Default)]
pub struct TxGasParameters {
    pub gas_limit: Option<u64>,
    pub gas_price: Option<u128>,
    pub max_fee_per_gas: Option<u128>,
    pub max_priority_fee_per_gas: Option<u128>,
}

/// Strategy describing how to derive missing EIP-1559 parameters.
#[derive(Debug, Clone, Copy)]
pub enum GasHeuristic {
    /// Derive a dynamic tip based on the block base fee (used for single-call simulations).
    DynamicTip {
        tip_divisor: u128,
        min_priority_fee: u128,
        headroom_divisor: u128,
        min_headroom: u128,
    },
    /// Apply a fixed default priority fee and multiplier (used for bundle simulations).
    Multiplier {
        default_priority_fee: u128,
        max_fee_multiplier: u128,
    },
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
pub struct ResolvedGasParameters {
    pub tx_type: GasTxType,
    pub gas_limit: u64,
    pub gas_price: u128,
    pub max_fee_per_gas: Option<u128>,
    pub max_priority_fee_per_gas: Option<u128>,
}

impl ResolvedGasParameters {
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
pub fn resolve_gas(
    override_params: Option<&TxGasParameters>,
    default_params: &TxGasParameters,
    inputs: GasInputs,
    context: GasResolutionContext<'_>,
    heuristic: GasHeuristic,
) -> Result<ResolvedGasParameters> {
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

    let mut explicit_max_fee = inputs
        .max_fee_per_gas
        .or(override_params.and_then(|p| p.max_fee_per_gas))
        .or(default_params.max_fee_per_gas);
    let mut priority_fee = inputs
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
        priority_fee = priority_fee.or_else(|| match heuristic {
            GasHeuristic::DynamicTip {
                tip_divisor,
                min_priority_fee,
                ..
            } => {
                let divisor = tip_divisor.max(1);
                let derived_tip = (effective_base_fee / divisor).max(min_priority_fee);
                Some(derived_tip)
            }
            GasHeuristic::Multiplier {
                default_priority_fee,
                ..
            } => Some(default_priority_fee),
        });

        let mut computed_max_fee = explicit_max_fee;

        computed_max_fee = Some(match (computed_max_fee, heuristic) {
            (Some(explicit), GasHeuristic::DynamicTip { .. }) => explicit,
            (Some(explicit), GasHeuristic::Multiplier { .. }) => explicit,
            (
                None,
                GasHeuristic::DynamicTip {
                    headroom_divisor,
                    min_headroom,
                    ..
                },
            ) => {
                let divisor = headroom_divisor.max(1);
                let headroom = (effective_base_fee / divisor).max(min_headroom);
                effective_base_fee
                    .saturating_add(priority_fee.unwrap_or(context.fee_defaults.min_priority_fee))
                    .saturating_add(headroom)
            }
            (
                None,
                GasHeuristic::Multiplier {
                    max_fee_multiplier, ..
                },
            ) => {
                let multiplier = max_fee_multiplier.max(1);
                effective_base_fee
                    .saturating_mul(multiplier)
                    .saturating_add(priority_fee.unwrap_or(context.fee_defaults.min_priority_fee))
            }
        });

        // Ensure priority fee is at least the global minimum.
        let resolved_priority = priority_fee
            .unwrap_or(context.fee_defaults.min_priority_fee)
            .max(context.fee_defaults.min_priority_fee);

        let min_required = effective_base_fee.saturating_add(resolved_priority);

        if let Some(ref mut max_fee) = computed_max_fee {
            if *max_fee < min_required {
                *max_fee = min_required;
            }
        } else {
            computed_max_fee = Some(min_required);
        }

        priority_fee = Some(resolved_priority);
        explicit_max_fee = computed_max_fee;

        ensure!(
            explicit_max_fee.is_some(),
            "failed to derive max_fee_per_gas for EIP-1559 transaction"
        );

        let effective_max_fee = explicit_max_fee.unwrap();
        let gas_price = effective_max_fee;

        Ok(ResolvedGasParameters {
            tx_type,
            gas_limit,
            gas_price,
            max_fee_per_gas: Some(effective_max_fee),
            max_priority_fee_per_gas: priority_fee,
        })
    } else {
        // Legacy transaction path.
        let fallback_base = context
            .base_fee
            .unwrap_or(context.fee_defaults.legacy_pre_london_base_fee);
        let resolved_price = legacy_gas_price.unwrap_or_else(|| {
            fallback_base.saturating_mul(context.fee_defaults.legacy_gas_price_multiplier)
        });

        Ok(ResolvedGasParameters {
            tx_type,
            gas_limit,
            gas_price: resolved_price,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
        })
    }
}
