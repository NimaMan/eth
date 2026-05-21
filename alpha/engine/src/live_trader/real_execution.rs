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
    derive_min_output_from_expected_output, FixedGasRankProvider, GasRankPlan, KartalBribeRequest,
    KartalClient, KartalClientConfig, KartalEthTxExecutorStatus, KartalExecutorClient,
    KartalExecutorClientConfig, KartalSimulationReference, KartalStatusBroadcastMode,
    LiveDirectRawTransactionRequest, LivePrioritySellPlanner, LivePrioritySellPlannerConfig,
    LivePrioritySellPlannerError, LivePrioritySellPlannerInput, LiveTraderTxSignal,
    PlannerTxContext, PreSubmitSimulation, PreSubmitSimulator, RankedFeeCandidate,
    StrategyGasRankDefaults, StrategyGasRankPolicy, TxPrepConfig, TxPrepRequestContext,
    UniswapV2TradingVaultBuyRouteBuilder, UniswapV2TradingVaultPreSubmitSimulator,
    UniswapV2TradingVaultSellRouteBuilder, VaultInternalAllowanceChecker,
};
use eth_strategies::alpha11::{HOLD3_VALIDATION_STRATEGY_NAME, LIVE_VALIDATION_ENTRY_BANKROLL_ETH};
use eyre::{eyre, Result, WrapErr};
use rust_decimal::Decimal;
use serde_json::json;

use crate::execution::real::{
    LiveTradingPlannerBridge, LiveTxPlanner, LiveTxPlanningInputResolver, TxExecutorAdapter,
};
use crate::{EngineExecutionAdapter, LiveChainSimExecutionAdapter, PositionValueSimulation};

use super::cli::{Args, RealExecutionArgs};

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
                        "gas_rank_provider": "shadow_dry_run_only",
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
                        "gas_rank_provider": "shadow_dry_run_only",
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

struct KartalRealPlanner<P> {
    sell_planner: P,
    resolver: LiveRealInputResolver,
    simulator: UniswapV2TradingVaultPreSubmitSimulator,
    buy_route_builder: UniswapV2TradingVaultBuyRouteBuilder,
    gas_plan: GasRankPlan,
}

#[async_trait]
impl<P> LiveTxPlanner for KartalRealPlanner<P>
where
    P: LiveTxPlanner,
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

