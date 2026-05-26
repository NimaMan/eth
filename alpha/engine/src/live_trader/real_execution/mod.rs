use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use alloy_primitives::U256;
use async_trait::async_trait;
use eth_alpha_core::{
    error::AlphaCoreError,
    execution::ExecutionReport,
    ids::TokenPoolId,
    market::PoolSnapshot,
    order::{OrderIntent, OrderSide},
    position::Position,
};
use eth_alpha_store::PostgresTradingStore;
use eth_live_trading::{
    derive_min_output_from_expected_output, ChainServerGasRankProvider, GasEstimateConfig,
    GasRankPlan, GasRankProvider, KartalBribeRequest, KartalExecutorClient,
    KartalExecutorClientConfig, KartalSimulationReference, LiveDirectRawTransactionRequest,
    LivePrioritySellPlanner, LivePrioritySellPlannerConfig, LivePrioritySellPlannerError,
    LiveTraderTxSignal, MempoolRaceGasRankProvider, PreSubmitSimulation, PreSubmitSimulator,
    PreparedSellRoute, RankedFeeCandidate, StrategyGasRankPolicy, TxPrepConfig, TxSubmissionPolicy,
    UniswapV2TradingVaultBuyRouteBuilder, UniswapV2TradingVaultPreSubmitSimulator,
    UniswapV2TradingVaultSellRouteBuilder, VaultInternalAllowanceChecker,
    ETH_UNSIGNED_TX_WIRE_PROTOCOL,
};
use eyre::{eyre, Result};
use rust_decimal::Decimal;
use serde_json::json;

use crate::execution::real::{
    LiveTradingPlannerBridge, LiveTxPlanner, LiveTxSubmissionResult, LiveTxSubmitter,
    TxExecutorAdapter,
};
use crate::{EngineExecutionAdapter, LiveChainSimExecutionAdapter, PositionValueSimulation};

use super::cli::RealExecutionArgs;
use super::gas_policy::LiveRealGasPolicy;

mod input_resolver;
mod preflight;
mod tail_entry;
use input_resolver::LiveRealInputResolver;
use preflight::parse_live_real_address;
pub(in crate::live_trader) use preflight::KartalRealPreflight;
pub(super) use preflight::{preflight_kartal_real, validate_flashbots_tail_max_block_span};
#[cfg(test)]
use preflight::{validate_kartal_real_status, HOLD16_DEPLOY_BUY_WEI};
#[cfg(test)]
use tail_entry::TailEntryOrderingEvidence;
use tail_entry::{
    buy_submission_policy, tail_entry_ordering_evidence, tail_entry_overlay_plan,
    validate_tail_entry_route,
};

struct RealExecutionWithValuation<E, V> {
    execution: E,
    valuation: V,
}

#[async_trait]
impl<E, V> EngineExecutionAdapter for RealExecutionWithValuation<E, V>
where
    E: EngineExecutionAdapter,
    V: EngineExecutionAdapter,
{
    async fn execute(
        &self,
        intent: eth_alpha_core::order::OrderIntent,
    ) -> eth_alpha_core::error::Result<ExecutionReport> {
        self.execution.execute(intent).await
    }

    async fn simulate_position_value(
        &self,
        position: &Position,
        pool: &PoolSnapshot,
    ) -> eth_alpha_core::error::Result<Option<PositionValueSimulation>> {
        self.valuation.simulate_position_value(position, pool).await
    }
}

struct KartalRealPlanner<P, G> {
    sell_planner: P,
    resolver: LiveRealInputResolver,
    simulator: UniswapV2TradingVaultPreSubmitSimulator,
    buy_route_builder: UniswapV2TradingVaultBuyRouteBuilder,
    gas_rank: G,
    gas_estimate: GasEstimateConfig,
    gas_policy: LiveRealGasPolicy,
    flashbots_tail_max_block_span: u64,
}

struct KartalPolicySubmitter {
    kartal: KartalExecutorClient,
}

#[async_trait]
impl LiveTxSubmitter for KartalPolicySubmitter {
    async fn submit_signal(
        &self,
        signal: &LiveTraderTxSignal,
    ) -> std::result::Result<LiveTxSubmissionResult, String> {
        self.kartal
            .submit_signal(signal)
            .await
            .map(Into::into)
            .map_err(|error| error.to_string())
    }
}

