use super::*;

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
                        "simulation_provider": "shadow_dry_run_only",
                        "gas_rank_provider": "shadow_dry_run_only",
                        "min_output_policy": "temporary_1_wei_dry_run_only"
                    }),
                },
                current_block,
                deadline_unix_secs: now.saturating_add(intent.deadline_secs),
            },
            intent: intent.clone(),
            position,
            pool,
            min_output_amount: Some("1".to_string()),
            source_metadata: json!({
                "decision_reason": intent.decision_reason.clone(),
                "trade_id": intent.trade_id.clone(),
            }),
        })
    }
}

impl LiveRealInputResolver {
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

#[derive(Clone)]
struct ShadowDryRunPreSubmitSimulator {
    expected_recovery_eth: Decimal,
}

#[async_trait]
impl PreSubmitSimulator for ShadowDryRunPreSubmitSimulator {
    async fn simulate(
        &self,
        input: &LivePrioritySellPlannerInput,
        _route: &PreparedSellRoute,
    ) -> std::result::Result<PreSubmitSimulation, LivePrioritySellPlannerError> {
        Ok(PreSubmitSimulation {
            block_number: input.context.current_block,
            block_hash: None,
            state_root: None,
            expected_output_token: Some("WETH".to_string()),
            expected_output_amount: Some("1".to_string()),
            min_output_amount: input.min_output_amount.clone(),
            expected_recovery_eth: self.expected_recovery_eth,
            would_revert: false,
            metadata: json!({
                "provider": "shadow_dry_run_only",
                "warning": "not a production final simulation"
            }),
        })
    }
}

pub(super) struct KartalRealPreflight {
    token: String,
    status: KartalEthTxExecutorStatus,
}

pub(super) async fn preflight_kartal_real(args: &Args) -> Result<KartalRealPreflight> {
    let token = load_kartal_bearer_token(&args.kartal_token_env)?;
    let status = KartalClient::new(KartalClientConfig::new(&args.kartal_url, token.clone()))
        .eth_tx_status()
        .await
        .wrap_err("failed to read Kartal ETH tx executor status")?;
    validate_kartal_real_status(&status)?;
    Ok(KartalRealPreflight { token, status })
}

pub(super) async fn build_kartal_real_adapter(
    args: &Args,
    preflight: KartalRealPreflight,
    store: PostgresTradingStore,
    run_id: String,
    valuation_adapter: LiveChainSimExecutionAdapter,
    pools: Arc<std::sync::Mutex<HashMap<TokenPoolId, PoolSnapshot>>>,
    current_block: Arc<AtomicU64>,
) -> Result<Box<dyn EngineExecutionAdapter>> {
    let from = parse_live_real_address(&args.live_real_from, "--live-real-from")?;
    let vault =
        parse_live_real_address(&args.live_real_vault_address, "--live-real-vault-address")?;
    let expected_recovery_eth = Decimal::from_str(&args.live_real_shadow_expected_recovery_eth)
        .wrap_err("invalid --live-real-shadow-expected-recovery-eth")?;
    let priority_fee_gwei = Decimal::from_str(&args.live_real_shadow_priority_fee_gwei)
        .wrap_err("invalid --live-real-shadow-priority-fee-gwei")?;
    let max_fee_gwei = Decimal::from_str(&args.live_real_shadow_max_fee_gwei)
        .wrap_err("invalid --live-real-shadow-max-fee-gwei")?;
    let predicted_base_fee_gwei = Decimal::from_str(&args.live_real_shadow_predicted_base_fee_gwei)
        .wrap_err("invalid --live-real-shadow-predicted-base-fee-gwei")?;

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
        ShadowDryRunPreSubmitSimulator {
            expected_recovery_eth,
        },
        FixedGasRankProvider::new(GasRankPlan {
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
        }),
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
    let bridge = LiveTradingPlannerBridge::new(planner, resolver);
    let submitter = KartalExecutorClient::new(KartalExecutorClientConfig::new(
        &args.kartal_url,
        preflight.token,
    ));
    let executor = TxExecutorAdapter::with_order_prefix(bridge, submitter, run_id);
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
