use alloy_primitives::U256;
use async_trait::async_trait;
use eth_alpha_core::{
    order::{OrderIntent, OrderSide},
    position::Position,
};
use serde_json::{json, Value};

use crate::{
    derive_min_output_from_expected_output, prepare_priority_sell, GasEstimateConfig,
    LpSignalSource, PreSubmitSimulation, PreparedSellRoute, PrioritySellPlan, PrioritySellTxPrep,
    SellUrgency, TxPrepOutcome,
};

use super::{
    AllowanceChecker, GasRankProvider, LivePrioritySellPlannerConfig, LivePrioritySellPlannerError,
    LivePrioritySellPlannerInput, PreSubmitSimulator, PrioritySellPlannerOutcome, SellRouteBuilder,
};

#[async_trait]
pub trait PrioritySellPlanner: Send + Sync {
    async fn plan_priority_sell(
        &self,
        input: LivePrioritySellPlannerInput,
    ) -> Result<PrioritySellPlannerOutcome, LivePrioritySellPlannerError>;
}

pub struct LivePrioritySellPlanner<R, S, G, A> {
    config: LivePrioritySellPlannerConfig,
    route_builder: R,
    simulator: S,
    gas_rank: G,
    allowance: A,
}

impl<R, S, G, A> LivePrioritySellPlanner<R, S, G, A> {
    pub fn new(
        config: LivePrioritySellPlannerConfig,
        route_builder: R,
        simulator: S,
        gas_rank: G,
        allowance: A,
    ) -> Self {
        Self {
            config,
            route_builder,
            simulator,
            gas_rank,
            allowance,
        }
    }
}

#[async_trait]
impl<R, S, G, A> PrioritySellPlanner for LivePrioritySellPlanner<R, S, G, A>
where
    R: SellRouteBuilder,
    S: PreSubmitSimulator,
    G: GasRankProvider,
    A: AllowanceChecker,
{
    async fn plan_priority_sell(
        &self,
        mut input: LivePrioritySellPlannerInput,
    ) -> Result<PrioritySellPlannerOutcome, LivePrioritySellPlannerError> {
        validate_position_matches_intent(&input.intent, &input.position)?;
        if !input.source_metadata.is_null() {
            input.context.tx.source_metadata = json!({
                "planner_source": input.context.tx.source_metadata,
                "input_source": input.source_metadata,
            });
        }

        let mut route = self.route_builder.build_route(&input).await?;
        if should_derive_vault_min_output(&input, &route) {
            let quote_simulation = self.simulator.simulate(&input, &route).await?;
            let min_output = derive_min_output_from_quote_simulation(
                &quote_simulation,
                input.intent.max_slippage_bps,
            )?;
            input.min_output_amount = Some(min_output.to_string());
            attach_min_output_quote_metadata(&mut input, &quote_simulation, min_output);
            route = self.route_builder.build_route(&input).await?;
        }

        let allowance = self.allowance.check_allowance(&input, route).await?;
        if self.config.require_existing_allowance {
            allowance.decision.ensure_preapproved()?;
        }
        let mut route = allowance.route;
        let simulation = self.simulator.simulate(&input, &route).await?;
        apply_simulated_gas_used(&mut route, &simulation, &self.config.gas_estimate)?;
        let gas_rank = self.gas_rank.ranked_fee_candidates(&input, &route).await?;
        let plan = priority_sell_plan(&self.config, &input);
        let gas_rank_policy = priority_sell_gas_rank_policy(&self.config, &plan);

        let tx_prep = PrioritySellTxPrep {
            context: input.context.tx.clone(),
            plan,
            route,
            simulation,
            predicted_base_fee_gwei: gas_rank.predicted_base_fee_gwei,
            expected_late_recovery_eth: self.config.expected_late_recovery_eth,
            ranked_fee_candidates: gas_rank.candidates,
            gas_rank_policy: Some(gas_rank_policy),
        };

        match prepare_priority_sell(&self.config.tx_prep, tx_prep) {
            TxPrepOutcome::Submit { signal, budget } => {
                Ok(PrioritySellPlannerOutcome::Submit { signal, budget })
            }
            TxPrepOutcome::Reject(reject) => Ok(PrioritySellPlannerOutcome::Reject(reject)),
        }
    }
}

