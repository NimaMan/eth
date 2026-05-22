use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use alloy_primitives::{Address, U256};
use async_trait::async_trait;
use chrono::Utc;
use eth_alpha_core::{
    error::AlphaCoreError,
    execution::ExecutionReport,
    ids::{PositionId, TokenPoolId},
    market::PoolSnapshot,
    order::{OrderIntent, OrderSide},
    position::{Position, PositionKey},
};
use eth_alpha_store::PostgresTradingStore;
use eth_live_trading::{
    derive_min_output_from_expected_output, ChainServerGasRankProvider, GasEstimateConfig,
    GasRankPlan, GasRankProvider, KartalBribeRequest, KartalClient, KartalClientConfig,
    KartalEthTxExecutorStatus, KartalExecutorClient, KartalExecutorClientConfig,
    KartalSimulationReference, KartalStatusBroadcastMode, LiveDirectRawTransactionRequest,
    LivePrioritySellPlanner, LivePrioritySellPlannerConfig, LivePrioritySellPlannerError,
    LivePrioritySellPlannerInput, LiveTraderTxSignal, PlannerTxContext, PreSubmitSimulation,
    PreSubmitSimulator, PreparedSellRoute, RankedFeeCandidate, StrategyGasRankPolicy, TxPrepConfig,
    TxPrepRequestContext, UniswapV2TradingVaultBuyRouteBuilder,
    UniswapV2TradingVaultPreSubmitSimulator, UniswapV2TradingVaultSellRouteBuilder,
    VaultInternalAllowanceChecker,
};
use eth_strategies::{
    alpha11::{
        HOLD3_VALIDATION_STRATEGY_NAME, LIVE_VALIDATION_ENTRY_BANKROLL_ETH,
        LIVE_VALIDATION_MAX_ENTRY_POOLS,
    },
    shared_rules::live::LiveStrategySpec,
};
use eyre::{eyre, Result, WrapErr};
use rust_decimal::Decimal;
use serde_json::json;

use crate::execution::real::{
    LiveTradingPlannerBridge, LiveTxPlanner, LiveTxPlanningInputResolver, TxExecutorAdapter,
};
use crate::{EngineExecutionAdapter, LiveChainSimExecutionAdapter, PositionValueSimulation};

use super::cli::{Args, RealExecutionArgs};
use super::gas_policy::LiveRealGasPolicy;

const LIVE_VALIDATION_BUY_WEI: &str = "10000000000000000";

struct RealExecutionWithValuation<E, V> {
    execution: E,
    valuation: V,
}