#[async_trait]
impl<P, G> LiveTxPlanner for KartalRealPlanner<P, G>
where
    P: LiveTxPlanner,
    G: GasRankProvider,
{
    async fn prepare_signal(
        &self,
        intent: &OrderIntent,
    ) -> eth_alpha_core::error::Result<LiveTraderTxSignal> {
        match intent.side {
            OrderSide::Sell => self.sell_planner.prepare_signal(intent).await,
            OrderSide::Buy => self.prepare_buy_signal(intent).await,
        }
    }
}

impl<P, G> KartalRealPlanner<P, G>
where
    G: GasRankProvider,
{
    async fn prepare_buy_signal(
        &self,
        intent: &OrderIntent,
    ) -> eth_alpha_core::error::Result<LiveTraderTxSignal> {
        let mut input = self.resolver.resolve_buy_input(intent).await?;
        let buy_policy_context = self.gas_policy.buy_policy_context(
            input
                .intent
                .decision_reason
                .as_ref()
                .map(|reason| reason.code.as_str()),
        );
        let gas_policy_action = buy_policy_context.action;
        let gas_policy_signal = buy_policy_context.signal.clone();
        let gas_policy_guard = buy_policy_context.guard;
        let gas_rank_policy = buy_policy_context.policy.clone();
        let tail_entry_ordering = tail_entry_ordering_evidence(&input.intent, gas_policy_action);
        let tail_entry_overlay = tail_entry_overlay_plan(&input, gas_policy_action)?;
        let (min_output, simulation, tail_entry_evidence) = if let Some(plan) = tail_entry_overlay {
            input.min_output_amount = Some(plan.min_output.to_string());
            input.context.tx.source_metadata = json!({
                "planner_source": input.context.tx.source_metadata,
                "input_source": input.source_metadata,
                "min_output_quote": {
                    "provider": "mempool_entry_exact_vault_overlay",
                    "simulation_block": plan.simulation.block_number,
                    "expected_output_token": plan.simulation.expected_output_token.clone(),
                    "expected_output_amount": plan.simulation.expected_output_amount.clone(),
                    "min_output_amount": plan.min_output.to_string(),
                    "max_slippage_bps": input.intent.max_slippage_bps,
                    "dependency_tx_hashes": plan.evidence.dependency_tx_hashes.clone(),
                }
            });
            (plan.min_output, plan.simulation, Some(plan.evidence))
        } else {
            let provisional_route = self
                .buy_route_builder
                .build_route(&input, U256::ZERO)
                .map_err(planner_error)?;
            let quote_simulation = self
                .simulator
                .simulate(&input, &provisional_route)
                .await
                .map_err(planner_error)?;
            let min_output =
                min_output_from_simulation(&quote_simulation, input.intent.max_slippage_bps)?;
            input.min_output_amount = Some(min_output.to_string());
            input.context.tx.source_metadata = json!({
                "planner_source": input.context.tx.source_metadata,
                "input_source": input.source_metadata,
                "min_output_quote": {
                    "provider": "exact_pre_submit_simulation",
                    "simulation_block": quote_simulation.block_number,
                    "expected_output_token": quote_simulation.expected_output_token.clone(),
                    "expected_output_amount": quote_simulation.expected_output_amount.clone(),
                    "min_output_amount": min_output.to_string(),
                    "max_slippage_bps": input.intent.max_slippage_bps,
                }
            });
            let route = self
                .buy_route_builder
                .build_route(&input, min_output)
                .map_err(planner_error)?;
            let simulation = self
                .simulator
                .simulate(&input, &route)
                .await
                .map_err(planner_error)?;
            (min_output, simulation, None)
        };
        let mut route = self
            .buy_route_builder
            .build_route(&input, min_output)
            .map_err(planner_error)?;
        if let Some(evidence) = tail_entry_evidence.as_ref() {
            validate_tail_entry_route(&input, &route, evidence)?;
        }
        ensure_simulation_ok(&simulation)?;
        apply_simulated_gas_used(&mut route, &simulation, &self.gas_estimate)?;
        let gas_rank = self
            .gas_rank
            .ranked_fee_candidates(&input, &route)
            .await
            .map_err(planner_error)?;
        let fee = select_entry_gas_fee(
            &gas_rank,
            &gas_rank_policy,
            &route,
            &self.gas_policy,
            input.context.current_block,
        )?;
        let estimated_gas_used = route
            .require_estimated_gas_used()
            .map_err(|error| AlphaCoreError::Execution(error.to_string()))?;
        let trade_id = input.intent.trade_id.clone().ok_or_else(|| {
            AlphaCoreError::Execution("live real buy signal requires trade_id".to_string())
        })?;
        let submission_policy = buy_submission_policy(
            self.flashbots_tail_max_block_span,
            gas_policy_action,
            &tail_entry_ordering,
            input.context.current_block,
        )?;
        let wire_protocol = transaction_wire_protocol(&submission_policy);
        let executor_boundary_label = executor_boundary(&submission_policy);

        Ok(LiveTraderTxSignal {
            strategy_name: input.context.tx.strategy_name.clone(),
            strategy_run_id: input.context.tx.strategy_run_id.clone(),
            trade_id: Some(trade_id.clone()),
            token_address: Some(input.intent.token_address),
            pool_address: Some(input.intent.pool_address.clone()),
            observed_block: Some(input.context.current_block),
            submission_policy,
            request: LiveDirectRawTransactionRequest {
                attempt_id: Some(format!(
                    "{}-entry-{}",
                    trade_id.0, input.context.current_block
                )),
                chain_id: input.context.tx.chain_id,
                from: input.context.tx.from.clone(),
                to: route.router_address.clone(),
                value: route.value_wei.clone(),
                data: route.calldata.clone(),
                gas_limit: route.gas_limit.to_string(),
                max_fee_per_gas: decimal_gwei_to_wei_string(fee.max_fee_per_gas_gwei),
                max_priority_fee_per_gas: decimal_gwei_to_wei_string(fee.priority_fee_gwei),
                nonce: None,
                bribe: Some(KartalBribeRequest {
                    priority_fee_per_gas: decimal_gwei_to_wei_string(fee.priority_fee_gwei),
                    max_fee_per_gas: Some(decimal_gwei_to_wei_string(fee.max_fee_per_gas_gwei)),
                }),
                simulation: Some(KartalSimulationReference {
                    block_number: simulation.block_number,
                    block_hash: simulation.block_hash.clone(),
                    state_root: simulation.state_root.clone(),
                    expected_output_token: simulation.expected_output_token.clone(),
                    expected_output_amount: simulation.expected_output_amount.clone(),
                    min_output_amount: simulation.min_output_amount.clone(),
                    metadata: simulation.metadata(),
                }),
                metadata: json!({
                    "wire_protocol": wire_protocol,
                    "intent_kind": "entry_buy",
                    "executor_boundary": executor_boundary_label,
                    "tx_prep_version": 1,
                    "reason": input.intent.decision_reason.as_ref().map(|reason| reason.code.clone()).unwrap_or_else(|| "entry.live_buy".to_string()),
                    "route": {
                        "protocol": route.protocol,
                        "router_address": route.router_address,
                        "gas_limit": route.gas_limit,
                        "estimated_gas_used": route.estimated_gas_used,
                        "max_slippage_bps": route.max_slippage_bps,
                    },
                    "gas_plan": {
                        "label": fee.label,
                        "priority_fee_gwei": fee.priority_fee_gwei,
                        "max_fee_per_gas_gwei": fee.max_fee_per_gas_gwei,
                        "estimated_priority_spend_eth": fee.estimated_priority_spend_eth(estimated_gas_used),
                        "estimated_max_cost_eth": fee.estimated_max_cost_eth(estimated_gas_used),
                        "source": fee.source,
                    },
                    "gas_policy": {
                        "action": gas_policy_action,
                        "signal": gas_policy_signal,
                        "status": "selected",
                        "profiles": gas_policy_profile_labels(&gas_rank_policy),
                        "selected_profile": fee.label,
                        "gas_rank_source": fee.source,
                        "guard": gas_policy_guard,
                        "estimated_max_cost_eth": fee.estimated_max_cost_eth(estimated_gas_used),
                        "estimated_priority_spend_eth": fee.estimated_priority_spend_eth(estimated_gas_used),
                    },
                    "tail_entry_ordering": {
                        "tail_after_tx_hash": tail_entry_ordering.as_ref().and_then(|evidence| evidence.tail_after_tx_hash.clone()),
                        "dependency_priority_fee_wei": tail_entry_ordering.as_ref().and_then(|evidence| evidence.dependency_priority_fee_wei.clone()),
                        "dependency_gas_price_wei": tail_entry_ordering.as_ref().and_then(|evidence| evidence.dependency_gas_price_wei.clone()),
                    },
                    "production_gas_guard": {
                        "required_gas_rank_source": self.gas_policy.required_gas_rank_source.as_str(),
                        "max_priority_fee_gwei": self.gas_policy.max_priority_fee_gwei,
                        "max_estimated_gas_fee_eth": self.gas_policy.entry_max_estimated_gas_fee_eth,
                    },
                    "strategy_gas_rank_policy": gas_rank_policy,
                    "state_dependency": {
                        "decision_block": input.context.current_block,
                        "signal_observed_block": input.context.tx.observed_block,
                        "required_state_block": input.context.tx.required_state_block,
                        "pool_creation_block": input.pool.creation_block,
                        "pool_latest_block": input.pool.latest_block,
                        "simulation_block": simulation.block_number,
                    },
                    "simulation": simulation.metadata(),
                    "source": input.context.tx.source_metadata,
                }),
            },
        })
    }
}

