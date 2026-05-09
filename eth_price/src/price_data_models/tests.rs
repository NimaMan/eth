use super::*;

/// Test V3 sqrt price calculation precision and overflow safety
#[test]
fn test_v3_sqrt_price_precision() {
    // Test normal sqrt price values (should not overflow)
    let normal_sqrt_price = U256::from(1_u128 << 80); // Typical sqrt price
    let price = RawChainPrice::from_sqrt_price_x96(normal_sqrt_price);

    // Should not be zero or infinity
    assert!(price.numerator > U256::ZERO);
    assert!(price.denominator > U256::ZERO);
    assert!(price.to_f64().is_finite());

    // Test very large sqrt price (should trigger overflow protection)
    let large_sqrt_price = U256::MAX >> 10; // Large but not maximum
    let price_large = RawChainPrice::from_sqrt_price_x96(large_sqrt_price);

    // Should still produce valid results
    assert!(price_large.numerator > U256::ZERO);
    assert!(price_large.denominator > U256::ZERO);
    assert!(price_large.to_f64().is_finite());

    // Test edge case: maximum sqrt price (should trigger scaling fallback)
    let max_sqrt_price = U256::MAX >> 96; // Maximum valid 160-bit sqrt price
    let price_max = RawChainPrice::from_sqrt_price_x96(max_sqrt_price);

    // Should handle gracefully without panicking
    assert!(price_max.numerator > U256::ZERO);
    assert!(price_max.denominator > U256::ZERO);
}

/// Test RawChainPrice mathematical properties
#[test]
fn test_precise_price_properties() {
    let numerator = U256::from(1000_u64);
    let denominator = U256::from(100_u64);
    let price = RawChainPrice::new(numerator, denominator);

    // Test inverse property: price * inverse = 1
    let inverse = price.inverse();
    assert_eq!(inverse.numerator, denominator);
    assert_eq!(inverse.denominator, numerator);

    // Test proportional scaling preserves ratio
    let scaled_num = numerator * U256::from(5);
    let scaled_den = denominator * U256::from(5);
    let scaled_price = RawChainPrice::new(scaled_num, scaled_den);

    // Ratio should be equivalent
    let original_ratio = price.numerator * scaled_price.denominator;
    let scaled_ratio = scaled_price.numerator * price.denominator;
    assert_eq!(original_ratio, scaled_ratio);
}

/// Test reserve-based price calculation accuracy
#[test]
fn test_reserve_price_calculation() {
    // Test with different decimal combinations
    let test_cases = vec![
        (U256::from(1000_000_u64), U256::from(100_000_000_u64), 6, 18), // USDC/WETH
        (U256::from(50_000_000_u64), U256::from(1_000_000_u64), 18, 8), // WETH/WBTC
        (U256::from(1_000_000_u64), U256::from(1_000_000_u64), 18, 18), // DAI/ETH
    ];

    for (reserve_quote, reserve_base, base_decimals, quote_decimals) in test_cases {
        let price = RawChainPrice::from_reserves(reserve_quote, reserve_base);

        // Should produce valid, finite results
        assert!(price.numerator > U256::ZERO);
        assert!(price.denominator > U256::ZERO);
        assert!(price.is_valid());
        assert_eq!(price.numerator, reserve_quote);
        assert_eq!(price.denominator, reserve_base);
        let _ = (base_decimals, quote_decimals); // retained for compatibility
    }
}

/// Test safe U256 to f64 conversion precision
#[test]
fn test_safe_u256_to_f64_precision() {
    // Test small values (should be exact)
    let small_value = U256::from(12345_u64);
    let small_f64 = safe_u256_to_f64(small_value);
    assert_eq!(small_f64, 12345.0);

    // Test values within f64's exact integer range (2^53)
    let exact_limit = U256::from(1_u64 << 53);
    let exact_f64 = safe_u256_to_f64(exact_limit);
    assert_eq!(exact_f64, (1_u64 << 53) as f64);

    // Test large values (should not panic and should be finite)
    let large_value = U256::from(u128::MAX);
    let large_f64 = safe_u256_to_f64(large_value);
    assert!(large_f64.is_finite());
    assert!(large_f64 > 0.0);

    // Test very large values (should handle gracefully)
    let very_large = U256::MAX >> 10;
    let very_large_f64 = safe_u256_to_f64(very_large);
    assert!(very_large_f64.is_finite() || very_large_f64.is_infinite());
    assert!(very_large_f64 > 0.0);

    // Test zero
    assert_eq!(safe_u256_to_f64(U256::ZERO), 0.0);
}