fn apply_simulated_gas_used(
    route: &mut PreparedSellRoute,
    simulation: &PreSubmitSimulation,
    gas_estimate: &GasEstimateConfig,
) -> Result<(), LivePrioritySellPlannerError> {
    let gas_used = simulation.gas_used.ok_or_else(|| {
        LivePrioritySellPlannerError::Simulation(
            "exact pre-submit simulation did not report gas_used; fallback gas estimates are not allowed"
                .to_string(),
        )
    })?;
    route
        .apply_simulated_gas_used(gas_used, gas_estimate)
        .map_err(|error| LivePrioritySellPlannerError::Route(error.to_string()))
}

fn should_derive_vault_min_output(
    input: &LivePrioritySellPlannerInput,
    route: &PreparedSellRoute,
) -> bool {
    input
        .min_output_amount
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
        && route.protocol == "uniswap_v2_trading_vault"
}

fn derive_min_output_from_quote_simulation(
    simulation: &PreSubmitSimulation,
    max_slippage_bps: u32,
) -> Result<U256, LivePrioritySellPlannerError> {
    simulation.validate().map_err(|error| {
        LivePrioritySellPlannerError::Simulation(format!(
            "cannot derive min-output from failed provisional simulation: {error}"
        ))
    })?;
    let expected_output = simulation
        .expected_output_amount
        .as_deref()
        .ok_or_else(|| {
            LivePrioritySellPlannerError::Simulation(
                "provisional simulation did not return expected_output_amount".to_string(),
            )
        })
        .and_then(|value| parse_u256_quantity(value, "expected_output_amount"))?;
    let min_output = derive_min_output_from_expected_output(expected_output, max_slippage_bps)
        .map_err(|error| {
            LivePrioritySellPlannerError::Simulation(format!(
                "failed to derive min-output from provisional simulation: {error}"
            ))
        })?;
    if min_output.is_zero() {
        return Err(LivePrioritySellPlannerError::Simulation(
            "simulation-derived min-output is zero".to_string(),
        ));
    }
    Ok(min_output)
}

fn attach_min_output_quote_metadata(
    input: &mut LivePrioritySellPlannerInput,
    simulation: &PreSubmitSimulation,
    min_output: U256,
) {
    let metadata = json!({
        "provider": "exact_pre_submit_simulation",
        "simulation_block": simulation.block_number,
        "expected_output_token": simulation.expected_output_token,
        "expected_output_amount": simulation.expected_output_amount,
        "min_output_amount": min_output.to_string(),
        "max_slippage_bps": input.intent.max_slippage_bps,
    });

    match &mut input.context.tx.source_metadata {
        Value::Object(map) => {
            map.insert("min_output_quote".to_string(), metadata);
        }
        existing if existing.is_null() => {
            input.context.tx.source_metadata = json!({ "min_output_quote": metadata });
        }
        existing => {
            let previous = existing.take();
            input.context.tx.source_metadata = json!({
                "previous": previous,
                "min_output_quote": metadata,
            });
        }
    }
}

fn parse_u256_quantity(value: &str, label: &str) -> Result<U256, LivePrioritySellPlannerError> {
    let trimmed = value.trim();
    let parsed = if let Some(hex) = trimmed.strip_prefix("0x") {
        U256::from_str_radix(hex, 16)
    } else {
        U256::from_str_radix(trimmed, 10)
    };
    parsed.map_err(|error| {
        LivePrioritySellPlannerError::Simulation(format!(
            "invalid {label} quantity {trimmed:?}: {error}"
        ))
    })
}

fn validate_position_matches_intent(
    intent: &OrderIntent,
    position: &Position,
) -> Result<(), LivePrioritySellPlannerError> {
    if intent.side != OrderSide::Sell {
        return Err(LivePrioritySellPlannerError::UnsupportedIntent(
            "priority sell planner only supports sell intents".to_string(),
        ));
    }
    if !position.can_submit_exit() {
        return Err(LivePrioritySellPlannerError::InvalidInput(format!(
            "position {} cannot submit exit from state {:?}",
            position.id.0, position.state
        )));
    }
    if position.key.strategy_name != intent.strategy_name
        || position.key.token_address != intent.token_address
        || position.key.pool_address != intent.pool_address
    {
        return Err(LivePrioritySellPlannerError::InvalidInput(
            "sell intent does not match open position key".to_string(),
        ));
    }
    Ok(())
}