impl<E, V> RealExecutionWithValuation<E, V> {
    fn new(execution: E, valuation: V) -> Self {
        Self {
            execution,
            valuation,
        }
    }
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

#[derive(Clone)]
struct LiveRealInputResolver {
    store: PostgresTradingStore,
    pools: Arc<std::sync::Mutex<HashMap<TokenPoolId, PoolSnapshot>>>,
    current_block: Arc<AtomicU64>,
    from: String,
    run_id: String,
    chain_id: u64,
}

#[async_trait]
impl LiveTxPlanningInputResolver for LiveRealInputResolver {
    async fn resolve_priority_sell_input(
        &self,
        intent: &eth_alpha_core::order::OrderIntent,
    ) -> eth_alpha_core::error::Result<LivePrioritySellPlannerInput> {
        let position = self.resolve_position(intent).await?;
        let pool = {
            let pools = self
                .pools
                .lock()
                .map_err(|_| AlphaCoreError::Execution("pool cache lock poisoned".to_string()))?;
            pools.get(&intent.pool_address).cloned()
        }
        .ok_or_else(|| {
            AlphaCoreError::Execution(format!(
                "live real planner has no pool snapshot for {}",
                intent.pool_address
            ))
        })?;

        let current_block = match self.current_block.load(Ordering::Relaxed) {
            0 => pool.latest_block,
            block => block,
        };
        let now = Utc::now().timestamp().max(0) as u64;

        Ok(LivePrioritySellPlannerInput {
            context: PlannerTxContext {
                tx: TxPrepRequestContext {
                    chain_id: self.chain_id,
                    from: self.from.clone(),
                    strategy_name: intent.strategy_name.0.clone(),
                    strategy_run_id: Some(self.run_id.clone()),
                    observed_block: Some(current_block),
                    source_metadata: json!({
                        "resolver": "eth_alpha_live_trader_real_execution",
                        "execution_mode": "kartal-real",
                        "route": "uniswap_v2_trading_vault",
                        "simulation_provider": "reth_exact_calldata_uniswap_v2_trading_vault",
                        "gas_rank_provider": "eth_chain_server_gas_rank",
                        "min_output_policy": "exact_pre_submit_simulation_slippage_bps"
                    }),
                },
                current_block,
                deadline_unix_secs: now.saturating_add(intent.deadline_secs),
            },
            intent: intent.clone(),
            position,
            pool,
            min_output_amount: None,
            source_metadata: json!({
                "decision_reason": intent.decision_reason.clone(),
                "trade_id": intent.trade_id.clone(),
            }),
        })
    }
}

impl LiveRealInputResolver {
    async fn resolve_buy_input(
        &self,
        intent: &OrderIntent,
    ) -> eth_alpha_core::error::Result<LivePrioritySellPlannerInput> {
        let pool = self.resolve_pool(intent).await?;
        let current_block = match self.current_block.load(Ordering::Relaxed) {
            0 => pool.latest_block,
            block => block,
        };
        let now = Utc::now().timestamp().max(0) as u64;
        let trade_id = intent.trade_id.clone().ok_or_else(|| {
            AlphaCoreError::Execution(
                "live real buy planner requires engine-assigned trade_id".to_string(),
            )
        })?;
        let position = Position::with_trade_id(
            PositionId(trade_id.0.clone()),
            trade_id,
            PositionKey {
                portfolio_id: intent.portfolio_id.clone(),
                wallet_id: intent.wallet_id.clone(),
                strategy_name: intent.strategy_name.clone(),
                token_address: intent.token_address,
                pool_address: intent.pool_address.clone(),
                protocol: intent.protocol.clone(),
            },
        );

        Ok(LivePrioritySellPlannerInput {
            context: PlannerTxContext {
                tx: TxPrepRequestContext {
                    chain_id: self.chain_id,
                    from: self.from.clone(),
                    strategy_name: intent.strategy_name.0.clone(),
                    strategy_run_id: Some(self.run_id.clone()),
                    observed_block: Some(current_block),
                    source_metadata: json!({
                        "resolver": "eth_alpha_live_trader_real_execution",
                        "execution_mode": "kartal-real",
                        "route": "uniswap_v2_trading_vault",
                        "simulation_provider": "reth_exact_calldata_uniswap_v2_trading_vault",
                        "gas_rank_provider": "eth_chain_server_gas_rank",
                        "min_output_policy": "exact_pre_submit_simulation_slippage_bps"
                    }),
                },
                current_block,
                deadline_unix_secs: now.saturating_add(intent.deadline_secs),
            },
            intent: intent.clone(),
            position,
            pool,
            min_output_amount: None,
            source_metadata: json!({
                "decision_reason": intent.decision_reason.clone(),
                "trade_id": intent.trade_id.clone(),
            }),
        })
    }

    async fn resolve_pool(
        &self,
        intent: &OrderIntent,
    ) -> eth_alpha_core::error::Result<PoolSnapshot> {
        let pool = {
            let pools = self
                .pools
                .lock()
                .map_err(|_| AlphaCoreError::Execution("pool cache lock poisoned".to_string()))?;
            pools.get(&intent.pool_address).cloned()
        };
        pool.ok_or_else(|| {
            AlphaCoreError::Execution(format!(
                "live real planner has no pool snapshot for {}",
                intent.pool_address
            ))
        })
    }