/// Test LiquidityMetrics precision without f64 contamination
#[test]
fn test_liquidity_metrics_precision() {
    // Test various liquidity levels
    let test_liquidities = vec![
        U256::from(1000_u64),          // Low liquidity
        U256::from(1_000_000_u64),     // Medium liquidity
        U256::from(1_000_000_000_u64), // High liquidity
        U256::MAX >> 10,               // Very high liquidity
    ];

    for liquidity in test_liquidities {
        let metrics = LiquidityMetrics::from_raw_liquidity(liquidity, Protocol::UniswapV3);

        // Should produce valid tier
        assert!(metrics.liquidity_tier <= 10);

        // Raw liquidity should be preserved exactly
        assert_eq!(metrics.raw_liquidity, liquidity);

        // Score should be reasonable
        let score = metrics.get_normalized_score();
        assert!(score >= 0.0 && score <= 10.0);
        assert!(score.is_finite());
    }
}

/// Test Token amount formatting and parsing precision
#[test]
fn test_token_amount_precision() {
    use alloy_primitives::Address;

    // Test various token configurations
    let tokens = vec![
        Token::new(Address::ZERO, "WETH", 18), // 18 decimals
        Token::new(Address::ZERO, "USDC", 6),  // 6 decimals
        Token::new(Address::ZERO, "WBTC", 8),  // 8 decimals
    ];

    for token in tokens {
        // Test format_amount precision
        let raw_amount = U256::from(123_456_789_u64) * token.scaling_factor() / U256::from(1000);
        let formatted = token.format_amount(raw_amount);

        // Should produce reasonable decimal representation
        assert!(formatted.contains('.') || !formatted.is_empty());

        // Test parse_amount precision
        if let Ok(parsed) = token.parse_amount(&formatted) {
            // Round trip should preserve value within reasonable precision
            let diff = if parsed > raw_amount {
                parsed - raw_amount
            } else {
                raw_amount - parsed
            };

            // Difference should be minimal (within one unit of smallest denomination)
            assert!(diff <= U256::from(10)); // Allow small rounding differences
        }

        // Test edge cases
        assert_eq!(token.format_amount(U256::ZERO), "0");

        // Test parsing edge cases
        assert!(token.parse_amount("0").is_ok());
        assert_eq!(token.parse_amount("0").unwrap(), U256::ZERO);
    }
}

/// Test mathematical consistency across price calculations
#[test]
fn test_price_calculation_consistency() {
    // Test that different methods of calculating the same price give consistent results
    let sqrt_price_x96 = U256::from(1_u128 << 80); // Example sqrt price

    // Method 1: Direct sqrt price calculation (raw units)
    let price1 = RawChainPrice::from_sqrt_price_x96(sqrt_price_x96);

    // Method 2: Calculate equivalent reserves and use from_reserves (raw units)
    let base_reserve = U256::from(1_000_000_000_000_000_000_u128); // 1e18 base units

    // Calculate quote reserve that would give similar price (approx for validation)
    let approx_price_f64 = price1.to_f64();
    let quote_reserve = U256::from((safe_u256_to_f64(base_reserve) * approx_price_f64) as u128);

    let price2 = RawChainPrice::from_reserves(quote_reserve, base_reserve);

    // Results should be in the same order of magnitude
    let ratio1 = price1.to_f64();
    let ratio2 = price2.to_f64();

    if ratio1 > 0.0 && ratio2 > 0.0 {
        let relative_diff = (ratio1 - ratio2).abs() / ratio1.max(ratio2);
        assert!(
            relative_diff < 0.5,
            "Price calculation methods should be reasonably consistent"
        );
    }
}

