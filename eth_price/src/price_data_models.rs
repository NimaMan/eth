//! Precision-safe price data models.
//!
//! Runtime arithmetic stays in `U256` rational form. `f64` conversion is kept at display,
//! logging, and scoring boundaries only. See `price_data_models/README.md` for the
//! longer architecture note.

use alloy_primitives::{Address, U256};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};

/// Unified protocol enumeration covering AMMs, oracles, and aggregators
/// Single source of truth preventing protocol definition drift
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Protocol {
    // Automated Market Makers
    /// Uniswap V2: Classic constant product AMM (~$2.1B daily volume)
    UniswapV2,
    /// Uniswap V3: Concentrated liquidity with fee tiers (~$3.0B daily volume)  
    UniswapV3,
    /// SushiSwap: Community-driven Uniswap V2 fork (~$170M daily volume)
    SushiSwap,
    /// Curve Finance: Optimized for correlated assets (~$2.0B daily volume)
    Curve,
    /// Balancer V2: Weighted and stable pools with custom ratios
    Balancer,
    /// DODO: Proactive Market Maker with active pricing
    Dodo,
    /// Fraxswap: TWAMM-enabled AMM by Frax Finance
    Fraxswap,
    /// PancakeSwap V3: Concentrated liquidity AMM on BNB Chain/Ethereum
    PancakeswapV3,
    /// Synthetic simulation routes
    Simulation,

    // Oracle Networks
    /// Chainlink: Decentralized oracle network with multi-node consensus
    Chainlink,

    // Aggregators (future)
    /// 1inch: DEX aggregator with pathfinding
    OneInch,
    /// Paraswap: Multi-DEX routing and optimization
    Paraswap,
    /// 0x: Professional-grade aggregator and RFQ network
    ZeroX,
}

impl Protocol {
    /// Get protocol name as string
    pub fn as_str(&self) -> &'static str {
        match self {
            Protocol::UniswapV2 => "UniswapV2",
            Protocol::UniswapV3 => "UniswapV3",
            Protocol::SushiSwap => "SushiSwap",
            Protocol::Curve => "Curve",
            Protocol::Balancer => "Balancer",
            Protocol::Dodo => "DODO",
            Protocol::Fraxswap => "Fraxswap",
            Protocol::PancakeswapV3 => "PancakeSwapV3",
            Protocol::Simulation => "Simulation",
            Protocol::Chainlink => "Chainlink",
            Protocol::OneInch => "1inch",
            Protocol::Paraswap => "Paraswap",
            Protocol::ZeroX => "0x",
        }
    }

    /// Check if protocol is an AMM (provides liquidity-based pricing)
    pub fn is_amm(&self) -> bool {
        matches!(
            self,
            Protocol::UniswapV2
                | Protocol::UniswapV3
                | Protocol::SushiSwap
                | Protocol::Curve
                | Protocol::Balancer
                | Protocol::Dodo
                | Protocol::Fraxswap
                | Protocol::PancakeswapV3
        )
    }

    /// Check if protocol is an oracle (provides external price feeds)
    pub fn is_oracle(&self) -> bool {
        matches!(self, Protocol::Chainlink)
    }

    /// Check if protocol is an aggregator (routes across multiple sources)
    pub fn is_aggregator(&self) -> bool {
        matches!(
            self,
            Protocol::OneInch | Protocol::Paraswap | Protocol::ZeroX
        )
    }
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Unique identifier for a specific price source
/// Ensures each USD stablecoin offering can be distinguished and aggregated correctly
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct PriceId {
    /// Base token being priced (e.g., WETH, WBTC)
    pub base_token: Address,
    /// Quote token pricing is denominated in (e.g., USDC, USDT, DAI)  
    pub quote_token: Address,
    /// Protocol providing this price (e.g., UniswapV3, Curve)
    pub protocol: Protocol,
    /// Specific pool/contract address providing the price
    pub pool_address: Address,
    /// Fee tier for protocols supporting multiple tiers (basis points, e.g., 500 = 0.05%)
    pub fee_tier: Option<u32>,
}

impl PriceId {
    /// Create a new price identifier
    pub fn new(
        base_token: Address,
        quote_token: Address,
        protocol: Protocol,
        pool_address: Address,
        fee_tier: Option<u32>,
    ) -> Self {
        Self {
            base_token,
            quote_token,
            protocol,
            pool_address,
            fee_tier,
        }
    }

