use async_trait::async_trait;
use eth_alpha_core::{
    order::{OrderIntent, OrderSide},
    position::Position,
};
use serde_json::json;

use crate::{
    LpSignalSource, PrioritySellPlan, PrioritySellTxPrep, TxPrepOutcome, prepare_priority_sell,
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

        let route = self.route_builder.build_route(&input).await?;
        let allowance = self.allowance.check_allowance(&input, route).await?;
        if self.config.require_existing_allowance {
            allowance.decision.ensure_preapproved()?;
        }
        let route = allowance.route;
        let simulation = self.simulator.simulate(&input, &route).await?;
        let gas_rank = self.gas_rank.ranked_fee_candidates(&input, &route).await?;
        let plan = priority_sell_plan(&self.config, &input);

        let tx_prep = PrioritySellTxPrep {
            context: input.context.tx.clone(),
            plan,
            route,
            simulation,
            predicted_base_fee_gwei: gas_rank.predicted_base_fee_gwei,
            expected_late_recovery_eth: self.config.expected_late_recovery_eth,
            ranked_fee_candidates: gas_rank.candidates,
        };

        match prepare_priority_sell(&self.config.tx_prep, tx_prep) {
            TxPrepOutcome::Submit { signal, budget } => {
                Ok(PrioritySellPlannerOutcome::Submit { signal, budget })
            }
            TxPrepOutcome::Reject(reject) => Ok(PrioritySellPlannerOutcome::Reject(reject)),
        }
    }
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

    PrioritySellPlan {
        trade_id: input.position.trade_id.clone(),
        token_address: input.intent.token_address,
        pool_address: input.intent.pool_address.clone(),
        observed_block: input.context.current_block,
        signal_source: LpSignalSource::MempoolLpApproval,
        urgency: crate::SellUrgency::MempoolPreMine,
        route: config.priority_route.clone(),
        max_priority_fee_per_gas_gwei: config.max_priority_fee_per_gas_gwei,
        max_total_fee_eth: config.max_total_fee_eth,
        reason,
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

    use super::*;
    use crate::{
        FixedGasRankProvider, FixedPreSubmitSimulator, GasRankPlan, PlannerTxContext,
        RankedFeeCandidate, StaticAllowanceChecker, TxPrepRequestContext,
        UniswapV2SellRouteBuilder, UniswapV2TradingVaultSellRouteBuilder,
        VaultInternalAllowanceChecker,
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
            strategy_name: StrategyName(
                "alpha11-live-univ2-lp30-pool-update-block-hold20".to_string(),
            ),
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
                    "alpha11-live-univ2-lp30-pool-update-block-hold20".to_string(),
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
                    strategy_name: "alpha11-live-univ2-lp30-pool-update-block-hold20".to_string(),
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
            UniswapV2SellRouteBuilder::default(),
            FixedPreSubmitSimulator::new(crate::PreSubmitSimulation {
                block_number: 25_128_246,
                block_hash: Some("0xabc".to_string()),
                state_root: None,
                expected_output_token: Some("WETH".to_string()),
                expected_output_amount: Some("10000000000000000".to_string()),
                min_output_amount: Some("9000000000000000".to_string()),
                expected_recovery_eth: DecimalAmount::new(1, 2),
                would_revert: false,
                metadata: json!({ "sim": "ok" }),
            }),
            FixedGasRankProvider::new(GasRankPlan {
                predicted_base_fee_gwei: DecimalAmount::from(10),
                candidates: vec![RankedFeeCandidate {
                    label: "aggressive".to_string(),
                    priority_fee_gwei: DecimalAmount::from(40),
                    max_fee_per_gas_gwei: DecimalAmount::from(50),
                    rank_position_p50: Some(10),
                    gas_before_p50: Some(450_000),
                    likely_fits_at_p50: Some(true),
                    source: Some("test".to_string()),
                }],
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
            would_revert: false,
            metadata: json!({ "sim": "ok" }),
        })
    }

    fn gas_rank() -> FixedGasRankProvider {
        FixedGasRankProvider::new(GasRankPlan {
            predicted_base_fee_gwei: DecimalAmount::from(10),
            candidates: vec![RankedFeeCandidate {
                label: "aggressive".to_string(),
                priority_fee_gwei: DecimalAmount::from(40),
                max_fee_per_gas_gwei: DecimalAmount::from(50),
                rank_position_p50: Some(10),
                gas_before_p50: Some(450_000),
                likely_fits_at_p50: Some(true),
                source: Some("test".to_string()),
            }],
        })
    }

    #[tokio::test]
    async fn planner_prepares_value_capped_kartal_signal() {
        let outcome = planner(StaticAllowanceChecker::pre_approved())
            .plan_priority_sell(input())
            .await
            .unwrap();

        match outcome {
            PrioritySellPlannerOutcome::Submit { signal, .. } => {
                assert!(
                    signal
                        .request
                        .to
                        .eq_ignore_ascii_case("0x7a250d5630b4cf539739df2c5dacb4c659f2488d")
                );
                assert!(signal.request.data.starts_with("0x791ac947"));
                assert_eq!(signal.request.max_priority_fee_per_gas, "40000000000");
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
            UniswapV2TradingVaultSellRouteBuilder::with_default_gas(vault),
            simulator(),
            gas_rank(),
            VaultInternalAllowanceChecker,
        );
        let outcome = planner.plan_priority_sell(input()).await.unwrap();

        match outcome {
            PrioritySellPlannerOutcome::Submit { signal, .. } => {
                assert!(signal.request.to.eq_ignore_ascii_case(&vault.to_string()));
                assert!(signal.request.data.starts_with("0x5f413d10"));
            }
            other => panic!("expected submit, got {other:?}"),
        }
    }
}