    async fn resolve_position(
        &self,
        intent: &eth_alpha_core::order::OrderIntent,
    ) -> eth_alpha_core::error::Result<Position> {
        let positions = self
            .store
            .load_active_positions(&intent.strategy_name.0)
            .await
            .map_err(|error| AlphaCoreError::Execution(error.to_string()))?;

        positions
            .into_iter()
            .find(|position| {
                intent
                    .trade_id
                    .as_ref()
                    .map(|trade_id| &position.trade_id == trade_id)
                    .unwrap_or(true)
                    && position.key.strategy_name == intent.strategy_name
                    && position.key.token_address == intent.token_address
                    && position.key.pool_address == intent.pool_address
                    && position.can_submit_exit()
            })
            .ok_or_else(|| {
                AlphaCoreError::Execution(format!(
                    "no active sellable position found for strategy={} token={} pool={}",
                    intent.strategy_name.0, intent.token_address, intent.pool_address
                ))
            })
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
        let mut route = self
            .buy_route_builder
            .build_route(&input, min_output)
            .map_err(planner_error)?;
        let simulation = self
            .simulator
            .simulate(&input, &route)
            .await
            .map_err(planner_error)?;
        ensure_simulation_ok(&simulation)?;
        apply_simulated_gas_used(&mut route, &simulation, &self.gas_estimate)?;
        let gas_rank = self
            .gas_rank
            .ranked_fee_candidates(&input, &route)
            .await
            .map_err(planner_error)?;
        let gas_rank_policy = self.gas_policy.entry_buy_gas_rank_policy.clone();
        let fee = select_entry_gas_fee(&gas_rank, &gas_rank_policy, &route, &self.gas_policy)?;
        let estimated_gas_used = route
            .require_estimated_gas_used()
            .map_err(|error| AlphaCoreError::Execution(error.to_string()))?;
        let trade_id = input.intent.trade_id.clone().ok_or_else(|| {
            AlphaCoreError::Execution("live real buy signal requires trade_id".to_string())
        })?;

        Ok(LiveTraderTxSignal {
            strategy_name: input.context.tx.strategy_name.clone(),
            strategy_run_id: input.context.tx.strategy_run_id.clone(),
            trade_id: Some(trade_id.clone()),
            token_address: Some(input.intent.token_address),
            pool_address: Some(input.intent.pool_address.clone()),
            observed_block: Some(input.context.current_block),
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
                    "wire_protocol": "eth_direct_raw_v1",
                    "intent_kind": "entry_buy",
                    "executor_boundary": "kartal_eth_tx_executor",
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
                        "action": "entry_buy",
                        "signal": "entry.buy_eligible_pool_once",
                        "status": "selected",
                        "profiles": gas_policy_profile_labels(&gas_rank_policy),
                        "selected_profile": fee.label,
                        "gas_rank_source": fee.source,
                        "guard": "entry_estimated_gas_fee_cap",
                        "estimated_max_cost_eth": fee.estimated_max_cost_eth(estimated_gas_used),
                        "estimated_priority_spend_eth": fee.estimated_priority_spend_eth(estimated_gas_used),
                    },
                    "production_gas_guard": {
                        "required_gas_rank_source": self.gas_policy.required_gas_rank_source.as_str(),
                        "max_priority_fee_gwei": self.gas_policy.max_priority_fee_gwei,
                        "max_estimated_gas_fee_eth": self.gas_policy.entry_max_estimated_gas_fee_eth,
                    },
                    "strategy_gas_rank_policy": gas_rank_policy,
                    "simulation": simulation.metadata(),
                    "source": input.context.tx.source_metadata,
                }),
            },
        })
    }
}

fn planner_error(error: LivePrioritySellPlannerError) -> AlphaCoreError {
    AlphaCoreError::Execution(error.to_string())
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
        return Err(AlphaCoreError::Execution(
            "exact live buy simulation returned zero output".to_string(),
        ));
    }
    let min_output = derive_min_output_from_expected_output(expected_output, max_slippage_bps)
        .map_err(planner_error)?;
    if min_output.is_zero() {
        return Err(AlphaCoreError::Execution(
            "derived live buy min-output is zero".to_string(),
        ));
    }
    Ok(min_output)
}