impl<P> KartalRealPlanner<P> {
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
        let route = self
            .buy_route_builder
            .build_route(&input, min_output)
            .map_err(planner_error)?;
        let simulation = self
            .simulator
            .simulate(&input, &route)
            .await
            .map_err(planner_error)?;
        ensure_simulation_ok(&simulation)?;
        let gas_rank_policy = StrategyGasRankDefaults::entry_buy_policy();
        let fee = select_gas_fee(&self.gas_plan, &gas_rank_policy)?;
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
                        "source": fee.source,
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

fn select_gas_fee(
    plan: &GasRankPlan,
    policy: &StrategyGasRankPolicy,
) -> eth_alpha_core::error::Result<RankedFeeCandidate> {
    policy.choose_candidate(&plan.candidates).ok_or_else(|| {
        AlphaCoreError::Execution(
            "live real buy planner has no gas fee candidate matching strategy gas rank policy"
                .to_string(),
        )
    })
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
) -> Result<KartalRealPreflight> {
    let token = load_kartal_bearer_token(&args.kartal_token_env)?;
    let status = KartalClient::new(KartalClientConfig::new(&args.kartal_url, token.clone()))
        .eth_tx_status()
        .await
        .wrap_err("failed to read Kartal ETH tx executor status")?;
    validate_kartal_real_status(&status, args, live_args)?;
    Ok(KartalRealPreflight { token, status })
}

pub(super) async fn build_kartal_real_adapter(
    args: &RealExecutionArgs,
    preflight: KartalRealPreflight,
    store: PostgresTradingStore,
    run_id: String,
    valuation_adapter: LiveChainSimExecutionAdapter,
    exact_pre_submit_live_simulator: tx_simulator::LiveTxSimulator,
    pools: Arc<std::sync::Mutex<HashMap<TokenPoolId, PoolSnapshot>>>,
    current_block: Arc<AtomicU64>,
) -> Result<Box<dyn EngineExecutionAdapter>> {
    let from = parse_live_real_address(&args.live_real_from, "--live-real-from")?;
    let vault =
        parse_live_real_address(&args.live_real_vault_address, "--live-real-vault-address")?;
    let priority_fee_gwei = args
        .live_real_shadow_priority_fee_gwei
        .parse::<Decimal>()
        .wrap_err("invalid --live-real-shadow-priority-fee-gwei")?;
    let max_fee_gwei = args
        .live_real_shadow_max_fee_gwei
        .parse::<Decimal>()
        .wrap_err("invalid --live-real-shadow-max-fee-gwei")?;
    let predicted_base_fee_gwei = args
        .live_real_shadow_predicted_base_fee_gwei
        .parse::<Decimal>()
        .wrap_err("invalid --live-real-shadow-predicted-base-fee-gwei")?;
    let pre_submit_simulator =
        UniswapV2TradingVaultPreSubmitSimulator::new(exact_pre_submit_live_simulator, vault);
    let gas_rank_plan = GasRankPlan {
        predicted_base_fee_gwei,
        candidates: vec![RankedFeeCandidate {
            label: "aggressive".to_string(),
            priority_fee_gwei,
            max_fee_per_gas_gwei: max_fee_gwei,
            rank_position_p50: None,
            gas_before_p50: None,
            likely_fits_at_p50: None,
            source: Some("shadow_dry_run_only".to_string()),
        }],
    };

    let mut planner_config = LivePrioritySellPlannerConfig::default();
    planner_config.require_existing_allowance = false;
    planner_config.tx_prep = TxPrepConfig {
        max_total_fee_eth: Decimal::new(2, 2),
        max_priority_fee_gwei: Decimal::from(100),
        safety_buffer_eth: Decimal::new(1, 3),
        gas_rank_policy: StrategyGasRankPolicy::urgent_first(),
    };
    planner_config.max_priority_fee_per_gas_gwei = Decimal::from(100);
    planner_config.max_total_fee_eth = Decimal::new(2, 2);

    let planner = LivePrioritySellPlanner::new(
        planner_config,
        UniswapV2TradingVaultSellRouteBuilder::with_default_gas(vault),
        pre_submit_simulator.clone(),
        FixedGasRankProvider::new(gas_rank_plan.clone()),
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
        buy_route_builder: UniswapV2TradingVaultBuyRouteBuilder::with_default_gas(vault),
        gas_plan: gas_rank_plan,
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
            validate_public_mempool_live_validation(status, live_args)
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
) -> Result<()> {
    if args.strategy_set.as_deref() != Some(HOLD3_VALIDATION_STRATEGY_NAME) {
        return Err(eyre!(
            "public mempool validation requires --strategy-set {HOLD3_VALIDATION_STRATEGY_NAME}"
        ));
    }
    if args.max_entry_pools != Some(1) {
        return Err(eyre!(
            "public mempool validation requires --max-entry-pools 1"
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

    let buy_wei = parse_policy_wei(&args.buy_wei, "--buy-wei")?;
    let max_buy_wei = parse_policy_wei(LIVE_VALIDATION_BUY_WEI, "validation buy cap")?;
    if buy_wei.is_zero() || buy_wei > max_buy_wei {
        return Err(eyre!(
            "public mempool validation requires 0 < --buy-wei <= {LIVE_VALIDATION_BUY_WEI}; got {}",
            args.buy_wei
        ));
    }

    let entry_bankroll_eth = args
        .entry_bankroll_eth
        .as_deref()
        .unwrap_or(LIVE_VALIDATION_ENTRY_BANKROLL_ETH);
    let entry_bankroll_wei =
        super::support::parse_eth_decimal_to_wei(entry_bankroll_eth, "--entry-bankroll-eth")?;
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
        .or_else(|| {
            std::env::var("KARTAL_API_TOKEN")
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .ok_or_else(|| eyre!("missing Kartal bearer token in {token_env} or KARTAL_API_TOKEN"))?;
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

    use super::*;

    fn live_args() -> Args {
        Args {
            poll_interval_ms: 2_000,
            mempool_since_days: 14,
            signal_limit: 200,
            buy_wei: LIVE_VALIDATION_BUY_WEI.to_string(),
            min_liquidity_eth: "0.5".to_string(),
            min_liquidity_usd: "1000".to_string(),
            run_id: None,
            disable_entry: false,
            replay_current: false,
            once: false,
            max_entry_pools: Some(1),
            entry_bankroll_eth: Some(LIVE_VALIDATION_ENTRY_BANKROLL_ETH.to_string()),
            stop_loss_ratio: None,
            take_profit_ratio: None,
            strategy_set: Some(HOLD3_VALIDATION_STRATEGY_NAME.to_string()),
        }
    }

    fn real_args(allow_public_mempool_live_validation: bool) -> RealExecutionArgs {
        RealExecutionArgs {
            kartal_url: "http://127.0.0.1:5004".to_string(),
            kartal_token_env: "KARTAL_API_TOKEN".to_string(),
            live_real_from: "0x2348E8a3A21DBe64Ace84853D7b4B696E8A1fC27".to_string(),
            live_real_vault_address: "0x28474cbCd780AeEb3ED1501B68254bEd87cF5597".to_string(),
            live_real_shadow_priority_fee_gwei: "40".to_string(),
            live_real_shadow_max_fee_gwei: "50".to_string(),
            live_real_shadow_predicted_base_fee_gwei: "10".to_string(),
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
        )
        .unwrap();
    }

    #[test]
    fn public_mempool_requires_explicit_validation_flag() {
        let error = validate_kartal_real_status(
            &status(KartalStatusBroadcastMode::PublicMempool),
            &real_args(false),
            &live_args(),
        )
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("--allow-public-mempool-live-validation"));
    }

    #[test]
    fn public_mempool_validation_requires_hold3_single_pool_scope() {
        let mut args = live_args();
        args.strategy_set = Some("alpha11-live-univ2-lp30-pool-update-block-hold15".to_string());

        let error = validate_kartal_real_status(
            &status(KartalStatusBroadcastMode::PublicMempool),
            &real_args(true),
            &args,
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
        )
        .unwrap();
    }

    #[test]
    fn public_mempool_validation_requires_value_cap_for_buy() {
        let mut status = status(KartalStatusBroadcastMode::PublicMempool);
        status.policy.max_value_wei = "0".to_string();

        let error =
            validate_kartal_real_status(&status, &real_args(true), &live_args()).unwrap_err();

        assert!(error.to_string().contains("max_value_wei"));
    }
}