fn priority_sell_plan(
    config: &LivePrioritySellPlannerConfig,
    input: &LivePrioritySellPlannerInput,
) -> PrioritySellPlan {
    let reason = input
        .intent
        .decision_reason
        .as_ref()
        .map(|reason| reason.code.clone())
        .unwrap_or_else(|| "exit.live_priority_sell".to_string());
    let (signal_source, urgency) = priority_sell_classification(&reason);

    PrioritySellPlan {
        trade_id: input.position.trade_id.clone(),
        token_address: input.intent.token_address,
        pool_address: input.intent.pool_address.clone(),
        observed_block: input.context.current_block,
        signal_source,
        urgency,
        route: config.priority_route.clone(),
        max_priority_fee_per_gas_gwei: config.max_priority_fee_per_gas_gwei,
        max_total_fee_eth: config.max_total_fee_eth,
        reason,
    }
}

fn priority_sell_classification(reason: &str) -> (LpSignalSource, SellUrgency) {
    match reason {
        "exit.mempool_liquidity_removal_signal" => (
            LpSignalSource::MempoolLpApproval,
            SellUrgency::MempoolPreMine,
        ),
        "exit.lp_approval" | "exit.lp_approval_mined_race" => (
            LpSignalSource::MinedLpApproval,
            SellUrgency::MinedApprovalRace,
        ),
        "exit.lp_approval_buy_confirm_block" => (
            LpSignalSource::MinedLpApproval,
            SellUrgency::BuyConfirmBlockApproval,
        ),
        _ => (LpSignalSource::StrategyExit, SellUrgency::NormalExit),
    }
}

