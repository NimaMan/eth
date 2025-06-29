//! Common utility functions used throughout the system
//! 
//! Provides helper functions for common operations like formatting,
//! conversions, and calculations.

use ethers::prelude::*;
use ethers::utils::{format_ether, format_units};
use std::time::{SystemTime, UNIX_EPOCH};

/// Format wei amount as ETH with specified decimal places
pub fn format_wei_to_eth(wei: U256, decimals: usize) -> String {
    let eth = format_ether(wei);
    let parts: Vec<&str> = eth.split('.').collect();
    if parts.len() == 2 && decimals > 0 {
        format!("{}.{}", parts[0], &parts[1][..decimals.min(parts[1].len())])
    } else {
        parts[0].to_string()
    }
}

/// Format token amount with proper decimals
pub fn format_token_amount(amount: U256, decimals: u8) -> String {
    format_units(amount, decimals as u32).unwrap_or_else(|_| amount.to_string())
}

/// Calculate price impact percentage
pub fn calculate_price_impact(
    amount_in: U256,
    amount_out: U256,
    reserve_in: U256,
    reserve_out: U256,
) -> f64 {
    if reserve_in.is_zero() || reserve_out.is_zero() {
        return 100.0; // Max impact if no reserves
    }
    
    // Spot price before swap
    let spot_price_before = reserve_out.as_u128() as f64 / reserve_in.as_u128() as f64;
    
    // Effective price of swap
    let effective_price = amount_out.as_u128() as f64 / amount_in.as_u128() as f64;
    
    // Price impact percentage
    ((spot_price_before - effective_price) / spot_price_before * 100.0).abs()
}

/// Apply slippage to an amount
pub fn apply_slippage(amount: U256, slippage: f64) -> U256 {
    let factor = 1.0 - slippage;
    let adjusted = amount.as_u128() as f64 * factor;
    U256::from(adjusted as u128)
}

/// Get current timestamp
pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Get timestamp N seconds from now
pub fn timestamp_from_now(seconds: u64) -> u64 {
    current_timestamp() + seconds
}

/// Check if a timestamp has expired
pub fn is_expired(timestamp: u64) -> bool {
    current_timestamp() > timestamp
}

/// Convert gas price from gwei to wei
pub fn gwei_to_wei(gwei: f64) -> U256 {
    let wei_per_gwei = 1_000_000_000u64;
    U256::from((gwei * wei_per_gwei as f64) as u64)
}

/// Convert gas price from wei to gwei
pub fn wei_to_gwei(wei: U256) -> f64 {
    let wei_per_gwei = 1_000_000_000u64;
    wei.as_u64() as f64 / wei_per_gwei as f64
}

/// Calculate total transaction cost
pub fn calculate_tx_cost(gas_limit: U256, gas_price: U256) -> U256 {
    gas_limit * gas_price
}

/// Truncate address for display
pub fn truncate_address(address: Address) -> String {
    let addr_str = format!("{:?}", address);
    format!("{}...{}", &addr_str[..6], &addr_str[addr_str.len()-4..])
}

/// Parse ether string to U256
pub fn parse_ether_safe(value: &str) -> Result<U256, String> {
    ethers::utils::parse_ether(value)
        .map_err(|e| format!("Invalid ether value '{}': {}", value, e))
}

/// Calculate percentage
pub fn calculate_percentage(value: U256, total: U256) -> f64 {
    if total.is_zero() {
        return 0.0;
    }
    (value.as_u128() as f64 / total.as_u128() as f64) * 100.0
}

/// Round U256 to nearest multiple
pub fn round_to_multiple(value: U256, multiple: U256) -> U256 {
    if multiple.is_zero() {
        return value;
    }
    let remainder = value % multiple;
    if remainder.is_zero() {
        value
    } else {
        value + multiple - remainder
    }
}

/// Generate a unique request ID
pub fn generate_request_id() -> String {
    use rand::Rng;
    let timestamp = current_timestamp();
    let random: u32 = rand::thread_rng().gen();
    format!("{:x}-{:x}", timestamp, random)
}

/// Retry helper with exponential backoff
pub async fn retry_with_backoff<F, T, E>(
    mut operation: F,
    max_retries: u32,
    base_delay_ms: u64,
) -> Result<T, E>
where
    F: FnMut() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + Send>>,
    E: std::fmt::Display,
{
    let mut delay = base_delay_ms;
    
    for attempt in 0..max_retries {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) if attempt < max_retries - 1 => {
                tracing::warn!(
                    "Operation failed (attempt {}/{}): {}. Retrying in {}ms...",
                    attempt + 1,
                    max_retries,
                    e,
                    delay
                );
                tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                delay *= 2; // Exponential backoff
            }
            Err(e) => return Err(e),
        }
    }
    
    unreachable!()
}

/// Validate Ethereum address
pub fn is_valid_address(address: &str) -> bool {
    address.parse::<Address>().is_ok()
}

/// Compare U256 values with tolerance
pub fn u256_approx_eq(a: U256, b: U256, tolerance_bps: u64) -> bool {
    let diff = if a > b { a - b } else { b - a };
    let tolerance = b * U256::from(tolerance_bps) / U256::from(10_000);
    diff <= tolerance
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_format_wei_to_eth() {
        let wei = ethers::utils::parse_ether("1.23456789").unwrap();
        assert_eq!(format_wei_to_eth(wei, 2), "1.23");
        assert_eq!(format_wei_to_eth(wei, 4), "1.2345");
    }
    
    #[test]
    fn test_calculate_price_impact() {
        let amount_in = U256::from(1000);
        let amount_out = U256::from(900);
        let reserve_in = U256::from(100_000);
        let reserve_out = U256::from(100_000);
        
        let impact = calculate_price_impact(amount_in, amount_out, reserve_in, reserve_out);
        assert!((impact - 10.0).abs() < 0.1); // ~10% impact
    }
    
    #[test]
    fn test_apply_slippage() {
        let amount = U256::from(1000);
        let slippage = 0.01; // 1%
        let result = apply_slippage(amount, slippage);
        assert_eq!(result, U256::from(990));
    }
}