fn planner_error(error: LivePrioritySellPlannerError) -> AlphaCoreError {
    if error.is_deferrable() {
        return AlphaCoreError::ExecutionDeferred {
            reason: error.to_string(),
            block_number: error.deferral_block_number(),
        };
    }
    AlphaCoreError::Execution(error.to_string())
}

// Kartal policy evaluates and signs the inner DirectRawTransactionRequest;
// submission_policy only selects the transport/order route.
fn transaction_wire_protocol(_submission_policy: &TxSubmissionPolicy) -> &'static str {
    ETH_UNSIGNED_TX_WIRE_PROTOCOL
}

fn executor_boundary(submission_policy: &TxSubmissionPolicy) -> &'static str {
    match submission_policy {
        TxSubmissionPolicy::PublicMempool => "kartal_eth_tx_executor",
        TxSubmissionPolicy::FlashbotsMevShare { .. } => "kartal_eth_tx_executor_policy",
    }
}

fn min_output_from_simulation(
    simulation: &PreSubmitSimulation,
    max_slippage_bps: u32,
) -> eth_alpha_core::error::Result<U256> {
    ensure_simulation_ok(simulation)?;
    let expected_output = simulation
        .expected_output_amount
        .as_deref()
        .ok_or_else(|| {
            AlphaCoreError::Execution(
                "exact live buy simulation did not report expected output amount".to_string(),
            )
        })?;
    let expected_output = parse_u256_quantity(expected_output, "simulation expected output")?;
    if expected_output.is_zero() {
        return Err(cancelled_execution_at(
            "exact live buy simulation returned zero output",
            simulation.block_number,
        ));
    }
    let min_output = derive_min_output_from_expected_output(expected_output, max_slippage_bps)
        .map_err(planner_error)?;
    if min_output.is_zero() {
        return Err(cancelled_execution_at(
            "derived live buy min-output is zero",
            simulation.block_number,
        ));
    }
    Ok(min_output)
}