    /// Create a hash for this price ID for efficient lookups
    pub fn hash_value(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        Hash::hash(self, &mut hasher);
        hasher.finish()
    }

    /// Get a human-readable description of this price source
    pub fn describe(&self, base_symbol: &str, quote_symbol: &str) -> String {
        match self.fee_tier {
            Some(fee) => format!(
                "{}/{} on {} ({}% fee) [{}]",
                base_symbol,
                quote_symbol,
                self.protocol,
                fee as f64 / 10000.0,
                self.pool_address
            ),
            None => format!(
                "{}/{} on {} [{}]",
                base_symbol, quote_symbol, self.protocol, self.pool_address
            ),
        }
    }

    /// Check if this price ID represents the same token pair (ignoring order)
    pub fn same_pair(&self, other: &PriceId) -> bool {
        (self.base_token == other.base_token && self.quote_token == other.quote_token)
            || (self.base_token == other.quote_token && self.quote_token == other.base_token)
    }
}

impl fmt::Display for PriceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.fee_tier {
            Some(fee) => write!(
                f,
                "{}:{}:{}:{:.2}%",
                self.protocol,
                self.base_token,
                self.quote_token,
                fee as f64 / 10000.0
            ),
            None => write!(
                f,
                "{}:{}:{}",
                self.protocol, self.base_token, self.quote_token
            ),
        }
    }
}

/// Precision-safe price representation using U256 arithmetic
/// Avoids floating-point precision loss while supporting display conversion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawChainPrice {
    /// Price numerator (how many quote token units)
    pub numerator: U256,
    /// Price denominator (per this many base token units)
    pub denominator: U256,
    /// Decimals adjustment already applied
    pub decimals_adjusted: bool,
}

impl RawChainPrice {
    /// Create a new precise price from numerator and denominator
    pub fn new(numerator: U256, denominator: U256) -> Self {
        Self {
            numerator,
            denominator,
            decimals_adjusted: false,
        }
    }

    /// Create from raw reserves WITHOUT applying token decimal scaling (raw units)
    pub fn from_reserves(reserve_quote: U256, reserve_base: U256) -> Self {
        Self {
            numerator: reserve_quote,
            denominator: reserve_base,
            decimals_adjusted: false,
        }
    }

    /// Create from V3 sqrt price using canonical high-precision formula (raw units)
    /// Formula: price = (sqrtPriceX96^2) / 2^192
    pub fn from_sqrt_price_x96(sqrt_price_x96: U256) -> Self {
        // Extract 160-bit sqrt price (Uniswap V3 standard)
        // Mask to ensure we only use the valid 160 bits
        let sqrt_price: U256 = sqrt_price_x96 & (U256::MAX >> 96);

        // Handle edge case: zero sqrt price
        if sqrt_price == U256::ZERO {
            return Self {
                numerator: U256::ZERO,
                denominator: U256::from(1),
                decimals_adjusted: false,
            };
        }

        // Calculate (sqrtPrice)^2 using overflow-safe arithmetic.
        // If overflow on squaring, scale sqrt down by a power of two and adjust denominator accordingly.
        let (squared, denom_pow2) = match sqrt_price.checked_mul(sqrt_price) {
            Some(result) => (result, 192u32),
            None => {
                // Determine shift so that (sqrt >> shift)^2 fits in 256 bits; ensure post-shift bitlen <= 128
                let mut tmp = sqrt_price;
                let mut bitlen: u32 = 0;
                while tmp > U256::from(1) {
                    tmp >>= 1;
                    bitlen += 1;
                }
                let shift = bitlen.saturating_sub(128);
                let scaled = if shift > 0 {
                    sqrt_price >> shift
                } else {
                    sqrt_price
                };
                let sq = scaled * scaled;
                let denom_pow2 = 192u32.saturating_sub(2 * shift);
                (sq, denom_pow2)
            }
        };
        let denominator = U256::from(2).pow(U256::from(denom_pow2));

        Self {
            numerator: squared,
            denominator,
            decimals_adjusted: false,
        }
    }

    /// Convert to f64 for display purposes only - NOT for calculations
    pub fn to_f64(&self) -> f64 {
        safe_u256_to_f64(self.numerator) / safe_u256_to_f64(self.denominator)
    }

