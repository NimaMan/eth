use alloy_primitives::U256;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

pub type DecimalAmount = Decimal;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Amount {
    pub raw: U256,
    pub decimals: u8,
}

impl Amount {
    pub const fn zero(decimals: u8) -> Self {
        Self {
            raw: U256::ZERO,
            decimals,
        }
    }

    /// Convert raw amount to decimal value.
    /// E.g. `Amount { raw: 1_000_000_000_000_000_000, decimals: 18 }` → `1.0`
    pub fn to_decimal(&self) -> DecimalAmount {
        let s = self.raw.to_string();
        let mut dec = DecimalAmount::from_str_exact(&s).unwrap_or_default();
        if self.decimals > 0 {
            let divisor = DecimalAmount::from(10i64.pow(self.decimals as u32));
            dec = dec / divisor;
        }
        dec
    }

    /// Convert a decimal value to raw amount.
    /// E.g. `DecimalAmount::from_str_exact("1.0").unwrap()` with `decimals = 18` →
    /// `Amount { raw: 1_000_000_000_000_000_000, decimals: 18 }`
    pub fn from_decimal(dec: DecimalAmount, decimals: u8) -> Self {
        let scaled = (dec * DecimalAmount::from(10i64.pow(decimals as u32))).normalize();
        let raw = U256::from_str_radix(&scaled.to_string(), 10).unwrap_or(U256::ZERO);
        Self { raw, decimals }
    }
}
