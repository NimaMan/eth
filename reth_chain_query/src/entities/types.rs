/// Common types and utilities for entity analysis
use alloy_primitives::U256;

/// Entity type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityType {
    Stablecoin,
    CEX,
    ETF,
    Unknown,
}

/// Format token amount with proper decimals for display
pub fn format_token_amount(amount: U256, decimals: u8) -> f64 {
    let divisor = U256::from(10).pow(U256::from(decimals));
    let whole = amount / divisor;
    let fraction = amount % divisor;

    let whole_f64 = whole.to_string().parse::<f64>().unwrap_or(0.0);
    let fraction_f64 =
        fraction.to_string().parse::<f64>().unwrap_or(0.0) / 10_f64.powi(decimals as i32);

    whole_f64 + fraction_f64
}
