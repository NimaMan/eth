use eth_strategies::shared_rules::lp_approval_warning_exit;
use eyre::Result;
use serde_json::Value;

#[derive(Clone, Debug)]
pub struct StrategySuiteOptions {
    pub strategy_name: String,
    pub strategy_impl: String,
    pub strategy_suite: Option<String>,
    pub stop_loss_ratio: Option<String>,
    pub take_profit_ratio: Option<String>,
    pub max_hold_blocks: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct BacktestStrategySpec {
    pub(crate) strategy_name: String,
    pub(crate) strategy_impl: String,
    /// Backtest-harness signal-replay declaration: when true, the historical
    /// runner loads stored `mempool_signal` rows so the engine's fundamental
    /// (always-on) liquidity-removal exit is exercised against mempool signals.
    /// This is NOT a strategy on/off toggle — the engine exit is unconditional —
    /// it only gates which historical events the backtest replays.
    pub(crate) exit_on_liquidity_removal: bool,
    pub(crate) exit_on_tax: bool,
    pub(crate) exit_on_lp_approval: bool,
    pub(crate) exit_on_critical_lp_approval_only: bool,
    pub(crate) exit_on_scam: bool,
    pub(crate) allowed_protocols: Vec<String>,
    pub(crate) block_entry_on_lp_approval: bool,
    pub(crate) lp_approval_gate_min_pct: Option<String>,
    pub(crate) defer_buy_confirm_block_lp_approval_to_max_hold: bool,
    pub(crate) lp_approval_exit_defer_max_trading_enabled_age_blocks: Option<u64>,
    pub(crate) min_sell_pool_denom_reserve: Option<String>,
    pub(crate) stop_loss_ratio: Option<String>,
    pub(crate) take_profit_ratio: Option<String>,
    pub(crate) max_hold_blocks: Option<u64>,
}

impl BacktestStrategySpec {
    /// Project a canonical resolved [`eth_strategies::StrategySpec`] (the single
    /// source of truth the live runtime also uses) into the backtest spec. The
    /// backtest does not model entry bankroll, and buy size / liquidity floors
    /// come from CLI, so those resolved-spec fields are intentionally dropped
    /// here — that is the documented live/backtest parity boundary.
    fn from_resolved_spec(spec: &eth_strategies::StrategySpec) -> Self {
        Self {
            strategy_name: spec.strategy_name.clone(),
            strategy_impl: spec.strategy_impl.clone(),
            // Liquidity-removal exit is fundamental/always-on, so the engine
            // reacts to mempool liquidity-removal signals for every resolved
            // spec; replay them in historical backtests. (Was previously the
            // now-removed `spec.exit_liquidity_removal`, which was always true
            // for the live specs projected here.)
            exit_on_liquidity_removal: true,
            exit_on_tax: spec.exit_tax,
            exit_on_lp_approval: spec.exit_lp_approval,
            exit_on_critical_lp_approval_only: spec.exit_lp_approval_critical_only,
            exit_on_scam: spec.exit_scam,
            allowed_protocols: spec.allowed_protocols.clone(),
            block_entry_on_lp_approval: spec.block_entry_on_lp_approval,
            lp_approval_gate_min_pct: spec.lp_approval_gate_min_pct.clone(),
            defer_buy_confirm_block_lp_approval_to_max_hold: spec
                .defer_buy_confirm_block_lp_approval_to_max_hold,
            lp_approval_exit_defer_max_trading_enabled_age_blocks: spec
                .lp_approval_exit_defer_max_trading_enabled_age_blocks,
            min_sell_pool_denom_reserve: spec.min_sell_pool_denom_reserve.clone(),
            stop_loss_ratio: spec.stop_loss_ratio.clone(),
            take_profit_ratio: spec.take_profit_ratio.clone(),
            max_hold_blocks: spec.max_hold_blocks,
        }
    }

