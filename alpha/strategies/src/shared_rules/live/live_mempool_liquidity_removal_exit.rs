pub const DEFAULT_STRATEGY_NAME: &str = "snipe-all";
pub const DEFAULT_STRATEGY_LABEL: &str = "Snipe All";
pub const STRATEGY_RUNTIME: &str = "live";
pub const SUITE_NAME: &str = "mempool-live-exits";
pub const SUITE_OBSERVATION_NAME: &str = "snipe-all-live-suite";
pub const RISK_ATLAS_LP_GATE_HOLD15_BUY_CONFIRM_LP_MAXHOLD_STRATEGY_NAME: &str =
    "snipe-all-risk-atlas-lp-gate-hold15-buy-confirm-lp-maxhold";
pub const ALPHA11_LIVE_GASGUARD_HOLD_SWEEP_SUITE_NAME: &str = "alpha11-live-gasguard-hold-sweep";
const ALPHA11_MIN_SELL_POOL_DENOM_RESERVE: &str = "0";

#[derive(Clone, Debug, Default)]
pub struct LiveStrategySpecOptions {
    pub stop_loss_ratio: Option<String>,
    pub take_profit_ratio: Option<String>,
    pub max_hold_blocks: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct LiveStrategySpec {
    pub strategy_name: String,
    pub strategy_impl: String,
    pub strategy_label: String,
    pub exit_liquidity_removal: bool,
    pub exit_tax: bool,
    pub exit_lp_approval: bool,
    pub exit_lp_approval_critical_only: bool,
    pub exit_scam: bool,
    pub allowed_protocols: Vec<String>,
    pub block_entry_on_lp_approval: bool,
    pub lp_approval_gate_min_pct: Option<String>,
    pub defer_buy_confirm_block_lp_approval_to_max_hold: bool,
    pub min_sell_pool_denom_reserve: Option<String>,
    pub stop_loss_ratio: Option<String>,
    pub take_profit_ratio: Option<String>,
    pub max_hold_blocks: Option<u64>,
}

pub fn default_strategy_spec(options: &LiveStrategySpecOptions) -> LiveStrategySpec {
    LiveStrategySpec {
        strategy_name: DEFAULT_STRATEGY_NAME.to_string(),
        strategy_impl: DEFAULT_STRATEGY_NAME.to_string(),
        strategy_label: DEFAULT_STRATEGY_LABEL.to_string(),
        exit_liquidity_removal: true,
        exit_tax: true,
        exit_lp_approval: true,
        exit_lp_approval_critical_only: false,
        exit_scam: true,
        allowed_protocols: Vec::new(),
        block_entry_on_lp_approval: false,
        lp_approval_gate_min_pct: None,
        defer_buy_confirm_block_lp_approval_to_max_hold: false,
        min_sell_pool_denom_reserve: None,
        stop_loss_ratio: options.stop_loss_ratio.clone(),
        take_profit_ratio: options.take_profit_ratio.clone(),
        max_hold_blocks: options.max_hold_blocks,
    }
}

pub fn suite_specs(
    suite_name: &str,
    options: &LiveStrategySpecOptions,
) -> Result<Vec<LiveStrategySpec>, String> {
    match suite_name {
        SUITE_NAME => Ok(liquidity_removal_exit_specs(options)),
        crate::shared_rules::lp_approval_warning_exit::SUITE_NAME
        | crate::shared_rules::lp_approval_warning_exit::STRATEGY_NAME => {
            Ok(vec![lp_approval_warning_exit_spec(options)])
        }
        RISK_ATLAS_LP_GATE_HOLD15_BUY_CONFIRM_LP_MAXHOLD_STRATEGY_NAME => {
            Ok(vec![risk_atlas_lp_gate_hold15_buy_confirm_lp_maxhold_spec(
                options,
            )])
        }
        ALPHA11_LIVE_GASGUARD_HOLD_SWEEP_SUITE_NAME => {
            Ok(alpha11_live_gasguard_hold_sweep_specs(options))
        }
        other => Err(format!("unsupported strategy suite: {other}")),
    }
}

pub fn observation_strategy_name(strategy_specs: &[LiveStrategySpec]) -> String {
    if strategy_specs.len() == 1 {
        strategy_specs[0].strategy_name.clone()
    } else {
        SUITE_OBSERVATION_NAME.to_string()
    }
}

fn liquidity_removal_exit_specs(options: &LiveStrategySpecOptions) -> Vec<LiveStrategySpec> {
    [10_u64, 20, 50]
        .into_iter()
        .flat_map(|max_hold_blocks| {
            [
                LiveStrategySpec {
                    strategy_name: format!(
                        "snipe-all-live-hold{max_hold_blocks}-pool-updates-liquidity-exit"
                    ),
                    strategy_impl: DEFAULT_STRATEGY_NAME.to_string(),
                    strategy_label: format!(
                        "Snipe All live hold {max_hold_blocks} pool updates + liquidity exit"
                    ),
                    exit_liquidity_removal: true,
                    exit_tax: false,
                    exit_lp_approval: false,
                    exit_lp_approval_critical_only: false,
                    exit_scam: false,
                    allowed_protocols: Vec::new(),
                    block_entry_on_lp_approval: false,
                    lp_approval_gate_min_pct: None,
                    defer_buy_confirm_block_lp_approval_to_max_hold: false,
                    min_sell_pool_denom_reserve: None,
                    stop_loss_ratio: options.stop_loss_ratio.clone(),
                    take_profit_ratio: options.take_profit_ratio.clone(),
                    max_hold_blocks: Some(max_hold_blocks),
                },
                LiveStrategySpec {
                    strategy_name: format!(
                        "snipe-all-live-hold{max_hold_blocks}-pool-updates-liquidity-critical-lp-exit"
                    ),
                    strategy_impl: DEFAULT_STRATEGY_NAME.to_string(),
                    strategy_label: format!(
                        "Snipe All live hold {max_hold_blocks} pool updates + liquidity + critical LP exit"
                    ),
                    exit_liquidity_removal: true,
                    exit_tax: false,
                    exit_lp_approval: true,
                    exit_lp_approval_critical_only: true,
                    exit_scam: false,
                    allowed_protocols: Vec::new(),
                    block_entry_on_lp_approval: false,
                    lp_approval_gate_min_pct: None,
                    defer_buy_confirm_block_lp_approval_to_max_hold: false,
                    min_sell_pool_denom_reserve: None,
                    stop_loss_ratio: options.stop_loss_ratio.clone(),
                    take_profit_ratio: options.take_profit_ratio.clone(),
                    max_hold_blocks: Some(max_hold_blocks),
                },
            ]
        })
        .collect()
}

fn lp_approval_warning_exit_spec(options: &LiveStrategySpecOptions) -> LiveStrategySpec {
    LiveStrategySpec {
        strategy_name: crate::shared_rules::lp_approval_warning_exit::STRATEGY_NAME.to_string(),
        strategy_impl: DEFAULT_STRATEGY_NAME.to_string(),
        strategy_label: crate::shared_rules::lp_approval_warning_exit::STRATEGY_LABEL.to_string(),
        exit_liquidity_removal:
            crate::shared_rules::lp_approval_warning_exit::EXIT_LIQUIDITY_REMOVAL,
        exit_tax: crate::shared_rules::lp_approval_warning_exit::EXIT_TAX,
        exit_lp_approval: crate::shared_rules::lp_approval_warning_exit::EXIT_LP_APPROVAL,
        exit_lp_approval_critical_only:
            crate::shared_rules::lp_approval_warning_exit::EXIT_LP_APPROVAL_CRITICAL_ONLY,
        exit_scam: crate::shared_rules::lp_approval_warning_exit::EXIT_SCAM,
        allowed_protocols: Vec::new(),
        block_entry_on_lp_approval: false,
        lp_approval_gate_min_pct: None,
        defer_buy_confirm_block_lp_approval_to_max_hold: false,
        min_sell_pool_denom_reserve: None,
        stop_loss_ratio: options.stop_loss_ratio.clone(),
        take_profit_ratio: options.take_profit_ratio.clone(),
        max_hold_blocks: None,
    }
}

fn risk_atlas_lp_gate_hold15_buy_confirm_lp_maxhold_spec(
    options: &LiveStrategySpecOptions,
) -> LiveStrategySpec {
    LiveStrategySpec {
        strategy_name: RISK_ATLAS_LP_GATE_HOLD15_BUY_CONFIRM_LP_MAXHOLD_STRATEGY_NAME.to_string(),
        strategy_impl: DEFAULT_STRATEGY_NAME.to_string(),
        strategy_label: "Snipe All risk atlas LP gate hold 15 buy-confirm LP maxhold".to_string(),
        exit_liquidity_removal: true,
        exit_tax: false,
        exit_lp_approval: true,
        exit_lp_approval_critical_only: false,
        exit_scam: false,
        allowed_protocols: Vec::new(),
        block_entry_on_lp_approval: true,
        lp_approval_gate_min_pct: Some(
            crate::shared_rules::lp_approval::DEFAULT_GATE_MIN_APPROVED_PCT.to_string(),
        ),
        defer_buy_confirm_block_lp_approval_to_max_hold: true,
        min_sell_pool_denom_reserve: None,
        stop_loss_ratio: options.stop_loss_ratio.clone(),
        take_profit_ratio: options.take_profit_ratio.clone(),
        max_hold_blocks: Some(15),
    }
}

fn alpha11_live_gasguard_hold_sweep_specs(
    options: &LiveStrategySpecOptions,
) -> Vec<LiveStrategySpec> {
    [12_u64, 15, 20]
        .into_iter()
        .enumerate()
        .map(|(index, max_hold_blocks)| {
            alpha11_live_gasguard_hold_spec(index + 1, max_hold_blocks, options)
        })
        .collect()
}

fn alpha11_live_gasguard_hold_spec(
    ordinal: usize,
    max_hold_blocks: u64,
    options: &LiveStrategySpecOptions,
) -> LiveStrategySpec {
    LiveStrategySpec {
        strategy_name: format!("alpha11-{ordinal:02}-live-v2-hold{max_hold_blocks}-gasguard"),
        strategy_impl: DEFAULT_STRATEGY_NAME.to_string(),
        strategy_label: format!("Alpha11 live V2 hold {max_hold_blocks} gasguard"),
        exit_liquidity_removal: true,
        exit_tax: true,
        exit_lp_approval: true,
        exit_lp_approval_critical_only: false,
        exit_scam: true,
        allowed_protocols: vec!["UNISWAP-V2".to_string()],
        block_entry_on_lp_approval: true,
        lp_approval_gate_min_pct: Some(
            crate::shared_rules::lp_approval::DEFAULT_GATE_MIN_APPROVED_PCT.to_string(),
        ),
        defer_buy_confirm_block_lp_approval_to_max_hold: true,
        min_sell_pool_denom_reserve: Some(ALPHA11_MIN_SELL_POOL_DENOM_RESERVE.to_string()),
        stop_loss_ratio: options.stop_loss_ratio.clone(),
        take_profit_ratio: options.take_profit_ratio.clone(),
        max_hold_blocks: Some(max_hold_blocks),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suite_enables_liquidity_removal_for_every_variant() {
        let specs = suite_specs(SUITE_NAME, &LiveStrategySpecOptions::default()).unwrap();

        assert_eq!(specs.len(), 6);
        assert!(specs.iter().all(|spec| spec.exit_liquidity_removal));
    }

    #[test]
    fn critical_lp_variants_keep_critical_lp_filter() {
        let specs = suite_specs(SUITE_NAME, &LiveStrategySpecOptions::default()).unwrap();

        let critical_lp_specs = specs
            .iter()
            .filter(|spec| spec.strategy_name.contains("critical-lp"))
            .collect::<Vec<_>>();
        assert_eq!(critical_lp_specs.len(), 3);
        assert!(critical_lp_specs.iter().all(|spec| spec.exit_lp_approval));
        assert!(critical_lp_specs
            .iter()
            .all(|spec| spec.exit_lp_approval_critical_only));
    }

    #[test]
    fn lp_approval_warning_suite_is_single_non_hold_strategy() {
        let specs = suite_specs(
            crate::shared_rules::lp_approval_warning_exit::SUITE_NAME,
            &LiveStrategySpecOptions {
                max_hold_blocks: Some(20),
                ..LiveStrategySpecOptions::default()
            },
        )
        .unwrap();

        assert_eq!(specs.len(), 1);
        let spec = &specs[0];
        assert_eq!(
            spec.strategy_name,
            crate::shared_rules::lp_approval_warning_exit::STRATEGY_NAME
        );
        assert_eq!(spec.max_hold_blocks, None);
        assert!(spec.exit_liquidity_removal);
        assert!(spec.exit_lp_approval);
        assert!(!spec.exit_lp_approval_critical_only);
        assert!(!spec.exit_tax);
        assert!(!spec.exit_scam);
    }

    #[test]
    fn risk_atlas_buy_confirm_maxhold_suite_has_no_protocol_filter() {
        let specs = suite_specs(
            RISK_ATLAS_LP_GATE_HOLD15_BUY_CONFIRM_LP_MAXHOLD_STRATEGY_NAME,
            &LiveStrategySpecOptions::default(),
        )
        .unwrap();

        assert_eq!(specs.len(), 1);
        let spec = &specs[0];
        assert_eq!(
            spec.strategy_name,
            RISK_ATLAS_LP_GATE_HOLD15_BUY_CONFIRM_LP_MAXHOLD_STRATEGY_NAME
        );
        assert!(spec.allowed_protocols.is_empty());
        assert!(spec.block_entry_on_lp_approval);
        assert_eq!(spec.lp_approval_gate_min_pct.as_deref(), Some("30"));
        assert!(spec.defer_buy_confirm_block_lp_approval_to_max_hold);
        assert_eq!(spec.max_hold_blocks, Some(15));
        assert!(spec.exit_liquidity_removal);
        assert!(spec.exit_lp_approval);
        assert!(!spec.exit_lp_approval_critical_only);
        assert!(!spec.exit_tax);
        assert!(!spec.exit_scam);
    }

    #[test]
    fn alpha11_live_gasguard_suite_matches_hold_sweep() {
        let specs = suite_specs(
            ALPHA11_LIVE_GASGUARD_HOLD_SWEEP_SUITE_NAME,
            &LiveStrategySpecOptions::default(),
        )
        .unwrap();

        assert_eq!(specs.len(), 3);
        assert_eq!(
            specs
                .iter()
                .map(|spec| spec.max_hold_blocks)
                .collect::<Vec<_>>(),
            vec![Some(12), Some(15), Some(20)]
        );
        assert!(specs.iter().all(|spec| spec.exit_liquidity_removal));
        assert!(specs.iter().all(|spec| spec.exit_lp_approval));
        assert!(specs
            .iter()
            .all(|spec| !spec.exit_lp_approval_critical_only));
        assert!(specs.iter().all(|spec| spec.exit_tax));
        assert!(specs.iter().all(|spec| spec.exit_scam));
        assert!(specs.iter().all(|spec| spec.block_entry_on_lp_approval));
        assert!(specs
            .iter()
            .all(|spec| spec.lp_approval_gate_min_pct.as_deref() == Some("30")));
        assert!(specs
            .iter()
            .all(|spec| spec.defer_buy_confirm_block_lp_approval_to_max_hold));
        assert!(specs
            .iter()
            .all(|spec| spec.min_sell_pool_denom_reserve.as_deref() == Some("0")));
        assert!(specs
            .iter()
            .all(|spec| spec.allowed_protocols == vec!["UNISWAP-V2".to_string()]));
    }
}