    /// Convert to a human-scaled f64: quote tokens per 1 base token.
    /// This applies token decimal scaling only for display (no effect on core math).
    pub fn to_scaled_f64(&self, base_decimals: u8, quote_decimals: u8) -> f64 {
        let raw = self.to_f64();
        let diff = base_decimals as i32 - quote_decimals as i32;
        raw * 10_f64.powi(diff)
    }

    /// Render a human-readable string like: "1 BASE = X QUOTE"
    /// Uses decimal scaling for display only and formats to the given precision.
    pub fn to_human_string(&self, base: &Token, quote: &Token, precision: usize) -> String {
        let scaled = self.to_scaled_f64(base.decimals, quote.decimals);
        // Clamp precision for very large/small numbers to keep output readable
        let prec = precision.min(12);
        format!(
            "1 {} = {:.prec$} {}",
            base.symbol,
            scaled,
            quote.symbol,
            prec = prec
        )
    }

    /// Get the inverse price (quote/base instead of base/quote)
    pub fn inverse(&self) -> Self {
        Self {
            numerator: self.denominator,
            denominator: self.numerator,
            decimals_adjusted: self.decimals_adjusted,
        }
    }

    /// Calculate amount out for given amount in (with precise arithmetic)
    pub fn calculate_amount_out(&self, amount_in: U256) -> U256 {
        if self.denominator == U256::ZERO {
            return U256::ZERO;
        }
        // amount_out = (amount_in * numerator) / denominator
        (amount_in * self.numerator) / self.denominator
    }

    /// Calculate amount in for given amount out (with precise arithmetic)  
    pub fn calculate_amount_in(&self, amount_out: U256) -> U256 {
        if self.numerator == U256::ZERO {
            return U256::ZERO;
        }
        // amount_in = (amount_out * denominator) / numerator
        (amount_out * self.denominator) / self.numerator
    }

    /// Check if price is valid (non-zero denominator)
    pub fn is_valid(&self) -> bool {
        self.denominator > U256::ZERO && self.numerator > U256::ZERO
    }
}

impl fmt::Display for RawChainPrice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.6}", self.to_f64())
    }
}

/// Enhanced safe conversion from U256 to f64 with maximum precision preservation
///
/// **Algorithm**: Uses multi-stage precision-preserving conversion:
/// 1. Direct conversion for values ≤ u128::MAX (full precision)
/// 2. Bit-position analysis for optimal scaling of large values
/// 3. Maintains maximum significant digits within f64's 53-bit mantissa
/// 4. Scientific notation fallback for extreme values
///
/// **Precision Guarantees**:
/// - Values ≤ 2^53: Exact representation (no precision loss)
/// - Values > 2^53: Maintains ~15-16 decimal digits of precision
/// - Automatic scaling preserves relative accuracy
///
fn safe_u256_to_f64(value: U256) -> f64 {
    if value == U256::ZERO {
        return 0.0;
    }

    // Fast path: values that fit in u128 and are exactly representable
    if value <= U256::from(u128::MAX) {
        if let Ok(as_u128) = TryInto::<u128>::try_into(value) {
            // Check if value fits in f64's exact integer range (2^53)
            if as_u128 <= (1u128 << 53) {
                return as_u128 as f64; // Exact representation
            } else {
                return as_u128 as f64; // Some precision loss but acceptable
            }
        } else {
            return 0.0;
        }
    }

    // For large values: find the most significant bits and scale appropriately
    // Goal: preserve maximum precision within f64's 53-bit mantissa

    // Find the position of the highest set bit (log2 approximation)
    let mut temp_value = value;
    let mut bit_position = 0u32;
    while temp_value > U256::from(1) {
        temp_value >>= 1;
        bit_position += 1;
    }

    if bit_position <= 53 {
        // Should fit in f64 mantissa exactly, try direct conversion
        if let Ok(as_u128) = TryInto::<u128>::try_into(value) {
            return as_u128 as f64;
        }
    }

    // Scale down to fit in f64 mantissa while preserving maximum precision
    let scale_down_bits = bit_position.saturating_sub(53);
    let scaled_value = value >> scale_down_bits;

    if let Ok(scaled_u128) = TryInto::<u128>::try_into(scaled_value) {
        let mantissa = scaled_u128 as f64;
        let scale_factor = 2_f64.powi(scale_down_bits as i32);
        return mantissa * scale_factor;
    }

    // Final fallback: split into high and low parts with better precision handling
    let high_part = value >> 128;
    let low_part = value & U256::from(u128::MAX);

    if let (Ok(high_u128), Ok(low_u128)) = (
        TryInto::<u128>::try_into(high_part),
        TryInto::<u128>::try_into(low_part),
    ) {
        let high_f64 = high_u128 as f64;
        let low_f64 = low_u128 as f64;

        // Use higher precision arithmetic
        let result = high_f64 * (2_f64.powi(128)) + low_f64;

        // Check for infinity overflow
        if result.is_finite() {
            return result;
        }
    }

    // Extreme value fallback: return scientific notation representation
    // This indicates a value too large for meaningful f64 representation
    if bit_position > 250 {
        f64::INFINITY
    } else {
        // Estimate magnitude using bit position
        let magnitude = (bit_position as f64) * std::f64::consts::LOG10_2;
        10_f64.powf(magnitude)
    }
}

