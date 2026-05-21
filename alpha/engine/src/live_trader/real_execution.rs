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
    PlannerTxContext, PreSubmitSimulation, PreSubmitSimulator, RankedFeeCandidate, TxPrepConfig,
    TxPrepRequestContext, UniswapV2TradingVaultBuyRouteBuilder,
    UniswapV2TradingVaultPreSubmitSimulator, UniswapV2TradingVaultSellRouteBuilder,
    VaultInternalAllowanceChecker,
};
use eyre::{eyre, Result, WrapErr};
use rust_decimal::Decimal;
use serde_json::json;

use crate::execution::real::{
    LiveTradingPlannerBridge, LiveTxPlanner, LiveTxPlanningInputResolver, TxExecutorAdapter,
};
use crate::{EngineExecutionAdapter, LiveChainSimExecutionAdapter, PositionValueSimulation};

use super::cli::RealExecutionArgs;

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
        let fee = fixed_gas_fee(&self.gas_plan)?;
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

fn fixed_gas_fee(plan: &GasRankPlan) -> eth_alpha_core::error::Result<RankedFeeCandidate> {
    plan.candidates.first().cloned().ok_or_else(|| {
        AlphaCoreError::Execution("live real buy planner has no gas fee candidate".to_string())
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

pub(super) async fn preflight_kartal_real(args: &RealExecutionArgs) -> Result<KartalRealPreflight> {
    let token = load_kartal_bearer_token(&args.kartal_token_env)?;
    let status = KartalClient::new(KartalClientConfig::new(&args.kartal_url, token.clone()))
        .eth_tx_status()
        .await
        .wrap_err("failed to read Kartal ETH tx executor status")?;
    validate_kartal_real_status(&status)?;
    Ok(KartalRealPreflight { token, status })
}

pub(super) async fn build_kartal_real_adapter(
    args: &RealExecutionArgs,
    preflight: KartalRealPreflight,
    store: PostgresTradingStore,
    run_id: String,
    valuation_adapter: LiveChainSimExecutionAdapter,
    reth_datadir: &str,
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
    let pre_submit_simulator = UniswapV2TradingVaultPreSubmitSimulator::new(
        tx_simulator::LiveTxSimulator::new(reth_datadir)
            .wrap_err("failed to initialize exact pre-submit tx simulator")?,
        vault,
    );
    let gas_rank_plan = GasRankPlan {
        predicted_base_fee_gwei,
        candidates: vec![RankedFeeCandidate {
            label: "shadow_dry_run_fixed".to_string(),
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

fn validate_kartal_real_status(status: &KartalEthTxExecutorStatus) -> Result<()> {
    if status.execution_disabled {
        return Err(eyre!("Kartal ETH tx executor kill switch is active"));
    }
    if !status.enabled {
        return Err(eyre!("Kartal ETH tx executor is disabled"));
    }
    if !status.signer_available {
        return Err(eyre!("Kartal ETH signer is not available"));
    }
    if status.broadcast_mode != KartalStatusBroadcastMode::DryRun {
        return Err(eyre!(
            "kartal-real trader currently requires Kartal broadcast_mode=dry_run; got {:?}",
            status.broadcast_mode
        ));
    }
    if status.policy.allowed_target_count == 0 || status.policy.allowed_selector_count == 0 {
        return Err(eyre!(
            "Kartal ETH tx policy must have non-empty target and selector allowlists"
        ));
    }
    Ok(())
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