    pub fn config_json(&self) -> Value {
        serde_json::json!({
            "strategy_name": self.strategy_name,
            "strategy_impl": self.strategy_impl,
            "exit_tax": self.exit_on_tax,
            "exit_lp_approval": self.exit_on_lp_approval,
            "exit_lp_approval_critical_only": self.exit_on_critical_lp_approval_only,
            "exit_scam": self.exit_on_scam,
            "allowed_protocols": self.allowed_protocols,
            "block_entry_on_lp_approval": self.block_entry_on_lp_approval,
            "lp_approval_gate_min_pct": self.lp_approval_gate_min_pct,
            "defer_buy_confirm_block_lp_approval_to_max_hold": self.defer_buy_confirm_block_lp_approval_to_max_hold,
            "lp_approval_exit_defer_max_trading_enabled_age_blocks": self.lp_approval_exit_defer_max_trading_enabled_age_blocks,
            "min_sell_pool_denom_reserve": self.min_sell_pool_denom_reserve,
            "stop_loss_ratio": self.stop_loss_ratio,
            "take_profit_ratio": self.take_profit_ratio,
            "max_hold_blocks": self.max_hold_blocks,
        })
    }

    pub fn uses_signal_risk_events(&self) -> bool {
        self.exit_on_liquidity_removal
            || self.exit_on_lp_approval
            || self.exit_on_tax
            || self.exit_on_scam
    }
}

/// Every accepted `--strategy-suite` value, in the order the dispatch match in
/// [`build_strategy_specs`] lists them. This is the single source of truth for
/// the `--list-strategy-suites` discovery flag and for the "valid suites" list
/// rendered in error messages, so a fresh agent never has to read this file to
/// learn what suites exist. Keep this list in lockstep with the match below.
pub fn strategy_suite_names() -> Vec<&'static str> {
    vec![
        "historical-pool-update-hold",
        "risk-atlas-edge-v1",
        "risk-atlas-edge-v2",
        "risk-atlas-edge-v3",
        "risk-atlas-edge-v4",
        "risk-atlas-edge-v5",
        "alpha-10-risk-atlas",
        "alpha-10-risk-atlas-leader",
        "alpha-11-risk-atlas",
        "gamma-10-risk-atlas",
        "gamma-10-risk-atlas-leader",
        "risk-atlas-lp-buy-confirm-block-comparison",
        "risk-atlas-lp-buy-confirm-block-comparison-uniswap-v2-only",
        lp_approval_warning_exit::MEMPOOL_AWARE_HISTORICAL_SUITE_NAME,
        lp_approval_warning_exit::MEMPOOL_AWARE_HISTORICAL_STRATEGY_NAME,
    ]
}