/// Token representation with essential on-chain data
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Token {
    /// ERC20 contract address (should be checksum format)
    pub address: Address,
    /// Token symbol (e.g., "WETH", "USDC")
    pub symbol: String,
    /// Number of decimals (e.g., 18 for WETH, 6 for USDC)
    pub decimals: u8,
}

impl Token {
    /// Create a new token with validation
    pub fn new(address: Address, symbol: impl Into<String>, decimals: u8) -> Self {
        Self {
            address,
            symbol: symbol.into(),
            decimals,
        }
    }

    /// Get the scaling factor for this token (10^decimals)
    pub fn scaling_factor(&self) -> U256 {
        U256::from(10).pow(U256::from(self.decimals))
    }

    /// Convert raw amount to human-readable string with decimal precision
    /// Returns a string representation avoiding f64 precision loss
    pub fn format_amount(&self, raw_amount: U256) -> String {
        let scale = self.scaling_factor();

        // Handle zero case
        if raw_amount == U256::ZERO {
            return "0".to_string();
        }

        // Extract integer and fractional parts
        let integer_part = raw_amount / scale;
        let fractional_part = raw_amount % scale;

        // Convert integer part to string
        let mut result = integer_part.to_string();

        // Add decimal point and fractional part if not zero
        if fractional_part != U256::ZERO && self.decimals > 0 {
            result.push('.');

            // Convert fractional part to string with proper zero-padding
            let frac_str = fractional_part.to_string();
            let expected_digits = self.decimals as usize;

            // Pad with leading zeros if necessary
            if frac_str.len() < expected_digits {
                let zeros_needed = expected_digits - frac_str.len();
                result.push_str(&"0".repeat(zeros_needed));
            }

            result.push_str(&frac_str);

            // Remove trailing zeros for clean display
            result = result
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string();
        }

        result
    }

    /// Convert human-readable string amount to raw U256 amount
    /// Accepts decimal strings like "123.456" and converts to proper token units
    pub fn parse_amount(&self, human_amount: &str) -> Result<U256, String> {
        let parts: Vec<&str> = human_amount.split('.').collect();

        match parts.len() {
            1 => {
                // Integer only
                let integer = parts[0]
                    .parse::<u128>()
                    .map_err(|_| "Invalid integer part")?;
                Ok(U256::from(integer) * self.scaling_factor())
            }
            2 => {
                // Integer and fractional parts
                let integer = parts[0]
                    .parse::<u128>()
                    .map_err(|_| "Invalid integer part")?;
                let fractional_str = parts[1];

                // Ensure fractional part doesn't exceed token decimals
                if fractional_str.len() > self.decimals as usize {
                    return Err(format!("Too many decimal places. Max: {}", self.decimals));
                }

                // Parse fractional part and scale appropriately
                let fractional = fractional_str
                    .parse::<u128>()
                    .map_err(|_| "Invalid fractional part")?;

                // Calculate scaling factor for fractional part
                let frac_decimals = fractional_str.len();
                let frac_scale = U256::from(10).pow(U256::from(frac_decimals));

                let integer_raw = U256::from(integer) * self.scaling_factor();
                let fractional_raw = U256::from(fractional) * self.scaling_factor() / frac_scale;

                Ok(integer_raw + fractional_raw)
            }
            _ => Err("Invalid decimal format. Use format like '123.456'".to_string()),
        }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.symbol, self.address)
    }
}