fn ensure_simulation_ok(simulation: &PreSubmitSimulation) -> eth_alpha_core::error::Result<()> {
    if simulation.would_revert {
        return Err(cancelled_execution_at(
            format!(
                "exact live transaction simulation would revert at block {}",
                simulation.block_number
            ),
            simulation.block_number,
        ));
    }
    Ok(())
}

fn cancelled_execution_at(reason: impl Into<String>, block_number: u64) -> AlphaCoreError {
    AlphaCoreError::ExecutionCancelled {
        reason: reason.into(),
        block_number: Some(block_number),
    }
}

fn apply_simulated_gas_used(
    route: &mut PreparedSellRoute,
    simulation: &PreSubmitSimulation,
    gas_estimate: &GasEstimateConfig,
) -> eth_alpha_core::error::Result<()> {
    let gas_used = simulation.gas_used.ok_or_else(|| {
        AlphaCoreError::Execution(
            "exact live transaction simulation did not report gas_used; fallback gas estimates are not allowed"
                .to_string(),
        )
    })?;
    route
        .apply_simulated_gas_used(gas_used, gas_estimate)
        .map_err(|error| AlphaCoreError::Execution(error.to_string()))
}

fn select_entry_gas_fee(
    plan: &GasRankPlan,
    policy: &StrategyGasRankPolicy,
    route: &PreparedSellRoute,
    gas_policy: &LiveRealGasPolicy,
    current_block: u64,
) -> eth_alpha_core::error::Result<RankedFeeCandidate> {
    let estimated_gas_used = route
        .require_estimated_gas_used()
        .map_err(|error| AlphaCoreError::Execution(error.to_string()))?;
    let candidates = plan
        .candidates
        .iter()
        .filter(|candidate| {
            candidate.source.as_deref() == Some(gas_policy.required_gas_rank_source.as_str())
                && candidate.priority_fee_gwei <= gas_policy.max_priority_fee_gwei
                && candidate.estimated_max_cost_eth(estimated_gas_used)
                    <= gas_policy.entry_max_estimated_gas_fee_eth
        })
        .cloned()
        .collect::<Vec<_>>();

    policy.choose_candidate(&candidates).ok_or_else(|| {
        let reason = format!(
            "live real buy planner has no gas fee candidate matching production gas guard: required_source={} max_priority_fee_gwei={} max_estimated_gas_fee_eth={} candidates={}",
            gas_policy.required_gas_rank_source,
            gas_policy.max_priority_fee_gwei,
            gas_policy.entry_max_estimated_gas_fee_eth,
            serde_json::to_string(&plan.candidates).unwrap_or_else(|_| "[]".to_string())
        );
        AlphaCoreError::ExecutionCancelled {
            reason,
            block_number: Some(current_block),
        }
    })
}

