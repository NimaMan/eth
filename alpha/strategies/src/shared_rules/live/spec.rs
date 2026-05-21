pub const DEFAULT_STRATEGY_NAME: &str = "snipe-all";
pub const DEFAULT_STRATEGY_LABEL: &str = "Snipe All";
pub const STRATEGY_RUNTIME: &str = "live";
pub const SUITE_OBSERVATION_NAME: &str = "snipe-all-live-suite";

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
    pub entry_bankroll_eth: Option<String>,
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
        entry_bankroll_eth: None,
        stop_loss_ratio: options.stop_loss_ratio.clone(),
        take_profit_ratio: options.take_profit_ratio.clone(),
        max_hold_blocks: options.max_hold_blocks,
    }
}

pub fn observation_strategy_name(strategy_specs: &[LiveStrategySpec]) -> String {
    if strategy_specs.len() == 1 {
        strategy_specs[0].strategy_name.clone()
    } else {
        SUITE_OBSERVATION_NAME.to_string()
    }
}