/// Extensible pool mechanics supporting different AMM types
/// Replaces optional fields with explicit pool type variants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PoolKind {
    /// Uniswap V2 / SushiSwap: Constant product AMM
    V2 {
        /// Reserve of token0 in the pool
        reserve0: U256,
        /// Reserve of token1 in the pool  
        reserve1: U256,
        /// Fee in basis points (e.g., 300 = 0.3%)
        fee_bps: u32,
    },

    /// Uniswap V3: Concentrated liquidity AMM
    V3 {
        /// Square root of current price * 2^96
        sqrt_price_x96: U256,
        /// Current active tick
        tick: i32,
        /// Fee in basis points (100, 500, 3000, 10000)
        fee_bps: u32,
        /// Current active liquidity in the pool
        liquidity: U256,
    },

    /// Curve Finance: Optimized for correlated assets
    Curve {
        /// Balances of all tokens in the pool
        balances: Vec<U256>,
        /// Amplification parameter (A)
        amplification: U256,
        /// Fee in basis points
        fee_bps: u32,
        /// Admin fee in basis points
        admin_fee_bps: u32,
    },

    /// Balancer V2: Weighted pools
    Balancer {
        /// Balances of all tokens in the pool  
        balances: Vec<U256>,
        /// Weights of each token (should sum to 1e18)
        weights: Vec<U256>,
        /// Swap fee in basis points
        swap_fee_bps: u32,
    },
}

impl PoolKind {
    /// Get the fee in basis points for this pool type
    pub fn fee_bps(&self) -> u32 {
        match self {
            PoolKind::V2 { fee_bps, .. } => *fee_bps,
            PoolKind::V3 { fee_bps, .. } => *fee_bps,
            PoolKind::Curve { fee_bps, .. } => *fee_bps,
            PoolKind::Balancer { swap_fee_bps, .. } => *swap_fee_bps,
        }
    }

    /// Check if this pool type supports size-aware quoting
    pub fn supports_quotes(&self) -> bool {
        match self {
            PoolKind::V2 { .. } => true,
            // Until we implement proper V3 size-aware quoting, return false to avoid misleading callers
            PoolKind::V3 { .. } => false,
            PoolKind::Curve { .. } => false, // Requires complex curve math
            PoolKind::Balancer { .. } => false, // Requires weighted math
        }
    }

    /// Get pool type name
    pub fn type_name(&self) -> &'static str {
        match self {
            PoolKind::V2 { .. } => "V2",
            PoolKind::V3 { .. } => "V3",
            PoolKind::Curve { .. } => "Curve",
            PoolKind::Balancer { .. } => "Balancer",
        }
    }
}

/// DEX pool with extensible mechanics and token ordering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pool {
    /// Pool contract address
    pub address: Address,
    /// Which DEX protocol this pool belongs to
    pub protocol: Protocol,
    /// First token in the pair (smaller address for deterministic ordering)
    pub token0: Token,
    /// Second token in the pair (larger address)
    pub token1: Token,
    /// Pool-specific mechanics and state
    pub kind: PoolKind,
}

impl Pool {
    /// Create a new pool with automatic token ordering
    pub fn new(
        address: Address,
        protocol: Protocol,
        token_a: Token,
        token_b: Token,
        kind: PoolKind,
    ) -> Self {
        // Ensure deterministic token ordering (token0 < token1)
        let (token0, token1) = if token_a.address < token_b.address {
            (token_a, token_b)
        } else {
            (token_b, token_a)
        };

        Self {
            address,
            protocol,
            token0,
            token1,
            kind,
        }
    }

    /// Get pool display name with fee information
    pub fn display_name(&self) -> String {
        let fee_display = match self.kind.fee_bps() {
            0 => String::new(),
            fee => format!(" ({:.2}%)", fee as f64 / 10000.0),
        };

        format!(
            "{}/{} on {}{}",
            self.token0.symbol, self.token1.symbol, self.protocol, fee_display
        )
    }