/// Render the valid-suite list for fail-fast error messages.
fn valid_strategy_suites_hint() -> String {
    strategy_suite_names()
        .into_iter()
        .map(|name| format!("  --strategy-suite {name}"))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn build_strategy_specs(args: &StrategySuiteOptions) -> Result<Vec<BacktestStrategySpec>> {
    if let Some(suite) = args.strategy_suite.as_deref() {
        return match suite {
            "historical-pool-update-hold" => Ok(historical_pool_update_hold_suite_specs(args)),
            "risk-atlas-edge-v1" => Ok(risk_atlas_edge_suite_v1_specs(args)),
            "risk-atlas-edge-v2" => Ok(risk_atlas_edge_suite_v2_specs(args)),
            "risk-atlas-edge-v3" => Ok(risk_atlas_edge_suite_v3_specs(args)),
            "risk-atlas-edge-v4" => Ok(risk_atlas_edge_suite_v4_specs(args)),
            "risk-atlas-edge-v5" => Ok(risk_atlas_edge_suite_v5_specs(args)),
            "alpha-10-risk-atlas" => Ok(alpha_10_risk_atlas_suite_specs(args)),
            "alpha-10-risk-atlas-leader" => Ok(alpha_10_risk_atlas_leader_spec(args)),
            "alpha-11-risk-atlas" => Ok(alpha_11_risk_atlas_suite_specs(args)),
            "gamma-10-risk-atlas" => Ok(gamma_10_risk_atlas_suite_specs(args)),
            "gamma-10-risk-atlas-leader" => Ok(gamma_10_risk_atlas_leader_spec(args)),
            "risk-atlas-lp-buy-confirm-block-comparison" => {
                Ok(risk_atlas_lp_buy_confirm_block_comparison_specs(args))
            }
            "risk-atlas-lp-buy-confirm-block-comparison-uniswap-v2-only" => {
                Ok(risk_atlas_lp_buy_confirm_block_comparison_uniswap_v2_only_specs(args))
            }
            lp_approval_warning_exit::MEMPOOL_AWARE_HISTORICAL_SUITE_NAME
            | lp_approval_warning_exit::MEMPOOL_AWARE_HISTORICAL_STRATEGY_NAME => Ok(vec![
                historical_mempool_aware_lp_approval_warning_exit_spec(args),
            ]),
            lp_approval_warning_exit::SUITE_NAME | lp_approval_warning_exit::STRATEGY_NAME => {
                Err(eyre::eyre!(
                    "strategy suite '{suite}' is the non-mempool-aware variant and is rejected for historical replay: replaying stored mempool_signal rows must use a mempool-aware name. Use --strategy-suite {}.\nValid suites:\n{}",
                    lp_approval_warning_exit::MEMPOOL_AWARE_HISTORICAL_SUITE_NAME,
                    valid_strategy_suites_hint()
                ))
            }
            "mempool-history-exits" => Err(eyre::eyre!(
                "strategy suite 'mempool-history-exits' was removed; historical backtests no longer replay mempool signals. Use --strategy-suite historical-pool-update-hold.\nValid suites:\n{}",
                valid_strategy_suites_hint()
            )),
            other => Err(eyre::eyre!(
                "unknown --strategy-suite '{other}'. Run with --list-strategy-suites to print every valid name.\nValid suites:\n{}",
                valid_strategy_suites_hint()
            )),
        };
    }

    Ok(vec![BacktestStrategySpec {
        strategy_name: args.strategy_name.clone(),
        strategy_impl: args.strategy_impl.clone(),
        exit_on_liquidity_removal: false,
        exit_on_tax: false,
        exit_on_lp_approval: false,
        exit_on_critical_lp_approval_only: false,
        exit_on_scam: false,
        allowed_protocols: Vec::new(),
        block_entry_on_lp_approval: false,
        lp_approval_gate_min_pct: None,
        defer_buy_confirm_block_lp_approval_to_max_hold: false,
        lp_approval_exit_defer_max_trading_enabled_age_blocks: None,
        min_sell_pool_denom_reserve: None,
        stop_loss_ratio: args.stop_loss_ratio.clone(),
        take_profit_ratio: args.take_profit_ratio.clone(),
        max_hold_blocks: args.max_hold_blocks,
    }])
}

fn historical_pool_update_hold_suite_specs(
    args: &StrategySuiteOptions,
) -> Vec<BacktestStrategySpec> {
    [1_u64, 2, 3, 5, 10]
        .into_iter()
        .map(|max_hold_blocks| BacktestStrategySpec {
            strategy_name: format!("snipe-all-hold{max_hold_blocks}-pool-updates"),
            strategy_impl: "snipe-all".to_string(),
            exit_on_liquidity_removal: false,
            exit_on_tax: false,
            exit_on_lp_approval: false,
            exit_on_critical_lp_approval_only: false,
            exit_on_scam: false,
            allowed_protocols: Vec::new(),
            block_entry_on_lp_approval: false,
            lp_approval_gate_min_pct: None,
            defer_buy_confirm_block_lp_approval_to_max_hold: false,
            lp_approval_exit_defer_max_trading_enabled_age_blocks: None,
            min_sell_pool_denom_reserve: None,
            stop_loss_ratio: args.stop_loss_ratio.clone(),
            take_profit_ratio: args.take_profit_ratio.clone(),
            max_hold_blocks: Some(max_hold_blocks),
        })
        .collect()
}

fn risk_atlas_edge_suite_v1_specs(args: &StrategySuiteOptions) -> Vec<BacktestStrategySpec> {
    vec![
        risk_atlas_uniswap_v2_only_spec(
            "snipe-all-risk-atlas-lp-immediate-exit-v1-uniswap-v2-only",
            true,
            false,
            None,
            args,
        ),
        risk_atlas_uniswap_v2_only_spec(
            "snipe-all-risk-atlas-lp-launch-gate-v1-uniswap-v2-only",
            true,
            true,
            None,
            args,
        ),
        risk_atlas_uniswap_v2_only_spec(
            "snipe-all-risk-atlas-active-horizon-hold10-v1-uniswap-v2-only",
            true,
            true,
            Some(10),
            args,
        ),
        risk_atlas_uniswap_v2_only_spec(
            "snipe-all-risk-atlas-v2-backdoor-fast-hold5-v1-uniswap-v2-only",
            false,
            true,
            Some(5),
            args,
        ),
        risk_atlas_uniswap_v2_only_spec(
            "snipe-all-risk-atlas-protocol-guard-hold50-v1-uniswap-v2-only",
            true,
            true,
            Some(50),
            args,
        ),
    ]
}

fn risk_atlas_edge_suite_v2_specs(args: &StrategySuiteOptions) -> Vec<BacktestStrategySpec> {
    [5_u64, 8, 10, 12, 15]
        .into_iter()
        .map(|max_hold_blocks| {
            risk_atlas_uniswap_v2_only_spec(
                &format!("snipe-all-risk-atlas-lp-gate-hold{max_hold_blocks}-v2-uniswap-v2-only"),
                true,
                true,
                Some(max_hold_blocks),
                args,
            )
        })
        .collect()
}

fn risk_atlas_edge_suite_v3_specs(args: &StrategySuiteOptions) -> Vec<BacktestStrategySpec> {
    [15_u64, 20, 25, 30, 40]
        .into_iter()
        .map(|max_hold_blocks| {
            risk_atlas_uniswap_v2_only_spec(
                &format!("snipe-all-risk-atlas-lp-gate-hold{max_hold_blocks}-v3-uniswap-v2-only"),
                true,
                true,
                Some(max_hold_blocks),
                args,
            )
        })
        .collect()
}

fn risk_atlas_edge_suite_v4_specs(args: &StrategySuiteOptions) -> Vec<BacktestStrategySpec> {
    [20_u64, 30, 40]
        .into_iter()
        .flat_map(|max_hold_blocks| {
            [2_u64, 3, 5, 8].into_iter().map(move |take_profit| {
                risk_atlas_uniswap_v2_only_spec_with_price_exits(
                    &format!(
                        "snipe-all-risk-atlas-lp-gate-hold{max_hold_blocks}-tp{take_profit}x-v4-uniswap-v2-only"
                    ),
                    Some(max_hold_blocks),
                    None,
                    Some(&format!("{take_profit}.0")),
                    args,
                )
            })
        })
        .collect()
}

fn risk_atlas_edge_suite_v5_specs(args: &StrategySuiteOptions) -> Vec<BacktestStrategySpec> {
    [20_u64, 30, 40]
        .into_iter()
        .flat_map(|max_hold_blocks| {
            [
                ("sl70", "0.70", "tp3x", "3.0"),
                ("sl70", "0.70", "tp5x", "5.0"),
                ("sl85", "0.85", "tp3x", "3.0"),
                ("sl85", "0.85", "tp5x", "5.0"),
            ]
            .into_iter()
            .map(move |(sl_label, stop_loss, tp_label, take_profit)| {
                risk_atlas_uniswap_v2_only_spec_with_price_exits(
                    &format!(
                        "snipe-all-risk-atlas-lp-gate-hold{max_hold_blocks}-{sl_label}-{tp_label}-v5-uniswap-v2-only"
                    ),
                    Some(max_hold_blocks),
                    Some(stop_loss),
                    Some(take_profit),
                    args,
                )
            })
        })
        .collect()
}

fn risk_atlas_lp_buy_confirm_block_comparison_specs(
    args: &StrategySuiteOptions,
) -> Vec<BacktestStrategySpec> {
    let immediate = risk_atlas_spec(
        "snipe-all-risk-atlas-lp-gate-hold15-immediate-lp-exit",
        true,
        true,
        Some(15),
        args,
    );
    let mut buy_confirm_block_hold = risk_atlas_spec(
        "snipe-all-risk-atlas-lp-gate-hold15-buy-confirm-lp-maxhold",
        true,
        true,
        Some(15),
        args,
    );
    buy_confirm_block_hold.defer_buy_confirm_block_lp_approval_to_max_hold = true;

    vec![immediate, buy_confirm_block_hold]
}

fn risk_atlas_lp_buy_confirm_block_comparison_uniswap_v2_only_specs(
    args: &StrategySuiteOptions,
) -> Vec<BacktestStrategySpec> {
    let immediate = risk_atlas_uniswap_v2_only_spec(
        "snipe-all-risk-atlas-lp-gate-hold15-immediate-lp-exit-uniswap-v2-only",
        true,
        true,
        Some(15),
        args,
    );
    let mut buy_confirm_block_hold = risk_atlas_uniswap_v2_only_spec(
        "snipe-all-risk-atlas-lp-gate-hold15-buy-confirm-lp-maxhold-uniswap-v2-only",
        true,
        true,
        Some(15),
        args,
    );
    buy_confirm_block_hold.defer_buy_confirm_block_lp_approval_to_max_hold = true;

    vec![immediate, buy_confirm_block_hold]
}

fn alpha_10_risk_atlas_suite_specs(args: &StrategySuiteOptions) -> Vec<BacktestStrategySpec> {
    let mut baseline = risk_atlas_spec(
        "alpha10-01-baseline-hold15-buy-confirm",
        true,
        true,
        Some(15),
        args,
    );
    baseline.defer_buy_confirm_block_lp_approval_to_max_hold = true;

    let mut v2_baseline = risk_atlas_uniswap_v2_only_spec(
        "alpha10-02-v2-hold15-buy-confirm",
        true,
        true,
        Some(15),
        args,
    );
    v2_baseline.defer_buy_confirm_block_lp_approval_to_max_hold = true;

    let hold20 =
        risk_atlas_uniswap_v2_only_spec("alpha10-03-v2-hold20", true, true, Some(20), args);
    let hold30 =
        risk_atlas_uniswap_v2_only_spec("alpha10-04-v2-hold30", true, true, Some(30), args);
    let hold40 =
        risk_atlas_uniswap_v2_only_spec("alpha10-05-v2-hold40", true, true, Some(40), args);
    let tp3 = risk_atlas_uniswap_v2_only_spec_with_price_exits(
        "alpha10-06-v2-hold30-tp3x",
        Some(30),
        None,
        Some("3.0"),
        args,
    );
    let tp5 = risk_atlas_uniswap_v2_only_spec_with_price_exits(
        "alpha10-07-v2-hold30-tp5x",
        Some(30),
        None,
        Some("5.0"),
        args,
    );
    let sl70_tp3 = risk_atlas_uniswap_v2_only_spec_with_price_exits(
        "alpha10-08-v2-hold30-sl70-tp3x",
        Some(30),
        Some("0.70"),
        Some("3.0"),
        args,
    );
    let sl85_tp3 = risk_atlas_uniswap_v2_only_spec_with_price_exits(
        "alpha10-09-v2-hold30-sl85-tp3x",
        Some(30),
        Some("0.85"),
        Some("3.0"),
        args,
    );
    vec![
        baseline,
        v2_baseline,
        hold20,
        hold30,
        hold40,
        tp3,
        tp5,
        sl70_tp3,
        sl85_tp3,
    ]
}

fn alpha_10_risk_atlas_leader_spec(args: &StrategySuiteOptions) -> Vec<BacktestStrategySpec> {
    let mut leader = risk_atlas_uniswap_v2_only_spec(
        "alpha10-02-v2-hold15-buy-confirm",
        true,
        true,
        Some(15),
        args,
    );
    leader.defer_buy_confirm_block_lp_approval_to_max_hold = true;
    vec![leader]
}

fn alpha_11_risk_atlas_suite_specs(_args: &StrategySuiteOptions) -> Vec<BacktestStrategySpec> {
    // Resolve through the single alpha11 factory so backtest is at parity with
    // live: the full 10-variant set (incl. the all-pools hold16 variant),
    // strategy_impl="alpha11", and the alpha11 exit policy come from one source
    // of truth rather than a hand-maintained duplicate.
    let options = eth_strategies::shared_rules::live::LiveStrategySpecOptions::default();
    eth_strategies::strategies::alpha11::factory::specs(&options)
        .iter()
        .map(BacktestStrategySpec::from_resolved_spec)
        .collect()
}

fn gamma_10_risk_atlas_suite_specs(args: &StrategySuiteOptions) -> Vec<BacktestStrategySpec> {
    let same_confirm_immediate = risk_atlas_uniswap_v2_only_spec(
        "gamma10-01-v2-hold15-immediate-lp-exit",
        true,
        true,
        Some(15),
        args,
    );

    let hold5 = gamma_10_v2_buy_confirm_spec("gamma10-02-v2-hold5-buy-confirm", 5, args);
    let hold8 = gamma_10_v2_buy_confirm_spec("gamma10-03-v2-hold8-buy-confirm", 8, args);
    let hold10 = gamma_10_v2_buy_confirm_spec("gamma10-04-v2-hold10-buy-confirm", 10, args);
    let hold12 = gamma_10_v2_buy_confirm_spec("gamma10-05-v2-hold12-buy-confirm", 12, args);
    let hold15 = gamma_10_v2_buy_confirm_spec("gamma10-06-v2-hold15-buy-confirm", 15, args);
    let hold20 = gamma_10_v2_buy_confirm_spec("gamma10-07-v2-hold20-buy-confirm", 20, args);
    let stop_loss_take_profit = gamma_10_v2_buy_confirm_price_spec(
        "gamma10-10-v2-hold15-sl85-tp5x",
        15,
        Some("0.85"),
        Some("5.0"),
        args,
    );

    vec![
        same_confirm_immediate,
        hold5,
        hold8,
        hold10,
        hold12,
        hold15,
        hold20,
        stop_loss_take_profit,
    ]
}

fn gamma_10_risk_atlas_leader_spec(args: &StrategySuiteOptions) -> Vec<BacktestStrategySpec> {
    vec![gamma_10_v2_buy_confirm_spec(
        "gamma10-06-v2-hold15-buy-confirm",
        15,
        args,
    )]
}

fn gamma_10_v2_buy_confirm_spec(
    strategy_name: &str,
    max_hold_blocks: u64,
    args: &StrategySuiteOptions,
) -> BacktestStrategySpec {
    let mut spec =
        risk_atlas_uniswap_v2_only_spec(strategy_name, true, true, Some(max_hold_blocks), args);
    spec.defer_buy_confirm_block_lp_approval_to_max_hold = true;
    spec
}

fn gamma_10_v2_buy_confirm_price_spec(
    strategy_name: &str,
    max_hold_blocks: u64,
    stop_loss_ratio: Option<&str>,
    take_profit_ratio: Option<&str>,
    args: &StrategySuiteOptions,
) -> BacktestStrategySpec {
    let mut spec = risk_atlas_uniswap_v2_only_spec_with_price_exits(
        strategy_name,
        Some(max_hold_blocks),
        stop_loss_ratio,
        take_profit_ratio,
        args,
    );
    spec.defer_buy_confirm_block_lp_approval_to_max_hold = true;
    spec
}

fn risk_atlas_spec(
    strategy_name: &str,
    exit_on_lp_approval: bool,
    block_entry_on_lp_approval: bool,
    max_hold_blocks: Option<u64>,
    args: &StrategySuiteOptions,
) -> BacktestStrategySpec {
    BacktestStrategySpec {
        strategy_name: strategy_name.to_string(),
        strategy_impl: "snipe-all".to_string(),
        exit_on_liquidity_removal: true,
        exit_on_tax: false,
        exit_on_lp_approval,
        exit_on_critical_lp_approval_only: false,
        exit_on_scam: false,
        allowed_protocols: Vec::new(),
        block_entry_on_lp_approval,
        lp_approval_gate_min_pct: Some(
            eth_strategies::shared_rules::lp_approval::DEFAULT_GATE_MIN_APPROVED_PCT.to_string(),
        ),
        defer_buy_confirm_block_lp_approval_to_max_hold: false,
        lp_approval_exit_defer_max_trading_enabled_age_blocks: None,
        min_sell_pool_denom_reserve: None,
        stop_loss_ratio: args.stop_loss_ratio.clone(),
        take_profit_ratio: args.take_profit_ratio.clone(),
        max_hold_blocks,
    }
}

fn risk_atlas_uniswap_v2_only_spec(
    strategy_name: &str,
    exit_on_lp_approval: bool,
    block_entry_on_lp_approval: bool,
    max_hold_blocks: Option<u64>,
    args: &StrategySuiteOptions,
) -> BacktestStrategySpec {
    let mut spec = risk_atlas_spec(
        strategy_name,
        exit_on_lp_approval,
        block_entry_on_lp_approval,
        max_hold_blocks,
        args,
    );
    spec.allowed_protocols = vec!["UNISWAP-V2".to_string()];
    spec
}

fn risk_atlas_spec_with_price_exits(
    strategy_name: &str,
    max_hold_blocks: Option<u64>,
    stop_loss_ratio: Option<&str>,
    take_profit_ratio: Option<&str>,
    args: &StrategySuiteOptions,
) -> BacktestStrategySpec {
    let mut spec = risk_atlas_spec(strategy_name, true, true, max_hold_blocks, args);
    if let Some(stop_loss_ratio) = stop_loss_ratio {
        spec.stop_loss_ratio = Some(stop_loss_ratio.to_string());
    }
    if let Some(take_profit_ratio) = take_profit_ratio {
        spec.take_profit_ratio = Some(take_profit_ratio.to_string());
    }
    spec
}

fn risk_atlas_uniswap_v2_only_spec_with_price_exits(
    strategy_name: &str,
    max_hold_blocks: Option<u64>,
    stop_loss_ratio: Option<&str>,
    take_profit_ratio: Option<&str>,
    args: &StrategySuiteOptions,
) -> BacktestStrategySpec {
    let mut spec = risk_atlas_spec_with_price_exits(
        strategy_name,
        max_hold_blocks,
        stop_loss_ratio,
        take_profit_ratio,
        args,
    );
    spec.allowed_protocols = vec!["UNISWAP-V2".to_string()];
    spec
}

fn historical_mempool_aware_lp_approval_warning_exit_spec(
    args: &StrategySuiteOptions,
) -> BacktestStrategySpec {
    BacktestStrategySpec {
        strategy_name: lp_approval_warning_exit::MEMPOOL_AWARE_HISTORICAL_STRATEGY_NAME.to_string(),
        strategy_impl: "snipe-all".to_string(),
        exit_on_liquidity_removal: lp_approval_warning_exit::EXIT_LIQUIDITY_REMOVAL,
        exit_on_tax: lp_approval_warning_exit::EXIT_TAX,
        exit_on_lp_approval: lp_approval_warning_exit::EXIT_LP_APPROVAL,
        exit_on_critical_lp_approval_only: lp_approval_warning_exit::EXIT_LP_APPROVAL_CRITICAL_ONLY,
        exit_on_scam: lp_approval_warning_exit::EXIT_SCAM,
        allowed_protocols: Vec::new(),
        block_entry_on_lp_approval: false,
        lp_approval_gate_min_pct: None,
        defer_buy_confirm_block_lp_approval_to_max_hold: false,
        lp_approval_exit_defer_max_trading_enabled_age_blocks: None,
        min_sell_pool_denom_reserve: None,
        stop_loss_ratio: args.stop_loss_ratio.clone(),
        take_profit_ratio: args.take_profit_ratio.clone(),
        max_hold_blocks: None,
    }
}

pub fn validate_historical_signal_replay_names(specs: &[BacktestStrategySpec]) -> Result<()> {
    for spec in specs {
        if spec.uses_signal_risk_events() && !spec.strategy_name.contains("mempool-aware") {
            return Err(eyre::eyre!(
                "historical strategy {} replays stored mempool_signal risk events but is not named mempool-aware",
                spec.strategy_name
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn suite_options() -> StrategySuiteOptions {
        StrategySuiteOptions {
            strategy_name: "ignored".to_string(),
            strategy_impl: "ignored".to_string(),
            strategy_suite: Some("alpha-11-risk-atlas".to_string()),
            stop_loss_ratio: None,
            take_profit_ratio: None,
            max_hold_blocks: None,
        }
    }

    /// Backtest alpha-11 suite must be the projection of the single live alpha11
    /// factory: same variant set (incl. the all-pools hold16 variant),
    /// strategy_impl, and exit policy. This is the live/backtest parity proof at
    /// the backtest boundary (entry bankroll / buy size / liquidity are
    /// CLI-controlled in backtest and intentionally not compared).
    #[test]
    fn alpha_11_suite_is_projection_of_live_factory() {
        let backtest = build_strategy_specs(&suite_options()).unwrap();
        let live = eth_strategies::strategies::alpha11::factory::specs(
            &eth_strategies::shared_rules::live::LiveStrategySpecOptions::default(),
        );

        assert_eq!(backtest.len(), live.len());
        assert_eq!(backtest.len(), 10, "10-variant set incl hold16-all-pools");

        for (bt, lv) in backtest.iter().zip(live.iter()) {
            assert_eq!(bt.strategy_name, lv.strategy_name);
            assert_eq!(bt.strategy_impl, lv.strategy_impl);
            assert_eq!(bt.strategy_impl, "alpha11");
            // Liquidity-removal exit is fundamental/always-on; the projection
            // declares signal replay unconditionally (no live spec field).
            assert!(bt.exit_on_liquidity_removal);
            assert_eq!(bt.exit_on_tax, lv.exit_tax);
            assert_eq!(bt.exit_on_lp_approval, lv.exit_lp_approval);
            assert_eq!(
                bt.exit_on_critical_lp_approval_only,
                lv.exit_lp_approval_critical_only
            );
            assert_eq!(bt.exit_on_scam, lv.exit_scam);
            assert_eq!(bt.allowed_protocols, lv.allowed_protocols);
            assert_eq!(bt.block_entry_on_lp_approval, lv.block_entry_on_lp_approval);
            assert_eq!(bt.lp_approval_gate_min_pct, lv.lp_approval_gate_min_pct);
            assert_eq!(
                bt.defer_buy_confirm_block_lp_approval_to_max_hold,
                lv.defer_buy_confirm_block_lp_approval_to_max_hold
            );
            assert_eq!(
                bt.lp_approval_exit_defer_max_trading_enabled_age_blocks,
                lv.lp_approval_exit_defer_max_trading_enabled_age_blocks
            );
            assert_eq!(bt.min_sell_pool_denom_reserve, lv.min_sell_pool_denom_reserve);
            assert_eq!(bt.max_hold_blocks, lv.max_hold_blocks);
        }

        assert!(
            backtest
                .iter()
                .any(|s| s.strategy_name == "alpha11-all-pools-lp30-pool-update-block-hold16"),
            "backtest now includes the all-pools hold16 variant (was missing)"
        );
    }

    fn options_for(suite: &str) -> StrategySuiteOptions {
        StrategySuiteOptions {
            strategy_name: "ignored".to_string(),
            strategy_impl: "ignored".to_string(),
            strategy_suite: Some(suite.to_string()),
            stop_loss_ratio: None,
            take_profit_ratio: None,
            max_hold_blocks: None,
        }
    }

    /// Every name advertised by `--list-strategy-suites` must actually resolve
    /// through the dispatch match. This is the lockstep guard that keeps the
    /// enumerated discovery list from drifting away from the real suites.
    #[test]
    fn every_listed_suite_builds() {
        for name in strategy_suite_names() {
            let specs = build_strategy_specs(&options_for(name))
                .unwrap_or_else(|err| panic!("listed suite '{name}' failed to build: {err}"));
            assert!(!specs.is_empty(), "listed suite '{name}' built zero specs");
        }
    }

    /// An unknown suite must fail with a message that points the agent at the
    /// discovery flag and lists the valid names.
    #[test]
    fn unknown_suite_errors_with_valid_list() {
        let err = build_strategy_specs(&options_for("does-not-exist"))
            .expect_err("unknown suite must error");
        let message = format!("{err}");
        assert!(message.contains("--list-strategy-suites"), "{message}");
        assert!(message.contains("alpha-11-risk-atlas"), "{message}");
    }

    /// The non-mempool-aware variant is deliberately rejected for historical
    /// replay; the error must name the mempool-aware replacement.
    #[test]
    fn non_mempool_aware_variant_rejected_with_hint() {
        let err = build_strategy_specs(&options_for(lp_approval_warning_exit::SUITE_NAME))
            .expect_err("non-mempool-aware variant must be rejected");
        let message = format!("{err}");
        assert!(
            message.contains(lp_approval_warning_exit::MEMPOOL_AWARE_HISTORICAL_SUITE_NAME),
            "{message}"
        );
    }
}
