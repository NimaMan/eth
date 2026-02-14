//! Input validation utilities
//!
//! Provides validation for trading parameters to ensure safety

use crate::common::errors::KartalError;
use ethers::types::Address;

/// Slippage validation constants
pub const MIN_SLIPPAGE_PERCENT: f64 = 0.1; // 0.1%
pub const MAX_SLIPPAGE_PERCENT: f64 = 10.0; // 10%
pub const DEFAULT_SLIPPAGE_PERCENT: f64 = 3.0; // 3%

/// Validate slippage is within acceptable bounds
pub fn validate_slippage(slippage: f64) -> Result<f64, KartalError> {
    if slippage < 0.0 {
        return Err(KartalError::Validation(format!(
            "Slippage cannot be negative: {}",
            slippage
        )));
    }

    // Convert to percentage if it seems to be in decimal form (e.g., 0.05 instead of 5.0)
    let slippage_percent = if slippage < 1.0 {
        slippage * 100.0
    } else {
        slippage
    };

    if slippage_percent < MIN_SLIPPAGE_PERCENT {
        return Err(KartalError::Validation(format!(
            "Slippage {} too low, minimum is {}%",
            slippage_percent, MIN_SLIPPAGE_PERCENT
        )));
    }

    if slippage_percent > MAX_SLIPPAGE_PERCENT {
        return Err(KartalError::Validation(format!(
            "Slippage {} too high, maximum is {}%",
            slippage_percent, MAX_SLIPPAGE_PERCENT
        )));
    }

    // Return as decimal (0.05 for 5%)
    Ok(slippage_percent / 100.0)
}

/// Known DEX router addresses for validation
pub mod known_routers {
    use ethers::types::Address;
    use lazy_static::lazy_static;
    use std::collections::HashSet;

    lazy_static! {
        pub static ref UNISWAP_V2_ROUTER: Address = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D"
            .parse()
            .unwrap();
        pub static ref SUSHISWAP_ROUTER: Address = "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F"
            .parse()
            .unwrap();
        pub static ref KNOWN_ROUTERS: HashSet<Address> = {
            let mut set = HashSet::new();
            set.insert(*UNISWAP_V2_ROUTER);
            set.insert(*SUSHISWAP_ROUTER);
            set
        };
    }
}

/// Validate pool address is from a known DEX
pub fn validate_pool_address(pool: Address) -> Result<(), KartalError> {
    // For now, just check it's not zero address
    // In production, would check against factory-created pools
    if pool == Address::zero() {
        return Err(KartalError::Validation(
            "Invalid pool address: zero address".to_string(),
        ));
    }

    Ok(())
}

/// Validate token address
pub fn validate_token_address(token: Address) -> Result<(), KartalError> {
    // Check not zero address
    if token == Address::zero() {
        return Err(KartalError::Validation(
            "Invalid token address: zero address".to_string(),
        ));
    }

    // Could add more checks here:
    // - Check against known scam list
    // - Verify contract has expected functions
    // - Check liquidity thresholds

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slippage_validation() {
        // Valid cases
        assert!(validate_slippage(0.05).is_ok()); // 5% as decimal
        assert!(validate_slippage(5.0).is_ok()); // 5% as percentage
        assert!(validate_slippage(3.0).is_ok()); // Default

        // Invalid cases
        assert!(validate_slippage(-1.0).is_err()); // Negative
        assert!(validate_slippage(0.0001).is_err()); // Too low (0.01%)
        assert!(validate_slippage(15.0).is_err()); // Too high

        // Edge cases
        assert!(validate_slippage(0.1).is_ok()); // Minimum (0.1%)
        assert!(validate_slippage(10.0).is_ok()); // Maximum
    }

    #[test]
    fn test_slippage_conversion() {
        // Test decimal to percentage conversion
        let result = validate_slippage(0.05).unwrap();
        assert!((result - 0.05).abs() < 0.0001);

        let result = validate_slippage(5.0).unwrap();
        assert!((result - 0.05).abs() < 0.0001);
    }
}