fn priority_sell_gas_rank_policy(
    config: &LivePrioritySellPlannerConfig,
    plan: &PrioritySellPlan,
) -> crate::StrategyGasRankPolicy {
    match plan.urgency {
        SellUrgency::NormalExit => config.normal_exit_gas_rank_policy.clone(),
        SellUrgency::MempoolPreMine => config.mempool_pre_mine_gas_rank_policy.clone(),
        SellUrgency::MinedApprovalRace | SellUrgency::BuyConfirmBlockApproval => {
            config.lp_approval_exit_gas_rank_policy.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{Address, U256};
    use eth_alpha_core::{
        amount::{Amount, DecimalAmount},
        ids::{PoolAddress, PortfolioId, PositionId, StrategyName, TradeId, WalletId},
        market::{PoolProtocol, PoolSnapshot},
        order::OrderIntent,
        position::{Position, PositionKey},
    };
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::{
        FixedGasRankProvider, FixedPreSubmitSimulator, GasRankPlan, PlannerTxContext,
        RankedFeeCandidate, RouteBuildRequest, StaticAllowanceChecker, TxPrepRequestContext,
        UniswapV2SellRouteBuilder, UniswapV2TradingVaultSellRouteBuilder,
        VaultInternalAllowanceChecker, UNISWAP_V2_DIRECT_SELL_GAS_LIMIT,
        UNISWAP_V2_TRADING_VAULT_SELL_GAS_LIMIT,
    };

    fn token() -> Address {
        Address::with_last_byte(0x11)
    }

    fn pool_contract() -> Address {
        Address::with_last_byte(0x22)
    }

    fn pool_address() -> PoolAddress {
        PoolAddress::new(token(), pool_contract().to_string())
    }

    fn intent() -> OrderIntent {
        OrderIntent {
            trade_id: Some(TradeId("trd_test".to_string())),
            portfolio_id: PortfolioId("portfolio".to_string()),
            wallet_id: WalletId("wallet".to_string()),
            strategy_name: StrategyName("alpha11-univ2-lp30-pool-update-block-hold20".to_string()),
            side: OrderSide::Sell,
            token_address: token(),
            pool_address: pool_address(),
            protocol: PoolProtocol::UniswapV2,
            amount: Amount {
                raw: U256::from(1_000_000_000_000_000_000u128),
                decimals: 18,
            },
            route: None,
            max_slippage_bps: 500,
            deadline_secs: 60,
            decision_reason: None,
        }
    }

    fn position() -> Position {
        let mut position = Position::with_trade_id(
            PositionId("trd_test".to_string()),
            TradeId("trd_test".to_string()),
            PositionKey {
                portfolio_id: PortfolioId("portfolio".to_string()),
                wallet_id: WalletId("wallet".to_string()),
                strategy_name: StrategyName(
                    "alpha11-univ2-lp30-pool-update-block-hold20".to_string(),
                ),
                token_address: token(),
                pool_address: pool_address(),
                protocol: PoolProtocol::UniswapV2,
            },
        );
        position.state = eth_alpha_core::position::PositionState::BuyConfirmed;
        position
    }

    fn pool() -> PoolSnapshot {
        PoolSnapshot {
            address: pool_address(),
            token_address: token(),
            protocol: PoolProtocol::UniswapV2,
            denom_address: Some(alloy_primitives::address!(
                "C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
            )),
            denom_symbol: Some("WETH".to_string()),
            denom_reserve: DecimalAmount::from(10),
            token_reserve: DecimalAmount::from(100),
            price_denom_per_token: Some(DecimalAmount::new(1, 1)),
            initial_price_denom_per_token: Some(DecimalAmount::new(1, 1)),
            price_ratio_to_initial: Some(DecimalAmount::from(1)),
            creation_block: Some(25_128_246),
            token_decimals: Some(18),
            fee_tier: None,
            uniswap_v4: None,
            latest_block: 25_128_246,
            can_buy: true,
            can_sell: true,
            is_scam: false,
        }
    }

    fn input() -> LivePrioritySellPlannerInput {
        LivePrioritySellPlannerInput {
            context: PlannerTxContext {
                tx: TxPrepRequestContext {
                    chain_id: 1,
                    from: Address::with_last_byte(0x33).to_string(),
                    strategy_name: "alpha11-univ2-lp30-pool-update-block-hold20".to_string(),
                    strategy_run_id: Some("run-1".to_string()),
                    observed_block: Some(25_128_246),
                    source_metadata: json!({ "signal_id": 222 }),
                },
                current_block: 25_128_246,
                deadline_unix_secs: 1_800_000_000,
            },
            intent: intent(),
            position: position(),
            pool: pool(),
            min_output_amount: Some("9000000000000000".to_string()),
            source_metadata: json!({ "planner": "test" }),
        }
    }

    fn planner(
        allowance: StaticAllowanceChecker,
    ) -> LivePrioritySellPlanner<
        UniswapV2SellRouteBuilder,
        FixedPreSubmitSimulator,
        FixedGasRankProvider,
        StaticAllowanceChecker,
    > {
        LivePrioritySellPlanner::new(
            LivePrioritySellPlannerConfig::default(),
            UniswapV2SellRouteBuilder::new(RouteBuildRequest::new(
                UNISWAP_V2_DIRECT_SELL_GAS_LIMIT,
            )),
            FixedPreSubmitSimulator::new(crate::PreSubmitSimulation {
                block_number: 25_128_246,
                block_hash: Some("0xabc".to_string()),
                state_root: None,
                expected_output_token: Some("WETH".to_string()),
                expected_output_amount: Some("10000000000000000".to_string()),
                min_output_amount: Some("9000000000000000".to_string()),
                expected_recovery_eth: DecimalAmount::new(1, 2),
                gas_used: Some(150_000),
                would_revert: false,
                metadata: json!({ "sim": "ok" }),
            }),
            FixedGasRankProvider::new(GasRankPlan {
                predicted_base_fee_gwei: DecimalAmount::from(10),
                candidates: vec![
                    RankedFeeCandidate {
                        label: "p50".to_string(),
                        priority_fee_gwei: DecimalAmount::from(2),
                        max_fee_per_gas_gwei: DecimalAmount::from(12),
                        rank_position_p50: Some(25),
                        gas_before_p50: Some(900_000),
                        likely_fits_at_p50: Some(true),
                        source: Some("test".to_string()),
                        metadata: None,
                    },
                    RankedFeeCandidate {
                        label: "p90".to_string(),
                        priority_fee_gwei: DecimalAmount::from(30),
                        max_fee_per_gas_gwei: DecimalAmount::from(40),
                        rank_position_p50: Some(10),
                        gas_before_p50: Some(450_000),
                        likely_fits_at_p50: Some(true),
                        source: Some("test".to_string()),
                        metadata: None,
                    },
                ],
            }),
            allowance,
        )
    }

    fn simulator() -> FixedPreSubmitSimulator {
        FixedPreSubmitSimulator::new(crate::PreSubmitSimulation {
            block_number: 25_128_246,
            block_hash: Some("0xabc".to_string()),
            state_root: None,
            expected_output_token: Some("WETH".to_string()),
            expected_output_amount: Some("10000000000000000".to_string()),
            min_output_amount: Some("9000000000000000".to_string()),
            expected_recovery_eth: DecimalAmount::new(1, 2),
            gas_used: Some(150_000),
            would_revert: false,
            metadata: json!({ "sim": "ok" }),
        })
    }

    fn gas_rank() -> FixedGasRankProvider {
        FixedGasRankProvider::new(GasRankPlan {
            predicted_base_fee_gwei: DecimalAmount::from(10),
            candidates: vec![
                RankedFeeCandidate {
                    label: "p50".to_string(),
                    priority_fee_gwei: DecimalAmount::from(2),
                    max_fee_per_gas_gwei: DecimalAmount::from(12),
                    rank_position_p50: Some(25),
                    gas_before_p50: Some(900_000),
                    likely_fits_at_p50: Some(true),
                    source: Some("test".to_string()),
                    metadata: None,
                },
                RankedFeeCandidate {
                    label: "p90".to_string(),
                    priority_fee_gwei: DecimalAmount::from(30),
                    max_fee_per_gas_gwei: DecimalAmount::from(40),
                    rank_position_p50: Some(10),
                    gas_before_p50: Some(450_000),
                    likely_fits_at_p50: Some(true),
                    source: Some("test".to_string()),
                    metadata: None,
                },
            ],
        })
    }

    #[test]
    fn max_hold_exit_is_normal_strategy_exit_not_mempool_race() {
        let (signal_source, urgency) = priority_sell_classification("exit.max_hold_active_blocks");

        assert_eq!(signal_source, LpSignalSource::StrategyExit);
        assert_eq!(urgency, SellUrgency::NormalExit);
    }

    #[test]
    fn lp_approval_exit_keeps_race_urgency() {
        for reason in ["exit.lp_approval", "exit.lp_approval_mined_race"] {
            let (signal_source, urgency) = priority_sell_classification(reason);

            assert_eq!(signal_source, LpSignalSource::MinedLpApproval);
            assert_eq!(urgency, SellUrgency::MinedApprovalRace);
        }
    }

    #[test]
    fn buy_confirm_lp_approval_exit_uses_shared_lp_approval_bucket() {
        let (signal_source, urgency) =
            priority_sell_classification("exit.lp_approval_buy_confirm_block");

        assert_eq!(signal_source, LpSignalSource::MinedLpApproval);
        assert_eq!(urgency, SellUrgency::BuyConfirmBlockApproval);
    }

    #[test]
    fn mempool_exit_keeps_race_urgency() {
        let (signal_source, urgency) =
            priority_sell_classification("exit.mempool_liquidity_removal_signal");

        assert_eq!(signal_source, LpSignalSource::MempoolLpApproval);
        assert_eq!(urgency, SellUrgency::MempoolPreMine);
    }

    #[derive(Clone, Default)]
    struct RecordingPreSubmitSimulator {
        min_output_calls: Arc<Mutex<Vec<Option<String>>>>,
    }

    #[async_trait]
    impl PreSubmitSimulator for RecordingPreSubmitSimulator {
        async fn simulate(
            &self,
            input: &LivePrioritySellPlannerInput,
            _route: &PreparedSellRoute,
        ) -> Result<PreSubmitSimulation, LivePrioritySellPlannerError> {
            self.min_output_calls
                .lock()
                .unwrap()
                .push(input.min_output_amount.clone());
            Ok(PreSubmitSimulation {
                block_number: 25_128_246,
                block_hash: Some("0xabc".to_string()),
                state_root: None,
                expected_output_token: Some("ETH".to_string()),
                expected_output_amount: Some("10000000000000000".to_string()),
                min_output_amount: input.min_output_amount.clone(),
                expected_recovery_eth: DecimalAmount::new(1, 2),
                gas_used: Some(150_000),
                would_revert: false,
                metadata: json!({ "sim": "ok" }),
            })
        }
    }

    #[tokio::test]
    async fn planner_prepares_value_capped_kartal_signal() {
        let outcome = planner(StaticAllowanceChecker::pre_approved())
            .plan_priority_sell(input())
            .await
            .unwrap();

        match outcome {
            PrioritySellPlannerOutcome::Submit { signal, .. } => {
                assert!(signal
                    .request
                    .to
                    .eq_ignore_ascii_case("0x7a250d5630b4cf539739df2c5dacb4c659f2488d"));
                assert!(signal.request.data.starts_with("0x791ac947"));
                assert_eq!(signal.request.max_priority_fee_per_gas, "2000000000");
                assert_eq!(
                    signal.request.metadata["wire_protocol"],
                    json!("eth_direct_raw_v1")
                );
            }
            other => panic!("expected submit, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn planner_rejects_missing_preapproval() {
        let error = planner(StaticAllowanceChecker::missing())
            .plan_priority_sell(input())
            .await
            .expect_err("missing allowance should reject");

        assert!(error.to_string().contains("pre-existing token allowance"));
    }

    #[tokio::test]
    async fn planner_can_submit_uniswap_v2_trading_vault_route_without_eoa_preapproval() {
        let vault = Address::with_last_byte(0xaa);
        let planner = LivePrioritySellPlanner::new(
            LivePrioritySellPlannerConfig::default(),
            UniswapV2TradingVaultSellRouteBuilder::with_gas_limit(
                vault,
                UNISWAP_V2_TRADING_VAULT_SELL_GAS_LIMIT,
            ),
            simulator(),
            gas_rank(),
            VaultInternalAllowanceChecker,
        );
        let outcome = planner.plan_priority_sell(input()).await.unwrap();

        match outcome {
            PrioritySellPlannerOutcome::Submit { signal, .. } => {
                assert!(signal.request.to.eq_ignore_ascii_case(&vault.to_string()));
                assert!(signal.request.data.starts_with("0x5f413d10"));
                assert_eq!(
                    signal.request.metadata["route"]["estimated_gas_used"],
                    json!(187_500)
                );
                assert_eq!(
                    signal.request.metadata["budget"]["estimated_gas_used"],
                    json!(187_500)
                );
                assert_eq!(
                    signal.request.metadata["simulation"]["gas_used"],
                    json!(150_000)
                );
            }
            other => panic!("expected submit, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn planner_derives_vault_min_output_from_provisional_exact_simulation() {
        let vault = Address::with_last_byte(0xaa);
        let simulator = RecordingPreSubmitSimulator::default();
        let calls = Arc::clone(&simulator.min_output_calls);
        let planner = LivePrioritySellPlanner::new(
            LivePrioritySellPlannerConfig::default(),
            UniswapV2TradingVaultSellRouteBuilder::with_gas_limit(
                vault,
                UNISWAP_V2_TRADING_VAULT_SELL_GAS_LIMIT,
            ),
            simulator,
            gas_rank(),
            VaultInternalAllowanceChecker,
        );
        let mut input = input();
        input.min_output_amount = None;

        let outcome = planner.plan_priority_sell(input).await.unwrap();

        match outcome {
            PrioritySellPlannerOutcome::Submit { signal, .. } => {
                assert_eq!(
                    calls.lock().unwrap().as_slice(),
                    &[None, Some("9500000000000000".to_string())]
                );
                assert_eq!(
                    signal
                        .request
                        .simulation
                        .as_ref()
                        .unwrap()
                        .min_output_amount
                        .as_deref(),
                    Some("9500000000000000")
                );
                assert_eq!(
                    signal.request.metadata["source"]["min_output_quote"]["provider"],
                    json!("exact_pre_submit_simulation")
                );
            }
            other => panic!("expected submit, got {other:?}"),
        }
    }
}
