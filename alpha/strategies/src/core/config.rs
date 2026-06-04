use alloy_primitives::U256;
use eth_alpha_core::{
    amount::{Amount, DecimalAmount},
    ids::{PortfolioId, StrategyName, WalletId},
};
use eth_pool_classification::PoolClassificationConfig;
use rust_decimal::{prelude::ToPrimitive, Decimal};

use crate::core::rules::entry::init_policy::EntryInitPolicyConfig;

#[derive(Clone, Debug)]
pub struct StrategyConfig {
    pub strategy_name: StrategyName,
    pub portfolio_id: PortfolioId,
    pub wallet_id: WalletId,
    pub buy_amount: Amount,
    /// Fraction of token holdings to sell on exit (1.0 = 100%).
    pub sell_fraction: DecimalAmount,
    pub min_denom_reserve: DecimalAmount,
    pub min_stable_denom_reserve: DecimalAmount,
    /// Do not submit sell orders when the current pool denomination reserve is
    /// below this floor. This avoids spending gas on already-drained pools.
    pub min_sell_pool_denom_reserve: DecimalAmount,
    pub supported_denom_symbols: Vec<String>,
    pub max_slippage_bps: u32,
    pub deadline_secs: u64,
    /// If false, the strategy will not open new positions. Existing positions
    /// can still be managed and exited.
    pub entry_enabled: bool,
    /// Hard cap on distinct pools this strategy may buy in one run. Restored
    /// bought pools count toward this limit.
    pub max_entry_pools: Option<usize>,
    /// Starting ETH/WETH bankroll for entries. Buys consume it, confirmed sells
    /// replenish it, and profitable sells increase the amount available.
    /// Restored seen pools without a loaded position count as one configured
    /// buy amount until their position is available.
    pub entry_bankroll_wei: Option<U256>,
    pub exit_on_liquidity_removal: bool,
    pub exit_on_tax: bool,
    pub exit_on_lp_approval: bool,
    pub exit_on_critical_lp_approval_only: bool,
    pub exit_on_scam: bool,
    /// If non-empty, only enter pools whose protocol label is present here.
    pub allowed_protocols: Vec<String>,
    /// Treat any observed LP approval risk for this pool as an entry blocker.
    /// This is separate from the global critical-risk policy so historical
    /// mined-chain LP approvals can be warnings for exits while still gating
    /// the launch-gate variants.
    pub block_entry_on_lp_approval: bool,
    /// Optional strict LP approval percentage gate shared by entry and exit.
    /// When set, LP approval only blocks entry or triggers exit if approved_pct > threshold.
    /// None preserves the legacy "any LP approval" behavior.
    pub lp_approval_gate_min_pct: Option<DecimalAmount>,
    /// Shared Gate 2 entry initialization policy. This owns pool age and
    /// price/initial thresholds for already-active pools.
    pub entry_init_policy: EntryInitPolicyConfig,
    /// If true, an LP approval observed in the same block as buy confirmation
    /// is not an immediate LP-approval exit. The position remains governed by
    /// proactive exits such as max active-hold blocks.
    pub defer_buy_confirm_block_lp_approval_to_max_hold: bool,
    /// When set, an LP approval observed within this many chain blocks from
    /// trading-enabled/pool-start age is treated as launch-window approval and
    /// is deferred to the normal hold policy instead of forcing an immediate
    /// LP-approval exit.
    pub lp_approval_exit_defer_max_trading_enabled_age_blocks: Option<u64>,
    /// Asymmetric price-ratio exits.
    /// Sell if price drops to this ratio of entry price (e.g., 0.7 = -30% stop-loss).
    /// None = disabled.
    pub stop_loss_ratio: Option<DecimalAmount>,
    /// Sell if price rises to this multiple of entry price (e.g., 3.0 = +200% take-profit).
    /// None = disabled (let winners run).
    pub take_profit_ratio: Option<DecimalAmount>,
    /// Force sell after this many distinct pool-update blocks while open.
    /// None = disabled (hold indefinitely).
    pub max_hold_blocks: Option<u64>,
}

impl Default for StrategyConfig {
    fn default() -> Self {
        let buy_amount = Amount {
            raw: U256::from(10_000_000_000_000_000u64),
            decimals: 18,
        };
        Self {
            strategy_name: StrategyName("snipe-all".to_string()),
            portfolio_id: PortfolioId("chain-sim".to_string()),
            wallet_id: WalletId("chain-sim-wallet".to_string()),
            sell_fraction: DecimalAmount::from(1),
            buy_amount,
            min_denom_reserve: Decimal::new(5, 1),
            min_stable_denom_reserve: Decimal::from(1_000u64),
            min_sell_pool_denom_reserve: Decimal::new(1, 2),
            supported_denom_symbols: ["ETH", "WETH", "USDC", "USDT", "DAI"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            max_slippage_bps: 500,
            deadline_secs: 30,
            entry_enabled: true,
            max_entry_pools: None,
            entry_bankroll_wei: None,
            exit_on_liquidity_removal: true,
            exit_on_tax: true,
            exit_on_lp_approval: true,
            exit_on_critical_lp_approval_only: false,
            exit_on_scam: true,
            allowed_protocols: Vec::new(),
            block_entry_on_lp_approval: false,
            lp_approval_gate_min_pct: None,
            entry_init_policy: EntryInitPolicyConfig::default(),
            defer_buy_confirm_block_lp_approval_to_max_hold: false,
            lp_approval_exit_defer_max_trading_enabled_age_blocks: None,
            stop_loss_ratio: None,
            take_profit_ratio: None,
            max_hold_blocks: None,
        }
    }
}

impl StrategyConfig {
    pub fn classification_config(&self) -> PoolClassificationConfig {
        PoolClassificationConfig {
            supported_quote_symbols: self.supported_denom_symbols.clone(),
            min_eth_liquidity: self.min_denom_reserve.to_f64().unwrap_or(0.0),
            min_stable_liquidity: self.min_stable_denom_reserve.to_f64().unwrap_or(0.0),
            ..PoolClassificationConfig::default()
        }
    }
}