    /// Calculate mid-price for this pool (token1 per token0)
    pub fn calculate_mid_price(&self) -> Option<RawChainPrice> {
        match &self.kind {
            PoolKind::V2 {
                reserve0, reserve1, ..
            } => {
                if *reserve0 > U256::ZERO && *reserve1 > U256::ZERO {
                    Some(RawChainPrice::from_reserves(
                        *reserve1, // quote (token1) in raw units
                        *reserve0, // base (token0) in raw units
                    ))
                } else {
                    None
                }
            }
            PoolKind::V3 { sqrt_price_x96, .. } => {
                Some(RawChainPrice::from_sqrt_price_x96(*sqrt_price_x96))
            }
            PoolKind::Curve { .. } => {
                // Curve pricing requires complex math - simplified for now
                None
            }
            PoolKind::Balancer { .. } => {
                // Balancer pricing requires weighted calculations - simplified for now
                None
            }
        }
    }

    /// Calculate amount out for given amount in (simplified implementation)
    pub fn get_amount_out(&self, amount_in: U256, zero_for_one: bool) -> Option<U256> {
        match &self.kind {
            PoolKind::V2 {
                reserve0,
                reserve1,
                fee_bps,
            } => {
                let (reserve_in, reserve_out) = if zero_for_one {
                    (*reserve0, *reserve1)
                } else {
                    (*reserve1, *reserve0)
                };

                if reserve_in > U256::ZERO && reserve_out > U256::ZERO {
                    // Apply fee: amount_in_with_fee = amount_in * (10000 - fee_bps) / 10000
                    let fee_multiplier = U256::from(10000 - fee_bps);
                    let amount_in_with_fee = (amount_in * fee_multiplier) / U256::from(10000);

                    // Constant product: amount_out = (amount_in_with_fee * reserve_out) / (reserve_in + amount_in_with_fee)
                    let numerator = amount_in_with_fee * reserve_out;
                    let denominator = reserve_in + amount_in_with_fee;
                    Some(numerator / denominator)
                } else {
                    None
                }
            }
            PoolKind::V3 { .. } => {
                // V3 quotes are very complex - simplified for now
                None
            }
            _ => None,
        }
    }

    /// Create a PriceId for this pool
    pub fn create_price_id(&self, base_token: Address, quote_token: Address) -> PriceId {
        PriceId::new(
            base_token,
            quote_token,
            self.protocol,
            self.address,
            Some(self.kind.fee_bps()),
        )
    }
}

/// Block-scoped pool state snapshot with precision-safe pricing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolState {
    /// The pool this state belongs to
    pub pool: Pool,
    /// Block number when this state was captured
    pub block_number: u64,
    /// Block timestamp
    pub timestamp: u64,
}

impl PoolState {
    /// Create a new pool state snapshot
    pub fn new(pool: Pool, block_number: u64, timestamp: u64) -> Self {
        Self {
            pool,
            block_number,
            timestamp,
        }
    }

    /// Calculate mid-price from the pool's current state
    pub fn calculate_mid_price(&self) -> Option<RawChainPrice> {
        self.pool.calculate_mid_price()
    }

    /// Calculate amount out for a given input with fee consideration
    pub fn get_amount_out(&self, amount_in: U256, zero_for_one: bool) -> Option<U256> {
        self.pool.get_amount_out(amount_in, zero_for_one)
    }

    /// Create a price ID for this pool state
    pub fn create_price_id(&self, base_token: Address, quote_token: Address) -> PriceId {
        self.pool.create_price_id(base_token, quote_token)
    }
}

/// Enhanced price observation with unique identification and precision-safe pricing  
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceObservation {
    /// Unique identifier for this price source
    pub price_id: PriceId,
    /// Block number when price was observed  
    pub block_number: u64,
    /// Block timestamp (for freshness tracking)
    pub timestamp: u64,

    /// Precision-safe mid-price (base per quote)
    pub mid_price: RawChainPrice,
    /// Inverse mid-price (quote per base) for convenience
    pub inverse_mid_price: RawChainPrice,

    /// Execution pricing (includes fees)
    pub execution_fee_bps: u32,
    /// Whether this pool supports size-aware quoting
    pub supports_quotes: bool,

    /// Liquidity metrics for aggregation scoring
    pub liquidity_metrics: LiquidityMetrics,
}