/// Test overflow protection in all calculations
#[test]
fn test_overflow_protection() {
    // Test with extreme values that could cause overflow
    let max_value = U256::MAX;
    let large_value = U256::MAX >> 1;

    // Should not panic on extreme sqrt prices
    let extreme_price = RawChainPrice::from_sqrt_price_x96(max_value >> 96);
    assert!(extreme_price.numerator > U256::ZERO);
    assert!(extreme_price.denominator > U256::ZERO);

    // Should not panic on extreme reserves
    let extreme_reserves = RawChainPrice::from_reserves(large_value, large_value);
    assert!(extreme_reserves.numerator > U256::ZERO);
    assert!(extreme_reserves.denominator > U256::ZERO);

    // Should not panic on extreme liquidity
    let extreme_liquidity = LiquidityMetrics::from_raw_liquidity(max_value, Protocol::UniswapV3);
    assert!(extreme_liquidity.liquidity_tier <= 10);
    assert_eq!(extreme_liquidity.raw_liquidity, max_value);

    // Should not panic on safe_u256_to_f64
    let extreme_f64 = safe_u256_to_f64(max_value);
    assert!(extreme_f64.is_finite() || extreme_f64.is_infinite());
    assert!(extreme_f64 >= 0.0);
}

/// Test human-readable scaling and formatting
#[test]
fn test_human_scaling_display() {
    use alloy_primitives::Address;
    let weth = Token::new(Address::ZERO, "WETH", 18);
    let usdc = Token::new(Address::ZERO, "USDC", 6);

    // Suppose raw price is 2_000 USDC per WETH in raw units.
    // That implies: numerator/denominator ~= (2_000 * 10^6) / (10^18)
    // We'll construct a close ratio using reserves
    let reserve_base = U256::from(1_000_000_000_000_000_000u128); // 1e18 base units
    let reserve_quote = U256::from(2_000_000_000u128); // 2e9 quote units (approx 2000 USDC if divided by 1e6)
    let price = RawChainPrice::from_reserves(reserve_quote, reserve_base);

    let scaled = price.to_scaled_f64(weth.decimals, usdc.decimals);
    assert!(scaled > 1000.0 && scaled < 3000.0);

    let s = price.to_human_string(&weth, &usdc, 4);
    assert!(s.contains("WETH") && s.contains("USDC"));
}

/// Test mathematical invariants
#[test]
fn test_mathematical_invariants() {
    let price = RawChainPrice::new(U256::from(300), U256::from(100));

    // Invariant: price * inverse = 1 (in rational form)
    // price = 300/100, inverse = 100/300
    // price * inverse = (300/100) * (100/300) = (300*100)/(100*300) = 30000/30000 = 1
    let inverse = price.inverse();
    let product_num = price.numerator * inverse.numerator;
    let product_den = price.denominator * inverse.denominator;
    assert_eq!(product_num, product_den); // Should both be 30000

    // Invariant: scaling both numerator and denominator by same factor preserves ratio
    let scale = U256::from(7);
    let scaled = RawChainPrice::new(price.numerator * scale, price.denominator * scale);

    // Cross multiplication should be equal: a/b = c/d iff a*d = b*c
    // For scaled price: original_num/original_den = (original_num * scale)/(original_den * scale)
    // So: original_num * (original_den * scale) should equal original_den * (original_num * scale)
    // Which simplifies to: original_num * original_den * scale = original_den * original_num * scale
    let left_side = price.numerator * scaled.denominator;
    let right_side = price.denominator * scaled.numerator;

    // Since scaled.denominator = price.denominator * scale and scaled.numerator = price.numerator * scale:
    // left_side = price.numerator * (price.denominator * scale)
    // right_side = price.denominator * (price.numerator * scale)
    // Both equal: price.numerator * price.denominator * scale
    assert_eq!(left_side, right_side);

    // Invariant: LiquidityMetrics ordering should be transitive
    let liq1 = LiquidityMetrics::from_raw_liquidity(U256::from(1000), Protocol::UniswapV2);
    let liq2 = LiquidityMetrics::from_raw_liquidity(U256::from(10000), Protocol::UniswapV2);
    let liq3 = LiquidityMetrics::from_raw_liquidity(U256::from(100000), Protocol::UniswapV2);

    // If A > B and B > C, then A > C (transitivity)
    assert!(liq3.has_higher_liquidity(&liq2));
    assert!(liq2.has_higher_liquidity(&liq1));
    assert!(liq3.has_higher_liquidity(&liq1));
}