fn gas_policy_profile_labels(policy: &StrategyGasRankPolicy) -> Vec<&'static str> {
    policy
        .preference_order
        .iter()
        .map(|profile| profile.label())
        .collect()
}

fn decimal_gwei_to_wei_string(value: Decimal) -> String {
    let wei = (value.max(Decimal::ZERO) * Decimal::from(1_000_000_000u64)).trunc();
    decimal_integer_string(wei)
}

fn decimal_integer_string(value: Decimal) -> String {
    let text = value.normalize().to_string();
    let integer = text
        .split('.')
        .next()
        .unwrap_or("0")
        .trim_start_matches('+');
    if integer.is_empty() {
        "0".to_string()
    } else {
        integer.to_string()
    }
}

fn parse_u256_quantity(value: &str, label: &str) -> eth_alpha_core::error::Result<U256> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AlphaCoreError::Execution(format!(
            "{label} quantity is empty"
        )));
    }
    if let Some(hex) = trimmed.strip_prefix("0x") {
        U256::from_str_radix(hex, 16).map_err(|error| {
            AlphaCoreError::Execution(format!("invalid {label} hex quantity {trimmed:?}: {error}"))
        })
    } else {
        U256::from_str_radix(trimmed, 10).map_err(|error| {
            AlphaCoreError::Execution(format!(
                "invalid {label} decimal quantity {trimmed:?}: {error}"
            ))
        })
    }
}

