use alloy_primitives::U256;
use eth_alpha_core::{
    amount::{Amount, DecimalAmount},
    ids::{PortfolioId, WalletId},
    market::PoolSnapshot,
};
use eth_token_eligibility::{
    evaluate_pool_with_config, EligibilityConfig, EligibilityDecision, PoolEligibilityInput,
};
use rust_decimal::{prelude::ToPrimitive, Decimal};

#[derive(Clone, Debug)]
pub struct SnipeAllConfig {
    pub portfolio_id: PortfolioId,
    pub wallet_id: WalletId,
    pub buy_amount: Amount,
    pub sell_amount: Amount,
    pub min_denom_reserve: DecimalAmount,
    pub min_stable_denom_reserve: DecimalAmount,
    pub supported_denom_symbols: Vec<String>,
    pub max_slippage_bps: u32,
    pub deadline_secs: u64,
    pub sell_on_liquidity_removal: bool,
}

impl Default for SnipeAllConfig {
    fn default() -> Self {
        let buy_amount = Amount {
            raw: U256::from(10_000_000_000_000_000u64),
            decimals: 18,
        };
        Self {
            portfolio_id: PortfolioId("paper".to_string()),
            wallet_id: WalletId("paper-wallet".to_string()),
            sell_amount: buy_amount.clone(),
            buy_amount,
            min_denom_reserve: Decimal::new(5, 1),
            min_stable_denom_reserve: Decimal::from(500u64),
            supported_denom_symbols: ["ETH", "WETH", "USDC", "USDT"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            max_slippage_bps: 500,
            deadline_secs: 30,
            sell_on_liquidity_removal: true,
        }
    }
}

impl SnipeAllConfig {
    pub fn eligibility_config(&self) -> EligibilityConfig {
        EligibilityConfig {
            supported_quote_symbols: self.supported_denom_symbols.clone(),
            min_eth_liquidity: self.min_denom_reserve.to_f64().unwrap_or(0.0),
            min_stable_liquidity: self.min_stable_denom_reserve.to_f64().unwrap_or(0.0),
            ..EligibilityConfig::default()
        }
    }

    pub fn eligibility_input(&self, pool: &PoolSnapshot) -> PoolEligibilityInput {
        PoolEligibilityInput::new(
            normalized_denom_symbol(pool),
            pool.denom_reserve.to_f64(),
            pool.can_buy,
            pool.can_sell,
            pool.is_scam,
        )
    }

    pub fn eligibility_decision(&self, pool: &PoolSnapshot) -> EligibilityDecision {
        evaluate_pool_with_config(&self.eligibility_input(pool), &self.eligibility_config())
    }
}

fn normalized_denom_symbol(pool: &PoolSnapshot) -> Option<String> {
    pool.denom_symbol
        .as_deref()
        .map(str::trim)
        .filter(|symbol| !symbol.is_empty())
        .map(|symbol| symbol.to_ascii_uppercase())
}
