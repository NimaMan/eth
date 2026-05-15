pub const DEFAULT_STRATEGY_NAME: &str = "snipe-all-v1";
pub const DEFAULT_STRATEGY_LABEL: &str = "Snipe All v1";
pub const STRATEGY_RUNTIME: &str = "live";
pub const SUITE_NAME: &str = "mempool-live-exits";
pub const LEGACY_SUITE_NAME: &str = "live-mempool-exits";
pub const SUITE_OBSERVATION_NAME: &str = "snipe-all-live-suite";

#[derive(Clone, Debug, Default)]
pub struct LiveStrategySpecOptions {
    pub stop_loss_ratio: Option<String>,
    pub take_profit_ratio: Option<String>,
    pub max_hold_blocks: Option<u64>,
    pub exit_retry_interval_blocks: Option<u64>,
    pub max_exit_retries: Option<u32>,
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
    pub stop_loss_ratio: Option<String>,
    pub take_profit_ratio: Option<String>,
    pub max_hold_blocks: Option<u64>,
    pub exit_retry_interval_blocks: Option<u64>,
    pub max_exit_retries: Option<u32>,
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
        stop_loss_ratio: options.stop_loss_ratio.clone(),
        take_profit_ratio: options.take_profit_ratio.clone(),
        max_hold_blocks: options.max_hold_blocks,
        exit_retry_interval_blocks: options.exit_retry_interval_blocks,
        max_exit_retries: options.max_exit_retries,
    }
}

pub fn suite_specs(
    suite_name: &str,
    options: &LiveStrategySpecOptions,
) -> Result<Vec<LiveStrategySpec>, String> {
    match suite_name {
        SUITE_NAME | LEGACY_SUITE_NAME => Ok(liquidity_removal_exit_specs(options)),
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
                    strategy_name: format!("snipe-all-live-maxhold{max_hold_blocks}-liq-exit"),
                    strategy_impl: DEFAULT_STRATEGY_NAME.to_string(),
                    strategy_label: format!(
                        "Snipe All live maxhold {max_hold_blocks} liquidity exit"
                    ),
                    exit_liquidity_removal: true,
                    exit_tax: false,
                    exit_lp_approval: false,
                    exit_lp_approval_critical_only: false,
                    exit_scam: false,
                    stop_loss_ratio: options.stop_loss_ratio.clone(),
                    take_profit_ratio: options.take_profit_ratio.clone(),
                    max_hold_blocks: Some(max_hold_blocks),
                    exit_retry_interval_blocks: options.exit_retry_interval_blocks,
                    max_exit_retries: options.max_exit_retries,
                },
                LiveStrategySpec {
                    strategy_name: format!(
                        "snipe-all-live-maxhold{max_hold_blocks}-critical-lp-exit"
                    ),
                    strategy_impl: DEFAULT_STRATEGY_NAME.to_string(),
                    strategy_label: format!(
                        "Snipe All live maxhold {max_hold_blocks} critical LP + liquidity exit"
                    ),
                    exit_liquidity_removal: true,
                    exit_tax: false,
                    exit_lp_approval: true,
                    exit_lp_approval_critical_only: true,
                    exit_scam: false,
                    stop_loss_ratio: options.stop_loss_ratio.clone(),
                    take_profit_ratio: options.take_profit_ratio.clone(),
                    max_hold_blocks: Some(max_hold_blocks),
                    exit_retry_interval_blocks: options.exit_retry_interval_blocks,
                    max_exit_retries: options.max_exit_retries,
                },
            ]
        })
        .collect()
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
}
