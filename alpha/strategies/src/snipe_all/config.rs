use alloy_primitives::U256;
use eth_alpha_core::{
    amount::{Amount, DecimalAmount},
    ids::{PortfolioId, WalletId},
    market::PoolSnapshot,
};
use rust_decimal::Decimal;

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
            min_denom_reserve: Decimal::ZERO,
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
    pub fn is_supported_denom(&self, pool: &PoolSnapshot) -> bool {
        let Some(symbol) = normalized_denom_symbol(pool) else {
            return false;
        };
        self.supported_denom_symbols
            .iter()
            .any(|supported| supported.eq_ignore_ascii_case(&symbol))
    }

    pub fn min_reserve_for_pool(&self, pool: &PoolSnapshot) -> DecimalAmount {
        match normalized_denom_symbol(pool).as_deref() {
            Some("USDC") | Some("USDT") => self.min_stable_denom_reserve,
            _ => self.min_denom_reserve,
        }
    }
}

fn normalized_denom_symbol(pool: &PoolSnapshot) -> Option<String> {
    pool.denom_symbol
        .as_deref()
        .map(str::trim)
        .filter(|symbol| !symbol.is_empty())
        .map(|symbol| symbol.to_ascii_uppercase())
}
