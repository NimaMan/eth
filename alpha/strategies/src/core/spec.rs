/// Descriptive `strategy_impl` label for strategies that compose the core
/// engine without a product-specific impl tag. Dispatch no longer branches on
/// it; it is persisted/displayed only. (Historically the deployable "snipe-all"
/// identity; that bare-default deploy target has been removed.)
pub const CORE_STRATEGY_IMPL: &str = "snipe-all";
pub const STRATEGY_RUNTIME: &str = "live";
pub const SUITE_OBSERVATION_NAME: &str = "snipe-all-strategy-set";

/// Identifier a runtime hands to the registry to resolve a strategy (or strategy
/// set) into its complete resolved [`StrategySpec`]s. It is either a single
/// strategy name or a set name; the registry owns the mapping.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StrategyId(pub String);

impl StrategyId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for StrategyId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for StrategyId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

#[derive(Clone, Debug, Default)]
pub struct LiveStrategySpecOptions;

#[derive(Clone, Debug, PartialEq)]
pub struct LiveEntryInitPolicySpec {
    pub max_age_blocks: Option<u64>,
    pub require_pool_creation_block: bool,
    pub max_price_ratio_to_initial: Option<String>,
    pub allow_missing_price_ratio: bool,
}

impl Default for LiveEntryInitPolicySpec {
    fn default() -> Self {
        Self {
            max_age_blocks: None,
            require_pool_creation_block: false,
            max_price_ratio_to_initial: None,
            allow_missing_price_ratio: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LiveStrategySpec {
    pub strategy_name: String,
    pub strategy_impl: String,
    pub strategy_label: String,
    pub exit_tax: bool,
    pub exit_lp_approval: bool,
    pub exit_lp_approval_critical_only: bool,
    pub exit_scam: bool,
    pub allowed_protocols: Vec<String>,
    pub block_entry_on_lp_approval: bool,
    pub lp_approval_gate_min_pct: Option<String>,
    pub entry_init_policy: LiveEntryInitPolicySpec,
    pub defer_buy_confirm_block_lp_approval_to_max_hold: bool,
    pub lp_approval_exit_defer_max_trading_enabled_age_blocks: Option<u64>,
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

/// Canonical, complete, resolved strategy spec. Every field is fully resolved
/// (no further defaulting) so live and backtest instantiate the identical
/// engine. `LiveStrategySpec` is the historical name; `StrategySpec` is the
/// mode-neutral alias the registry/factory speak in.
pub type StrategySpec = LiveStrategySpec;

pub fn observation_strategy_name(strategy_specs: &[LiveStrategySpec]) -> String {
    if strategy_specs.len() == 1 {
        strategy_specs[0].strategy_name.clone()
    } else {
        SUITE_OBSERVATION_NAME.to_string()
    }
}