impl PriceObservation {
    /// Create from pool state with automatic price ID generation
    pub fn from_pool_state(
        pool_state: &PoolState,
        base_token: Address,
        quote_token: Address,
    ) -> Option<Self> {
        let mid_price = pool_state.calculate_mid_price()?;
        let inverse_mid_price = mid_price.inverse();

        let price_id = pool_state.create_price_id(base_token, quote_token);

        // Calculate liquidity metrics based on pool type
        let liquidity_metrics = LiquidityMetrics::from_pool(&pool_state.pool);

        Some(Self {
            price_id,
            block_number: pool_state.block_number,
            timestamp: pool_state.timestamp,
            mid_price,
            inverse_mid_price,
            execution_fee_bps: pool_state.pool.kind.fee_bps(),
            supports_quotes: pool_state.pool.kind.supports_quotes(),
            liquidity_metrics,
        })
    }

    /// Get price in specified direction (base/quote or quote/base)
    pub fn get_price(&self, base_per_quote: bool) -> &RawChainPrice {
        if base_per_quote {
            &self.mid_price
        } else {
            &self.inverse_mid_price
        }
    }

    /// **⚠️ APPROXIMATION ONLY - DO NOT USE FOR ROUTING**
    ///
    /// Calculate APPROXIMATE execution price including fees for given size.
    ///
    /// **WARNING**: This is a naive approximation that:
    /// - Ignores pool curvature and size impact  
    /// - Ignores liquidity distribution (V3 ticks, Curve slippage)
    /// - Simply applies flat fee to mid-price
    /// - Will produce inaccurate results for large trades
    ///
    /// **Use Cases**:
    /// - ✅ Quick price estimates for UI display
    /// - ✅ Rough order-of-magnitude calculations  
    /// - ✅ Statistical analysis where precision isn't critical
    ///
    /// **Do NOT Use For**:
    /// - ❌ Trade routing decisions
    /// - ❌ Arbitrage calculations
    /// - ❌ Production trading systems
    /// - ❌ Risk management
    ///
    /// For accurate execution prices, use the actual pool's `get_amount_out` method
    /// or implement proper AMM mathematics for each pool type.
    pub fn get_execution_price_approximation(
        &self,
        _amount_in: U256,
        _zero_for_one: bool,
    ) -> Option<RawChainPrice> {
        if !self.supports_quotes {
            return None;
        }

        // APPROXIMATE calculation: applies flat fee to mid-price
        // Real implementation needs pool state and proper curve mathematics
        let fee_adjustment = U256::from(10000 - self.execution_fee_bps);
        let adjusted_numerator = (self.mid_price.numerator * fee_adjustment) / U256::from(10000);

        Some(RawChainPrice::new(
            adjusted_numerator,
            self.mid_price.denominator,
        ))
    }

    /// Get freshness score (1.0 = very fresh, 0.0 = stale)
    pub fn freshness_score(&self, current_timestamp: u64) -> f64 {
        let age_seconds = current_timestamp.saturating_sub(self.timestamp);
        match age_seconds {
            0..=60 => 1.0,     // Fresh (< 1 minute)
            61..=300 => 0.8,   // Recent (< 5 minutes)
            301..=900 => 0.6,  // Acceptable (< 15 minutes)
            901..=3600 => 0.4, // Stale (< 1 hour)
            _ => 0.2,          // Very stale (> 1 hour)
        }
    }

    /// Get a comprehensive description of this price observation
    pub fn describe(&self, base_symbol: &str, quote_symbol: &str) -> String {
        format!(
            "{}: {} {} per {} {} (fee: {:.2}%, liquidity: {})",
            self.price_id.describe(base_symbol, quote_symbol),
            self.mid_price,
            quote_symbol,
            base_symbol,
            if self.supports_quotes {
                "✓ quotes"
            } else {
                "✗ no quotes"
            },
            self.execution_fee_bps as f64 / 10000.0,
            self.liquidity_metrics.display_summary()
        )
    }

    /// Check if this observation is suitable for aggregation (valid price, reasonable freshness)
    pub fn is_suitable_for_aggregation(&self, current_timestamp: u64) -> bool {
        self.mid_price.is_valid()
            && self.freshness_score(current_timestamp) > 0.3
            && self.liquidity_metrics.is_reasonable()
    }
}

mod liquidity_metrics;
pub use liquidity_metrics::LiquidityMetrics;

#[cfg(test)]
mod tests;