fn ensure_simulation_ok(simulation: &PreSubmitSimulation) -> eth_alpha_core::error::Result<()> {
    if simulation.would_revert {
        return Err(AlphaCoreError::Execution(format!(
            "exact live transaction simulation would revert at block {}",
            simulation.block_number
        )));
    }
    Ok(())
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
        AlphaCoreError::Execution(
            format!(
                "live real buy planner has no gas fee candidate matching production gas guard: required_source={} max_priority_fee_gwei={} max_estimated_gas_fee_eth={} candidates={}",
                gas_policy.required_gas_rank_source,
                gas_policy.max_priority_fee_gwei,
                gas_policy.entry_max_estimated_gas_fee_eth,
                serde_json::to_string(&plan.candidates).unwrap_or_else(|_| "[]".to_string())
            ),
        )
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

pub(super) struct KartalRealPreflight {
    pub(super) token: String,
    pub(super) status: KartalEthTxExecutorStatus,
}

pub(super) async fn preflight_kartal_real(
    args: &RealExecutionArgs,
    live_args: &Args,
    strategy_specs: &[LiveStrategySpec],
) -> Result<KartalRealPreflight> {
    let token = load_kartal_bearer_token(&args.kartal_token_env)?;
    let status = KartalClient::new(KartalClientConfig::new(&args.kartal_url, token.clone()))
        .eth_tx_status()
        .await
        .wrap_err("failed to read Kartal ETH tx executor status")?;
    validate_kartal_real_status(&status, args, live_args, strategy_specs)?;
    Ok(KartalRealPreflight { token, status })
}

pub(super) async fn build_kartal_real_adapter(
    args: &RealExecutionArgs,
    preflight: KartalRealPreflight,
    chain_server_url: String,
    store: PostgresTradingStore,
    run_id: String,
    valuation_adapter: LiveChainSimExecutionAdapter,
    exact_pre_submit_live_simulator: tx_simulator::LiveTxSimulator,
    pools: Arc<std::sync::Mutex<HashMap<TokenPoolId, PoolSnapshot>>>,
    current_block: Arc<AtomicU64>,
    gas_policy: LiveRealGasPolicy,
) -> Result<Box<dyn EngineExecutionAdapter>> {
    let from = parse_live_real_address(&args.live_real_from, "--live-real-from")?;
    let vault =
        parse_live_real_address(&args.live_real_vault_address, "--live-real-vault-address")?;
    let pre_submit_simulator =
        UniswapV2TradingVaultPreSubmitSimulator::new(exact_pre_submit_live_simulator, vault);
    let gas_rank_provider = ChainServerGasRankProvider::new(chain_server_url)
        .with_lookback_blocks(gas_policy.gas_rank_lookback_blocks);

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
    planner_config.mempool_pre_mine_gas_rank_policy =
        gas_policy.mempool_pre_mine_gas_rank_policy.clone();
    planner_config.mined_approval_race_gas_rank_policy =
        gas_policy.mined_approval_race_gas_rank_policy.clone();
    planner_config.buy_confirm_block_approval_gas_rank_policy = gas_policy
        .buy_confirm_block_approval_gas_rank_policy
        .clone();
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
    };
    let submitter = KartalExecutorClient::new(KartalExecutorClientConfig::new(
        &args.kartal_url,
        preflight.token,
    ));
    let executor = TxExecutorAdapter::with_order_prefix(real_planner, submitter, run_id);
    Ok(Box::new(RealExecutionWithValuation::new(
        executor,
        valuation_adapter,
    )))
}

fn validate_kartal_real_status(
    status: &KartalEthTxExecutorStatus,
    args: &RealExecutionArgs,
    live_args: &Args,
    strategy_specs: &[LiveStrategySpec],
) -> Result<()> {
    if status.execution_disabled {
        return Err(eyre!("Kartal ETH tx executor kill switch is active"));
    }
    if !status.enabled {
        return Err(eyre!("Kartal ETH tx executor is disabled"));
    }
    if !status.signer_available {
        return Err(eyre!("Kartal ETH signer is not available"));
    }
    if status.policy.allowed_target_count == 0 || status.policy.allowed_selector_count == 0 {
        return Err(eyre!(
            "Kartal ETH tx policy must have non-empty target and selector allowlists"
        ));
    }
    match status.broadcast_mode {
        KartalStatusBroadcastMode::DryRun => Ok(()),
        KartalStatusBroadcastMode::PublicMempool if args.allow_public_mempool_live_validation => {
            validate_public_mempool_live_validation(status, live_args, strategy_specs)
        }
        KartalStatusBroadcastMode::PublicMempool => Err(eyre!(
            "kartal-real trader requires broadcast_mode=dry_run unless --allow-public-mempool-live-validation is set for the hold3 validation strategy"
        )),
        KartalStatusBroadcastMode::Unknown => Err(eyre!(
            "kartal-real trader cannot run with unknown Kartal broadcast_mode"
        )),
    }
}

