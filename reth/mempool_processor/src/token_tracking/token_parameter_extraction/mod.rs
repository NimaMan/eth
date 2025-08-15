/// Token Parameter Extraction Module
/// 
/// Extracts token parameters like buy/sell taxes using sequential buy/sell simulation

pub mod tax_calculator;
pub mod tax_calculator_from_sequential_simulation;
// pub mod token_info_fetcher;  // TODO: fix compilation errors

pub use tax_calculator::{
    calculate_buy_tax, 
    calculate_sell_tax,
    calculate_buy_tax_from_movements,
    calculate_sell_tax_from_movements,
    TaxCalculationResult,
};
pub use tax_calculator_from_sequential_simulation::{TaxCalculator, TokenInfo, PoolReserves};
// pub use token_info_fetcher::TokenInfoFetcher;