pub(super) async fn build_kartal_real_adapter(
    args: &RealExecutionArgs,
    preflight: KartalRealPreflight,
    chain_server_url: String,
    store: PostgresTradingStore,
    run_id: String,
    valuation_adapter: LiveChainSimExecutionAdapter,
    exact_pre_submit_live_simulator: Option<tx_simulator::LiveTxSimulator>,
    pools: Arc<std::sync::Mutex<HashMap<TokenPoolId, PoolSnapshot>>>,
    current_block: Arc<AtomicU64>,
    flashbots_tail_max_block_span: u64,
    gas_policy: LiveRealGasPolicy,
) -> Result<Box<dyn EngineExecutionAdapter>> {
    let from = parse_live_real_address(&args.live_real_from, "--live-real-from")?;
    let vault =
        parse_live_real_address(&args.live_real_vault_address, "--live-real-vault-address")?;
    let Some(exact_pre_submit_live_simulator) = exact_pre_submit_live_simulator else {
        return Err(eyre!(
            "kartal-real execution requires a live-only in-memory LiveTxSimulator; Reth historical context is not allowed for real pre-submit"
        ));
    };
    let pre_submit_simulator =
        UniswapV2TradingVaultPreSubmitSimulator::new(exact_pre_submit_live_simulator, vault);
    let gas_rank_provider = MempoolRaceGasRankProvider::new(
        ChainServerGasRankProvider::new(chain_server_url)
            .with_lookback_blocks(gas_policy.gas_rank_lookback_blocks)
            .with_priority_tie_breaker_gwei(gas_policy.gas_rank_priority_tie_breaker_gwei),
        preflight.status.rpc_url.clone(),
    )
    .with_priority_buffer_range_gwei(
        gas_policy.mempool_race_priority_buffer_min_gwei,
        gas_policy.mempool_race_priority_buffer_max_gwei,
    );

    let mut planner_config = LivePrioritySellPlannerConfig::default();
    planner_config.require_existing_allowance = false;
    planner_config.tx_prep = TxPrepConfig {
        max_total_fee_eth: gas_policy.exit_max_estimated_gas_fee_eth,
        max_priority_fee_gwei: gas_policy.max_priority_fee_gwei,
        safety_buffer_eth: gas_policy.safety_buffer_eth,
        gas_rank_policy: gas_policy.normal_exit_gas_rank_policy.clone(),
        required_gas_rank_source: Some(gas_policy.required_gas_rank_source.clone()),
    };
    planner_config.max_priority_fee_per_gas_gwei = gas_policy.max_priority_fee_gwei;
    planner_config.max_total_fee_eth = gas_policy.exit_max_estimated_gas_fee_eth;
    planner_config
        .gas_estimate
        .simulated_gas_estimate_buffer_bps = gas_policy.simulated_gas_buffer_bps;
    planner_config.normal_exit_gas_rank_policy = gas_policy.normal_exit_gas_rank_policy.clone();
    planner_config.mempool_pre_mine_gas_rank_policy = StrategyGasRankPolicy::mempool_race_only();
    planner_config.lp_approval_exit_gas_rank_policy =
        gas_policy.lp_approval_exit_gas_rank_policy.clone();
    let gas_estimate = planner_config.gas_estimate.clone();

    let planner = LivePrioritySellPlanner::new(
        planner_config,
        UniswapV2TradingVaultSellRouteBuilder::with_gas_limit(
            vault,
            gas_policy.v2_vault_sell_gas_limit,
        ),
        pre_submit_simulator.clone(),
        gas_rank_provider.clone(),
        VaultInternalAllowanceChecker,
    );
    let resolver = LiveRealInputResolver {
        store,
        pools,
        current_block,
        from: from.to_string(),
        run_id: run_id.clone(),
        chain_id: preflight.status.chain_id,
    };
    let bridge = LiveTradingPlannerBridge::new(planner, resolver.clone());
    let real_planner = KartalRealPlanner {
        sell_planner: bridge,
        resolver,
        simulator: pre_submit_simulator,
        buy_route_builder: UniswapV2TradingVaultBuyRouteBuilder::with_gas_limit(
            vault,
            gas_policy.v2_vault_buy_gas_limit,
        ),
        gas_rank: gas_rank_provider,
        gas_estimate,
        gas_policy,
        flashbots_tail_max_block_span,
    };
    let kartal = KartalExecutorClient::new(KartalExecutorClientConfig::new(
        &args.kartal_url,
        preflight.token.clone(),
    ));
    let submitter = KartalPolicySubmitter { kartal };
    let executor = TxExecutorAdapter::with_order_prefix(real_planner, submitter, run_id);
    Ok(Box::new(RealExecutionWithValuation {
        execution: executor,
        valuation: valuation_adapter,
    }))
}

#[cfg(test)]
mod tests;