fn validate_public_mempool_live_validation(
    status: &KartalEthTxExecutorStatus,
    args: &Args,
    strategy_specs: &[LiveStrategySpec],
) -> Result<()> {
    if args.strategy_set.as_deref() != Some(HOLD3_VALIDATION_STRATEGY_NAME) {
        return Err(eyre!(
            "public mempool validation requires --strategy-set {HOLD3_VALIDATION_STRATEGY_NAME}"
        ));
    }
    if strategy_specs.len() != 1
        || strategy_specs[0].strategy_name != HOLD3_VALIDATION_STRATEGY_NAME
    {
        return Err(eyre!(
            "public mempool validation requires exactly one resolved strategy spec named {HOLD3_VALIDATION_STRATEGY_NAME}"
        ));
    }
    let spec = &strategy_specs[0];
    if spec.max_entry_pools != Some(LIVE_VALIDATION_MAX_ENTRY_POOLS) {
        return Err(eyre!(
            "public mempool validation requires strategy spec max_entry_pools={LIVE_VALIDATION_MAX_ENTRY_POOLS}"
        ));
    }
    if args.disable_entry {
        return Err(eyre!(
            "public mempool validation requires entries enabled for the single validation buy"
        ));
    }
    if args.once {
        return Err(eyre!(
            "public mempool validation must keep running after the buy so receipt reconciliation and hold3 sell can complete"
        ));
    }
    if args.replay_current {
        return Err(eyre!(
            "public mempool validation must not use --replay-current; start from fresh live observations only"
        ));
    }

    let buy_wei = parse_policy_wei(&spec.buy_wei, "strategy buy_wei")?;
    let max_buy_wei = parse_policy_wei(LIVE_VALIDATION_BUY_WEI, "validation buy cap")?;
    if buy_wei.is_zero() || buy_wei > max_buy_wei {
        return Err(eyre!(
            "public mempool validation requires 0 < strategy buy_wei <= {LIVE_VALIDATION_BUY_WEI}; got {}",
            spec.buy_wei
        ));
    }

    let entry_bankroll_eth = spec
        .entry_bankroll_eth
        .as_deref()
        .ok_or_else(|| eyre!("public mempool validation requires strategy entry_bankroll_eth"))?;
    let entry_bankroll_wei = super::support::parse_eth_decimal_to_wei(
        entry_bankroll_eth,
        "strategy entry_bankroll_eth",
    )?;
    if entry_bankroll_wei.is_zero() || entry_bankroll_wei > max_buy_wei {
        return Err(eyre!(
            "public mempool validation requires entry bankroll in (0, {LIVE_VALIDATION_ENTRY_BANKROLL_ETH}] ETH; got {entry_bankroll_eth}"
        ));
    }

    let max_value_wei = parse_policy_wei(&status.policy.max_value_wei, "policy max_value_wei")?;
    if max_value_wei < buy_wei {
        return Err(eyre!(
            "Kartal max_value_wei {} is below validation buy value {buy_wei}",
            status.policy.max_value_wei
        ));
    }
    let max_transaction_cost_wei = parse_policy_wei(
        &status.policy.max_transaction_cost_wei,
        "policy max_transaction_cost_wei",
    )?;
    if max_transaction_cost_wei.is_zero() {
        return Err(eyre!(
            "Kartal max_transaction_cost_wei must be nonzero for public mempool validation"
        ));
    }
    let max_daily_cost_wei = parse_policy_wei(
        &status.policy.max_daily_cost_wei,
        "policy max_daily_cost_wei",
    )?;
    if max_daily_cost_wei < max_transaction_cost_wei {
        return Err(eyre!(
            "Kartal max_daily_cost_wei {} is below max_transaction_cost_wei {}",
            status.policy.max_daily_cost_wei,
            status.policy.max_transaction_cost_wei
        ));
    }
    if !status.policy.require_simulation || status.policy.max_simulation_age_blocks > 2 {
        return Err(eyre!(
            "public mempool validation requires fresh simulation policy: require_simulation=true and max_simulation_age_blocks <= 2"
        ));
    }
    Ok(())
}

fn parse_policy_wei(value: &str, label: &str) -> Result<U256> {
    U256::from_str_radix(value.trim(), 10)
        .wrap_err_with(|| format!("invalid {label} decimal wei value {value:?}"))
}

fn load_kartal_bearer_token(token_env: &str) -> Result<String> {
    let token = std::env::var(token_env)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| eyre!("missing Kartal bearer token in {token_env}"))?;
    Ok(token)
}

fn parse_live_real_address(value: &str, label: &str) -> Result<Address> {
    value
        .parse::<Address>()
        .wrap_err_with(|| format!("invalid {label} address {value:?}"))
}

#[cfg(test)]
mod tests {
    use eth_live_trading::{
        KartalDailySpendStatus, KartalEthTxPolicyStatus, KartalStatusBroadcastMode,
    };
    use eth_strategies::shared_rules::live::{strategy_set_specs, LiveStrategySpecOptions};

    use super::*;

    fn live_args() -> Args {
        Args {
            poll_interval_ms: 2_000,
            mempool_since_days: 14,
            signal_limit: 200,
            run_id: None,
            disable_entry: false,
            replay_current: false,
            once: false,
            strategy_set: Some(HOLD3_VALIDATION_STRATEGY_NAME.to_string()),
        }
    }

