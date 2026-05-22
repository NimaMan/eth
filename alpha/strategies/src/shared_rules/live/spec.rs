pub const DEFAULT_STRATEGY_NAME: &str = "snipe-all";
pub const DEFAULT_STRATEGY_LABEL: &str = "Snipe All";
pub const STRATEGY_RUNTIME: &str = "live";
pub const SUITE_OBSERVATION_NAME: &str = "snipe-all-strategy-set";

#[derive(Clone, Debug, Default)]
pub struct LiveStrategySpecOptions;

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
    pub max_entry_price_ratio_to_initial: Option<String>,
    pub defer_buy_confirm_block_lp_approval_to_max_hold: bool,
    pub min_sell_pool_denom_reserve: Option<String>,
    pub buy_wei: String,
    pub min_liquidity_eth: String,
    pub min_liquidity_usd: String,
    pub max_entry_pools: Option<usize>,
    pub entry_bankroll_eth: Option<String>,
    pub stop_loss_ratio: Option<String>,
    pub take_profit_ratio: Option<String>,
    pub max_hold_blocks: Option<u64>,
}

pub fn default_strategy_spec(_options: &LiveStrategySpecOptions) -> LiveStrategySpec {
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
        max_entry_price_ratio_to_initial: None,
        defer_buy_confirm_block_lp_approval_to_max_hold: false,
        min_sell_pool_denom_reserve: None,
        buy_wei: "10000000000000000".to_string(),
        min_liquidity_eth: "0.5".to_string(),
        min_liquidity_usd: "1000".to_string(),
        max_entry_pools: None,
        entry_bankroll_eth: None,
        stop_loss_ratio: None,
        take_profit_ratio: None,
        max_hold_blocks: None,
    }
}

pub fn observation_strategy_name(strategy_specs: &[LiveStrategySpec]) -> String {
    if strategy_specs.len() == 1 {
        strategy_specs[0].strategy_name.clone()
    } else {
        SUITE_OBSERVATION_NAME.to_string()
    }
}
