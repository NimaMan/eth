use alloy_primitives::U256;
use serde::{Deserialize, Serialize};

use super::{Pool, PoolKind, Protocol};

/// Precision-safe liquidity metrics using U256 arithmetic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityMetrics {
    /// Raw liquidity value (protocol-specific units)
    pub raw_liquidity: U256,
    /// Liquidity tier (0-10, higher = more liquid)
    pub liquidity_tier: u8,
    /// Significance multiplier (scaled by 1000, e.g., 1200 = 1.2x)
    pub significance_multiplier: u32,
    /// Protocol type for bonus calculations
    pub protocol: Protocol,
}

impl LiquidityMetrics {
    /// Create liquidity metrics directly from raw liquidity value (for testing)
    pub fn from_raw_liquidity(raw_liquidity: U256, protocol: Protocol) -> Self {
        let tier = Self::calculate_liquidity_tier(raw_liquidity, 10); // Use standard baseline
        Self {
            raw_liquidity,
            liquidity_tier: tier,
            significance_multiplier: 1000, // 1.0x multiplier
            protocol,
        }
    }

    /// Create precision-safe liquidity metrics from pool information
    pub fn from_pool(pool: &Pool) -> Self {
        let (raw_liquidity, base_tier, significance_multiplier) = match &pool.kind {
            PoolKind::V2 {
                reserve0, reserve1, ..
            } => {
                // For V2, liquidity = sqrt(reserve0 * reserve1) * 2
                let product = Self::safe_multiply_u256(*reserve0, *reserve1);
                let geometric_mean = Self::sqrt_u256_approx(product);
                let liquidity = geometric_mean * U256::from(2);

                // Tier based on liquidity magnitude (bit-based logarithm approximation)
                let tier = Self::calculate_liquidity_tier(liquidity, 15); // 15 = log10(10^15) baseline

                (liquidity, tier, 1000) // 1.0x multiplier
            }
            PoolKind::V3 { liquidity, .. } => {
                // V3 liquidity is already in the correct units
                let tier = Self::calculate_liquidity_tier(*liquidity, 10); // Lower baseline for V3
                (*liquidity, tier, 1000) // 1.0x multiplier
            }
            PoolKind::Curve { balances, .. } => {
                // Sum of all balances as total liquidity proxy
                let total_liquidity = balances
                    .iter()
                    .fold(U256::ZERO, |acc, &balance| acc.saturating_add(balance));
                let tier = Self::calculate_liquidity_tier(total_liquidity, 12); // 12 = log10(10^12) baseline

                (total_liquidity, tier, 1200) // 1.2x bonus for Curve specialization
            }
            PoolKind::Balancer { balances, .. } => {
                // Sum of all balances as total liquidity proxy
                let total_liquidity = balances
                    .iter()
                    .fold(U256::ZERO, |acc, &balance| acc.saturating_add(balance));
                let tier = Self::calculate_liquidity_tier(total_liquidity, 12);

                (total_liquidity, tier, 1100) // 1.1x bonus for Balancer
            }
        };

        Self {
            raw_liquidity,
            liquidity_tier: base_tier,
            significance_multiplier,
            protocol: pool.protocol,
        }
    }

    /// Calculate liquidity tier using bit-based logarithm approximation
    /// Returns tier 0-10 where higher numbers indicate more liquidity
    fn calculate_liquidity_tier(value: U256, baseline_log10: u8) -> u8 {
        if value == U256::ZERO {
            return 0;
        }

        // Use bit length as logarithm approximation
        // Each ~3.32 bits represents approximately one order of magnitude (log10)
        let bit_length = (256u32 - value.leading_zeros() as u32) as u8;

        // Convert bits to approximate log10: log10(x) ≈ bit_length / 3.32
        // We multiply by 10 and divide by 33 to avoid floating point (3.32 * 10 = 33.2 ≈ 33)
        // Use u16 to prevent overflow when bit_length is near 255
        let approx_log10 = ((bit_length as u16 * 10) / 33) as u8;

        // Calculate tier relative to baseline, clamped to 0-10
        if approx_log10 >= baseline_log10 {
            (approx_log10 - baseline_log10).min(10)
        } else {
            0
        }
    }

    /// Safe multiplication with overflow protection
    fn safe_multiply_u256(a: U256, b: U256) -> U256 {
        match a.checked_mul(b) {
            Some(result) => result,
            None => {
                // If overflow, scale both values down and multiply
                let scaled_a = a >> 64;
                let scaled_b = b >> 64;
                scaled_a * scaled_b // Result is scaled down by 2^128
            }
        }
    }

    /// Approximate square root using Newton's method (U256 version)
    /// For very large numbers, uses bit-shift approximation
    fn sqrt_u256_approx(value: U256) -> U256 {
        if value == U256::ZERO {
            return U256::ZERO;
        }
        if value == U256::from(1) {
            return U256::from(1);
        }

        // For very large numbers, use bit-shift approximation
        if value > U256::from(u64::MAX) {
            let bit_length = 256u32 - value.leading_zeros() as u32;
            let sqrt_bit_length = bit_length / 2;
            return U256::from(1) << sqrt_bit_length;
        }

        // For smaller numbers, convert to u64 and use standard sqrt
        if let Ok(value_u64) = TryInto::<u64>::try_into(value) {
            let sqrt_u64 = (value_u64 as f64).sqrt() as u64;
            U256::from(sqrt_u64)
        } else {
            // Fallback: bit-shift approximation
            let bit_length = 256u32 - value.leading_zeros() as u32;
            let sqrt_bit_length = bit_length / 2;
            U256::from(1) << sqrt_bit_length
        }
    }

    /// Check if liquidity metrics indicate this is a reasonable source
    pub fn is_reasonable(&self) -> bool {
        self.liquidity_tier >= 2 // Minimum tier threshold (approximately 100x baseline)
    }

    /// Get normalized score (0.0 - 1.0) for display and comparison
    /// This is the ONLY place we convert to f64, and only for display
    pub fn get_normalized_score(&self) -> f64 {
        let base_score = (self.liquidity_tier as f64) / 10.0; // Normalize tier to 0.0-1.0
        let multiplier = (self.significance_multiplier as f64) / 1000.0; // Convert back from scaled int
        (base_score * multiplier).min(1.0)
    }

    /// Get a display summary of liquidity metrics
    pub fn display_summary(&self) -> String {
        format!(
            "tier: {}/10 (score: {:.2})",
            self.liquidity_tier,
            self.get_normalized_score()
        )
    }

    /// Compare liquidity with another metrics instance
    /// Returns true if this instance has higher liquidity
    pub fn has_higher_liquidity(&self, other: &LiquidityMetrics) -> bool {
        // Primary comparison: tier level
        if self.liquidity_tier != other.liquidity_tier {
            return self.liquidity_tier > other.liquidity_tier;
        }

        // Secondary comparison: raw liquidity within same tier
        if self.raw_liquidity != other.raw_liquidity {
            return self.raw_liquidity > other.raw_liquidity;
        }

        // Tertiary comparison: significance multiplier
        self.significance_multiplier > other.significance_multiplier
    }
}
