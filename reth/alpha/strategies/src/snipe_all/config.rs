use alloy_primitives::U256;
use eth_alpha_core::{
    amount::{Amount, DecimalAmount},
    ids::{PortfolioId, WalletId},
};
use rust_decimal::Decimal;

#[derive(Clone, Debug)]
pub struct SnipeAllConfig {
    pub portfolio_id: PortfolioId,
    pub wallet_id: WalletId,
    pub buy_amount: Amount,
    pub sell_amount: Amount,
    pub min_denom_reserve: DecimalAmount,
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
            max_slippage_bps: 500,
            deadline_secs: 30,
            sell_on_liquidity_removal: true,
        }
    }
}
