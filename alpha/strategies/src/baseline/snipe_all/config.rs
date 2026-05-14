use alloy_primitives::U256;
use eth_alpha_core::{
    amount::{Amount, DecimalAmount},
    ids::{PortfolioId, WalletId},
    market::PoolSnapshot,
};
use eth_pool_classification::{
    classify_pool_with_config, PoolClassification, PoolClassificationConfig,
    PoolClassificationInput,
};
use rust_decimal::{prelude::ToPrimitive, Decimal};

#[derive(Clone, Debug)]
pub struct SnipeAllConfig {
    pub portfolio_id: PortfolioId,
    pub wallet_id: WalletId,
    pub buy_amount: Amount,
    /// Fraction of token holdings to sell on exit (1.0 = 100%).
    pub sell_fraction: DecimalAmount,
    pub min_denom_reserve: DecimalAmount,
    pub min_stable_denom_reserve: DecimalAmount,
    pub supported_denom_symbols: Vec<String>,
    pub max_slippage_bps: u32,
    pub deadline_secs: u64,
    pub exit_on_liquidity_removal: bool,
    pub exit_on_tax: bool,
    pub exit_on_lp_approval: bool,
    pub exit_on_scam: bool,
    /// Asymmetric price-ratio exits.
    /// Sell if price drops to this ratio of entry price (e.g., 0.7 = -30% stop-loss).
    /// None = disabled.
    pub stop_loss_ratio: Option<DecimalAmount>,
    /// Sell if price rises to this multiple of entry price (e.g., 3.0 = +200% take-profit).
    /// None = disabled (let winners run).
    pub take_profit_ratio: Option<DecimalAmount>,
    /// Force sell after this many blocks regardless of price.
    /// None = disabled (hold indefinitely).
    pub max_hold_blocks: Option<u64>,
    /// Retry a failed exit after this many blocks.
    /// None = disabled, preserving the no-retry baseline.
    pub exit_retry_interval_blocks: Option<u64>,
    /// Maximum failed exit reports to allow before the strategy stops retrying.
    /// None = unlimited while the failure remains retryable.
    pub max_exit_retries: Option<u32>,
}

impl Default for SnipeAllConfig {
    fn default() -> Self {
        let buy_amount = Amount {
            raw: U256::from(10_000_000_000_000_000u64),
            decimals: 18,
        };
        Self {
            portfolio_id: PortfolioId("chain-sim".to_string()),
            wallet_id: WalletId("chain-sim-wallet".to_string()),
            sell_fraction: DecimalAmount::from(1),
            buy_amount,
            min_denom_reserve: Decimal::new(5, 1),
            min_stable_denom_reserve: Decimal::from(1_000u64),
            supported_denom_symbols: ["ETH", "WETH", "USDC", "USDT", "DAI"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            max_slippage_bps: 500,
            deadline_secs: 30,
            exit_on_liquidity_removal: true,
            exit_on_tax: true,
            exit_on_lp_approval: true,
            exit_on_scam: true,
            stop_loss_ratio: None,
            take_profit_ratio: None,
            max_hold_blocks: None,
            exit_retry_interval_blocks: None,
            max_exit_retries: None,
        }
    }
}

impl SnipeAllConfig {
    pub fn classification_config(&self) -> PoolClassificationConfig {
        PoolClassificationConfig {
            supported_quote_symbols: self.supported_denom_symbols.clone(),
            min_eth_liquidity: self.min_denom_reserve.to_f64().unwrap_or(0.0),
            min_stable_liquidity: self.min_stable_denom_reserve.to_f64().unwrap_or(0.0),
            ..PoolClassificationConfig::default()
        }
    }

    pub fn classification_input(&self, pool: &PoolSnapshot) -> PoolClassificationInput {
        PoolClassificationInput::new(
            normalized_denom_symbol(pool),
            pool.denom_reserve.to_f64(),
            pool.can_buy,
            pool.can_sell,
            pool.is_scam,
        )
    }

    pub fn classification_decision(&self, pool: &PoolSnapshot) -> PoolClassification {
        classify_pool_with_config(
            &self.classification_input(pool),
            &self.classification_config(),
        )
    }
}

fn normalized_denom_symbol(pool: &PoolSnapshot) -> Option<String> {
    pool.denom_symbol
        .as_deref()
        .map(str::trim)
        .filter(|symbol| !symbol.is_empty())
        .map(|symbol| symbol.to_ascii_uppercase())
}