    fn specs(args: &Args) -> Vec<LiveStrategySpec> {
        strategy_set_specs(
            args.strategy_set.as_deref().expect("test strategy set"),
            &LiveStrategySpecOptions,
        )
        .expect("test strategy specs")
    }

    fn real_args(allow_public_mempool_live_validation: bool) -> RealExecutionArgs {
        RealExecutionArgs {
            kartal_url: "http://127.0.0.1:5004".to_string(),
            kartal_token_env: "ETH_TX_EXECUTOR_API_TOKEN".to_string(),
            live_real_from: "0x2348E8a3A21DBe64Ace84853D7b4B696E8A1fC27".to_string(),
            live_real_vault_address: "0x28474cbCd780AeEb3ED1501B68254bEd87cF5597".to_string(),
            allow_public_mempool_live_validation,
        }
    }

    fn status(mode: KartalStatusBroadcastMode) -> KartalEthTxExecutorStatus {
        KartalEthTxExecutorStatus {
            service: "eth_tx_executor".to_string(),
            enabled: true,
            api_token_configured: true,
            signer_available: true,
            execution_disabled: false,
            broadcast_mode: mode,
            chain_id: 1,
            rpc_url: "http://172.18.0.1:8545".to_string(),
            journal_path: Some("/data/eth-tx-executions.jsonl".to_string()),
            direct_raw_endpoint: "/eth/tx/direct-raw".to_string(),
            policy: KartalEthTxPolicyStatus {
                version: "eth_tx_policy_v1".to_string(),
                allowed_from_count: 1,
                allowed_target_count: 1,
                allowed_selector_count: 2,
                max_value_wei: LIVE_VALIDATION_BUY_WEI.to_string(),
                max_gas_limit: 500_000,
                max_fee_per_gas_wei: "1000000000000".to_string(),
                max_priority_fee_per_gas_wei: "500000000000".to_string(),
                max_transaction_cost_wei: "30000000000000000".to_string(),
                max_daily_cost_wei: "50000000000000000".to_string(),
                daily_spend: KartalDailySpendStatus {
                    spend_day: "2026-05-21".to_string(),
                    spent_wei: "0".to_string(),
                    remaining_daily_cost_wei: Some("50000000000000000".to_string()),
                },
                require_simulation: true,
                max_simulation_age_blocks: 2,
                required_metadata_fields: vec![
                    "wire_protocol".to_string(),
                    "intent_kind".to_string(),
                    "strategy_name".to_string(),
                    "strategy_run_id".to_string(),
                    "trade_id".to_string(),
                    "token_address".to_string(),
                    "pool_address".to_string(),
                ],
            },
        }
    }

    #[test]
    fn dry_run_status_is_allowed_without_public_validation_flag() {
        validate_kartal_real_status(
            &status(KartalStatusBroadcastMode::DryRun),
            &real_args(false),
            &live_args(),
            &specs(&live_args()),
        )
        .unwrap();
    }

    #[test]
    fn public_mempool_requires_explicit_validation_flag() {
        let error = validate_kartal_real_status(
            &status(KartalStatusBroadcastMode::PublicMempool),
            &real_args(false),
            &live_args(),
            &specs(&live_args()),
        )
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("--allow-public-mempool-live-validation"));
    }

    #[test]
    fn public_mempool_validation_requires_hold3_single_pool_scope() {
        let mut args = live_args();
        args.strategy_set = Some("alpha11-univ2-lp30-pool-update-block-hold15".to_string());

        let error = validate_kartal_real_status(
            &status(KartalStatusBroadcastMode::PublicMempool),
            &real_args(true),
            &args,
            &specs(&args),
        )
        .unwrap_err();

        assert!(error.to_string().contains(HOLD3_VALIDATION_STRATEGY_NAME));
    }

    #[test]
    fn public_mempool_validation_accepts_hold3_single_pool_scope() {
        validate_kartal_real_status(
            &status(KartalStatusBroadcastMode::PublicMempool),
            &real_args(true),
            &live_args(),
            &specs(&live_args()),
        )
        .unwrap();
    }

    #[test]
    fn public_mempool_validation_requires_value_cap_for_buy() {
        let mut status = status(KartalStatusBroadcastMode::PublicMempool);
        status.policy.max_value_wei = "0".to_string();

        let error = validate_kartal_real_status(
            &status,
            &real_args(true),
            &live_args(),
            &specs(&live_args()),
        )
        .unwrap_err();

        assert!(error.to_string().contains("max_value_wei"));
    }
}
